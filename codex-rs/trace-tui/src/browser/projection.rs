//! Current-level row projection, visibility, and initial scope discovery.

use std::collections::HashMap;

use codex_trace::GroupId;
use codex_trace::GroupVisibility;
use codex_trace::PresentationIndex;
use codex_trace::PresentationScope;
use codex_trace::SessionTrace;
use codex_trace::TraceIndex;
use codex_trace::TraceNode;
use codex_trace::TraceNodeKind;
use codex_trace::TraceNodeLocator;
use codex_trace::TraceRecordClass;

use crate::TraceLens;
use crate::TraceStartScope;

use super::BrowserState;
use super::display::group_class;
use super::location::BrowserLocation;
use super::rows::BrowserRow;
use super::rows::BrowserRowId;
use super::structured;
use super::structured::JsonPath;
use super::structured::JsonPathComponent;

impl BrowserState {
    pub(super) fn selected_row_id(&self) -> Option<BrowserRowId> {
        self.rows
            .get(self.selection.index())
            .and_then(|row| row.id(&self.trace))
    }

    pub(super) fn rebuild_rows(&mut self, preferred: Option<&BrowserRowId>) {
        self.rows = self.project_rows();
        let mut hidden_rows = 0;
        let rows = std::mem::take(&mut self.rows);
        self.rows = rows
            .into_iter()
            .filter(|row| {
                let visible = self.row_visible(row);
                hidden_rows += usize::from(!visible);
                visible
            })
            .collect();
        self.hidden_rows = hidden_rows;
        let selected = preferred
            .and_then(|id| {
                self.rows
                    .iter()
                    .position(|row| row.id(&self.trace).as_ref() == Some(id))
            })
            .unwrap_or(0);
        self.selection.set(selected, self.rows.len());
        self.invalidate_detail();
    }

    fn project_rows(&self) -> Vec<BrowserRow> {
        match &self.location {
            BrowserLocation::Lens {
                lens: TraceLens::Collapsed,
                scope,
                ..
            } => self
                .presentation
                .groups_in_scope(scope)
                .map(|group| BrowserRow::Group(group.id.clone()))
                .collect(),
            BrowserLocation::Lens {
                lens: TraceLens::Expanded,
                scope,
                ..
            } => self
                .presentation
                .events_in_scope(scope)
                .iter()
                .map(|event| BrowserRow::Event {
                    node_position: event.node_position,
                    group: event.primary_group.clone(),
                    band: event.band,
                })
                .collect(),
            BrowserLocation::Lens {
                lens: TraceLens::Structural,
                structural_container,
                ..
            } => match structural_container {
                Some(container) => self
                    .index
                    .child_positions(&self.trace, container)
                    .map(BrowserRow::Structural)
                    .collect(),
                None => self
                    .index
                    .root_positions(&self.trace)
                    .map(BrowserRow::Structural)
                    .collect(),
            },
            BrowserLocation::Group(id) => self.group_rows(id),
            BrowserLocation::Structured { locator, path } => self.structured_rows(locator, path),
            BrowserLocation::Detail(_) => Vec::new(),
        }
    }

    fn group_rows(&self, id: &GroupId) -> Vec<BrowserRow> {
        let Some(group) = self.presentation.group(id) else {
            return Vec::new();
        };
        let ranks = self
            .presentation
            .events_in_scope(&group.scope)
            .iter()
            .enumerate()
            .map(|(rank, event)| (event.node_position, rank))
            .collect::<HashMap<_, _>>();
        let mut ordered = group
            .members
            .iter()
            .map(|member| {
                (
                    ranks
                        .get(&member.node_position)
                        .copied()
                        .unwrap_or(usize::MAX),
                    BrowserRow::Member {
                        node_position: member.node_position,
                        role: member.role,
                    },
                )
            })
            .chain(group.child_groups.iter().filter_map(|child_id| {
                let child = self.presentation.group(child_id)?;
                let rank = child
                    .members
                    .iter()
                    .filter_map(|member| ranks.get(&member.node_position))
                    .copied()
                    .min()
                    .unwrap_or(usize::MAX);
                Some((rank, BrowserRow::ChildGroup(child_id.clone())))
            }))
            .collect::<Vec<_>>();
        ordered.sort_by_key(|(rank, row)| {
            let child_rank = usize::from(matches!(row, BrowserRow::ChildGroup(_)));
            (*rank, child_rank)
        });
        let mut rows = ordered.into_iter().map(|(_, row)| row).collect::<Vec<_>>();
        rows.extend(
            group
                .references
                .iter()
                .map(|reference| BrowserRow::Reference {
                    relation: reference.relation,
                    target: reference.target.clone(),
                    node_position: reference.node_position,
                }),
        );
        rows
    }

    fn structured_rows(&self, locator: &TraceNodeLocator, path: &JsonPath) -> Vec<BrowserRow> {
        let Some(value) = self
            .index
            .node(&self.trace, locator)
            .and_then(|node| path.resolve(&node.detail))
        else {
            return Vec::new();
        };
        if path.at_max_depth()
            && matches!(
                value,
                serde_json::Value::Object(_) | serde_json::Value::Array(_)
            )
        {
            return vec![BrowserRow::Notice(
                "maximum structured depth reached".to_string(),
            )];
        }
        let (mut rows, omitted) = match value {
            serde_json::Value::Object(map) => {
                let mut entries = map.iter().collect::<Vec<_>>();
                entries.sort_by_key(|(key, _)| *key);
                (
                    entries
                        .into_iter()
                        .take(structured::MAX_STRUCTURED_CHILDREN)
                        .map(|(key, value)| BrowserRow::Structured {
                            component: JsonPathComponent::Key(key.clone()),
                            kind: structured::value_kind(value),
                            preview: structured::preview(value),
                        })
                        .collect(),
                    map.len()
                        .saturating_sub(structured::MAX_STRUCTURED_CHILDREN),
                )
            }
            serde_json::Value::Array(values) => (
                values
                    .iter()
                    .take(structured::MAX_STRUCTURED_CHILDREN)
                    .enumerate()
                    .map(|(index, value)| BrowserRow::Structured {
                        component: JsonPathComponent::Index(index),
                        kind: structured::value_kind(value),
                        preview: structured::preview(value),
                    })
                    .collect(),
                values
                    .len()
                    .saturating_sub(structured::MAX_STRUCTURED_CHILDREN),
            ),
            serde_json::Value::Null
            | serde_json::Value::Bool(_)
            | serde_json::Value::Number(_)
            | serde_json::Value::String(_) => (Vec::new(), 0),
        };
        if omitted > 0 {
            rows.push(BrowserRow::Notice(format!(
                "{omitted} structured children omitted at the safe bound"
            )));
        }
        rows
    }

