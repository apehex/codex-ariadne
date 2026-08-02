//! Typed listing configuration and the host-provided visual rendering contract.

use std::path::Path;

use codex_trace::EvidenceGrade;
use codex_trace::TraceContentDocument;
use codex_trace::TraceRecordClass;
use codex_trace::TraceStatus;
use ratatui::style::Style;
use ratatui::text::Line;

/// Metadata columns that may appear to the right of a trace record label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceColumn {
    /// Normalized node kind.
    Kind,
    /// Semantic record class.
    Class,
    /// Runtime or completion status.
    Status,
    /// Source timestamp.
    Timestamp,
    /// Ordinary, rich, or merged provenance.
    Source,
    /// Evidence strength.
    Evidence,
    /// Stable node identifier.
    Identifier,
}

/// Policy controlling whether listing column headers are rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderMode {
    /// Show headers when the viewport has enough room.
    Auto,
    /// Always reserve a header row.
    Always,
    /// Never render column headers.
    Never,
}

/// Policy controlling whether spare listing width is used for content previews.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewMode {
    /// Show previews only when a useful amount of width remains.
    Auto,
    /// Show previews whenever any practical width remains.
    Always,
    /// Never render content previews in listings.
    Never,
}

/// Renderer-independent browser projection selected for one trace scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TraceLens {
    /// One row per top-level presentation group.
    Collapsed,
    /// One row per canonical primary event with group annotations.
    Expanded,
    /// Canonical containment through the normalized trace graph.
    Structural,
}

impl TraceLens {
    /// Advances through the stable collapsed, expanded, and structural cycle.
    pub fn next(self) -> Self {
        match self {
            Self::Collapsed => Self::Expanded,
            Self::Expanded => Self::Structural,
            Self::Structural => Self::Collapsed,
        }
    }

    /// Moves backward through the stable lens cycle.
    pub fn previous(self) -> Self {
        match self {
            Self::Collapsed => Self::Structural,
            Self::Expanded => Self::Collapsed,
            Self::Structural => Self::Expanded,
        }
    }
}

/// Initial presentation scope requested by a browser host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceStartScope {
    /// Select the first structurally rooted thread that owns presentable events.
    RootThread,
    /// Select presentation events without a recorded thread owner.
    Session,
    /// Select one source-authored thread identity.
    Thread(String),
}

/// Explicit view settings passed through trace row preparation and rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceViewOptions {
    /// Ordered metadata columns requested by the caller.
    pub columns: Vec<TraceColumn>,
    /// Column-header display policy.
    pub headers: HeaderMode,
    /// Inline content-preview display policy.
    pub preview: PreviewMode,
    /// Lens installed when a selected session finishes loading.
    pub initial_lens: TraceLens,
    /// Presentation scope installed when a selected session finishes loading.
    pub initial_scope: TraceStartScope,
}

impl Default for TraceViewOptions {
    fn default() -> Self {
        Self {
            columns: vec![
                TraceColumn::Kind,
                TraceColumn::Class,
                TraceColumn::Status,
                TraceColumn::Timestamp,
                TraceColumn::Source,
                TraceColumn::Evidence,
            ],
            headers: HeaderMode::Auto,
            preview: PreviewMode::Auto,
            initial_lens: TraceLens::Collapsed,
            initial_scope: TraceStartScope::RootThread,
        }
    }
}

/// Full-screen representation selected for one trace record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentMode {
    /// Semantically render Markdown, JSON, and code.
    Rendered,
    /// Display extracted semantic content as plain text.
    Text,
    /// Display exact raw artifacts when available, otherwise labeled normalized JSON.
    Raw,
}

impl ContentMode {
    /// Advances through the stable Rendered, Text, and Raw cycle.
    pub fn next(self) -> Self {
        match self {
            Self::Rendered => Self::Text,
            Self::Text => Self::Raw,
            Self::Raw => Self::Rendered,
        }
    }
}

/// Bounded request passed to the host TUI's semantic content renderer.
pub struct TraceRenderRequest<'a> {
    /// Bounded semantic document to render.
    pub document: &'a TraceContentDocument,
    /// Available content width in terminal columns.
    pub width: usize,
    /// Session working directory used to resolve local Markdown links.
    pub cwd: Option<&'a Path>,
}

/// Semantic and interaction facts used to style every listing record.
#[derive(Debug, Clone, Copy)]
pub struct TraceRowStyleRequest {
    /// Semantic class of the row.
    pub class: TraceRecordClass,
    /// Runtime or completion status when known.
    pub status: Option<TraceStatus>,
    /// Evidence strength represented by the row.
    pub evidence: EvidenceGrade,
    /// Whether the row owns the current selection.
    pub selected: bool,
}

/// Host-provided visual language for trace rows and rich record content.
///
/// Implementations must treat source content as inert terminal data and must
/// return owned lines that remain valid after a background render job ends.
pub trait TraceVisualRenderer: Send + Sync {
    /// Produces owned display lines for one bounded detail document.
    fn render_content(&self, request: TraceRenderRequest<'_>) -> Vec<Line<'static>>;

    /// Selects a terminal-aware style for one listing row.
    fn row_style(&self, request: TraceRowStyleRequest) -> Style;
}

/// Deterministic plain renderer used when no parent-TUI visual adapter is supplied.
#[derive(Debug, Default)]
pub struct PlainTraceVisualRenderer;

impl TraceVisualRenderer for PlainTraceVisualRenderer {
    fn render_content(&self, request: TraceRenderRequest<'_>) -> Vec<Line<'static>> {
        let width = request.width.max(1);
        request
            .document
            .text
            .split('\n')
            .flat_map(|line| {
                let wrapped = textwrap::wrap(line, width);
                if wrapped.is_empty() {
                    vec![Line::from("")]
                } else {
                    wrapped
                        .into_iter()
                        .map(|line| Line::from(line.into_owned()))
                        .collect()
                }
            })
            .collect()
    }

    fn row_style(&self, _request: TraceRowStyleRequest) -> Style {
        Style::default()
    }
}
