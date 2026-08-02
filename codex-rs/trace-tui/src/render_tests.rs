use std::fs;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use clap::Parser;
use codex_rollout_trace::RawPayloadKind;
use codex_rollout_trace::RawTraceEventPayload;
use codex_rollout_trace::TraceWriter;
use codex_trace::SanitizedPayload;
use codex_trace::TraceNodeKind;
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
use super::HeaderMode;
use super::PlainTraceVisualRenderer;
use super::PreviewMode;
use super::TraceColumn;
use super::TraceViewOptions;
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
    let root_path = write_rollout(
        temp.path(),
        ROOT_ID,
        ROOT_ID,
        /*parent*/ None,
        "root needle",
    );
    write_rollout(
        temp.path(),
        ROOT_ID,
        CHILD_ID,
        /*parent*/ Some(ROOT_ID),
        "child needle",
    );
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

    let mut picker = App::loading(
        /*preferred_session*/ None, /*auto_open_rich*/ false,
    );
    assert!(matches!(
        picker.install_catalog(catalog.clone()),
        AppAction::None
    ));
    assert_snapshot!("root_picker", render_app(&mut picker, 120, 22));
    picker.handle_key(key(KeyCode::Char('/')));
    for ch in "synthetic".chars() {
        picker.handle_key(key(KeyCode::Char(ch)));
    }
    assert_snapshot!("picker_filter", render_app(&mut picker, 100, 18));
    picker.handle_key(key(KeyCode::Esc));

    let trace = catalog.load_session(ROOT_ID).await.unwrap();
    picker.install_session(trace);
    assert_snapshot!("wide_browser", render_app(&mut picker, 140, 30));
    let Screen::Browser(browser) = &picker.screen else {
        panic!("expected browser");
    };
    assert_eq!(
        browser.row_count(),
        browser.rows_window(0, browser.row_count()).len()
    );
    assert_snapshot!("medium_browser", render_app(&mut picker, 96, 26));
    assert_snapshot!("narrow_browser", render_app(&mut picker, 66, 24));
    assert_eq!(fs::read(root_path).unwrap(), before);
}

#[tokio::test]
async fn navigation_search_and_parent_keys_preserve_context() {
    let temp = TempDir::new().unwrap();
    write_rollout(
        temp.path(),
        ROOT_ID,
        ROOT_ID,
        /*parent*/ None,
        "root needle",
    );
    write_rollout(
        temp.path(),
        ROOT_ID,
        CHILD_ID,
        /*parent*/ Some(ROOT_ID),
        "child needle",
    );
    let catalog = TraceRepository::new(temp.path().to_path_buf())
        .discover()
        .await;
    let trace = catalog.load_session(ROOT_ID).await.unwrap();
    let mut app = App::loading(
        /*preferred_session*/ None, /*auto_open_rich*/ false,
    );
    app.install_session(trace);

    let Screen::Browser(browser) = &app.screen else {
        panic!("expected browser");
    };
    assert_eq!(browser.row_count(), 1);
    move_to_kind(&mut app, TraceNodeKind::Thread);
    let thread_id = selected_id(&app);
    app.handle_key(key(KeyCode::Enter));
    let Screen::Browser(browser) = &app.screen else {
        panic!("expected browser");
    };
    assert!(
        browser
            .rows_window(0, browser.row_count())
            .iter()
            .any(|row| browser.row_display(row).label.contains(CHILD_ID))
    );
    app.handle_key(key(KeyCode::Esc));
    assert_eq!(selected_id(&app), thread_id);

    app.handle_key(key(KeyCode::Char('/')));
    for ch in "needle".chars() {
        app.handle_key(key(KeyCode::Char(ch)));
    }
    assert_snapshot!("search_overlay", render_app(&mut app, 100, 25));
    app.handle_key(key(KeyCode::Enter));
    complete_search(&mut app);
    app.handle_key(key(KeyCode::Enter));
    let first_hit = selected_id(&app);
    app.handle_key(key(KeyCode::Char('n')));
    assert_ne!(selected_id(&app), first_hit);
    app.handle_key(key(KeyCode::Char('N')));
    assert_eq!(selected_id(&app), first_hit);
    assert_snapshot!("search_navigation", render_app(&mut app, 100, 25));
}

