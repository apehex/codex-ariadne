//! Typed current-level rows derived from immutable trace indexes.

use codex_trace::EvidenceGrade;
use codex_trace::GroupId;
use codex_trace::GroupMemberRole;
use codex_trace::TraceNodeLocator;
use codex_trace::TraceObjectRef;
use codex_trace::TraceRecordClass;
use codex_trace::TraceRelation;
use codex_trace::TraceStatus;

use super::structured::JsonPathComponent;

/// Stable identity used to restore selection independently of row position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum BrowserRowId {
    Group(GroupId),
    Node(TraceNodeLocator),
    Reference(TraceRelation, TraceObjectRef),
    Structured(JsonPathComponent),
}

/// One selectable row in the current browser location.
#[derive(Debug, Clone)]
pub(crate) enum BrowserRow {
    Group(GroupId),
    Event {
        node_position: usize,
        group: GroupId,
        band: codex_trace::OrderBandId,
    },
    Structural(usize),
    Member {
        node_position: usize,
        role: GroupMemberRole,
    },
    ChildGroup(GroupId),
    Reference {
        relation: TraceRelation,
        target: TraceObjectRef,
        node_position: Option<usize>,
    },
    Structured {
        component: JsonPathComponent,
        kind: &'static str,
        preview: String,
    },
    Notice(String),
}

/// Owned display facts prepared only for a visible browser row.
pub(crate) struct BrowserRowDisplay {
    pub(crate) label: String,
    pub(crate) preview: Option<String>,
    pub(crate) kind: String,
    pub(crate) class: TraceRecordClass,
    pub(crate) class_label: String,
    pub(crate) status: Option<TraceStatus>,
    pub(crate) timestamp: String,
    pub(crate) source: String,
    pub(crate) evidence: EvidenceGrade,
    pub(crate) evidence_label: String,
    pub(crate) identifier: String,
}

impl BrowserRow {
    pub(super) fn id(&self, trace: &codex_trace::SessionTrace) -> Option<BrowserRowId> {
        match self {
            Self::Group(id) | Self::ChildGroup(id) => Some(BrowserRowId::Group(id.clone())),
            Self::Event { node_position, .. }
            | Self::Structural(node_position)
            | Self::Member { node_position, .. } => trace
                .nodes
                .get(*node_position)
                .map(|node| BrowserRowId::Node(node.locator.clone())),
            Self::Reference {
                relation, target, ..
            } => Some(BrowserRowId::Reference(*relation, target.clone())),
            Self::Structured { component, .. } => Some(BrowserRowId::Structured(component.clone())),
            Self::Notice(_) => None,
        }
    }
}
