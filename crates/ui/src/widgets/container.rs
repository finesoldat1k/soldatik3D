//! Layout containers and cards.
//!
//! The core container is implemented in [`crate::frame::Ui`]; this module adds
//! the directional shortcuts used by game HUDs and settings screens.

use crate::frame::Ui;
use crate::layout::Layout;
use crate::tree::Response;

impl Ui {
    /// Vertical stack with the given gap.
    pub fn column(&mut self, gap: f32, body: impl FnOnce(&mut Self)) -> Response {
        self.container(Layout::column().gap(gap), body)
    }

    /// Horizontal row with the given gap.
    pub fn row(&mut self, gap: f32, body: impl FnOnce(&mut Self)) -> Response {
        self.container(Layout::row().gap(gap), body)
    }

    /// Rounded card with theme padding and background.
    pub fn card(&mut self, padding: f32, body: impl FnOnce(&mut Self)) -> Response {
        self.panel(
            Layout::column().padding(padding).gap(self.theme.spacing),
            body,
        )
    }
}
