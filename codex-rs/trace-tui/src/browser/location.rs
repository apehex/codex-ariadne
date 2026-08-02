//! Stable browser locations and reversible list frames.

use codex_trace::GroupId;
use codex_trace::PresentationScope;
use codex_trace::TraceNodeLocator;

use crate::ContentMode;
use crate::TraceLens;

use super::horizontal::HorizontalState;
use super::rows::BrowserRow;
use super::rows::BrowserRowId;
use super::structured::JsonPath;

/// One explicit surface in the single-depth trace browser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum BrowserLocation {
    /// A collapsed, expanded, or structural presentation of one scope.
    Lens {
        lens: TraceLens,
        scope: PresentationScope,
        structural_container: Option<TraceNodeLocator>,
    },
    /// Direct evidence, child groups, and references for one group.
    Group(GroupId),
    /// Full-screen semantic or raw detail for one canonical node.
    Detail(TraceNodeLocator),
    /// One browser-local location inside normalized JSON detail.
    Structured {
        locator: TraceNodeLocator,
        path: JsonPath,
    },
}

/// Exact list state restored after one reversible descent.
#[derive(Debug, Clone)]
pub(super) struct NavigationFrame {
    pub(super) location: BrowserLocation,
    pub(super) selected: Option<BrowserRowId>,
    pub(super) viewport: usize,
    pub(super) detail_scroll: usize,
    pub(super) content_mode: ContentMode,
    pub(super) horizontal: HorizontalState,
    pub(super) rows: Vec<BrowserRow>,
    pub(super) hidden_rows: usize,
    pub(super) selected_index: usize,
    pub(super) visibility_generation: u64,
}