#[tokio::test]
async fn detail_modes_filters_and_help_have_stable_snapshots() {
    let temp = TempDir::new().unwrap();
    write_rollout(
        temp.path(),
        ROOT_ID,
        ROOT_ID,
        /*parent*/ None,
        "first line\nsecond line with **markdown**",
    );
    let catalog = TraceRepository::new(temp.path().to_path_buf())
        .discover()
        .await;
    let trace = catalog.load_session(ROOT_ID).await.unwrap();
    let mut app = App::loading(
        /*preferred_session*/ None, /*auto_open_rich*/ false,
    );
    app.install_session(trace);

    move_to_kind(&mut app, TraceNodeKind::Thread);
    app.handle_key(key(KeyCode::Enter));
    assert_snapshot!("record_level_with_preview", render_app(&mut app, 150, 24));

    move_to_kind(&mut app, TraceNodeKind::RolloutRecord);
    app.handle_key(key(KeyCode::Down));
    app.handle_key(key(KeyCode::Char('i')));
    assert_snapshot!("rendered_record_detail", render_app(&mut app, 100, 24));
    app.handle_key(key(KeyCode::Char('v')));
    app.handle_key(key(KeyCode::Char('v')));
    assert_snapshot!(
        "normalized_raw_record_detail",
        render_app(&mut app, 100, 24)
    );
    app.handle_key(key(KeyCode::Esc));

    app.handle_key(key(KeyCode::Char('f')));
    assert_snapshot!("record_filter_overlay", render_app(&mut app, 100, 28));
    app.handle_key(key(KeyCode::Char(' ')));
    app.handle_key(key(KeyCode::Enter));
    let Screen::Browser(browser) = &app.screen else {
        panic!("expected browser");
    };
    assert_eq!(browser.row_count(), 1);
    app.handle_key(key(KeyCode::Char('F')));
    app.handle_key(key(KeyCode::Char('?')));
    assert_snapshot!("trace_help_overlay", render_app(&mut app, 100, 24));
}

#[tokio::test]
async fn view_options_control_columns_headers_and_previews() {
    let temp = TempDir::new().unwrap();
    write_rollout(
        temp.path(),
        ROOT_ID,
        ROOT_ID,
        /*parent*/ None,
        "PREVIEW-TOKEN",
    );
    let catalog = TraceRepository::new(temp.path().to_path_buf())
        .discover()
        .await;
    let trace = catalog.load_session(ROOT_ID).await.unwrap();
    let mut app = App::loading_with_visuals(
        /*preferred_session*/ None,
        /*auto_open_rich*/ false,
        TraceViewOptions {
            columns: vec![TraceColumn::Class],
            headers: HeaderMode::Never,
            preview: PreviewMode::Never,
            ..TraceViewOptions::default()
        },
        Arc::new(PlainTraceVisualRenderer),
    );
    app.install_session(trace);
    move_to_kind(&mut app, TraceNodeKind::Thread);
    app.handle_key(key(KeyCode::Enter));

    let rendered = render_app(&mut app, 100, 20);
    assert!(!rendered.contains("NAME"));
    assert!(!rendered.contains("PREVIEW-TOKEN"));
    assert!(rendered.contains("user"));
}

