//! Hybrid immediate-mode UI for Soldatik3D.
//!
//! The API is immediate: widgets are called every frame and return a
//! [`Response`]. Behind the scenes a retained arena tree keeps stable ids,
//! hover/press state, scroll offsets and dropdown state, so animations and
//! touch controls do not need manual bookkeeping.
//!
//! # Quick start
//!
//! ```no_run
//! use ui::{Layout, Theme, Ui};
//! # use ui::input::UiInput;
//! # fn draw(ui: &mut Ui) {
//! ui.begin_frame_snapshot(UiInput::default(), ui::Vec2::new(1280.0, 720.0));
//! ui.panel(Layout::column().padding(16.0).gap(8.0), |ui| {
//!     ui.heading("Soldatik3D");
//!     if ui.button_primary("Play").clicked {
//!         // start the game
//!     }
//! });
//! ui.end_frame();
//! # }
//! ```
//!
//! # Architecture
//!
//! - [`Ui`] owns the widget tree, theme, input snapshot and draw list.
//! - [`Layout`] describes row/column flow, padding, gaps and fill sizing.
//! - [`UiDrawList`] produces backend-neutral triangles and owns the TTF text
//!   system from the `text` crate.
//! - Widgets live in [`widgets`] and are also available as methods on [`Ui`].
//!
//! Coordinates are physical pixels with the origin in the top-left corner.
//! The renderer uploads the dynamically growing glyph atlas once per frame
//! when it changes.

pub mod draw;
pub mod frame;
pub mod geometry;
pub mod input;
pub mod layout;
pub mod theme;
pub mod tree;
pub mod widgets;

pub use draw::{UiDrawList, UiVertex};
pub use frame::Ui;
pub use geometry::{EdgeInsets, Rect};
pub use input::{PointerKind, TouchInputProvider, TouchStateProvider, UiInput, UiPointer};
pub use layout::{AlignItems, FlexDirection, JustifyContent, Layout, Size, UiId, stable_id};
pub use theme::Theme;
pub use tree::{Node, Response, Sense, UiTree, WidgetKind};
pub use widgets::{
    ButtonStyle, DropdownStyle, JoystickState, JoystickStyle, LabelStyle, ProgressStyle,
    ScrollPanelStyle, SliderStyle, ToggleStyle, TouchCameraState, TouchCameraStyle,
};

/// Re-exported so games do not need a direct `glam` dependency for UI code.
pub use glam::Vec2;

/// Re-exported text types so games do not need a direct `text` dependency.
pub use text::{TextAlign, TextLayout, TextStyle};
