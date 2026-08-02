use crossterm::event::KeyCode;
use insta::assert_snapshot;
use pretty_assertions::assert_eq;

use super::ContentLayout;
use super::TraceLens;
use super::TraceViewOptions;
use super::app::Screen;
use super::phase4_tests::key;
use super::phase4_tests::render_app;
use super::phase4_tests::select_label;
use super::phase4_tests::test_app;
use super::phase4_tests::test_app_with_options;

#[tokio::test]
async fn horizontal_motions_restore_with_parent_and_reset_between_lenses() {
    let message = "a long semantic preview ".repeat(20);
    let mut app = test_app(&[&message, "second event", "third event"]).await;
    render_app(&mut app, 150, 18);

    app.handle_key(key(KeyCode::Char('l')));
    let Screen::Browser(browser) = &app.screen else {
        panic!("expected browser");
    };
    assert_eq!(browser.horizontal_position().0, 4);
    assert!(browser.horizontal_position().1 > 4);
    app.handle_key(key(KeyCode::Char('H')));
    app.handle_key(key(KeyCode::Char('L')));
    let Screen::Browser(browser) = &app.screen else {
        panic!("expected browser");
    };
    assert_eq!(browser.horizontal_position().0, 74);
    app.handle_key(key(KeyCode::Char('0')));
    app.handle_key(key(KeyCode::Char('l')));
    assert_snapshot!("phase5_collapsed_horizontal", render_app(&mut app, 150, 18));

    app.handle_key(key(KeyCode::Enter));
    let Screen::Browser(browser) = &app.screen else {
        panic!("expected browser");
    };
    assert_eq!(browser.horizontal_position().0, 0);
    app.handle_key(key(KeyCode::Esc));
    let Screen::Browser(browser) = &app.screen else {
        panic!("expected browser");
    };
    assert_eq!(browser.horizontal_position().0, 4);

    app.handle_key(key(KeyCode::Tab));
    render_app(&mut app, 150, 18);
    app.handle_key(key(KeyCode::Char('L')));
    let Screen::Browser(browser) = &app.screen else {
        panic!("expected browser");
    };
    assert_eq!(browser.lens(), TraceLens::Expanded);
    assert_eq!(browser.horizontal_position().0, 74);
    assert_snapshot!("phase5_expanded_horizontal", render_app(&mut app, 150, 18));

    app.handle_key(key(KeyCode::Tab));
    render_app(&mut app, 92, 16);
    app.handle_key(key(KeyCode::Char('$')));
    let Screen::Browser(browser) = &app.screen else {
        panic!("expected browser");
    };
    assert_eq!(browser.lens(), TraceLens::Structural);
    assert_eq!(
        browser.horizontal_position().0,
        browser.horizontal_position().1
    );
    assert_snapshot!("phase5_structural_horizontal", render_app(&mut app, 92, 16));
}

#[tokio::test]
async fn wrapped_and_unwrapped_leaf_surfaces_have_independent_horizontal_behavior() {
    let message = "0123456789".repeat(80);
    let mut unwrapped = test_app(&[&message]).await;
    unwrapped.handle_key(key(KeyCode::Enter));
    unwrapped.handle_key(key(KeyCode::Enter));
    render_app(&mut unwrapped, 72, 16);
    complete_detail(&mut unwrapped);
    render_app(&mut unwrapped, 72, 16);
    unwrapped.handle_key(key(KeyCode::Char('L')));
    let Screen::Browser(browser) = &unwrapped.screen else {
        panic!("expected browser");
    };
    assert!(browser.horizontal_position().0 > 0);
    assert_snapshot!(
        "phase5_unwrapped_detail",
        render_app(&mut unwrapped, 72, 16)
    );

    let mut wrapped = test_app_with_options(
        &[&message],
        TraceViewOptions {
            content_layout: ContentLayout::Wrapped,
            ..TraceViewOptions::default()
        },
    )
    .await;
    wrapped.handle_key(key(KeyCode::Enter));
    wrapped.handle_key(key(KeyCode::Enter));
    render_app(&mut wrapped, 72, 16);
    complete_detail(&mut wrapped);
    render_app(&mut wrapped, 72, 16);
    wrapped.handle_key(key(KeyCode::Char('l')));
    let Screen::Browser(browser) = &wrapped.screen else {
        panic!("expected browser");
    };
    assert_eq!(browser.horizontal_position(), (0, 0));
    assert_snapshot!("phase5_wrapped_detail", render_app(&mut wrapped, 72, 16));
}

fn complete_detail(app: &mut super::app::App) {
    let job = app
        .take_detail_render_job()
        .expect("expected detail render");
    app.install_detail_render(job.run());
}

#[tokio::test]
async fn structured_scalar_and_narrow_footer_preserve_navigation_state() {
    let message = "structured horizontal value ".repeat(40);
    let mut app = test_app(&[&message]).await;
    assert_snapshot!("phase5_narrow_footer", render_app(&mut app, 30, 10));

    app.handle_key(key(KeyCode::Char('s')));
    select_label(&mut app, "payload");
    app.handle_key(key(KeyCode::Enter));
    select_label(&mut app, "message");
    app.handle_key(key(KeyCode::Enter));
    render_app(&mut app, 70, 12);
    app.handle_key(key(KeyCode::Char('$')));
    let Screen::Browser(browser) = &app.screen else {
        panic!("expected browser");
    };
    assert!(browser.horizontal_position().0 > 0);
    assert_snapshot!("phase5_structured_horizontal", render_app(&mut app, 70, 12));
}