    pub(super) fn structured_value(&self) -> Option<&serde_json::Value> {
        let BrowserLocation::Structured { locator, path } = &self.location else {
            return None;
        };
        self.index
            .node(&self.trace, locator)
            .and_then(|node| path.resolve(&node.detail))
    }

    fn row_visible(&self, row: &BrowserRow) -> bool {
        let class = self.row_class(row);
        let class_visible = class.is_none_or(|class| self.visible_classes.contains(&class));
        if !class_visible {
            return self.row_locator(row).as_ref() == self.temporary_reveal.as_ref();
        }
        let group = match row {
            BrowserRow::Group(id) | BrowserRow::ChildGroup(id) => self.presentation.group(id),
            BrowserRow::Event { group, .. } => self.presentation.group(group),
            BrowserRow::Structural(_)
            | BrowserRow::Member { .. }
            | BrowserRow::Reference { .. }
            | BrowserRow::Structured { .. }
            | BrowserRow::Notice(_) => None,
        };
        group.is_none_or(|group| {
            self.show_hidden_groups || group.default_visibility == GroupVisibility::Shown
        }) || self.row_locator(row).as_ref() == self.temporary_reveal.as_ref()
    }

    fn row_class(&self, row: &BrowserRow) -> Option<TraceRecordClass> {
        match row {
            BrowserRow::Group(id) | BrowserRow::ChildGroup(id) => self
                .presentation
                .group(id)
                .map(|group| group_class(group.kind)),
            BrowserRow::Event { node_position, .. }
            | BrowserRow::Structural(node_position)
            | BrowserRow::Member { node_position, .. }
            | BrowserRow::Reference {
                node_position: Some(node_position),
                ..
            } => self
                .trace
                .nodes
                .get(*node_position)
                .map(|node| node.presentation.class),
            BrowserRow::Reference {
                node_position: None,
                ..
            }
            | BrowserRow::Structured { .. }
            | BrowserRow::Notice(_) => None,
        }
    }

    fn row_locator(&self, row: &BrowserRow) -> Option<TraceNodeLocator> {
        self.node_for_row(row).map(|node| node.locator.clone())
    }

    pub(super) fn node_for_row<'a>(&'a self, row: &'a BrowserRow) -> Option<&'a TraceNode> {
        let position = match row {
            BrowserRow::Event { node_position, .. }
            | BrowserRow::Structural(node_position)
            | BrowserRow::Member { node_position, .. }
            | BrowserRow::Reference {
                node_position: Some(node_position),
                ..
            } => Some(*node_position),
            BrowserRow::Group(id) | BrowserRow::ChildGroup(id) => self.representative_position(id),
            BrowserRow::Reference {
                node_position: None,
                ..
            }
            | BrowserRow::Structured { .. }
            | BrowserRow::Notice(_) => None,
        };
        position.and_then(|position| self.trace.nodes.get(position))
    }

    fn representative_position(&self, id: &GroupId) -> Option<usize> {
        let group = self.presentation.group(id)?;
        group
            .members
            .first()
            .map(|member| member.node_position)
            .or_else(|| {
                group
                    .child_groups
                    .first()
                    .and_then(|child| self.representative_position(child))
            })
    }
}

pub(super) fn initial_scope(
    trace: &SessionTrace,
    index: &TraceIndex,
    presentation: &PresentationIndex,
    session_root: Option<&TraceNodeLocator>,
    requested: &TraceStartScope,
) -> PresentationScope {
    match requested {
        TraceStartScope::Session => PresentationScope::Session,
        TraceStartScope::Thread(thread_id) => PresentationScope::Thread(thread_id.clone()),
        TraceStartScope::RootThread => session_root
            .and_then(|root| {
                index.child_positions(trace, root).find(|position| {
                    trace
                        .nodes
                        .get(*position)
                        .is_some_and(|node| node.locator.kind == TraceNodeKind::Thread)
                })
            })
            .and_then(|thread_position| {
                first_scope_below(trace, index, presentation, thread_position)
            })
            .unwrap_or(PresentationScope::Session),
    }
}

fn first_scope_below(
    trace: &SessionTrace,
    index: &TraceIndex,
    presentation: &PresentationIndex,
    root_position: usize,
) -> Option<PresentationScope> {
    let mut pending = vec![root_position];
    while let Some(position) = pending.pop() {
        if let Some(group) = presentation.group_for_node(position) {
            return Some(group.scope.clone());
        }
        let node = trace.nodes.get(position)?;
        let mut children = index
            .child_positions(trace, &node.locator)
            .filter(|child| {
                trace
                    .nodes
                    .get(*child)
                    .is_none_or(|node| node.locator.kind != TraceNodeKind::Thread)
            })
            .collect::<Vec<_>>();
        children.reverse();
        pending.extend(children);
    }
    None
}
