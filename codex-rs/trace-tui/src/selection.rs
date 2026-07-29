//! Reusable bounded selection behavior for terminal lists.

/// One index kept valid against a caller-owned collection length.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Selection(usize);

impl Selection {
    /// Returns the selected zero-based index.
    pub(crate) fn index(self) -> usize {
        self.0
    }

    /// Replaces the selection and clamps it to the collection.
    pub(crate) fn set(&mut self, index: usize, len: usize) {
        self.0 = index.min(len.saturating_sub(1));
    }

    /// Selects the first item, or zero for an empty collection.
    pub(crate) fn first(&mut self) {
        self.0 = 0;
    }

    /// Selects the last retained item, or zero for an empty collection.
    pub(crate) fn last(&mut self, len: usize) {
        self.0 = len.saturating_sub(1);
    }

    /// Moves by a signed amount without leaving the collection.
    pub(crate) fn move_clamped(&mut self, delta: isize, len: usize) {
        if len == 0 {
            self.0 = 0;
            return;
        }
        self.0 = self
            .0
            .saturating_add_signed(delta)
            .min(len.saturating_sub(1));
    }

    /// Moves cyclically through a non-empty collection.
    pub(crate) fn move_wrapped(&mut self, delta: isize, len: usize) {
        if len == 0 {
            self.0 = 0;
            return;
        }
        self.0 = self.0.wrapping_add_signed(delta) % len;
    }
}

#[cfg(test)]
#[path = "selection_tests.rs"]
mod tests;
