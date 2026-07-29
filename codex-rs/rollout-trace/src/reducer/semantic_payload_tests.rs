use std::fs;

use pretty_assertions::assert_eq;
use tempfile::TempDir;

use super::SemanticPayloadReadLimit;
use super::SemanticPayloadReader;
use crate::RawPayloadKind;
use crate::RawPayloadRef;

#[test]
fn reads_contained_payload_at_the_exact_limit() {
    let temp = TempDir::new().expect("temp dir");
    let bundle = temp.path().join("bundle");
    let payloads = bundle.join("payloads");
    fs::create_dir_all(&payloads).expect("create payload directory");
    let contents = br#"{"ok":true}"#;
    fs::write(payloads.join("inside.json"), contents).expect("write inside payload");

    let reader =
        SemanticPayloadReader::new(bundle, SemanticPayloadReadLimit::Bytes(contents.len()));
    assert_eq!(
        reader
            .read_json(&payload("payloads/inside.json"))
            .expect("read exact payload"),
        serde_json::json!({"ok": true}),
    );
}

#[test]
fn rejects_oversized_and_escaping_payloads() {
    let temp = TempDir::new().expect("temp dir");
    let bundle = temp.path().join("bundle");
    let payloads = bundle.join("payloads");
    fs::create_dir_all(&payloads).expect("create payload directory");
    fs::write(payloads.join("large.json"), r#"{"value":"large"}"#).expect("write payload");
    fs::write(temp.path().join("outside.json"), "{}").expect("write outside payload");
    let reader = SemanticPayloadReader::new(bundle, SemanticPayloadReadLimit::Bytes(2));

    assert!(
        reader
            .read_json(&payload("payloads/large.json"))
            .expect_err("oversized payload must fail")
            .to_string()
            .contains("semantic replay limit")
    );
    assert!(
        reader
            .read_json(&payload("../outside.json"))
            .expect_err("escape must fail")
            .to_string()
            .contains("escapes trace bundle")
    );
}

#[test]
fn rejects_zero_limit_invalid_utf8_and_absolute_paths() {
    let temp = TempDir::new().expect("temp dir");
    let bundle = temp.path().join("bundle");
    let payloads = bundle.join("payloads");
    fs::create_dir_all(&payloads).expect("create payload directory");
    fs::write(payloads.join("one.json"), b"0").expect("write one-byte payload");
    fs::write(payloads.join("invalid.json"), b"\xff").expect("write invalid payload");

    let zero_reader =
        SemanticPayloadReader::new(bundle.clone(), SemanticPayloadReadLimit::Bytes(0));
    assert!(
        zero_reader
            .read_json(&payload("payloads/one.json"))
            .expect_err("zero limit must reject nonempty payload")
            .to_string()
            .contains("semantic replay limit")
    );
    let reader = SemanticPayloadReader::new(bundle, SemanticPayloadReadLimit::Bytes(1));
    assert!(
        reader
            .read_json(&payload("payloads/invalid.json"))
            .expect_err("invalid UTF-8 JSON must fail")
            .to_string()
            .contains("parse payload")
    );
    assert!(
        reader
            .read_json(&payload(
                temp.path().join("absolute.json").to_string_lossy().as_ref()
            ))
            .expect_err("absolute payload path must fail")
            .to_string()
            .contains("bundle-relative")
    );
}

#[cfg(unix)]
#[test]
fn rejects_symlink_escape() {
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().expect("temp dir");
    let bundle = temp.path().join("bundle");
    let payloads = bundle.join("payloads");
    fs::create_dir_all(&payloads).expect("create payload directory");
    let outside = temp.path().join("outside.json");
    fs::write(&outside, "{}").expect("write outside payload");
    symlink(&outside, payloads.join("link.json")).expect("create payload symlink");
    let reader = SemanticPayloadReader::new(bundle, SemanticPayloadReadLimit::Bytes(1024));

    assert!(
        reader
            .read_json(&payload("payloads/link.json"))
            .expect_err("symlink escape must fail")
            .to_string()
            .contains("escapes trace bundle")
    );
}

fn payload(path: &str) -> RawPayloadRef {
    RawPayloadRef {
        raw_payload_id: format!("raw:{path}"),
        kind: RawPayloadKind::ToolRuntimeEvent,
        path: path.to_string(),
    }
}
