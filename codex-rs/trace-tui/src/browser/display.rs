//! Breadcrumb and viewport-row presentation derived from typed browser state.

use std::collections::BTreeSet;

use codex_trace::EvidenceGrade;
use codex_trace::GroupAggregate;
use codex_trace::GroupEvidenceCounts;
use codex_trace::GroupId;
use codex_trace::GroupKind;
use codex_trace::PresentationScope;
use codex_trace::TraceNode;
use codex_trace::TraceNodeLocator;
use codex_trace::TraceObjectRef;
use codex_trace::TraceRecordClass;
use codex_trace::TraceSourceKind;

use crate::TraceLens;

use super::BrowserState;
use super::location::BrowserLocation;
use super::rows::BrowserRow;
use super::rows::BrowserRowDisplay;
use super::structured::JsonPath;
use super::structured::JsonPathComponent;

impl BrowserState {
    pub(crate) fn breadcrumb(&self) -> Vec<String> {
        let mut labels = vec![
            scope_label(&self.active_scope),
            lens_name(self.active_lens).into(),
        ];
        match &self.location {
            BrowserLocation::Lens {
                lens: TraceLens::Structural,
                structural_container,
                ..
            } => {
                labels = self.structural_breadcrumb(structural_container.as_ref());
                labels.push("structural".to_string());
            }
            BrowserLocation::Group(id) => labels.push(self.presentation.group(id).map_or_else(
                || "group".to_string(),
                |group| group_kind(group.kind).into(),
            )),
            BrowserLocation::Detail(locator) => labels.push(
                self.index
                    .node(&self.trace, locator)
                    .map_or_else(|| "detail".to_string(), |node| node.label.clone()),
            ),
            BrowserLocation::Structured { locator, path } => {
                if let Some(node) = self.index.node(&self.trace, locator) {
                    labels.push(node.label.clone());
                }
                labels.push(format!("json {}", display_pointer(path)));
            }
            BrowserLocation::Lens {
                lens: TraceLens::Collapsed | TraceLens::Expanded,
                ..
            } => {}
        }
        labels
    }

    pub(crate) fn row_display(&self, row: &BrowserRow) -> BrowserRowDisplay {
        match row {
            BrowserRow::Group(id) | BrowserRow::ChildGroup(id) => self.group_display(id),
            BrowserRow::Event {
                node_position,
                group,
                band,
            } => {
                let group = self
                    .presentation
                    .group(group)
                    .map_or("group".to_string(), |group| {
                        format!("group {}", group_kind(group.kind))
                    });
                let band = self
                    .presentation
                    .band(*band)
                    .map_or("unpositioned".to_string(), |band| {
                        format!("{:?}", band.kind).to_lowercase()
                    });
                self.node_display(*node_position, None, Some(format!("{group} · {band}")))
            }
            BrowserRow::Structural(position) => self.node_display(*position, None, None),
            BrowserRow::Member {
                node_position,
                role,
            } => self.node_display(*node_position, None, Some(format!("{role:?}"))),
            BrowserRow::Reference {
                relation,
                target,
                node_position,
            } => self.reference_display(*relation, target, *node_position),
            BrowserRow::Structured {
                component,
                kind,
                preview,
            } => structured_display(component, kind, preview),
            BrowserRow::Notice(message) => unavailable_display(message),
        }
    }

    pub(crate) fn row_node<'a>(&'a self, row: &'a BrowserRow) -> Option<&'a TraceNode> {
        self.node_for_row(row)
    }

    fn structural_breadcrumb(&self, locator: Option<&TraceNodeLocator>) -> Vec<String> {
        let mut labels = Vec::new();
        let mut current = locator;
        let mut visited = BTreeSet::new();
        while let Some(locator) = current {
            if !visited.insert(locator.clone()) {
                break;
            }
            let Some(node) = self.index.node(&self.trace, locator) else {
                break;
            };
            labels.push(node.label.clone());
            current = node.parent.as_ref();
        }
        labels.reverse();
        labels
    }

    fn group_display(&self, id: &GroupId) -> BrowserRowDisplay {
        let Some(group) = self.presentation.group(id) else {
            return unavailable_display("missing group");
        };
        let evidence = aggregate_evidence(group.metadata.evidence);
        let status = match group.metadata.status {
            GroupAggregate::Value(status) => Some(status),
            GroupAggregate::Unavailable | GroupAggregate::Conflicting => None,
        };
        BrowserRowDisplay {
            label: group
                .metadata
                .label
                .clone()
                .unwrap_or_else(|| group_kind(group.kind).to_string()),
            preview: group.metadata.preview.clone(),
            kind: group_kind(group.kind).to_string(),
            class: group_class(group.kind),
            class_label: class_name(group_class(group.kind)).to_string(),
            status,
            timestamp: "—".to_string(),
            source: format!("{:?}", group.origin).to_lowercase(),
            evidence,
            evidence_label: format!("{evidence:?}").to_lowercase(),
            identifier: format!("{id:?}"),
        }
    }

