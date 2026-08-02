use pretty_assertions::assert_eq;
use serde_json::json;

use crate::EvidenceGrade;
use crate::TraceContentDocument;
use crate::TraceContentFormat;
use crate::TraceNode;
use crate::TraceNodeKind;
use crate::TraceNodeLocator;
use crate::TraceRecordChannel;
use crate::TraceRecordClass;
use crate::TraceRecordPresentation;
use crate::TraceRecordRole;
use crate::TraceSourceKind;

#[test]
fn ordinary_user_record_has_semantic_presentation_and_unescaped_content() {
    let detail = json!({
        "timestamp": "2026-07-28T00:00:00Z",
        "type": "event_msg",
        "payload": {
            "type": "user_message",
            "message": "first line\nsecond line",
            "kind": "plain"
        }
    });
    let presentation = TraceRecordPresentation::from_detail(TraceNodeKind::RolloutRecord, &detail);
    assert_eq!(
        presentation,
        TraceRecordPresentation {
            class: TraceRecordClass::User,
            role: None,
            channel: None,
            status: None,
            preview: Some("first line second line".to_string()),
        }
    );
    assert_eq!(
        node(detail, presentation).content_document(/*byte_limit*/ 1024),
        TraceContentDocument {
            format: TraceContentFormat::Text,
            text: "first line\nsecond line".to_string(),
            truncated: false,
        }
    );
}

#[test]
fn nested_assistant_role_and_channel_select_markdown_final_answer() {
    let detail = json!({
        "type": "response_item",
        "payload": {
            "role": "assistant",
            "channel": "final",
            "content": [{"type": "output_text", "text": "**done**"}]
        }
    });
    let presentation =
        TraceRecordPresentation::from_detail(TraceNodeKind::ConversationItem, &detail);
    assert_eq!(
        presentation,
        TraceRecordPresentation {
            class: TraceRecordClass::FinalAnswer,
            role: Some(TraceRecordRole::Assistant),
            channel: Some(TraceRecordChannel::Final),
            status: None,
            preview: Some("**done**".to_string()),
        }
    );
    assert_eq!(
        node(detail, presentation).content_document(/*byte_limit*/ 1024),
        TraceContentDocument {
            format: TraceContentFormat::Markdown,
            text: "**done**".to_string(),
            truncated: false,
        }
    );
}

#[test]
fn semantic_content_respects_utf8_byte_limit() {
    let detail = json!({"role": "user", "text": "ééé\u{1b}"});
    let presentation =
        TraceRecordPresentation::from_detail(TraceNodeKind::ConversationItem, &detail);
    assert_eq!(
        node(detail, presentation).content_document(/*byte_limit*/ 3),
        TraceContentDocument {
            format: TraceContentFormat::Text,
            text: "é".to_string(),
            truncated: true,
        }
    );
}

#[test]
fn semantic_content_preserves_layout_but_neutralizes_terminal_controls() {
    let detail = json!({"role": "user", "text": "first\n\tsecond\u{1b}[31m"});
    let presentation =
        TraceRecordPresentation::from_detail(TraceNodeKind::ConversationItem, &detail);
    assert_eq!(
        node(detail, presentation)
            .content_document(/*byte_limit*/ 1024)
            .text,
        "first\n\tsecond�[31m"
    );
}

#[test]
fn tool_json_is_prepared_for_semantic_highlighting() {
    let detail = json!({
        "role": "tool",
        "type": "function_call_output",
        "output": "{\"ok\":true}"
    });
    let presentation =
        TraceRecordPresentation::from_detail(TraceNodeKind::ConversationItem, &detail);
    assert_eq!(
        node(detail, presentation).content_document(/*byte_limit*/ 1024),
        TraceContentDocument {
            format: TraceContentFormat::Json,
            text: "{\n  \"ok\": true\n}".to_string(),
            truncated: false,
        }
    );
}

fn node(detail: serde_json::Value, presentation: TraceRecordPresentation) -> TraceNode {
    TraceNode {
        locator: TraceNodeLocator::new("session", TraceNodeKind::RolloutRecord, "record"),
        parent: None,
        provenance: TraceSourceKind::Ordinary,
        evidence: EvidenceGrade::Semantic,
        timestamp: None,
        label: "record".to_string(),
        presentation,
        detail,
    }
}