#[tokio::test]
async fn overview_fields_are_rendered_as_single_terminal_lines() {
    let temp = TempDir::new().unwrap();
    write_rollout(
        temp.path(),
        ROOT_ID,
        ROOT_ID,
        /*parent*/ None,
        "semantic content",
    );
    let catalog = TraceRepository::new(temp.path().to_path_buf())
        .discover()
        .await;
    let mut trace = catalog.load_session(ROOT_ID).await.unwrap();
    let thread = trace
        .nodes
        .iter_mut()
        .find(|node| node.locator.kind == TraceNodeKind::Thread)
        .unwrap();
    thread.label = "thread\ncontinued\t\u{1b}[31m".to_string();
    thread.presentation.preview = Some("preview\ncontinued\t\u{7}".to_string());
    let mut app = App::loading(
        /*preferred_session*/ None, /*auto_open_rich*/ false,
    );
    app.install_session(trace);

    let rendered = render_app(&mut app, 100, 12);

    assert!(!rendered.contains('\u{1b}'));
    assert!(!rendered.contains('\u{7}'));
    assert_snapshot!("single_line_overview_controls", rendered);
}

#[tokio::test]
async fn stale_detail_render_cannot_replace_a_newer_content_mode() {
    let temp = TempDir::new().unwrap();
    write_rollout(
        temp.path(),
        ROOT_ID,
        ROOT_ID,
        /*parent*/ None,
        "semantic content",
    );
    let catalog = TraceRepository::new(temp.path().to_path_buf())
        .discover()
        .await;
    let trace = catalog.load_session(ROOT_ID).await.unwrap();
    let mut browser = super::browser::BrowserState::new(trace);
    open_record_detail(&mut browser);

    assert!(browser.detail_lines(/*width*/ 40).is_empty());
    let rendered_job = browser.take_detail_render_job().unwrap();
    browser.cycle_content_mode();
    assert!(browser.detail_lines(/*width*/ 40).is_empty());
    let text_job = browser.take_detail_render_job().unwrap();

    browser.install_detail_render(rendered_job.run());
    assert!(browser.detail_lines(/*width*/ 40).is_empty());
    browser.install_detail_render(text_job.run());
    assert!(!browser.detail_lines(/*width*/ 40).is_empty());
}

#[tokio::test]
async fn all_record_search_temporarily_reveals_a_filtered_hit() {
    let temp = TempDir::new().unwrap();
    write_rollout(
        temp.path(),
        ROOT_ID,
        ROOT_ID,
        /*parent*/ None,
        "visible user content",
    );
    let catalog = TraceRepository::new(temp.path().to_path_buf())
        .discover()
        .await;
    let trace = catalog.load_session(ROOT_ID).await.unwrap();
    let mut browser = super::browser::BrowserState::new(trace);
    browser.enter_selected();
    browser.toggle_filter_class();
    assert_eq!(browser.row_count(), 1);

    browser.begin_search(super::browser::SearchScope::All);
    for character in "session metadata".chars() {
        browser.search_push(character);
    }
    browser.accept_search();
    let job = browser.take_search_job().unwrap();
    browser.install_search(job.run());
    browser.accept_search();
    assert_eq!(
        browser.selected_node().unwrap().presentation.class,
        codex_trace::TraceRecordClass::Structure
    );
    assert_eq!(browser.row_count(), 2);

    browser.move_vertical(/*delta*/ 1);
    assert_eq!(browser.row_count(), 1);
}

#[tokio::test]
async fn stale_search_result_cannot_replace_a_newer_query() {
    let temp = TempDir::new().unwrap();
    write_rollout(
        temp.path(),
        ROOT_ID,
        ROOT_ID,
        /*parent*/ None,
        "first needle second",
    );
    let catalog = TraceRepository::new(temp.path().to_path_buf())
        .discover()
        .await;
    let trace = catalog.load_session(ROOT_ID).await.unwrap();
    let mut browser = super::browser::BrowserState::new(trace);
    browser.begin_search(super::browser::SearchScope::Visible);
    for character in "first".chars() {
        browser.search_push(character);
    }
    browser.accept_search();
    let stale_job = browser.take_search_job().unwrap();
    for character in " needle".chars() {
        browser.search_push(character);
    }
    browser.accept_search();
    let current_job = browser.take_search_job().unwrap();

    browser.install_search(stale_job.run());
    assert!(browser.search.as_ref().unwrap().loading);
    browser.install_search(current_job.run());
    let search = browser.search.as_ref().unwrap();
    assert!(!search.loading);
    assert_eq!(search.completed_query.as_deref(), Some("first needle"));
}

