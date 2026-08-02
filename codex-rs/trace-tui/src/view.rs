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

/// Policy controlling line layout for detail and structured scalar content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentLayout {
    /// Wrap content to the current terminal width and disable horizontal movement.
    Wrapped,
    /// Preserve bounded logical lines and expose them through a horizontal viewport.
    Unwrapped,
}

/// Policy controlling whether configured listing columns may be omitted responsively.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnLayout {
    /// Keep every configured column on the shared horizontal canvas.
    Stable,
    /// Omit lower-priority columns until the metadata prefix fits the viewport.
    Adaptive,
}

/// Validated number of display cells moved by one small horizontal action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HorizontalStep(u16);

impl HorizontalStep {
    /// Largest accepted small movement, preventing nonsensical configuration values.
    pub const MAX: u16 = 256;

    /// Creates a non-zero horizontal step within the supported bound.
    pub const fn new(columns: u16) -> Option<Self> {
        if columns == 0 || columns > Self::MAX {
            None
        } else {
            Some(Self(columns))
        }
    }

    /// Returns the configured movement in terminal display cells.
    pub const fn columns(self) -> usize {
        self.0 as usize
    }
}

impl Default for HorizontalStep {
    fn default() -> Self {
        Self(4)
    }
}

/// Validated maximum width of one logical horizontally scrollable line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogicalContentWidth(u16);

impl LogicalContentWidth {
    /// Hard safety bound for one logical line.
    pub const MAX: u16 = 4_096;

    /// Creates a non-zero logical width within the terminal rendering bound.
    pub const fn new(columns: u16) -> Option<Self> {
        if columns == 0 || columns > Self::MAX {
            None
        } else {
            Some(Self(columns))
        }
    }

    /// Returns the logical-line limit in terminal display cells.
    pub const fn columns(self) -> usize {
        self.0 as usize
    }
}

impl Default for LogicalContentWidth {
    fn default() -> Self {
        Self(Self::MAX)
    }
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
    /// Detail and structured-scalar line layout.
    pub content_layout: ContentLayout,
    /// Responsive or stable listing-column policy.
    pub column_layout: ColumnLayout,
    /// Display-cell movement used by Left, Right, `h`, and `l`.
    pub horizontal_step: HorizontalStep,
    /// Hard display-width limit for one logical line.
    pub max_content_width: LogicalContentWidth,
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
            content_layout: ContentLayout::Unwrapped,
            column_layout: ColumnLayout::Stable,
            horizontal_step: HorizontalStep::default(),
            max_content_width: LogicalContentWidth::default(),
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

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
