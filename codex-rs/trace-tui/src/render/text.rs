//! Terminal-safe single-line and display-width text primitives.

use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

/// Neutralizes controls while guaranteeing one logical terminal line.
pub(super) fn single_line(text: &str) -> String {
    text.chars()
        .map(|character| match character {
            '\n' | '\r' | '\t' => ' ',
            character if character.is_control() => '�',
            character => character,
        })
        .collect()
}

/// Neutralizes controls while preserving intentional multiline layout.
pub(super) fn multiline(text: &str) -> String {
    text.chars()
        .map(|character| {
            if matches!(character, '\n' | '\t') || !character.is_control() {
                character
            } else {
                '�'
            }
        })
        .collect()
}

/// Fits text to one terminal display width using an ellipsis when needed.
pub(super) fn fit(text: &str, width: usize) -> String {
    if UnicodeWidthStr::width(text) <= width {
        return text.to_string();
    }
    if width <= 1 {
        return "…".chars().take(width).collect();
    }
    let target = width - 1;
    let mut used = 0;
    let mut fitted = String::new();
    for character in text.chars() {
        let character_width = character.width().unwrap_or_default();
        if used + character_width > target {
            break;
        }
        fitted.push(character);
        used += character_width;
    }
    fitted.push('…');
    fitted
}

/// Fits and right-pads text to one exact terminal display width.
pub(super) fn pad_fit(text: &str, width: usize) -> String {
    let mut fitted = fit(text, width);
    fitted.push_str(&" ".repeat(width.saturating_sub(UnicodeWidthStr::width(fitted.as_str()))));
    fitted
}

#[cfg(test)]
#[path = "text_tests.rs"]
mod tests;
