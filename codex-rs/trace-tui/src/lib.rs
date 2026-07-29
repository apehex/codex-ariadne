//! Offline, read-only Ratatui browser for local Codex execution traces.

#![warn(missing_docs)]

mod app;
mod browser;
mod input;
mod jobs;
mod picker;
mod render;
mod view;

pub use codex_trace::EvidenceGrade;
pub use codex_trace::TraceContentDocument;
pub use codex_trace::TraceContentFormat;
pub use codex_trace::TraceRecordClass;
pub use codex_trace::TraceStatus;
pub use view::ContentMode;
pub use view::HeaderMode;
pub use view::PlainTraceVisualRenderer;
pub use view::PreviewMode;
pub use view::TraceColumn;
pub use view::TraceRenderRequest;
pub use view::TraceRowStyleRequest;
pub use view::TraceViewOptions;
pub use view::TraceVisualRenderer;

use std::io::IsTerminal;
use std::io::stdout;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use clap::Args;
use codex_trace::SanitizedPayload;
use codex_trace::TraceCatalog;
use codex_trace::TraceRepository;
use crossterm::cursor;
use crossterm::event;
use crossterm::execute;
use crossterm::terminal;
use crossterm::terminal::EnterAlternateScreen;
use crossterm::terminal::LeaveAlternateScreen;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use tokio::task::JoinHandle;

use crate::app::App;
use crate::app::AppAction;
use crate::browser::BrowserState;
use crate::jobs::DetailRenderResult;
use crate::jobs::SearchResult;

/// Arguments for the historical trace browser.
#[derive(Debug, Clone, Args)]
pub struct Cli {
    /// Root session ID to open directly.
    #[arg(value_name = "SESSION_ID")]
    pub session_id: Option<String>,

    /// Open one exact rollout-trace bundle directory.
    #[arg(
        long,
        value_name = "PATH",
        conflicts_with_all = ["trace_root", "session_id"]
    )]
    pub bundle: Option<PathBuf>,

    /// Add a directory containing rollout-trace bundles.
    #[arg(long, value_name = "DIR")]
    pub trace_root: Option<PathBuf>,
}

/// Runs the trace browser without initializing authentication, models, or networking.
pub async fn run(cli: Cli, codex_home: PathBuf) -> Result<()> {
    run_with_renderer(
        cli,
        codex_home,
        TraceViewOptions::default(),
        Arc::new(PlainTraceVisualRenderer),
    )
    .await
}

/// Runs the trace browser with host-provided view options and content styling.
pub async fn run_with_renderer(
    cli: Cli,
    codex_home: PathBuf,
    options: TraceViewOptions,
    renderer: Arc<dyn TraceVisualRenderer>,
) -> Result<()> {
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        bail!("codex trace requires an interactive terminal");
    }
    let exact_bundle = cli.bundle.is_some();
    let mut repository = TraceRepository::new(codex_home);
    let explicit_rich_source = cli.trace_root.is_some() || cli.bundle.is_some();
    if let Some(root) = cli.trace_root {
        repository = repository.with_rich_root(normalize_trace_root(&root)?);
    }
    if let Some(bundle) = cli.bundle {
        repository = repository.with_rich_bundle(normalize_bundle_path(&bundle)?);
    }
    if !explicit_rich_source && let Some(root) = std::env::var_os("CODEX_ROLLOUT_TRACE_ROOT") {
        repository = repository.with_rich_root(PathBuf::from(root));
    }

    let mut app = App::loading_with_visuals(cli.session_id, exact_bundle, options, renderer);
    let mut jobs = vec![tokio::spawn(async move {
        WorkerResult::Catalog(repository.discover().await)
    })];
    let mut terminal = initialize_terminal()?;
    let _restore = TerminalRestore;
    let run_result = run_event_loop(&mut terminal, &mut app, &mut jobs).await;
    terminal.show_cursor().context("restore terminal cursor")?;
    run_result
}

async fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
    jobs: &mut Vec<JoinHandle<WorkerResult>>,
) -> Result<()> {
    loop {
        while let Some(index) = jobs.iter().position(JoinHandle::is_finished) {
            let result = jobs
                .swap_remove(index)
                .await
                .context("trace browser worker stopped unexpectedly")?;
            let action = apply_worker_result(app, result);
            if dispatch_action(app, jobs, action) {
                return Ok(());
            }
        }
        terminal
            .draw(|frame| render::render(frame, app))
            .context("render trace browser")?;
        if let Some(job) = app.take_detail_render_job() {
            jobs.push(tokio::task::spawn_blocking(move || {
                WorkerResult::Detail(job.run())
            }));
        }
        if let Some(job) = app.take_search_job() {
            jobs.push(tokio::task::spawn_blocking(move || {
                WorkerResult::Search(job.run())
            }));
        }
        if event::poll(Duration::from_millis(50)).context("poll terminal events")?
            && let event::Event::Key(key) = event::read().context("read terminal event")?
        {
            let action = app.handle_key(key);
            if dispatch_action(app, jobs, action) {
                return Ok(());
            }
        }
    }
}

