use std::fs;
use std::path::Path;

use codex_trace::TraceRepository;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use insta::assert_snapshot;
use pretty_assertions::assert_eq;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use serde_json::json;
use tempfile::TempDir;

use super::PlainTraceVisualRenderer;
use super::TraceLens;
use super::TraceViewOptions;
use super::app::App;
use super::app::Screen;
use super::render;

const ROOT_ID: &str = "019d0000-0000-7000-8000-000000000201";

#[tokio::test]
async fn lenses_and_group_descent_restore_stable_selection_and_viewport() {
    let mut app = test_app(&["first", "second", "third"]).await;
    let Screen::Browser(browser) = &mut app.screen else {
        panic!("expected browser");
    };
    assert_eq!(browser.lens(), TraceLens::Collapsed);
    assert_eq!(browser.row_count(), 3);
    browser.move_vertical(/*delta*/ 1);
    let selected = browser.selected_node().unwrap().locator.clone();
    assert_snapshot!("phase4_collapsed_groups", render_app(&mut app, 110, 18));
    let Screen::Browser(browser) = &mut app.screen else {
        panic!("expected browser");
    };
    browser.viewport = 1;

    app.handle_key(key(KeyCode::Enter));
    let Screen::Browser(browser) = &app.screen else {
        panic!("expected browser");
    };
    assert_eq!(browser.row_count(), 1);
    assert_snapshot!("phase4_entered_group", render_app(&mut app, 110, 18));
    app.handle_key(key(KeyCode::Esc));
    let Screen::Browser(browser) = &app.screen else {
        panic!("expected browser");
    };
    assert_eq!(browser.selected_node().unwrap().locator, selected);
    assert_eq!(browser.viewport, 1);

    app.handle_key(key(KeyCode::Tab));
    let Screen::Browser(browser) = &app.screen else {
        panic!("expected browser");
    };
    assert_eq!(browser.lens(), TraceLens::Expanded);
    assert_eq!(browser.row_count(), 3);
    assert_snapshot!("phase4_expanded_events", render_app(&mut app, 110, 18));

    app.handle_key(key(KeyCode::BackTab));
    let Screen::Browser(browser) = &app.screen else {
        panic!("expected browser");
    };
    assert_eq!(browser.lens(), TraceLens::Collapsed);
    app.handle_key(key(KeyCode::Tab));
    app.handle_key(key(KeyCode::Tab));
    let Screen::Browser(browser) = &app.screen else {
        panic!("expected browser");
    };
    assert_eq!(browser.lens(), TraceLens::Structural);
    assert!(browser.row_count() >= 3);
}

#[tokio::test]
async fn structured_shortcut_descends_by_typed_key_to_interpreted_scalar() {
    let mut app = test_app(&["first line\nsecond line"]).await;
    app.handle_key(key(KeyCode::Char('s')));
    select_label(&mut app, "payload");
    app.handle_key(key(KeyCode::Enter));
    select_label(&mut app, "message");
    app.handle_key(key(KeyCode::Enter));

    let Screen::Browser(browser) = &app.screen else {
        panic!("expected browser");
    };
    assert!(browser.structured_is_scalar());
    assert_eq!(
        browser.structured_scalar(),
        Some(("first line\nsecond line".to_string(), false))
    );
    assert_eq!(
        browser.structured_pointer().as_deref(),
        Some("/payload/message")
    );
    assert_snapshot!("phase4_structured_scalar", render_app(&mut app, 88, 14));

    app.handle_key(key(KeyCode::Esc));
    let Screen::Browser(browser) = &app.screen else {
        panic!("expected browser");
    };
    assert_eq!(browser.structured_pointer().as_deref(), Some("/payload"));
}

#[tokio::test]
async fn filter_overlay_can_reveal_presentation_hidden_groups() {
    let mut app = test_app(&["visible message"]).await;
    let Screen::Browser(browser) = &app.screen else {
        panic!("expected browser");
    };
    assert_eq!(browser.row_count(), 1);

    app.handle_key(key(KeyCode::Char('f')));
    for _ in 0..super::TraceRecordClass::ALL.len() {
        app.handle_key(key(KeyCode::Down));
    }
    app.handle_key(key(KeyCode::Char(' ')));
    app.handle_key(key(KeyCode::Enter));

    let Screen::Browser(browser) = &app.screen else {
        panic!("expected browser");
    };
    assert!(browser.hidden_groups_visible());
    assert_eq!(browser.row_count(), 2);
}

pub(super) fn select_label(app: &mut App, expected: &str) {
    let Screen::Browser(browser) = &mut app.screen else {
        panic!("expected browser");
    };
    let position = (0..browser.row_count())
        .find(|index| {
            browser
                .rows_window(*index, 1)
                .first()
                .is_some_and(|row| browser.row_display(row).label == expected)
        })
        .unwrap_or_else(|| panic!("expected {expected} row"));
    browser.first();
    browser.move_vertical(isize::try_from(position).unwrap());
}

pub(super) async fn test_app(messages: &[&str]) -> App {
    test_app_with_options(messages, TraceViewOptions::default()).await
}

pub(super) async fn test_app_with_options(messages: &[&str], options: TraceViewOptions) -> App {
    let temp = TempDir::new().unwrap();
    write_rollout(temp.path(), messages);
    let trace = TraceRepository::new(temp.path().to_path_buf())
        .discover()
        .await
        .load_session(ROOT_ID)
        .await
        .unwrap();
    let mut app = App::loading_with_visuals(
        /*preferred_session*/ None,
        /*auto_open_rich*/ false,
        options,
        std::sync::Arc::new(PlainTraceVisualRenderer),
    );
    app.install_session(trace);
    app
}

fn write_rollout(codex_home: &Path, messages: &[&str]) {
    let directory = codex_home.join("sessions/2026/08/02");
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join(format!("rollout-2026-08-02T00-00-00-{ROOT_ID}.jsonl"));
    let mut lines = vec![json!({
        "timestamp": "2026-08-02T00:00:00Z",
        "type": "session_meta",
        "payload": {
            "session_id": ROOT_ID,
            "id": ROOT_ID,
            "parent_thread_id": null,
            "timestamp": "2026-08-02T00:00:00Z",
            "cwd": "/synthetic/phase4",
            "originator": "codex-trace-phase4-test",
            "cli_version": "0.0.0",
            "source": "cli",
            "model_provider": "synthetic",
            "base_instructions": null
        }
    })];
    lines.extend(messages.iter().enumerate().map(|(index, message)| {
        json!({
            "timestamp": format!("2026-08-02T00:00:{:02}Z", index + 1),
            "type": "event_msg",
            "payload": {
                "type": "user_message",
                "message": message,
                "kind": "plain"
            }
        })
    }));
    fs::write(
        path,
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
}

pub(super) fn render_app(app: &mut App, width: u16, height: u16) -> String {
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

pub(super) fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::from(code)
}
