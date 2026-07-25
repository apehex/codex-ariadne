use std::fs;
use std::path::Path;

use clap::Parser;
use codex_rollout_trace::RawPayloadKind;
use codex_rollout_trace::RawTraceEventPayload;
use codex_rollout_trace::TraceWriter;
use codex_trace::TraceRepository;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use insta::assert_snapshot;
use pretty_assertions::assert_eq;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use serde_json::json;
use tempfile::TempDir;

use super::Cli;
use super::app::App;
use super::app::AppAction;
use super::app::Screen;
use super::normalize_bundle_path;
use super::normalize_trace_root;
use super::render;

const ROOT_ID: &str = "019d0000-0000-7000-8000-000000000101";
const CHILD_ID: &str = "019d0000-0000-7000-8000-000000000102";

#[derive(Debug, Parser)]
struct TestCli {
    #[command(flatten)]
    trace: Cli,
}

#[test]
fn bundle_argument_conflicts_with_session_and_trace_root() {
    assert!(TestCli::try_parse_from(["test", ROOT_ID, "--bundle", "/trace"]).is_err());
    assert!(
        TestCli::try_parse_from(["test", "--bundle", "/trace", "--trace-root", "/root"]).is_err()
    );
}

#[test]
fn exact_bundle_path_validation_is_clear() {
    let temp = TempDir::new().unwrap();
    let bundle = temp.path().join("bundle");
    fs::create_dir_all(&bundle).unwrap();
    let manifest = bundle.join("manifest.json");
    fs::write(&manifest, "{}").unwrap();
    assert_eq!(normalize_bundle_path(&bundle).unwrap(), bundle);
    assert_eq!(normalize_bundle_path(&manifest).unwrap(), bundle);

    let missing = temp.path().join("missing");
    assert!(
        normalize_bundle_path(&missing)
            .unwrap_err()
            .to_string()
            .contains("does not exist")
    );
    let wrong_file = temp.path().join("trace.jsonl");
    fs::write(&wrong_file, "").unwrap();
    assert!(
        normalize_bundle_path(&wrong_file)
            .unwrap_err()
            .to_string()
            .contains("directory or manifest.json")
    );
    let empty = temp.path().join("empty");
    fs::create_dir(&empty).unwrap();
    assert!(
        normalize_bundle_path(&empty)
            .unwrap_err()
            .to_string()
            .contains("does not contain manifest.json")
    );
}

#[test]
fn trace_root_validation_rejects_missing_and_regular_files() {
    let temp = TempDir::new().unwrap();
    assert_eq!(
        normalize_trace_root(temp.path()).unwrap(),
        temp.path().to_path_buf()
    );
    let file = temp.path().join("file");
    fs::write(&file, "").unwrap();
    assert!(normalize_trace_root(&file).is_err());
    assert!(normalize_trace_root(&temp.path().join("missing")).is_err());
}

#[tokio::test]
async fn picker_and_adaptive_browser_have_stable_snapshots() {
    let temp = TempDir::new().unwrap();
    let root_path = write_rollout(temp.path(), ROOT_ID, None, "root needle");
    write_rollout(temp.path(), CHILD_ID, Some(ROOT_ID), "child needle");
    fs::write(
        temp.path()
            .join("sessions/2026/07/25/rollout-malformed.jsonl"),
        "{not json}\n",
    )
    .unwrap();
    let before = fs::read(&root_path).unwrap();
    let mut catalog = TraceRepository::new(temp.path().to_path_buf())
        .discover()
        .await;
    let temporary_root = temp.path().to_str().unwrap();
    for diagnostic in &mut catalog.diagnostics {
        diagnostic.message = diagnostic.message.replace(temporary_root, "/synthetic");
        diagnostic.path = diagnostic.path.as_ref().map(|path| {
            std::path::PathBuf::from(path.to_string_lossy().replace(temporary_root, "/synthetic"))
        });
    }

    let mut picker = App::loading(None, false);
    assert!(matches!(
        picker.install_catalog(catalog.clone()),
        AppAction::None
    ));
    assert_snapshot!("root_picker", render_app(&picker, 120, 22));
    picker.handle_key(key(KeyCode::Char('/')));
    for ch in "synthetic".chars() {
        picker.handle_key(key(KeyCode::Char(ch)));
    }
    assert_snapshot!("picker_filter", render_app(&picker, 100, 18));
    picker.handle_key(key(KeyCode::Esc));

    let trace = catalog.load_session(ROOT_ID).await.unwrap();
    picker.install_session(trace);
    assert_snapshot!("wide_browser", render_app(&picker, 140, 30));
    assert_snapshot!("medium_browser", render_app(&picker, 96, 26));
    assert_snapshot!("narrow_browser", render_app(&picker, 66, 24));
    assert_eq!(fs::read(root_path).unwrap(), before);
}