#[tokio::test]
async fn results_from_a_replaced_browser_are_rejected_for_the_same_session() {
    let temp = TempDir::new().unwrap();
    write_rollout(
        temp.path(),
        ROOT_ID,
        ROOT_ID,
        /*parent*/ None,
        "same session needle",
    );
    let catalog = TraceRepository::new(temp.path().to_path_buf())
        .discover()
        .await;
    let trace = catalog.load_session(ROOT_ID).await.unwrap();

    let mut previous = browser_with_epoch(trace.clone(), 1);
    previous.begin_search(super::browser::SearchScope::Visible);
    for character in "needle".chars() {
        previous.search_push(character);
    }
    previous.accept_search();
    let stale_search = previous.take_search_job().unwrap().run();
    let mut current = browser_with_epoch(trace.clone(), 2);
    current.begin_search(super::browser::SearchScope::Visible);
    for character in "needle".chars() {
        current.search_push(character);
    }
    current.accept_search();
    let current_search = current.take_search_job().unwrap().run();
    current.install_search(stale_search);
    assert!(current.search.as_ref().unwrap().loading);
    current.install_search(current_search);
    assert!(!current.search.as_ref().unwrap().loading);

    let mut previous = browser_with_epoch(trace.clone(), 3);
    open_record_detail(&mut previous);
    assert!(previous.detail_lines(/*width*/ 40).is_empty());
    let stale_detail = previous.take_detail_render_job().unwrap().run();
    let mut current = browser_with_epoch(trace, 4);
    open_record_detail(&mut current);
    assert!(current.detail_lines(/*width*/ 40).is_empty());
    let current_detail = current.take_detail_render_job().unwrap().run();
    current.install_detail_render(stale_detail);
    assert!(current.detail_lines(/*width*/ 40).is_empty());
    current.install_detail_render(current_detail);
    assert!(!current.detail_lines(/*width*/ 40).is_empty());
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
    let mut trace = catalog.load_session("rollout-ui").await.unwrap();
    for node in &mut trace.nodes {
        node.timestamp = node
            .timestamp
            .as_ref()
            .map(|_| "2026-07-28T00:00:00Z".to_string());
    }
    let mut app = App::loading(
        /*preferred_session*/ None, /*auto_open_rich*/ false,
    );
    app.install_session(trace);
    app.handle_key(key(KeyCode::Tab));
    app.handle_key(key(KeyCode::Tab));
    app.handle_key(key(KeyCode::Enter));
    app.handle_key(key(KeyCode::Down));
    assert_snapshot!("raw_payload_collapsed", render_app(&mut app, 100, 24));
    let AppAction::ReadPayload {
        browser_epoch,
        id,
        handle,
        limit,
    } = app.handle_key(key(KeyCode::Enter))
    else {
        panic!("expected lazy raw payload request");
    };
    let observed = handle.read(limit).await.unwrap();
    app.install_payload(
        super::request::BrowserEpoch::new(999),
        id.clone(),
        Ok(SanitizedPayload {
            text: "stale payload".to_string(),
            truncated: false,
            original_bytes_read: 13,
        }),
    );
    assert!(render_app(&mut app, 100, 24).contains("loading exact raw artifact"));
    app.install_payload(
        browser_epoch,
        id.clone(),
        Ok(SanitizedPayload {
            text: format!(
                "{}TAIL-MUST-NOT-BE-EAGER",
                "windowed payload\n".repeat(100_000)
            ),
            truncated: true,
            original_bytes_read: observed.original_bytes_read,
        }),
    );
    app.handle_key(key(KeyCode::Char('v')));
    app.handle_key(key(KeyCode::Char('v')));
    let Screen::Browser(browser) = &mut app.screen else {
        panic!("expected browser");
    };
    browser.detail_scroll = 20;
    let large_render = render_app(&mut app, 100, 24);
    assert!(!large_render.contains("TAIL-MUST-NOT-BE-EAGER"));
    app.install_payload(
        browser_epoch,
        id,
        Err(anyhow::anyhow!("bad \u{1b}[31m payload\u{7} path")),
    );
    let rendered = render_app(&mut app, 100, 24);
    assert!(!rendered.contains('\u{1b}'));
    assert_snapshot!("raw_payload_failure_sanitized", rendered);
    assert_eq!(snapshot_files(&bundle), before);
}

