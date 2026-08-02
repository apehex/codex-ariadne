//! UTF-8-safe bounds and terminal-safe text normalization.

pub(crate) fn one_line_preview(text: &str, limit: usize) -> String {
    let mut preview = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if preview.chars().count() > limit {
        preview = preview.chars().take(limit).collect();
        preview.push('…');
    }
    preview
}

pub(crate) fn truncate_utf8(text: &str, byte_limit: usize) -> (String, bool) {
    if text.len() <= byte_limit {
        return (text.to_string(), false);
    }
    let mut end = byte_limit.min(text.len());
    while !text.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    (text[..end].to_string(), true)
}

pub(crate) fn truncate_chars(text: &str, char_limit: usize) -> (String, bool) {
    if text.chars().count() <= char_limit {
        (text.to_string(), false)
    } else {
        (text.chars().take(char_limit).collect(), true)
    }
}

pub(crate) fn sanitize_terminal_text(text: &str) -> String {
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
