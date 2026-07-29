//! Data-driven overview column selection and row formatting.

use codex_trace::TraceNode;
use ratatui::text::Line;

use super::class_name;
use super::evidence_name;
use super::kind_tag;
use super::source_name;
use super::status_name;
use super::text::fit;
use super::text::pad_fit;
use super::text::single_line;
use crate::PreviewMode;
use crate::TraceColumn;

/// Width-dependent columns and elastic name/preview allocation.
pub(super) struct ColumnPlan {
    columns: Vec<TraceColumn>,
    name_width: usize,
    preview_width: Option<usize>,
    omitted: usize,
}

impl ColumnPlan {
    /// Selects configured columns and allocates one complete overview row.
    pub(super) fn new(requested: &[TraceColumn], preview_mode: PreviewMode, width: usize) -> Self {
        let mut columns = requested.to_vec();
        while required_width(&columns) > width {
            let Some((index, _)) = columns
                .iter()
                .enumerate()
                .max_by_key(|(_, column)| column_spec(**column).drop_priority)
            else {
                break;
            };
            columns.remove(index);
        }
        let fixed = fixed_width(&columns);
        let allow_preview = match preview_mode {
            PreviewMode::Auto => width.saturating_sub(fixed + 26) >= 32,
            PreviewMode::Always => width.saturating_sub(fixed + 18) >= 12,
            PreviewMode::Never => false,
        };
        let preview_width = allow_preview.then_some(width.saturating_sub(fixed + 26));
        let name_width = width
            .saturating_sub(fixed + preview_width.unwrap_or_default() + 2)
            .max(8);
        let omitted = requested.len().saturating_sub(columns.len());
        Self {
            columns,
            name_width,
            preview_width,
            omitted,
        }
    }

    /// Returns the number of configured columns omitted at this width.
    pub(super) fn omitted(&self) -> usize {
        self.omitted
    }

    /// Formats headers or one semantic record through the same layout.
    pub(super) fn row(
        &self,
        selected: bool,
        label: &str,
        node: Option<&TraceNode>,
        preview: Option<&str>,
    ) -> Line<'static> {
        let marker = if selected { "▶ " } else { "  " };
        let label = single_line(label);
        let mut text = format!(
            "{marker}{}",
            pad_fit(&label, self.name_width.saturating_sub(2))
        );
        for column in &self.columns {
            let spec = column_spec(*column);
            let value = node
                .map(|node| column_value(*column, node))
                .unwrap_or_else(|| spec.header.to_string());
            text.push(' ');
            text.push_str(&pad_fit(&single_line(&value), spec.width));
        }
        if let Some(preview_width) = self.preview_width {
            text.push(' ');
            text.push_str(&fit(
                &single_line(preview.unwrap_or_default()),
                preview_width,
            ));
        }
        Line::from(text)
    }
}

/// Static display policy for one configurable trace column.
struct ColumnSpec {
    header: &'static str,
    width: usize,
    drop_priority: u8,
}

/// Returns all display policy owned by one column.
fn column_spec(column: TraceColumn) -> ColumnSpec {
    match column {
        TraceColumn::Kind => ColumnSpec {
            header: "KIND",
            width: 4,
            drop_priority: 2,
        },
        TraceColumn::Class => ColumnSpec {
            header: "CLASS",
            width: 12,
            drop_priority: 1,
        },
        TraceColumn::Status => ColumnSpec {
            header: "STATUS",
            width: 9,
            drop_priority: 5,
        },
        TraceColumn::Timestamp => ColumnSpec {
            header: "TIME",
            width: 20,
            drop_priority: 6,
        },
        TraceColumn::Source => ColumnSpec {
            header: "SOURCE",
            width: 8,
            drop_priority: 3,
        },
        TraceColumn::Evidence => ColumnSpec {
            header: "EVIDENCE",
            width: 13,
            drop_priority: 4,
        },
        TraceColumn::Identifier => ColumnSpec {
            header: "ID",
            width: 16,
            drop_priority: 7,
        },
    }
}

/// Returns the fixed width occupied by metadata columns and separators.
fn fixed_width(columns: &[TraceColumn]) -> usize {
    columns
        .iter()
        .map(|column| column_spec(*column).width + 1)
        .sum()
}

/// Returns the minimum width before another metadata column must be dropped.
fn required_width(columns: &[TraceColumn]) -> usize {
    14 + fixed_width(columns)
}

/// Extracts one label-free metadata value from a trace node.
fn column_value(column: TraceColumn, node: &TraceNode) -> String {
    match column {
        TraceColumn::Kind => kind_tag(node.locator.kind).to_string(),
        TraceColumn::Class => class_name(node.presentation.class).to_string(),
        TraceColumn::Status => node
            .presentation
            .status
            .map_or_else(|| "—".to_string(), |status| status_name(status).to_string()),
        TraceColumn::Timestamp => node.timestamp.clone().unwrap_or_else(|| "—".to_string()),
        TraceColumn::Source => source_name(node.provenance).to_string(),
        TraceColumn::Evidence => evidence_name(node.evidence).to_string(),
        TraceColumn::Identifier => node.locator.id.clone(),
    }
}

#[cfg(test)]
#[path = "table_tests.rs"]
mod tests;
