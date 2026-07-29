use std::sync::Arc;

use codex_trace::PayloadReadLimit;
use codex_trace::RawPayloadHandle;
use codex_trace::SanitizedPayload;
#[cfg(test)]
use codex_trace::SessionTrace;
use codex_trace::TraceCatalog;
use codex_trace::TraceSourceKind;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;

#[cfg(test)]
use crate::PlainTraceVisualRenderer;
use crate::TraceViewOptions;
use crate::TraceVisualRenderer;
use crate::browser::BrowserState;
use crate::input::handle_browser_key;
use crate::jobs::DetailRenderJob;
use crate::jobs::DetailRenderResult;
use crate::jobs::SearchJob;
use crate::jobs::SearchResult;
use crate::picker::PickerAction;
use crate::picker::PickerState;
use crate::request::BrowserEpoch;

pub(crate) struct App {
    pub(crate) screen: Screen,
    pub(crate) catalog: Option<TraceCatalog>,
    preferred_session: Option<String>,
    auto_open_rich: bool,
    pending_session: Option<PendingSession>,
    next_browser_epoch: u64,
    pub(crate) notice: Option<String>,
    options: TraceViewOptions,
    renderer: Arc<dyn TraceVisualRenderer>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingSession {
    session_id: String,
    browser_epoch: BrowserEpoch,
}

#[derive(Debug)]
pub(crate) enum Screen {
    Loading(String),
    Picker(PickerState),
    Browser(Box<BrowserState>),
    Error(String),
}

#[derive(Debug)]
pub(crate) enum AppAction {
    None,
    Quit,
    CancelJob,
    LoadSession {
        session_id: String,
        browser_epoch: BrowserEpoch,
    },
    ReadPayload {
        browser_epoch: BrowserEpoch,
        id: String,
        handle: RawPayloadHandle,
        limit: PayloadReadLimit,
    },
}

impl App {
    #[cfg(test)]
    pub(crate) fn loading(preferred_session: Option<String>, auto_open_rich: bool) -> Self {
        Self::loading_with_visuals(
            preferred_session,
            auto_open_rich,
            TraceViewOptions::default(),
            Arc::new(PlainTraceVisualRenderer),
        )
    }

    pub(crate) fn loading_with_visuals(
        preferred_session: Option<String>,
        auto_open_rich: bool,
        options: TraceViewOptions,
        renderer: Arc<dyn TraceVisualRenderer>,
    ) -> Self {
        Self {
            screen: Screen::Loading("discovering local traces".to_string()),
            catalog: None,
            preferred_session,
            auto_open_rich,
            pending_session: None,
            next_browser_epoch: 0,
            notice: None,
            options,
            renderer,
        }
    }

    pub(crate) fn install_catalog(&mut self, catalog: TraceCatalog) -> AppAction {
        let selected_id = self.preferred_session.take().or_else(|| {
            self.auto_open_rich
                .then(|| {
                    catalog
                        .sessions
                        .iter()
                        .filter(|session| session.source != TraceSourceKind::Ordinary)
                        .map(|session| session.session_id.clone())
                        .collect::<Vec<_>>()
                })
                .and_then(|sessions| (sessions.len() == 1).then(|| sessions[0].clone()))
        });
        if self.auto_open_rich && selected_id.is_none() && !catalog.sessions.is_empty() {
            self.notice = Some(
                "the exact bundle could not be identified uniquely; select a rich session"
                    .to_string(),
            );
        }
        let valid_id = selected_id.filter(|id| {
            if catalog
                .sessions
                .iter()
                .any(|session| session.session_id == *id)
            {
                true
            } else {
                self.notice = Some(format!("trace session {id} was not found"));
                false
            }
        });
        self.catalog = Some(catalog);
        if let Some(id) = valid_id {
            self.begin_session_load(id)
        } else {
            self.screen = Screen::Picker(PickerState::new(self.catalog.as_ref()));
            AppAction::None
        }
    }

    pub(crate) fn install_browser(
        &mut self,
        session_id: &str,
        browser_epoch: BrowserEpoch,
        browser: BrowserState,
    ) {
        let expected = PendingSession {
            session_id: session_id.to_string(),
            browser_epoch,
        };
        if self.pending_session.as_ref() != Some(&expected) || browser.epoch() != browser_epoch {
            return;
        }
        self.pending_session = None;
        self.notice = None;
        self.screen = Screen::Browser(Box::new(browser));
    }

