use pretty_assertions::assert_eq;
use serde_json::json;

use crate::EvidenceGrade;
use crate::PresentationDiagnosticCode;
use crate::PresentationIndex;
use crate::PresentationLimits;
use crate::SessionSummary;
use crate::SessionTrace;
use crate::TraceCapabilities;
use crate::TraceNode;
use crate::TraceNodeFacts;
use crate::TraceNodeKind;
use crate::TraceNodeLocator;
use crate::TraceSourceKind;
use crate::TraceStatus;

#[test]
fn default_limit_entry_points_are_deeply_equal() {
    let trace = trace_with_user_message();
    let expected = PresentationIndex::new(&trace);

    assert_eq!(
        expected,
        PresentationIndex::new_with_limits(&trace, PresentationLimits::default())
    );
    assert_eq!(expected, trace.presentation_index());
    assert_eq!(
        expected,
        trace.presentation_index_with_limits(PresentationLimits::default())
    );
}

#[test]
fn limits_above_safety_maxima_match_defaults() {
    let trace = trace_with_user_message();
    let unbounded_request = PresentationLimits {
        max_references_per_group: usize::MAX,
        max_references_per_node: usize::MAX,
        max_summary_bytes_per_group: usize::MAX,
        max_total_summary_bytes: usize::MAX,
        max_preview_chars: usize::MAX,
        max_diagnostics: usize::MAX,
        max_diagnostic_message_chars: usize::MAX,
        max_group_depth: usize::MAX,
    };

    assert_eq!(
        trace.presentation_index(),
        trace.presentation_index_with_limits(unbounded_request)
    );
}

#[test]
fn zero_limits_retain_primary_evidence_and_a_truncation_diagnostic() {
    let trace = trace_with_user_message();
    let index = trace.presentation_index_with_limits(PresentationLimits {
        max_references_per_group: 0,
        max_references_per_node: 0,
        max_summary_bytes_per_group: 0,
        max_total_summary_bytes: 0,
        max_preview_chars: 0,
        max_diagnostics: 0,
        max_diagnostic_message_chars: 0,
        max_group_depth: 0,
    });

    assert_eq!(index.groups().len(), 1);
    assert_eq!(index.groups()[0].members.len(), 1);
    assert_eq!(index.groups()[0].metadata.label.as_deref(), Some(""));
    assert_eq!(index.groups()[0].metadata.preview.as_deref(), Some(""));
    assert_eq!(
        index.diagnostics()[0].code,
        PresentationDiagnosticCode::ResourceLimit
    );
}

#[test]
fn reduced_text_limits_apply_exact_utf8_safe_bounds() {
    let trace = trace_with_user_message();
    let index = trace.presentation_index_with_limits(PresentationLimits {
        max_summary_bytes_per_group: 5,
        max_total_summary_bytes: 5,
        max_preview_chars: 2,
        ..PresentationLimits::default()
    });

    assert_eq!(index.groups()[0].metadata.label.as_deref(), Some("user "));
    assert_eq!(index.groups()[0].metadata.preview.as_deref(), Some(""));
    assert!(
        index
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code == PresentationDiagnosticCode::ResourceLimit)
    );
}

fn trace_with_user_message() -> SessionTrace {
    let locator = TraceNodeLocator::new("session", TraceNodeKind::ConversationItem, "user");
    let node = TraceNode::projected(
        locator.clone(),
        /*parent*/ None,
        TraceSourceKind::Ordinary,
        EvidenceGrade::Semantic,
        /*timestamp*/ None,
        "user message".to_string(),
        json!({"role": "user", "text": "évidence preview"}),
    );
    SessionTrace {
        summary: SessionSummary {
            session_id: "session".to_string(),
            root_thread_id: "session".to_string(),
            source: TraceSourceKind::Ordinary,
            capabilities: TraceCapabilities::default(),
            created_at: None,
            cwd: None,
            model_provider: None,
            status: TraceStatus::Unknown,
            archived: false,
            thread_count: None,
        },
        nodes: vec![node],
        diagnostics: Vec::new(),
        facts: [(locator, TraceNodeFacts::default())].into_iter().collect(),
        payloads: Default::default(),
    }
}
