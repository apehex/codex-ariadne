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

use crate::browser::BrowserPane;
use crate::browser::BrowserState;

#[derive(Debug)]
pub(crate) struct App {
    pub(crate) screen: Screen,
    pub(crate) catalog: Option<TraceCatalog>,
    preferred_session: Option<String>,
    auto_open_rich: bool,
    pub(crate) notice: Option<String>,
}

#[derive(Debug)]
pub(crate) enum Screen {
    Loading(String),
    Picker(PickerState),
    Browser(Box<BrowserState>),
    Error(String),
}

#[derive(Debug, Default)]
pub(crate) struct PickerState {
    pub(crate) selected: usize,
    pub(crate) search: Option<String>,
}

#[derive(Debug)]
pub(crate) enum AppAction {
    None,
    Quit,
    CancelJob,
    LoadSession(String),
    ReadPayload {
        id: String,
        handle: RawPayloadHandle,
        limit: PayloadReadLimit,
    },
}

impl App {
    pub(crate) fn loading(preferred_session: Option<String>, auto_open_rich: bool) -> Self {
        Self {
            screen: Screen::Loading("discovering local traces".to_string()),
            catalog: None,
            preferred_session,
            auto_open_rich,
            notice: None,
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
            self.screen = Screen::Loading(format!("loading session {id}"));
            AppAction::LoadSession(id)
        } else {
            self.screen = Screen::Picker(PickerState::default());
            AppAction::None
        }
    }

    pub(crate) fn install_browser(&mut self, browser: BrowserState) {
        self.notice = None;
        self.screen = Screen::Browser(Box::new(browser));
    }

    #[cfg(test)]
    pub(crate) fn install_session(&mut self, trace: SessionTrace) {
        self.install_browser(BrowserState::new(trace));
    }

    pub(crate) fn install_payload(&mut self, id: String, result: anyhow::Result<SanitizedPayload>) {
        if let Screen::Browser(browser) = &mut self.screen {
            browser.install_payload(id, result);
        }
    }

    pub(crate) fn install_error(&mut self, error: String) {
        if self.catalog.is_some() {
            self.notice = Some(error);
            self.screen = Screen::Picker(PickerState::default());
        } else {
            self.screen = Screen::Error(error);
        }
    }

    pub(crate) fn cancel_loading(&mut self) {
        if self.catalog.is_some() {
            self.notice = Some("loading cancelled".to_string());
            self.screen = Screen::Picker(PickerState::default());
        }
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
            Screen::Picker(picker) => {
                let matches = picker_matches(self.catalog.as_ref(), picker.search.as_deref());
                let session_count = matches.len();
                if picker.search.is_some() {
                    match key.code {
                        KeyCode::Esc => {
                            picker.search = None;
                            picker.selected = 0;
                            return AppAction::None;
                        }
                        KeyCode::Enter => {}
                        KeyCode::Backspace => {
                            if let Some(search) = &mut picker.search {
                                search.pop();
                            }
                            picker.selected = 0;
                            return AppAction::None;
                        }
                        KeyCode::Down => {
                            picker.selected = picker
                                .selected
                                .saturating_add(1)
                                .min(session_count.saturating_sub(1));
                            return AppAction::None;
                        }
                        KeyCode::Up => {
                            picker.selected = picker.selected.saturating_sub(1);
                            return AppAction::None;
                        }
                        KeyCode::Char(ch) => {
                            if let Some(search) = &mut picker.search {
                                search.push(ch);
                            }
                            picker.selected = 0;
                            return AppAction::None;
                        }
                        _ => return AppAction::None,
                    }
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => AppAction::Quit,
                    KeyCode::Down | KeyCode::Char('j') => {
                        picker.selected = picker
                            .selected
                            .saturating_add(1)
                            .min(session_count.saturating_sub(1));
                        AppAction::None
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        picker.selected = picker.selected.saturating_sub(1);
                        AppAction::None
                    }
                    KeyCode::Home | KeyCode::Char('g') => {
                        picker.selected = 0;
                        AppAction::None
                    }
                    KeyCode::End | KeyCode::Char('G') => {
                        picker.selected = session_count.saturating_sub(1);
                        AppAction::None
                    }
                    KeyCode::Char('/') => {
                        picker.search = Some(String::new());
                        picker.selected = 0;
                        AppAction::None
                    }
                    KeyCode::Enter => {
                        let id = matches
                            .get(picker.selected)
                            .and_then(|index| {
                                self.catalog
                                    .as_ref()
                                    .and_then(|catalog| catalog.sessions.get(*index))
                            })
                            .map(|session| session.session_id.clone());
                        if let Some(id) = id {
                            self.screen = Screen::Loading(format!("loading session {id}"));
                            AppAction::LoadSession(id)
                        } else {
                            AppAction::None
                        }
                    }
                    _ => AppAction::None,
                }
            }
            Screen::Browser(browser) => handle_browser_key(browser, key),
            Screen::Error(_) => match key.code {
                KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter => AppAction::Quit,
                _ => AppAction::None,
            },
        }
    }
}

