use std::fs;

use tempfile::TempDir;

use super::resolve_payload_path;
use crate::RawPayloadKind;
use crate::RawPayloadRef;

#[test]
fn reducer_payload_paths_are_contained_and_regular() {
    let temp = TempDir::new().expect("temp dir");
    let bundle = temp.path().join("bundle");
    let payloads = bundle.join("payloads");
    fs::create_dir_all(&payloads).expect("create payload directory");
    fs::write(payloads.join("inside.json"), "{}").expect("write inside payload");
    fs::write(temp.path().join("outside.json"), "{}").expect("write outside payload");

    let inside = payload("payloads/inside.json");
    assert_eq!(
        resolve_payload_path(&bundle, &inside).expect("inside payload"),
        fs::canonicalize(payloads.join("inside.json")).expect("canonical inside payload"),
    );

    let escaped = payload("../outside.json");
    assert!(
        resolve_payload_path(&bundle, &escaped)
            .expect_err("escape must fail")
            .to_string()
            .contains("escapes trace bundle")
    );

    let directory = payload("payloads");
    assert!(
        resolve_payload_path(&bundle, &directory)
            .expect_err("directory must fail")
            .to_string()
            .contains("not a regular file")
    );
}

#[cfg(unix)]
#[test]
fn reducer_payload_paths_reject_symlink_escape() {
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().expect("temp dir");
    let bundle = temp.path().join("bundle");
    let payloads = bundle.join("payloads");
    fs::create_dir_all(&payloads).expect("create payload directory");
    let outside = temp.path().join("outside.json");
    fs::write(&outside, "{}").expect("write outside payload");
    symlink(&outside, payloads.join("link.json")).expect("create payload symlink");

    let escaped = payload("payloads/link.json");
    assert!(
        resolve_payload_path(&bundle, &escaped)
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
