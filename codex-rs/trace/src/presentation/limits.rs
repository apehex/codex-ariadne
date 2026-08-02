//! Typed resource limits for immutable presentation snapshots.

use serde::Deserialize;
use serde::Serialize;

pub(crate) const DEFAULT_MAX_REFERENCES_PER_GROUP: usize = 4_096;
pub(crate) const DEFAULT_MAX_REFERENCES_PER_NODE: usize = 4;
pub(crate) const DEFAULT_MAX_SUMMARY_BYTES_PER_GROUP: usize = 4 * 1_024;
pub(crate) const DEFAULT_MAX_TOTAL_SUMMARY_BYTES: usize = 16 * 1_024 * 1_024;
pub(crate) const DEFAULT_MAX_PREVIEW_CHARS: usize = 256;
pub(crate) const DEFAULT_MAX_DIAGNOSTICS: usize = 1_024;
pub(crate) const DEFAULT_MAX_DIAGNOSTIC_MESSAGE_CHARS: usize = 4 * 1_024;
pub(crate) const DEFAULT_MAX_GROUP_DEPTH: usize = 4;

/// Caller-selected bounds for one immutable presentation snapshot.
///
/// Values above the built-in safety maxima are clamped. Zero disables the
/// corresponding retained data, except that at least one diagnostic slot is
/// always retained so a bounded build can explain its degradation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresentationLimits {
    /// Maximum secondary references retained by one group.
    pub max_references_per_group: usize,
    /// Maximum total secondary references per canonical trace node.
    pub max_references_per_node: usize,
    /// Maximum summary bytes retained by one group.
    pub max_summary_bytes_per_group: usize,
    /// Maximum summary bytes retained across the snapshot.
    pub max_total_summary_bytes: usize,
    /// Maximum characters retained in one summary preview.
    pub max_preview_chars: usize,
    /// Maximum diagnostics retained by one build.
    pub max_diagnostics: usize,
    /// Maximum characters retained in one diagnostic message.
    pub max_diagnostic_message_chars: usize,
    /// Maximum nested presentation-group depth.
    pub max_group_depth: usize,
}

impl Default for PresentationLimits {
    fn default() -> Self {
        Self {
            max_references_per_group: DEFAULT_MAX_REFERENCES_PER_GROUP,
            max_references_per_node: DEFAULT_MAX_REFERENCES_PER_NODE,
            max_summary_bytes_per_group: DEFAULT_MAX_SUMMARY_BYTES_PER_GROUP,
            max_total_summary_bytes: DEFAULT_MAX_TOTAL_SUMMARY_BYTES,
            max_preview_chars: DEFAULT_MAX_PREVIEW_CHARS,
            max_diagnostics: DEFAULT_MAX_DIAGNOSTICS,
            max_diagnostic_message_chars: DEFAULT_MAX_DIAGNOSTIC_MESSAGE_CHARS,
            max_group_depth: DEFAULT_MAX_GROUP_DEPTH,
        }
    }
}

impl PresentationLimits {
    pub(crate) fn effective(self) -> Self {
        let defaults = Self::default();
        Self {
            max_references_per_group: self
                .max_references_per_group
                .min(defaults.max_references_per_group),
            max_references_per_node: self
                .max_references_per_node
                .min(defaults.max_references_per_node),
            max_summary_bytes_per_group: self
                .max_summary_bytes_per_group
                .min(defaults.max_summary_bytes_per_group),
            max_total_summary_bytes: self
                .max_total_summary_bytes
                .min(defaults.max_total_summary_bytes),
            max_preview_chars: self.max_preview_chars.min(defaults.max_preview_chars),
            max_diagnostics: self.max_diagnostics.clamp(1, defaults.max_diagnostics),
            max_diagnostic_message_chars: self
                .max_diagnostic_message_chars
                .min(defaults.max_diagnostic_message_chars),
            max_group_depth: self.max_group_depth.min(defaults.max_group_depth),
        }
    }
}
