//! Bounded plain-or-zstd JSONL reading for ordinary rollouts.

use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

pub(super) struct BoundedLine {
    pub(super) bytes: Vec<u8>,
    pub(super) oversized: bool,
}

pub(super) struct BoundedRolloutReader {
    reader: Option<BufReader<Box<dyn Read + Send>>>,
}

impl BoundedRolloutReader {
    pub(super) async fn open(path: &Path) -> std::io::Result<Self> {
        let path = path.to_path_buf();
        let reader = tokio::task::spawn_blocking(move || open_blocking(path))
            .await
            .map_err(std::io::Error::other)??;
        Ok(Self {
            reader: Some(reader),
        })
    }

    pub(super) async fn next_line(&mut self, limit: usize) -> std::io::Result<Option<BoundedLine>> {
        let Some(mut reader) = self.reader.take() else {
            return Err(std::io::Error::other("rollout reader is busy"));
        };
        let (result, reader) =
            tokio::task::spawn_blocking(move || (read_bounded_line(&mut reader, limit), reader))
                .await
                .map_err(std::io::Error::other)?;
        self.reader = Some(reader);
        result
    }
}

fn open_blocking(path: PathBuf) -> std::io::Result<BufReader<Box<dyn Read + Send>>> {
    let file = File::open(&path)?;
    let reader: Box<dyn Read + Send> =
        if path.extension().is_some_and(|extension| extension == "zst") {
            Box::new(zstd::stream::read::Decoder::new(file)?)
        } else {
            Box::new(file)
        };
    Ok(BufReader::new(reader))
}

pub(super) fn read_bounded_line(
    reader: &mut impl BufRead,
    limit: usize,
) -> std::io::Result<Option<BoundedLine>> {
    let mut bytes = Vec::with_capacity(limit.saturating_add(1).min(64 * 1024));
    let mut oversized = false;
    let mut saw_bytes = false;
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            break;
        }
        saw_bytes = true;
        let consumed = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(available.len(), |index| index + 1);
        let chunk = &available[..consumed];
        let without_newline = chunk.strip_suffix(b"\n").unwrap_or(chunk);
        let remaining = limit.saturating_sub(bytes.len());
        bytes.extend_from_slice(&without_newline[..without_newline.len().min(remaining)]);
        oversized |= without_newline.len() > remaining;
        let ends_with_newline = chunk.ends_with(b"\n");
        reader.consume(consumed);
        if ends_with_newline {
            break;
        }
    }
    if !saw_bytes {
        return Ok(None);
    }
    if bytes.last() == Some(&b'\r') {
        bytes.pop();
    }
    Ok(Some(BoundedLine { bytes, oversized }))
}
