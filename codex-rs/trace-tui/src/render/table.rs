//! Data-driven listing canvas selection and row formatting.

use ratatui::text::Line;

use super::status_name;
use super::text::pad_fit;
use super::text::single_line;
use crate::ColumnLayout;
use crate::PreviewMode;
use crate::TraceColumn;
use crate::browser::rows::BrowserRowDisplay;

const STABLE_NAME_WIDTH: usize = 24;
const ADAPTIVE_NAME_WIDTH: usize = 14;
const PREVIEW_WIDTH: usize = 256;

/// Width-dependent columns and one stable logical canvas.
pub(super) struct ColumnPlan {
    columns: Vec<TraceColumn>,
    name_width: usize,
    preview_width: Option<usize>,
    omitted: usize,
    canvas_width: usize,
}

impl ColumnPlan {
    /// Selects configured columns without inspecting any trace row.
    pub(super) fn new(
        requested: &[TraceColumn],
        preview_mode: PreviewMode,
        column_layout: ColumnLayout,
        viewport_width: usize,
        maximum_width: usize,
    ) -> Self {
        let mut columns = requested.to_vec();
        let minimum_name = match column_layout {
            ColumnLayout::Stable => STABLE_NAME_WIDTH,
            ColumnLayout::Adaptive => ADAPTIVE_NAME_WIDTH,
        };
        if column_layout == ColumnLayout::Adaptive {
            while minimum_name.saturating_add(fixed_width(&columns)) > viewport_width {
                let Some((index, _)) = columns
                    .iter()
                    .enumerate()
                    .max_by_key(|(_, column)| column_spec(**column).drop_priority)
                else {
                    break;
                };
                columns.remove(index);
            }
        }
        let omitted = requested.len().saturating_sub(columns.len());
        let fixed = fixed_width(&columns);
        let preview_threshold = match preview_mode {
            PreviewMode::Auto => 32,
            PreviewMode::Always => 1,
            PreviewMode::Never => usize::MAX,
        };
        let preview_width = (viewport_width.saturating_sub(fixed + minimum_name)
            >= preview_threshold)
            .then_some(PREVIEW_WIDTH.min(maximum_width));
        let initial_preview = preview_width.map_or(0, |_| preview_threshold.saturating_add(1));
        let name_width = viewport_width
            .saturating_sub(fixed + initial_preview)
            .max(minimum_name);
        let canvas_width = name_width
            .saturating_add(fixed)
            .saturating_add(preview_width.map_or(0, |width| width.saturating_add(1)))
            .min(maximum_width);
        Self {
            columns,
            name_width,
            preview_width,
            omitted,
            canvas_width,
        }
    }

    /// Returns the number of configured columns omitted at this width.
    pub(super) fn omitted(&self) -> usize {
        self.omitted
    }

    /// Returns the schema-derived logical canvas width.
    pub(super) fn canvas_width(&self) -> usize {
        self.canvas_width
    }

    /// Formats headers or one semantic row on the common logical canvas.
    pub(super) fn row(
        &self,
        label: &str,
        row: Option<&BrowserRowDisplay>,
        preview: Option<&str>,
    ) -> Line<'static> {
        let mut text = pad_fit(&single_line(label), self.name_width);
        for column in &self.columns {
            let spec = column_spec(*column);
            let value = row
                .map(|row| column_value(*column, row))
                .unwrap_or_else(|| spec.header.to_string());
            text.push(' ');
            text.push_str(&pad_fit(&single_line(&value), spec.width));
        }
        if let Some(preview_width) = self.preview_width {
            text.push(' ');
            text.push_str(&pad_fit(
                &single_line(preview.unwrap_or_default()),
                preview_width,
            ));
        }
        Line::from(text)
    }
}

struct ColumnSpec {
    header: &'static str,
    width: usize,
    drop_priority: u8,
}

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

fn fixed_width(columns: &[TraceColumn]) -> usize {
    columns
        .iter()
        .map(|column| column_spec(*column).width + 1)
        .sum()
}

fn column_value(column: TraceColumn, row: &BrowserRowDisplay) -> String {
    match column {
        TraceColumn::Kind => row.kind.clone(),
        TraceColumn::Class => row.class_label.clone(),
        TraceColumn::Status => row
            .status
            .map_or_else(|| "—".to_string(), |status| status_name(status).to_string()),
        TraceColumn::Timestamp => row.timestamp.clone(),
        TraceColumn::Source => row.source.clone(),
        TraceColumn::Evidence => row.evidence_label.clone(),
        TraceColumn::Identifier => row.identifier.clone(),
    }
}

#[cfg(test)]
#[path = "table_tests.rs"]
mod tests;