    fn node_display(
        &self,
        position: usize,
        group: Option<&GroupId>,
        role: Option<String>,
    ) -> BrowserRowDisplay {
        let Some(node) = self.trace.nodes.get(position) else {
            return unavailable_display("missing record");
        };
        let suffix = role
            .or_else(|| {
                group.map(|id| {
                    self.presentation
                        .group(id)
                        .map_or("group".to_string(), |group| {
                            format!("group {}", group_kind(group.kind))
                        })
                })
            })
            .map_or_else(String::new, |annotation| format!(" · {annotation}"));
        BrowserRowDisplay {
            label: format!("{}{suffix}", node.label),
            preview: node.presentation.preview.clone(),
            kind: node_kind(node.locator.kind).to_string(),
            class: node.presentation.class,
            class_label: class_name(node.presentation.class).to_string(),
            status: node.presentation.status,
            timestamp: node.timestamp.clone().unwrap_or_else(|| "—".to_string()),
            source: source_name(node.provenance).to_string(),
            evidence: node.evidence,
            evidence_label: format!("{:?}", node.evidence).to_lowercase(),
            identifier: node.locator.id.clone(),
        }
    }

    fn reference_display(
        &self,
        relation: codex_trace::TraceRelation,
        target: &TraceObjectRef,
        position: Option<usize>,
    ) -> BrowserRowDisplay {
        if let Some(position) = position {
            let mut display = self.node_display(position, None, Some(format!("{relation:?}")));
            display.kind = "REF".to_string();
            return display;
        }
        BrowserRowDisplay {
            label: format!("{relation:?} → {target:?}"),
            preview: Some("unresolved retained reference".to_string()),
            kind: "REF".to_string(),
            class: TraceRecordClass::Diagnostic,
            class_label: "diagnostic".to_string(),
            status: None,
            timestamp: "—".to_string(),
            source: "—".to_string(),
            evidence: EvidenceGrade::Unavailable,
            evidence_label: "unavailable".to_string(),
            identifier: format!("{target:?}"),
        }
    }
}

pub(super) fn group_class(kind: GroupKind) -> TraceRecordClass {
    match kind {
        GroupKind::UserMessage => TraceRecordClass::User,
        GroupKind::AssistantMessage => TraceRecordClass::Assistant,
        GroupKind::Commentary => TraceRecordClass::Commentary,
        GroupKind::FinalAnswer => TraceRecordClass::FinalAnswer,
        GroupKind::Reasoning => TraceRecordClass::Reasoning,
        GroupKind::SystemContext => TraceRecordClass::System,
        GroupKind::DeveloperContext => TraceRecordClass::Developer,
        GroupKind::DirectTool | GroupKind::ExplorationBatch => TraceRecordClass::ToolInput,
        GroupKind::Code => TraceRecordClass::Code,
        GroupKind::Delegation => TraceRecordClass::Delegation,
        GroupKind::Compaction => TraceRecordClass::Compaction,
        GroupKind::Diagnostic => TraceRecordClass::Diagnostic,
        GroupKind::StructuralRecord | GroupKind::Unknown => TraceRecordClass::Other,
    }
}

fn group_kind(kind: GroupKind) -> &'static str {
    match kind {
        GroupKind::UserMessage => "user",
        GroupKind::AssistantMessage => "assistant",
        GroupKind::Commentary => "commentary",
        GroupKind::FinalAnswer => "final",
        GroupKind::Reasoning => "reasoning",
        GroupKind::SystemContext => "system",
        GroupKind::DeveloperContext => "developer",
        GroupKind::DirectTool => "tool",
        GroupKind::ExplorationBatch => "exploration",
        GroupKind::Code => "code",
        GroupKind::Delegation => "delegation",
        GroupKind::Compaction => "compaction",
        GroupKind::Diagnostic => "diagnostic",
        GroupKind::StructuralRecord => "structural",
        GroupKind::Unknown => "unknown",
    }
}

