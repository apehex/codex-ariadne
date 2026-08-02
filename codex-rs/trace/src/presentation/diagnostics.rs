//! Bounded collection of presentation-build diagnostics.

use super::GroupId;
use super::PresentationDiagnostic;
use super::PresentationDiagnosticCode;
use super::PresentationLimits;

pub(crate) struct DiagnosticSink {
    message_limit: usize,
    limit: usize,
    diagnostics: Vec<PresentationDiagnostic>,
    saturated: bool,
}

impl DiagnosticSink {
    pub(crate) fn new(node_count: usize, limits: PresentationLimits) -> Self {
        Self {
            message_limit: limits.max_diagnostic_message_chars,
            limit: node_count
                .saturating_add(1)
                .min(limits.max_diagnostics)
                .max(1),
            diagnostics: Vec::new(),
            saturated: false,
        }
    }

    pub(crate) fn push(
        &mut self,
        code: PresentationDiagnosticCode,
        group_id: Option<GroupId>,
        node_position: Option<usize>,
        message: &str,
    ) {
        if self.diagnostics.len() < self.limit {
            self.diagnostics.push(PresentationDiagnostic {
                code,
                group_id,
                node_position,
                message: message.chars().take(self.message_limit).collect(),
            });
        } else {
            self.saturated = true;
        }
    }

    pub(crate) fn extend(&mut self, diagnostics: impl IntoIterator<Item = PresentationDiagnostic>) {
        for diagnostic in diagnostics {
            self.push(
                diagnostic.code,
                diagnostic.group_id,
                diagnostic.node_position,
                &diagnostic.message,
            );
        }
    }

    pub(crate) fn finish(mut self) -> Vec<PresentationDiagnostic> {
        if self.saturated && !self.diagnostics.is_empty() {
            let last = self.diagnostics.len() - 1;
            self.diagnostics[last] = PresentationDiagnostic {
                code: PresentationDiagnosticCode::ResourceLimit,
                group_id: None,
                node_position: None,
                message: "additional presentation diagnostics were bounded away"
                    .chars()
                    .take(self.message_limit)
                    .collect(),
            };
        }
        self.diagnostics
    }
}