#[tokio::test]
#[ignore = "manual deterministic 100k-node responsiveness profile"]
async fn profile_hundred_thousand_node_navigation() {
    const EVENT_COUNT: usize = 100_000;
    const WARM_ACTIONS: usize = 1_000;
    const LEVEL_ACTIONS: usize = 20;

    let temp = TempDir::new().unwrap();
    write_large_rollout(temp.path(), ROOT_ID, EVENT_COUNT);
    let catalog = TraceRepository::new(temp.path().to_path_buf())
        .discover()
        .await;
    let trace = catalog.load_session(ROOT_ID).await.unwrap();
    assert_eq!(trace.nodes.len(), 100_000);

    let build_started = Instant::now();
    let mut browser = super::browser::BrowserState::new(trace);
    let build_elapsed = build_started.elapsed();
    assert!(browser.row_count() > 50_000);

    let expanded_started = Instant::now();
    browser.cycle_lens(/*reverse*/ false);
    let expanded_elapsed = expanded_started.elapsed();
    assert!(browser.row_count() > 50_000);
    let structural_started = Instant::now();
    browser.cycle_lens(/*reverse*/ false);
    let structural_elapsed = structural_started.elapsed();
    assert!(browser.row_count() > 50_000);

    let mut samples = Vec::with_capacity(WARM_ACTIONS);
    for _ in 0..WARM_ACTIONS {
        let started = Instant::now();
        browser.move_vertical(/*delta*/ 1);
        samples.push(started.elapsed());
    }
    samples.sort_unstable();
    let navigation_p95 = percentile_95(&samples);

    browser.cycle_lens(/*reverse*/ false);
    browser.first();
    let mut entry_samples = Vec::with_capacity(LEVEL_ACTIONS);
    let mut return_samples = Vec::with_capacity(LEVEL_ACTIONS);
    for _ in 0..LEVEL_ACTIONS {
        let entry_started = Instant::now();
        browser.enter_selected();
        entry_samples.push(entry_started.elapsed());
        let return_started = Instant::now();
        browser.back();
        return_samples.push(return_started.elapsed());
    }
    entry_samples.sort_unstable();
    return_samples.sort_unstable();
    let entry_p95 = percentile_95(&entry_samples);
    let return_p95 = percentile_95(&return_samples);
    eprintln!(
        "trace-profile nodes=100000 build_ms={:.3} expanded_ms={:.3} structural_ms={:.3} navigation_p95_ms={:.3} group_entry_p95_ms={:.3} return_p95_ms={:.3}",
        duration_ms(build_elapsed),
        duration_ms(expanded_elapsed),
        duration_ms(structural_elapsed),
        duration_ms(navigation_p95),
        duration_ms(entry_p95),
        duration_ms(return_p95),
    );
    assert!(
        navigation_p95 < Duration::from_millis(/*millis*/ 50),
        "warm navigation p95 was {navigation_p95:?}"
    );
    assert!(
        entry_p95 < Duration::from_millis(/*millis*/ 50),
        "warm level entry p95 was {entry_p95:?}"
    );
    assert!(
        return_p95 < Duration::from_millis(/*millis*/ 50),
        "warm level return p95 was {return_p95:?}"
    );
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

fn render_app(app: &mut App, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| render::render(frame, app)).unwrap();
    let mut redraw = false;
    if let Some(job) = app.take_detail_render_job() {
        app.install_detail_render(job.run());
        redraw = true;
    }
    if let Some(job) = app.take_search_job() {
        app.install_search(job.run());
        redraw = true;
    }
    if redraw {
        terminal.draw(|frame| render::render(frame, app)).unwrap();
    }
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

fn complete_search(app: &mut App) {
    let job = app.take_search_job().expect("expected search job");
    app.install_search(job.run());
}

fn browser_with_epoch(
    trace: codex_trace::SessionTrace,
    epoch: u64,
) -> super::browser::BrowserState {
    super::browser::BrowserState::with_visuals(
        trace,
        TraceViewOptions::default(),
        Arc::new(PlainTraceVisualRenderer),
        super::request::BrowserEpoch::new(epoch),
    )
}

fn open_record_detail(browser: &mut super::browser::BrowserState) {
    move_browser_to_kind(browser, TraceNodeKind::Thread);
    browser.enter_selected();
    browser.last();
    browser.open_detail();
}

fn move_to_kind(app: &mut App, kind: TraceNodeKind) {
    let Screen::Browser(browser) = &mut app.screen else {
        panic!("expected browser");
    };
    move_browser_to_kind(browser, kind);
}

fn move_browser_to_kind(browser: &mut super::browser::BrowserState, kind: TraceNodeKind) {
    while browser.lens() != super::TraceLens::Structural {
        browser.cycle_lens(/*reverse*/ false);
    }
    if kind == TraceNodeKind::Thread {
        while browser.back() {}
    }
    loop {
        let position = (0..browser.row_count()).find(|index| {
            browser
                .rows_window(*index, 1)
                .first()
                .and_then(|row| browser.row_node(row))
                .is_some_and(|node| node.locator.kind == kind)
        });
        if let Some(position) = position {
            browser.first();
            browser.move_vertical(isize::try_from(position).unwrap());
            return;
        }
        if !browser.back() {
            panic!("expected {kind:?} row");
        }
    }
}

fn write_rollout(
    codex_home: &Path,
    session_id: &str,
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
                "session_id": session_id,
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

fn write_large_rollout(codex_home: &Path, id: &str, event_count: usize) {
    let directory = codex_home.join("sessions/2026/07/25");
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join(format!("rollout-2026-07-25T00-00-00-{id}.jsonl"));
    let file = File::create(path).unwrap();
    let mut writer = BufWriter::new(file);
    serde_json::to_writer(
        &mut writer,
        &json!({
            "timestamp": "2026-07-25T00:00:00Z",
            "type": "session_meta",
            "payload": {
                "session_id": id,
                "id": id,
                "parent_thread_id": null,
                "timestamp": "2026-07-25T00:00:00Z",
                "cwd": "/synthetic/large-workspace",
                "originator": "codex-trace-tui-profile",
                "cli_version": "0.0.0",
                "source": "cli",
                "model_provider": "synthetic",
                "base_instructions": null
            }
        }),
    )
    .unwrap();
    writer.write_all(b"\n").unwrap();
    for sequence in 0..event_count {
        serde_json::to_writer(
            &mut writer,
            &json!({
                "timestamp": "2026-07-25T00:00:01Z",
                "type": "event_msg",
                "payload": {
                    "type": "user_message",
                    "message": format!("synthetic event {sequence:06}"),
                    "kind": "plain"
                }
            }),
        )
        .unwrap();
        writer.write_all(b"\n").unwrap();
    }
    writer.flush().unwrap();
}

fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn percentile_95(samples: &[Duration]) -> Duration {
    samples[(samples.len() * 95 / 100).min(samples.len() - 1)]
}
