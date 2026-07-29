//! Structured detail rendering and lazy raw-payload state.

use std::sync::Arc;

use anyhow::Result;
use codex_trace::RawPayloadHandle;
use codex_trace::SanitizedPayload;
use codex_trace::TraceNodeKind;
use ratatui::text::Line;

use super::BrowserState;
use super::DetailCache;
use super::PayloadState;
use crate::ContentMode;
use crate::jobs::DetailRenderJob;
use crate::jobs::DetailRenderKey;
use crate::jobs::DetailRenderResult;
use crate::request::BrowserEpoch;

impl BrowserState {
    /// Opens the selected leaf in full-screen semantic detail mode.
    pub(crate) fn open_detail(&mut self) {
        if self.selected_node().is_some() {
            self.detail_open = true;
            self.detail_scroll = 0;
            self.content_mode = ContentMode::Rendered;
            self.invalidate_detail();
        }
    }

    /// Selects the next semantic/raw representation for the current detail.
    pub(crate) fn cycle_content_mode(&mut self) {
        self.content_mode = self.content_mode.next();
        self.detail_scroll = 0;
        self.invalidate_detail();
    }

    /// Moves the bounded detail viewport by a signed line count.
    pub(crate) fn scroll_detail(&mut self, delta: isize) {
        self.detail_scroll = self.detail_scroll.saturating_add_signed(delta);
    }

    /// Returns cached lines or queues a generation-tagged render for this width.
    pub(crate) fn detail_lines(&mut self, width: usize) -> &[Line<'static>] {
        let Some(node) = self.selected_node().cloned() else {
            return &[];
        };
        let key = DetailRenderKey {
            locator: node.locator.clone(),
            width,
            mode: self.content_mode,
            payload_generation: self.payload_generation,
        };
        if self
            .detail_cache
            .as_ref()
            .is_some_and(|cache| cache.key == key)
        {
            return self
                .detail_cache
                .as_ref()
                .map(|cache| cache.lines.as_slice())
                .unwrap_or_default();
        }
        if self
            .detail_requests
            .pending_key()
            .is_none_or(|pending| *pending != key)
        {
            self.detail_cache = None;
            let trace = Arc::clone(&self.trace);
            let index = Arc::clone(&self.index);
            let payload = self.payloads.get(&node.locator.id).and_then(|state| {
                if let PayloadState::Loaded(payload) = state {
                    Some(Arc::clone(payload))
                } else {
                    None
                }
            });
            let cwd = self.trace.summary.cwd.clone();
            let renderer = Arc::clone(&self.renderer);
            self.detail_requests
                .submit(key.clone(), move |token| DetailRenderJob {
                    key,
                    token,
                    trace,
                    index,
                    payload,
                    cwd,
                    renderer,
                });
        }
        &[]
    }

    /// Returns whether the selected node represents a lazy raw artifact.
    pub(crate) fn selected_is_raw_payload(&self) -> bool {
        self.selected_node()
            .is_some_and(|node| node.locator.kind == TraceNodeKind::RawPayload)
    }

    /// Returns whether the cached semantic document was byte-truncated.
    pub(crate) fn detail_truncated(&self) -> bool {
        self.detail_cache
            .as_ref()
            .is_some_and(|cache| cache.truncated)
    }

    /// Takes the next queued detail-render job.
    pub(crate) fn take_detail_render_job(&mut self) -> Option<DetailRenderJob> {
        self.detail_requests.take_job()
    }

    /// Installs a detail result only when its browser, generation, and view key are current.
    pub(crate) fn install_detail_render(&mut self, result: DetailRenderResult) {
        if !self.detail_requests.accept(result.token, &result.key) {
            return;
        }
        let is_current = self
            .selected_node()
            .is_some_and(|node| node.locator == result.key.locator)
            && self.content_mode == result.key.mode
            && self.payload_generation == result.key.payload_generation;
        if !is_current {
            return;
        }
        self.detail_cache = Some(DetailCache {
            key: result.key,
            truncated: result.truncated,
            lines: result.lines,
        });
    }

    /// Marks the selected payload as loading and returns its lazy read handle.
    pub(crate) fn selected_payload_request(
        &mut self,
    ) -> Option<(BrowserEpoch, String, RawPayloadHandle)> {
        let node = self.selected_node()?;
        if node.locator.kind != TraceNodeKind::RawPayload {
            return None;
        }
        let id = node.locator.id.clone();
        if self.payloads.contains_key(&id) {
            return None;
        }
        let handle = self.trace.raw_payload(&id)?;
        self.payloads.insert(id.clone(), PayloadState::Loading);
        self.payload_generation = self.payload_generation.wrapping_add(1);
        self.invalidate_detail();
        Some((self.epoch, id, handle))
    }

    /// Installs a completed payload read and invalidates dependent render state.
    pub(crate) fn install_payload(
        &mut self,
        browser_epoch: BrowserEpoch,
        id: String,
        result: Result<SanitizedPayload>,
    ) {
        if self.epoch != browser_epoch {
            return;
        }
        let state = match result {
            Ok(payload) => PayloadState::Loaded(Arc::new(payload)),
            Err(error) => PayloadState::Failed(format!("{error:#}")),
        };
        self.payloads.insert(id, state);
        self.payload_generation = self.payload_generation.wrapping_add(1);
        self.invalidate_detail();
    }

    /// Returns the selected payload's bounded loading or failure notice.
    pub(crate) fn payload_notice(&self) -> Option<&str> {
        let node = self.selected_node()?;
        match self.payloads.get(&node.locator.id) {
            Some(PayloadState::Loading) => Some("loading exact raw artifact…"),
            Some(PayloadState::Failed(message)) => Some(message),
            Some(PayloadState::Loaded(_)) | None => None,
        }
    }

    /// Invalidates all cached and queued detail work.
    pub(super) fn invalidate_detail(&mut self) {
        self.detail_cache = None;
        self.detail_requests.invalidate();
    }
}
