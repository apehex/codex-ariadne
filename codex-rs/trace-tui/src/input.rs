//! Key dispatch for the single-depth trace browser.

use codex_trace::PayloadReadLimit;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;

use crate::ContentMode;
use crate::app::AppAction;
use crate::browser::BrowserState;
use crate::browser::SearchScope;

/// Applies one browser key and reports whether navigation should return to the picker.
pub(crate) fn handle_browser_key(browser: &mut BrowserState, key: KeyEvent) -> (AppAction, bool) {
    if browser.help_open {
        let action = match key.code {
            KeyCode::Char('q') => AppAction::Quit,
            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Enter => {
                browser.help_open = false;
                AppAction::None
            }
            _ => AppAction::None,
        };
        return (action, false);
    }
    if browser.search.is_some() {
        let action = match key.code {
            KeyCode::Esc => {
                browser.cancel_search();
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
                browser.move_search(/*delta*/ 1);
                AppAction::None
            }
            KeyCode::Up => {
                browser.move_search(/*delta*/ -1);
                AppAction::None
            }
            KeyCode::Char(character) => {
                browser.search_push(character);
                AppAction::None
            }
            _ => AppAction::None,
        };
        return (action, false);
    }
    if browser.filter_open {
        let action = match key.code {
            KeyCode::Esc => {
                browser.filter_open = false;
                AppAction::None
            }
            KeyCode::Enter => {
                browser.apply_filter();
                AppAction::None
            }
            KeyCode::Char(' ') => {
                browser.toggle_filter_class();
                AppAction::None
            }
            KeyCode::Char('r') => {
                browser.reset_filter();
                AppAction::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                browser.move_filter(/*delta*/ 1);
                AppAction::None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                browser.move_filter(/*delta*/ -1);
                AppAction::None
            }
            KeyCode::Char('q') => AppAction::Quit,
            _ => AppAction::None,
        };
        return (action, false);
    }
    if browser.detail_open {
        let action = match key.code {
            KeyCode::Char('q') => AppAction::Quit,
            KeyCode::Esc | KeyCode::Backspace => {
                browser.back();
                AppAction::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                browser.scroll_detail(/*delta*/ 1);
                AppAction::None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                browser.scroll_detail(/*delta*/ -1);
                AppAction::None
            }
            KeyCode::PageDown => {
                browser
                    .scroll_detail(isize::try_from(browser.detail_page_size).unwrap_or(isize::MAX));
                AppAction::None
            }
            KeyCode::PageUp => {
                browser.scroll_detail(
                    -isize::try_from(browser.detail_page_size).unwrap_or(isize::MAX),
                );
                AppAction::None
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                browser.scroll_detail(
                    isize::try_from(browser.detail_page_size.div_ceil(2)).unwrap_or(isize::MAX),
                );
                AppAction::None
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                browser.scroll_detail(
                    -isize::try_from(browser.detail_page_size.div_ceil(2)).unwrap_or(isize::MAX),
                );
                AppAction::None
            }
            KeyCode::Char('v') => {
                browser.cycle_content_mode();
                if browser.selected_is_raw_payload() && browser.content_mode == ContentMode::Raw {
                    return (read_selected_payload(browser), false);
                }
                AppAction::None
            }
            KeyCode::Char('r') => read_selected_payload(browser),
            KeyCode::Char('?') => {
                browser.help_open = true;
                AppAction::None
            }
            _ => AppAction::None,
        };
        return (action, false);
    }
    if browser.pending_g {
        browser.pending_g = false;
        let action = match key.code {
            KeyCode::Char('g') => {
                browser.first();
                AppAction::None
            }
            KeyCode::Char('/') => {
                browser.begin_search(SearchScope::All);
                AppAction::None
            }
            _ => AppAction::None,
        };
        return (action, false);
    }

    let action = match key.code {
        KeyCode::Char('q') => AppAction::Quit,
        KeyCode::Esc | KeyCode::Backspace => {
            if !browser.back() {
                return (AppAction::None, true);
            }
            AppAction::None
        }
        KeyCode::Char('/') => {
            browser.begin_search(SearchScope::Visible);
            AppAction::None
        }
        KeyCode::Char('f') => {
            browser.filter_open = true;
            AppAction::None
        }
        KeyCode::Char('F') => {
            browser.reset_filter();
            AppAction::None
        }
        KeyCode::Char('?') => {
            browser.help_open = true;
            AppAction::None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            browser.move_vertical(/*delta*/ 1);
            AppAction::None
        }
        KeyCode::Up | KeyCode::Char('k') => {
            browser.move_vertical(/*delta*/ -1);
            AppAction::None
        }
        KeyCode::PageDown => {
            browser.page(/*delta*/ 1, browser.page_size);
            AppAction::None
        }
        KeyCode::PageUp => {
            browser.page(/*delta*/ -1, browser.page_size);
            AppAction::None
        }
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            browser.page(/*delta*/ 1, browser.page_size.div_ceil(2));
            AppAction::None
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            browser.page(/*delta*/ -1, browser.page_size.div_ceil(2));
            AppAction::None
        }
        KeyCode::Home => {
            browser.first();
            AppAction::None
        }
        KeyCode::End | KeyCode::Char('G') => {
            browser.last();
            AppAction::None
        }
        KeyCode::Char('g') => {
            browser.pending_g = true;
            AppAction::None
        }
        KeyCode::Enter => {
            browser.enter_selected();
            if browser.detail_open && browser.selected_is_raw_payload() {
                return (read_selected_payload(browser), false);
            }
            AppAction::None
        }
        KeyCode::Char('i') => {
            browser.open_detail();
            AppAction::None
        }
        KeyCode::Char('n') => {
            browser.jump_search(/*delta*/ 1);
            AppAction::None
        }
        KeyCode::Char('N') => {
            browser.jump_search(/*delta*/ -1);
            AppAction::None
        }
        KeyCode::Char('r') => read_selected_payload(browser),
        _ => AppAction::None,
    };
    (action, false)
}

/// Starts a bounded raw-payload read for the selected record when available.
fn read_selected_payload(browser: &mut BrowserState) -> AppAction {
    browser
        .selected_payload_request()
        .map_or(AppAction::None, |(browser_epoch, id, handle)| {
            AppAction::ReadPayload {
                browser_epoch,
                id,
                handle,
                limit: PayloadReadLimit::DEFAULT,
            }
        })
}
