use std::io::Cursor;
use std::path::PathBuf;

use pretty_assertions::assert_eq;

use super::OrdinaryTopology;
use super::read_bounded_line;
use crate::catalog::OrdinaryThread;

#[test]
fn bounded_line_retains_exact_limit_and_recovers_after_oversized_line() {
    let mut input = Cursor::new(b"1234\noversized\nok\n");

    let exact = read_bounded_line(&mut input, 4)
        .expect("exact line")
        .expect("line");
    let oversized = read_bounded_line(&mut input, 4)
        .expect("oversized line")
        .expect("line");
    let following = read_bounded_line(&mut input, 4)
        .expect("following line")
        .expect("line");

    assert_eq!((exact.bytes, exact.oversized), (b"1234".to_vec(), false));
    assert_eq!(
        (oversized.bytes, oversized.oversized),
        (b"over".to_vec(), true)
    );
    assert_eq!(
        (following.bytes, following.oversized),
        (b"ok".to_vec(), false)
    );
}

#[test]
fn bounded_line_supports_zero_byte_limit() {
    let mut input = Cursor::new(b"x\n\n");

    let nonempty = read_bounded_line(&mut input, 0)
        .expect("nonempty line")
        .expect("line");
    let empty = read_bounded_line(&mut input, 0)
        .expect("empty line")
        .expect("line");

    assert_eq!((nonempty.bytes, nonempty.oversized), (Vec::new(), true));
    assert_eq!((empty.bytes, empty.oversized), (Vec::new(), false));
}

#[test]
fn topology_resolves_deep_same_session_ancestry() {
    let threads = vec![
        thread("session", "root", None),
        thread("session", "child", Some("root")),
        thread("session", "grandchild", Some("child")),
    ];
    let topology = OrdinaryTopology::new(&threads);

    assert_eq!(topology.parent_id(&threads[0]), Ok(None));
    assert_eq!(topology.parent_id(&threads[1]), Ok(Some("root")));
    assert_eq!(topology.parent_id(&threads[2]), Ok(Some("child")));
}

#[test]
fn topology_rejects_conflicting_duplicate_parent_observations() {
    let threads = vec![
        thread("session", "root-a", None),
        thread("session", "root-b", None),
        thread("session", "parent", Some("root-a")),
        thread("session", "parent", Some("root-b")),
        thread("session", "child", Some("parent")),
    ];
    let topology = OrdinaryTopology::new(&threads);

    assert_eq!(
        topology.parent_id(&threads[4]),
        Err(
            "parent rollout parent has conflicting containment observations; attached thread child to session session"
                .to_string()
        )
    );
}

#[test]
fn topology_rejects_cross_session_parent_and_cycles() {
    let threads = vec![
        thread("other", "foreign", None),
        thread("session", "cross", Some("foreign")),
        thread("session", "cycle-a", Some("cycle-b")),
        thread("session", "cycle-b", Some("cycle-a")),
    ];
    let topology = OrdinaryTopology::new(&threads);

    assert_eq!(
        topology.parent_id(&threads[1]),
        Err(
            "parent rollout foreign belongs to session other; attached thread cross to session session"
                .to_string()
        )
    );
    assert_eq!(
        topology.parent_id(&threads[2]),
        Err("parent cycle at cycle-b; attached thread cycle-a to session session".to_string())
    );
}

fn thread(session_id: &str, thread_id: &str, parent_thread_id: Option<&str>) -> OrdinaryThread {
    OrdinaryThread {
        path: PathBuf::from(format!("{thread_id}.jsonl")),
        session_id: session_id.to_string(),
        thread_id: thread_id.to_string(),
        parent_thread_id: parent_thread_id.map(str::to_string),
        forked_from_thread_id: None,
        history_base: None,
        timestamp: "2026-07-29T00:00:00Z".to_string(),
        cwd: PathBuf::from("/workspace"),
        model_provider: Some("openai".to_string()),
        archived: false,
    }
}