fn aggregate_evidence(counts: GroupEvidenceCounts) -> EvidenceGrade {
    if counts.conflicting > 0 {
        EvidenceGrade::Conflicting
    } else if counts.exact > 0 {
        EvidenceGrade::Exact
    } else if counts.semantic > 0 {
        EvidenceGrade::Semantic
    } else if counts.reconstructed > 0 {
        EvidenceGrade::Reconstructed
    } else {
        EvidenceGrade::Unavailable
    }
}

fn source_name(source: TraceSourceKind) -> &'static str {
    match source {
        TraceSourceKind::Ordinary => "ordinary",
        TraceSourceKind::Rich => "rich",
        TraceSourceKind::Merged => "merged",
    }
}

fn class_name(class: TraceRecordClass) -> &'static str {
    match class {
        TraceRecordClass::Structure => "structure",
        TraceRecordClass::System => "system",
        TraceRecordClass::Developer => "developer",
        TraceRecordClass::User => "user",
        TraceRecordClass::Assistant => "assistant",
        TraceRecordClass::Commentary => "commentary",
        TraceRecordClass::FinalAnswer => "final",
        TraceRecordClass::Reasoning => "reasoning",
        TraceRecordClass::ToolInput => "tool input",
        TraceRecordClass::ToolOutput => "tool output",
        TraceRecordClass::Code => "code",
        TraceRecordClass::Delegation => "delegation",
        TraceRecordClass::Compaction => "compaction",
        TraceRecordClass::Diagnostic => "diagnostic",
        TraceRecordClass::RawArtifact => "raw",
        TraceRecordClass::Other => "other",
    }
}

fn node_kind(kind: codex_trace::TraceNodeKind) -> &'static str {
    match kind {
        codex_trace::TraceNodeKind::Session => "SES",
        codex_trace::TraceNodeKind::Thread => "THR",
        codex_trace::TraceNodeKind::Turn => "TRN",
        codex_trace::TraceNodeKind::Inference => "INF",
        codex_trace::TraceNodeKind::ConversationItem => "MSG",
        codex_trace::TraceNodeKind::ToolCall => "TOOL",
        codex_trace::TraceNodeKind::CodeCell => "CODE",
        codex_trace::TraceNodeKind::TerminalSession => "TERM",
        codex_trace::TraceNodeKind::TerminalOperation => "OP",
        codex_trace::TraceNodeKind::Compaction => "CMP",
        codex_trace::TraceNodeKind::CompactionRequest => "REQ",
        codex_trace::TraceNodeKind::InteractionEdge => "LINK",
        codex_trace::TraceNodeKind::RolloutRecord => "REC",
        codex_trace::TraceNodeKind::RawPayload => "RAW",
        codex_trace::TraceNodeKind::Diagnostic => "ERR",
    }
}

fn structured_display(
    component: &JsonPathComponent,
    kind: &str,
    preview: &str,
) -> BrowserRowDisplay {
    let label = match component {
        JsonPathComponent::Key(key) => key.clone(),
        JsonPathComponent::Index(index) => format!("[{index}]"),
    };
    BrowserRowDisplay {
        identifier: label.clone(),
        label,
        preview: Some(preview.to_string()),
        kind: kind.to_string(),
        class: TraceRecordClass::Other,
        class_label: "json".to_string(),
        status: None,
        timestamp: "—".to_string(),
        source: "normalized".to_string(),
        evidence: EvidenceGrade::Semantic,
        evidence_label: "semantic".to_string(),
    }
}

fn unavailable_display(label: &str) -> BrowserRowDisplay {
    BrowserRowDisplay {
        label: label.to_string(),
        preview: None,
        kind: "—".to_string(),
        class: TraceRecordClass::Diagnostic,
        class_label: "diagnostic".to_string(),
        status: None,
        timestamp: "—".to_string(),
        source: "—".to_string(),
        evidence: EvidenceGrade::Unavailable,
        evidence_label: "unavailable".to_string(),
        identifier: "—".to_string(),
    }
}

fn lens_name(lens: TraceLens) -> &'static str {
    match lens {
        TraceLens::Collapsed => "collapsed",
        TraceLens::Expanded => "expanded",
        TraceLens::Structural => "structural",
    }
}

fn scope_label(scope: &PresentationScope) -> String {
    match scope {
        PresentationScope::Session => "session".to_string(),
        PresentationScope::Thread(thread_id) => format!("thread {thread_id}"),
    }
}

fn display_pointer(path: &JsonPath) -> String {
    let pointer = path.pointer();
    if pointer.is_empty() {
        "/".to_string()
    } else {
        pointer
    }
}