fn apply_worker_result(app: &mut App, result: WorkerResult) -> AppAction {
    match result {
        WorkerResult::Catalog(catalog) => app.install_catalog(catalog),
        WorkerResult::Session { session_id, result } => match *result {
            Ok(browser) => {
                app.install_browser(&session_id, browser);
                AppAction::None
            }
            Err(error) => {
                app.install_error(format!("cannot load trace session: {error:#}"));
                AppAction::None
            }
        },
        WorkerResult::Payload {
            session_id,
            id,
            result,
        } => {
            app.install_payload(&session_id, id, result);
            AppAction::None
        }
        WorkerResult::Detail(result) => {
            app.install_detail_render(result);
            AppAction::None
        }
        WorkerResult::Search(result) => {
            app.install_search(result);
            AppAction::None
        }
    }
}

fn dispatch_action(
    app: &mut App,
    jobs: &mut Vec<JoinHandle<WorkerResult>>,
    action: AppAction,
) -> bool {
    match action {
        AppAction::None => false,
        AppAction::Quit => {
            for job in jobs.drain(..) {
                job.abort();
            }
            true
        }
        AppAction::CancelJob => {
            for job in jobs.drain(..) {
                job.abort();
            }
            false
        }
        AppAction::LoadSession(id) => {
            let Some(catalog) = app.catalog.clone() else {
                app.install_error("trace catalog is unavailable".to_string());
                return false;
            };
            let (options, renderer) = app.browser_visuals();
            jobs.push(tokio::spawn(async move {
                WorkerResult::Session {
                    session_id: id.clone(),
                    result: Box::new(
                        catalog
                            .load_session(&id)
                            .await
                            .map(|trace| BrowserState::with_visuals(trace, options, renderer)),
                    ),
                }
            }));
            false
        }
        AppAction::ReadPayload {
            session_id,
            id,
            handle,
            limit,
        } => {
            jobs.push(tokio::spawn(async move {
                WorkerResult::Payload {
                    session_id,
                    id,
                    result: handle.read(limit).await,
                }
            }));
            false
        }
    }
}

fn initialize_terminal() -> Result<Terminal<CrosstermBackend<std::io::Stdout>>> {
    terminal::enable_raw_mode().context("enable terminal raw mode")?;
    let mut output = stdout();
    if let Err(error) = execute!(output, EnterAlternateScreen, cursor::Hide) {
        let _ = terminal::disable_raw_mode();
        return Err(error).context("enter alternate terminal screen");
    }
    Terminal::new(CrosstermBackend::new(output)).context("initialize terminal")
}

fn normalize_bundle_path(path: &Path) -> Result<PathBuf> {
    if path.file_name().is_some_and(|name| name == "manifest.json") {
        if !path.is_file() {
            bail!("bundle manifest does not exist: {}", path.display());
        }
        return path
            .parent()
            .map(Path::to_path_buf)
            .context("bundle manifest has no parent directory");
    }
    if !path.exists() {
        bail!("bundle path does not exist: {}", path.display());
    }
    if path.is_file() {
        bail!(
            "bundle path must be a directory or manifest.json: {}",
            path.display()
        );
    }
    if !path.join("manifest.json").is_file() {
        bail!(
            "bundle directory does not contain manifest.json: {}",
            path.display()
        );
    }
    Ok(path.to_path_buf())
}

fn normalize_trace_root(path: &Path) -> Result<PathBuf> {
    if !path.is_dir() {
        bail!("trace root is not a directory: {}", path.display());
    }
    Ok(path.to_path_buf())
}

enum WorkerResult {
    Catalog(TraceCatalog),
    Session {
        session_id: String,
        result: Box<Result<BrowserState>>,
    },
    Payload {
        session_id: String,
        id: String,
        result: Result<SanitizedPayload>,
    },
    Detail(DetailRenderResult),
    Search(SearchResult),
}

struct TerminalRestore;

impl Drop for TerminalRestore {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen, cursor::Show);
    }
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