fn handle_browser_key(browser: &mut BrowserState, key: KeyEvent) -> AppAction {
    if browser.diagnostics_open {
        return match key.code {
            KeyCode::Esc | KeyCode::Char('d') => {
                browser.diagnostics_open = false;
                AppAction::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                browser.move_diagnostic(1);
                AppAction::None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                browser.move_diagnostic(-1);
                AppAction::None
            }
            KeyCode::Char('q') => AppAction::Quit,
            _ => AppAction::None,
        };
    }
    if browser.search.is_some() {
        return match key.code {
            KeyCode::Esc => {
                browser.search = None;
                AppAction::None
            }
            KeyCode::Enter => {
                browser.accept_search();
                AppAction::None
            }
            KeyCode::Backspace => {
                browser.search_pop();
                AppAction::None
            }
            KeyCode::Down => {
                browser.move_search(1);
                AppAction::None
            }
            KeyCode::Up => {
                browser.move_search(-1);
                AppAction::None
            }
            KeyCode::Char(ch) => {
                browser.search_push(ch);
                AppAction::None
            }
            _ => AppAction::None,
        };
    }
    match key.code {
        KeyCode::Char('q') => AppAction::Quit,
        KeyCode::Esc => {
            browser.pane = BrowserPane::Tree;
            AppAction::None
        }
        KeyCode::Char('/') => {
            browser.begin_search();
            AppAction::None
        }
        KeyCode::Char('d') => {
            browser.diagnostics_open = true;
            AppAction::None
        }
        KeyCode::Tab => {
            browser.pane = browser.pane.next();
            AppAction::None
        }
        KeyCode::BackTab => {
            browser.pane = browser.pane.previous();
            AppAction::None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            browser.move_vertical(1);
            AppAction::None
        }
        KeyCode::Up | KeyCode::Char('k') => {
            browser.move_vertical(-1);
            AppAction::None
        }
        KeyCode::Home | KeyCode::Char('g') => {
            browser.first();
            AppAction::None
        }
        KeyCode::End | KeyCode::Char('G') => {
            browser.last();
            AppAction::None
        }
        KeyCode::Right | KeyCode::Char('l') => {
            browser.expand();
            AppAction::None
        }
        KeyCode::Left | KeyCode::Char('h') => {
            browser.collapse_or_parent();
            AppAction::None
        }
        KeyCode::Backspace => {
            browser.collapse_or_parent();
            AppAction::None
        }
        KeyCode::Enter => {
            if browser.selected_is_raw_payload() {
                read_selected_payload(browser)
            } else {
                browser.expand_or_enter();
                AppAction::None
            }
        }
        KeyCode::Char('n') => {
            browser.jump_search(1);
            AppAction::None
        }
        KeyCode::Char('N') => {
            browser.jump_search(-1);
            AppAction::None
        }
        KeyCode::Char('r') => read_selected_payload(browser),
        _ => AppAction::None,
    }
}

fn read_selected_payload(browser: &mut BrowserState) -> AppAction {
    browser
        .selected_payload_request()
        .map_or(AppAction::None, |(id, handle)| AppAction::ReadPayload {
            id,
            handle,
            limit: PayloadReadLimit::DEFAULT,
        })
}

pub(crate) fn picker_matches(catalog: Option<&TraceCatalog>, query: Option<&str>) -> Vec<usize> {
    let needle = query.unwrap_or_default().trim().to_lowercase();
    catalog.map_or_else(Vec::new, |catalog| {
        catalog
            .sessions
            .iter()
            .enumerate()
            .filter(|(_, session)| {
                needle.is_empty()
                    || format!(
                        "{} {} {}",
                        session.session_id,
                        session
                            .cwd
                            .as_ref()
                            .map_or_else(String::new, |path| path.display().to_string()),
                        session.model_provider.as_deref().unwrap_or_default()
                    )
                    .to_lowercase()
                    .contains(&needle)
            })
            .map(|(index, _)| index)
            .collect()
    })
}

pub(crate) fn pane_name(pane: BrowserPane) -> &'static str {
    match pane {
        BrowserPane::Tree => "tree",
        BrowserPane::Children => "children",
        BrowserPane::Inspector => "inspector",
    }
}