    pub(crate) fn browser_visuals(&self) -> (TraceViewOptions, Arc<dyn TraceVisualRenderer>) {
        (self.options.clone(), Arc::clone(&self.renderer))
    }

    #[cfg(test)]
    pub(crate) fn install_session(&mut self, trace: SessionTrace) {
        let session_id = trace.summary.session_id.clone();
        let browser_epoch = self.next_browser_epoch();
        self.pending_session = Some(PendingSession {
            session_id: session_id.clone(),
            browser_epoch,
        });
        self.install_browser(
            &session_id,
            browser_epoch,
            BrowserState::with_visuals(
                trace,
                self.options.clone(),
                Arc::clone(&self.renderer),
                browser_epoch,
            ),
        );
    }

    pub(crate) fn install_payload(
        &mut self,
        browser_epoch: BrowserEpoch,
        id: String,
        result: anyhow::Result<SanitizedPayload>,
    ) {
        if let Screen::Browser(browser) = &mut self.screen {
            browser.install_payload(browser_epoch, id, result);
        }
    }

    pub(crate) fn take_detail_render_job(&mut self) -> Option<DetailRenderJob> {
        match &mut self.screen {
            Screen::Browser(browser) => browser.take_detail_render_job(),
            Screen::Loading(_) | Screen::Picker(_) | Screen::Error(_) => None,
        }
    }

    pub(crate) fn install_detail_render(&mut self, result: DetailRenderResult) {
        if let Screen::Browser(browser) = &mut self.screen {
            browser.install_detail_render(result);
        }
    }

    pub(crate) fn take_search_job(&mut self) -> Option<SearchJob> {
        match &mut self.screen {
            Screen::Browser(browser) => browser.take_search_job(),
            Screen::Loading(_) | Screen::Picker(_) | Screen::Error(_) => None,
        }
    }

    pub(crate) fn install_search(&mut self, result: SearchResult) {
        if let Screen::Browser(browser) = &mut self.screen {
            browser.install_search(result);
        }
    }

    pub(crate) fn install_error(&mut self, error: String) {
        self.pending_session = None;
        if self.catalog.is_some() {
            self.notice = Some(error);
            self.screen = Screen::Picker(PickerState::new(self.catalog.as_ref()));
        } else {
            self.screen = Screen::Error(error);
        }
    }

    pub(crate) fn cancel_loading(&mut self) {
        self.pending_session = None;
        if self.catalog.is_some() {
            self.notice = Some("loading cancelled".to_string());
            self.screen = Screen::Picker(PickerState::new(self.catalog.as_ref()));
        }
    }

    /// Begins one uniquely identified selected-session load.
    fn begin_session_load(&mut self, session_id: String) -> AppAction {
        let browser_epoch = self.next_browser_epoch();
        self.pending_session = Some(PendingSession {
            session_id: session_id.clone(),
            browser_epoch,
        });
        self.screen = Screen::Loading(format!("loading session {session_id}"));
        AppAction::LoadSession {
            session_id,
            browser_epoch,
        }
    }

    /// Allocates an epoch that is never reused by this application instance.
    fn next_browser_epoch(&mut self) -> BrowserEpoch {
        self.next_browser_epoch = self.next_browser_epoch.wrapping_add(1);
        BrowserEpoch::new(self.next_browser_epoch)
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> AppAction {
        if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            return AppAction::None;
        }
        match &mut self.screen {
            Screen::Loading(_) => match key.code {
                KeyCode::Char('q') => AppAction::Quit,
                KeyCode::Esc => {
                    if self.catalog.is_some() {
                        self.cancel_loading();
                        AppAction::CancelJob
                    } else {
                        AppAction::Quit
                    }
                }
                _ => AppAction::None,
            },
            Screen::Picker(picker) => match picker.handle_key(key.code) {
                PickerAction::Stay => AppAction::None,
                PickerAction::Quit => AppAction::Quit,
                PickerAction::Open(index) => self
                    .catalog
                    .as_ref()
                    .and_then(|catalog| catalog.sessions.get(index))
                    .map(|session| session.session_id.clone())
                    .map_or(AppAction::None, |id| self.begin_session_load(id)),
            },
            Screen::Browser(browser) => {
                let (action, return_to_picker) = handle_browser_key(browser, key);
                if return_to_picker {
                    self.screen = Screen::Picker(PickerState::new(self.catalog.as_ref()));
                }
                action
            }
            Screen::Error(_) => match key.code {
                KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter => AppAction::Quit,
                _ => AppAction::None,
            },
        }
    }
}