#[tokio::test]
async fn navigation_search_and_parent_keys_preserve_context() {
    let temp = TempDir::new().unwrap();
    write_rollout(temp.path(), ROOT_ID, None, "root needle");
    write_rollout(temp.path(), CHILD_ID, Some(ROOT_ID), "child needle");
    let catalog = TraceRepository::new(temp.path().to_path_buf())
        .discover()
        .await;
    let trace = catalog.load_session(ROOT_ID).await.unwrap();
    let mut app = App::loading(None, false);
    app.install_session(trace);

    let Screen::Browser(browser) = &app.screen else {
        panic!("expected browser");
    };
    assert_eq!(browser.visible_rows().len(), 2);
    app.handle_key(key(KeyCode::Down));
    let thread_id = selected_id(&app);
    app.handle_key(key(KeyCode::Enter));
    let Screen::Browser(browser) = &app.screen else {
        panic!("expected browser");
    };
    assert!(
        browser
            .visible_rows()
            .iter()
            .any(|row| row.node.label.contains(CHILD_ID))
    );
    app.handle_key(key(KeyCode::Enter));
    app.handle_key(key(KeyCode::Backspace));
    assert_ne!(selected_id(&app), thread_id);

    app.handle_key(key(KeyCode::Char('/')));
    for ch in "needle".chars() {
        app.handle_key(key(KeyCode::Char(ch)));
    }
    assert_snapshot!("search_overlay", render_app(&app, 100, 25));
    app.handle_key(key(KeyCode::Enter));
    let first_hit = selected_id(&app);
    app.handle_key(key(KeyCode::Char('n')));
    assert_ne!(selected_id(&app), first_hit);
    app.handle_key(key(KeyCode::Char('N')));
    assert_eq!(selected_id(&app), first_hit);
    assert_snapshot!("search_navigation", render_app(&app, 100, 25));
}

#[tokio::test]
async fn enter_requests_raw_payload_without_eagerly_reading_it() {
    let temp = TempDir::new().unwrap();
    let bundle = temp.path().join("bundle");
    let writer = TraceWriter::create(
        &bundle,
        "trace-ui".to_string(),
        "rollout-ui".to_string(),
        ROOT_ID.to_string(),
    )
    .unwrap();
    let payload = writer
        .write_json_payload(
            RawPayloadKind::SessionMetadata,
            &json!({"prompt": "synthetic payload"}),
        )
        .unwrap();
    writer
        .append(RawTraceEventPayload::ThreadStarted {
            thread_id: ROOT_ID.to_string(),
            agent_path: "/root".to_string(),
            metadata_payload: Some(payload),
        })
        .unwrap();
    drop(writer);
    let before = snapshot_files(&bundle);
    let catalog = TraceRepository::new(temp.path().join("empty"))
        .with_rich_bundle(bundle.clone())
        .discover()
        .await;
    let trace = catalog.load_session("rollout-ui").await.unwrap();
    let mut app = App::loading(None, false);
    app.install_session(trace);
    app.handle_key(key(KeyCode::Down));
    assert_snapshot!("raw_payload_collapsed", render_app(&app, 100, 24));
    let AppAction::ReadPayload { id, handle, limit } = app.handle_key(key(KeyCode::Enter)) else {
        panic!("expected lazy raw payload request");
    };
    app.install_payload(id.clone(), handle.read(limit).await);
    app.install_payload(id, Err(anyhow::anyhow!("bad \u{1b}[31m payload\u{7} path")));
    let rendered = render_app(&app, 100, 24);
    assert!(!rendered.contains('\u{1b}'));
    assert_snapshot!("raw_payload_failure_sanitized", rendered);
    assert_eq!(snapshot_files(&bundle), before);
}

fn snapshot_files(root: &Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(path) = pending.pop() {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                pending.push(entry.path());
            } else {
                files.push((
                    entry.path().strip_prefix(root).unwrap().to_path_buf(),
                    fs::read(entry.path()).unwrap(),
                ));
            }
        }
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    files
}

fn render_app(app: &App, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| render::render(frame, app)).unwrap();
    let buffer = terminal.backend().buffer();
    (0..height)
        .map(|y| {
            let mut line = String::new();
            for x in 0..width {
                line.push_str(buffer[(x, y)].symbol());
            }
            line.trim_end().to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim_end()
        .to_string()
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::from(code)
}

fn selected_id(app: &App) -> String {
    let Screen::Browser(browser) = &app.screen else {
        panic!("expected browser");
    };
    browser.selected_node().unwrap().locator.id.clone()
}

fn write_rollout(
    codex_home: &Path,
    id: &str,
    parent: Option<&str>,
    message: &str,
) -> std::path::PathBuf {
    let directory = codex_home.join("sessions/2026/07/25");
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join(format!("rollout-2026-07-25T00-00-00-{id}.jsonl"));
    let lines = [
        json!({
            "timestamp": "2026-07-25T00:00:00Z",
            "type": "session_meta",
            "payload": {
                "session_id": id,
                "id": id,
                "parent_thread_id": parent,
                "timestamp": "2026-07-25T00:00:00Z",
                "cwd": "/synthetic/workspace",
                "originator": "codex-trace-tui-test",
                "cli_version": "0.0.0",
                "source": "cli",
                "model_provider": "synthetic",
                "base_instructions": null
            }
        }),
        json!({
            "timestamp": "2026-07-25T00:00:01Z",
            "type": "event_msg",
            "payload": {
                "type": "user_message",
                "message": message,
                "kind": "plain"
            }
        }),
    ];
    fs::write(
        &path,
        format!(
            "{}\n",
            lines
                .iter()
                .map(serde_json::Value::to_string)
                .collect::<Vec<_>>()
                .join("\n")
        ),
    )
    .unwrap();
    path
}
