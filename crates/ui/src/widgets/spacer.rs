//! Spacing and separators.

use crate::frame::Ui;
use crate::layout::{Layout, Size};
use crate::tree::{Response, Sense, WidgetKind};

impl Ui {
    /// Empty space that participates in layout.
    pub fn spacer(&mut self, width: f32, height: f32) -> Response {
        let layout = Layout::default().size(width.max(0.0), height.max(0.0));
        self.spacer_with(layout)
    }

    /// Spacer with an explicit layout (for example `fill_width()`).
    pub fn spacer_with(&mut self, layout: Layout) -> Response {
        self.add_leaf(
            WidgetKind::Spacer,
            layout,
            glam::Vec2::ZERO,
            String::new(),
            self.theme.font_size,
            self.theme.text_muted,
            text::TextAlign::Left,
        );
        Response::default()
    }

    /// Horizontal divider using the theme's border colour.
    pub fn separator(&mut self) -> Response {
        let layout = Layout::default()
            .height(1.0)
            .margin_edges(crate::geometry::EdgeInsets::vertical(8.0))
            .fill_width();
        self.separator_with(layout)
    }

    /// Divider with an explicit layout and colour.
    pub fn separator_with(&mut self, layout: Layout) -> Response {
        let id = self.add_leaf(
            WidgetKind::Separator,
            layout,
            glam::Vec2::new(0.0, 1.0),
            String::new(),
            self.theme.font_size,
            self.theme.panel_border,
            text::TextAlign::Left,
        );
        self.interact_node(id, Sense::NONE)
    }

    /// Vertical spacing shortcut.
    pub fn gap(&mut self, height: f32) -> Response {
        self.spacer(0.0, height)
    }

    /// Flexible spacer that pushes later siblings to the end of a column.
    pub fn flexible_space(&mut self) -> Response {
        self.spacer_with(Layout::default().height_size(Size::Fill))
    }
}
