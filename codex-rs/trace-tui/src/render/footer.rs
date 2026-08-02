//! Deterministic left-hint and right-state footer degradation.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use unicode_width::UnicodeWidthStr;

use crate::ContentLayout;
use crate::ContentMode;
use crate::TraceLens;
use crate::app::App;
use crate::app::Screen;
use crate::browser::BrowserState;

pub(super) fn render_footer(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let width = usize::from(area.width);
    let line = match &app.screen {
        Screen::Loading(_) => plain_footer(" Esc cancel  q quit ", width),
        Screen::Picker(picker) if picker.search.is_some() => plain_footer(
            " type to filter  ↑↓ select  Enter open  Esc close search ",
            width,
        ),
        Screen::Picker(_) => plain_footer(" ↑↓/jk select  / filter  Enter open  q quit ", width),
        Screen::Browser(browser) => browser_footer(browser, width),
        Screen::Error(_) => plain_footer(" Enter/q close ", width),
    };
    frame.render_widget(Paragraph::new(line).dim(), area);
}

fn browser_footer(browser: &BrowserState, width: usize) -> Line<'static> {
    let (verbose_left, compact_left) = browser_hints(browser);
    let (offset, maximum) = browser.horizontal_position();
    let mode = mode_name(browser.content_mode);
    let layout = layout_name(browser.options().content_layout);
    let lens = lens_name(browser.lens());
    let right_options = [
        format!("{lens} · {mode} · {layout} · x {offset}/{maximum}"),
        format!(
            "{} · {} · {} · x {offset}/{maximum}",
            lens_abbreviation(browser.lens()),
            mode_abbreviation(browser.content_mode),
            layout_abbreviation(browser.options().content_layout),
        ),
        format!("x {offset}/{maximum}"),
    ];
    compose_footer(&verbose_left, &compact_left, &right_options, width)
}

fn compose_footer(
    verbose_left: &str,
    compact_left: &str,
    right_options: &[String],
    width: usize,
) -> Line<'static> {
    let right = right_options
        .iter()
        .find(|candidate| candidate.width() <= width)
        .cloned()
        .unwrap_or_else(|| right_tail(&right_options[2], width));
    let right_width = right.width();
    let available_left = width.saturating_sub(right_width.saturating_add(1));
    let left = [verbose_left, compact_left]
        .into_iter()
        .find(|candidate| candidate.width() <= available_left)
        .unwrap_or_default();
    let padding = width.saturating_sub(left.width() + right_width);
    Line::from(format!("{left}{}{right}", " ".repeat(padding)))
}

fn browser_hints(browser: &BrowserState) -> (String, String) {
    if browser.help_open {
        (
            "?/Esc close help  q quit".to_string(),
            "Esc close".to_string(),
        )
    } else if browser.search.is_some() {
        (
            "type query  Enter search/open  ↑↓ choose  Esc close".to_string(),
            "Enter search  Esc close".to_string(),
        )
    } else if browser.filter_open {
        (
            "Space toggle  ↑↓ choose  Enter/Esc close  r reset".to_string(),
            "Space toggle  Esc close".to_string(),
        )
    } else if browser.detail_open() {
        (
            "jk scroll  h/l horizontal  v view  Esc back  q quit".to_string(),
            "jk h/l  v view  Esc".to_string(),
        )
    } else if browser.structured_is_scalar() {
        (
            "jk scroll  h/l horizontal  Esc back  q quit".to_string(),
            "jk h/l  Esc".to_string(),
        )
    } else {
        (
            format!(
                "{}/{} visible · {} hidden · {} columns omitted  jk move  h/l horizontal  Enter descend  Tab lens  / search  ? help",
                browser.row_count(),
                browser.row_count() + browser.hidden_count(),
                browser.hidden_count(),
                browser.omitted_columns,
            ),
            "jk h/l  Enter  Tab  ?".to_string(),
        )
    }
}

fn plain_footer(text: &str, width: usize) -> Line<'static> {
    Line::from(super::text::fit(text, width))
}

fn right_tail(text: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let chars = text.chars().collect::<Vec<_>>();
    chars[chars.len().saturating_sub(width)..].iter().collect()
}

fn mode_name(mode: ContentMode) -> &'static str {
    match mode {
        ContentMode::Rendered => "rendered",
        ContentMode::Text => "text",
        ContentMode::Raw => "raw",
    }
}

fn mode_abbreviation(mode: ContentMode) -> &'static str {
    match mode {
        ContentMode::Rendered => "R",
        ContentMode::Text => "T",
        ContentMode::Raw => "Raw",
    }
}

fn layout_name(layout: ContentLayout) -> &'static str {
    match layout {
        ContentLayout::Wrapped => "wrap",
        ContentLayout::Unwrapped => "nowrap",
    }
}

fn layout_abbreviation(layout: ContentLayout) -> &'static str {
    match layout {
        ContentLayout::Wrapped => "W",
        ContentLayout::Unwrapped => "NW",
    }
}

fn lens_name(lens: TraceLens) -> &'static str {
    match lens {
        TraceLens::Collapsed => "collapsed",
        TraceLens::Expanded => "expanded",
        TraceLens::Structural => "structural",
    }
}

fn lens_abbreviation(lens: TraceLens) -> &'static str {
    match lens {
        TraceLens::Collapsed => "C",
        TraceLens::Expanded => "E",
        TraceLens::Structural => "S",
    }
}

#[cfg(test)]
#[path = "footer_tests.rs"]
mod tests;
