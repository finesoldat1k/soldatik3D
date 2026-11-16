//! Toggle switches.

use crate::frame::Ui;
use crate::layout::{Layout, Size};
use crate::theme::Theme;
use crate::tree::{Response, Sense, WidgetKind};

/// Visual style of a toggle row.
#[derive(Debug, Clone)]
pub struct ToggleStyle {
    pub height: f32,
    pub font_size: f32,
    pub track_off: [f32; 4],
    pub track_on: [f32; 4],
    pub text_color: [f32; 4],
}

impl ToggleStyle {
    pub fn from_theme(theme: &Theme) -> Self {
        Self {
            height: theme.control_height,
            font_size: theme.font_size,
            track_off: theme.toggle_off,
            track_on: theme.toggle_on,
            text_color: theme.text,
        }
    }
}

impl Ui {
    /// Label and switch on one touch-friendly row.
    pub fn toggle(&mut self, label: impl Into<String>, value: &mut bool) -> Response {
        let style = ToggleStyle::from_theme(self.theme());
        self.toggle_styled(label, value, style)
    }

    pub fn toggle_styled(
        &mut self,
        label: impl Into<String>,
        value: &mut bool,
        style: ToggleStyle,
    ) -> Response {
        self.toggle_with(Layout::default(), label, value, style)
    }

    pub fn toggle_with(
        &mut self,
        mut layout: Layout,
        label: impl Into<String>,
        value: &mut bool,
        style: ToggleStyle,
    ) -> Response {
        let label = label.into();
        if matches!(layout.width, Size::Auto) {
            layout.width = Size::Fill;
        }
        if matches!(layout.height, Size::Auto) {
            layout.height = Size::Fixed(style.height.max(1.0));
        }
        let text_size = self.draw.measure(&label, style.font_size);
        let id = self.add_leaf(
            WidgetKind::Toggle,
            layout,
            text_size,
            label,
            style.font_size,
            style.text_color,
            text::TextAlign::Left,
        );
        if let Some(node) = self.tree.node_mut(id) {
            node.toggled = *value;
            node.background = Some(style.track_off);
            node.fill_color = Some(style.track_on);
            node.interactive = true;
            node.sense = Sense::CLICK;
        }
        let mut response = self.interact_node(id, Sense::CLICK);
        if response.clicked {
            *value = !*value;
            response.changed = true;
            if let Some(node) = self.tree.node_mut(id) {
                node.toggled = *value;
                node.changed = true;
            }
        }
        response
    }
}
