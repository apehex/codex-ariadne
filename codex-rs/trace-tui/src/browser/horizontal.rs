//! Orthogonal horizontal viewport state for listing and leaf surfaces.

use crate::ContentLayout;

use super::BrowserState;

/// One conventional horizontal movement requested by input dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HorizontalMotion {
    SmallBackward,
    SmallForward,
    HalfBackward,
    HalfForward,
    Start,
    End,
}

/// Offset and current render extent for one horizontal surface family.
#[derive(Debug, Clone, Copy, Default)]
struct HorizontalViewport {
    offset: usize,
    maximum: usize,
    width: usize,
}

/// Independently restorable listing and leaf horizontal positions.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct HorizontalState {
    overview: HorizontalViewport,
    detail: HorizontalViewport,
}

impl BrowserState {
    /// Moves the focused horizontal surface and reports whether it is navigable.
    pub(crate) fn move_horizontal(&mut self, motion: HorizontalMotion) -> bool {
        if self.leaf_surface() && self.options.content_layout == ContentLayout::Wrapped {
            return false;
        }
        let step = self.options.horizontal_step.columns();
        let viewport = self.focused_horizontal_mut();
        let half_page = viewport.width.div_ceil(2).max(1);
        viewport.offset = match motion {
            HorizontalMotion::SmallBackward => viewport.offset.saturating_sub(step),
            HorizontalMotion::SmallForward => viewport.offset.saturating_add(step),
            HorizontalMotion::HalfBackward => viewport.offset.saturating_sub(half_page),
            HorizontalMotion::HalfForward => viewport.offset.saturating_add(half_page),
            HorizontalMotion::Start => 0,
            HorizontalMotion::End => viewport.maximum,
        }
        .min(viewport.maximum);
        true
    }

    /// Installs the current canvas extent and clamps the focused offset after resize.
    pub(crate) fn update_horizontal_extent(&mut self, canvas_width: usize, viewport_width: usize) {
        let viewport = self.focused_horizontal_mut();
        viewport.width = viewport_width;
        viewport.maximum = canvas_width.saturating_sub(viewport_width);
        viewport.offset = viewport.offset.min(viewport.maximum);
    }

    /// Returns the focused offset and maximum for rendering and footer state.
    pub(crate) fn horizontal_position(&self) -> (usize, usize) {
        let viewport = self.focused_horizontal();
        (viewport.offset, viewport.maximum)
    }

    pub(super) fn reset_horizontal(&mut self) {
        self.horizontal = HorizontalState::default();
    }

    fn focused_horizontal(&self) -> &HorizontalViewport {
        if self.leaf_surface() {
            &self.horizontal.detail
        } else {
            &self.horizontal.overview
        }
    }

    fn focused_horizontal_mut(&mut self) -> &mut HorizontalViewport {
        if self.leaf_surface() {
            &mut self.horizontal.detail
        } else {
            &mut self.horizontal.overview
        }
    }

    fn leaf_surface(&self) -> bool {
        self.detail_open() || self.structured_is_scalar()
    }
}
