//! Bounded browser-local navigation through normalized JSON values.

use serde_json::Value;

pub(super) const MAX_STRUCTURED_DEPTH: usize = 64;
pub(super) const MAX_STRUCTURED_CHILDREN: usize = 4_096;
pub(super) const MAX_STRUCTURED_SCALAR_BYTES: usize = 64 * 1024;
pub(super) const MAX_STRUCTURED_PREVIEW_CHARS: usize = 256;

/// One stable typed component in a normalized JSON value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum JsonPathComponent {
    Key(String),
    Index(usize),
}

/// Browser-local typed path whose display form is an RFC 6901 JSON Pointer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct JsonPath(Vec<JsonPathComponent>);

impl JsonPath {
    pub(super) fn root() -> Self {
        Self::default()
    }

    pub(super) fn child(&self, component: JsonPathComponent) -> Option<Self> {
        if self.0.len() >= MAX_STRUCTURED_DEPTH {
            return None;
        }
        let mut components = self.0.clone();
        components.push(component);
        Some(Self(components))
    }

    pub(super) fn resolve<'a>(&self, root: &'a Value) -> Option<&'a Value> {
        self.0
            .iter()
            .try_fold(root, |value, component| match (value, component) {
                (Value::Object(map), JsonPathComponent::Key(key)) => map.get(key),
                (Value::Array(values), JsonPathComponent::Index(index)) => values.get(*index),
                (
                    Value::Null
                    | Value::Bool(_)
                    | Value::Number(_)
                    | Value::String(_)
                    | Value::Array(_)
                    | Value::Object(_),
                    JsonPathComponent::Key(_) | JsonPathComponent::Index(_),
                ) => None,
            })
    }

    pub(super) fn pointer(&self) -> String {
        let mut pointer = String::new();
        for component in &self.0 {
            pointer.push('/');
            match component {
                JsonPathComponent::Key(key) => {
                    pointer.push_str(&key.replace('~', "~0").replace('/', "~1"));
                }
                JsonPathComponent::Index(index) => pointer.push_str(&index.to_string()),
            }
        }
        pointer
    }

    pub(super) fn at_max_depth(&self) -> bool {
        self.0.len() >= MAX_STRUCTURED_DEPTH
    }
}

pub(super) fn value_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

pub(super) fn preview(value: &Value) -> String {
    let text = match value {
        Value::String(text) => text.clone(),
        Value::Array(values) => format!("{} items", values.len()),
        Value::Object(map) => format!("{} fields", map.len()),
        Value::Null | Value::Bool(_) | Value::Number(_) => value.to_string(),
    };
    let mut chars = text.chars();
    let mut preview = chars
        .by_ref()
        .take(MAX_STRUCTURED_PREVIEW_CHARS)
        .collect::<String>();
    if chars.next().is_some() {
        preview.push('…');
    }
    preview
}

pub(super) fn bounded_scalar(value: &Value) -> (String, bool) {
    let mut text = match value {
        Value::String(text) => text.clone(),
        Value::Null | Value::Bool(_) | Value::Number(_) => value.to_string(),
        Value::Array(_) | Value::Object(_) => return (String::new(), false),
    };
    if text.len() <= MAX_STRUCTURED_SCALAR_BYTES {
        return (text, false);
    }
    let mut boundary = MAX_STRUCTURED_SCALAR_BYTES;
    while !text.is_char_boundary(boundary) {
        boundary = boundary.saturating_sub(1);
    }
    text.truncate(boundary);
    (text, true)
}

#[cfg(test)]
#[path = "structured_tests.rs"]
mod tests;
