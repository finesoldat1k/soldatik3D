//! Touch-friendly buttons.

use crate::frame::Ui;
use crate::layout::{Layout, Size};
use crate::theme::Theme;
use crate::tree::{Response, Sense, WidgetKind};

/// Visual style of a button.
#[derive(Debug, Clone)]
pub struct ButtonStyle {
    pub height: f32,
    pub font_size: f32,
    pub min_width: f32,
    pub padding_x: f32,
    pub background: [f32; 4],
    pub hovered: [f32; 4],
    pub pressed: [f32; 4],
    pub border: [f32; 4],
    pub text_color: [f32; 4],
    pub radius: f32,
}

impl ButtonStyle {
    pub fn from_theme(theme: &Theme) -> Self {
        Self {
            height: theme.control_height,
            font_size: theme.font_size,
            min_width: 72.0,
            padding_x: 16.0,
            background: theme.button,
            hovered: theme.button_hovered,
            pressed: theme.button_pressed,
            border: theme.button_border,
            text_color: theme.text,
            radius: theme.radius,
        }
    }

    pub fn primary(theme: &Theme) -> Self {
        Self {
            background: theme.accent,
            hovered: theme.accent_hovered,
            pressed: theme.accent_pressed,
            border: theme.accent_pressed,
            ..Self::from_theme(theme)
        }
    }

    pub fn danger(theme: &Theme) -> Self {
        Self {
            background: theme.danger,
            hovered: [
                theme.danger[0] * 1.12,
                theme.danger[1],
                theme.danger[2],
                theme.danger[3],
            ],
            pressed: [
                theme.danger[0] * 0.85,
                theme.danger[1] * 0.85,
                theme.danger[2] * 0.85,
                1.0,
            ],
            border: theme.danger,
            ..Self::from_theme(theme)
        }
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height.max(1.0);
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size.max(1.0);
        self
    }

    pub fn padding(mut self, padding: f32) -> Self {
        self.padding_x = padding.max(0.0);
        self
    }
}

impl Ui {
    /// Default themed button.
    pub fn button(&mut self, label: impl Into<String>) -> Response {
        let style = ButtonStyle::from_theme(self.theme());
        self.button_styled(label, style)
    }

    /// Accent-coloured primary action.
    pub fn button_primary(&mut self, label: impl Into<String>) -> Response {
        let style = ButtonStyle::primary(self.theme());
        self.button_styled(label, style)
    }

    /// Destructive action.
    pub fn button_danger(&mut self, label: impl Into<String>) -> Response {
        let style = ButtonStyle::danger(self.theme());
        self.button_styled(label, style)
    }

    pub fn button_styled(&mut self, label: impl Into<String>, style: ButtonStyle) -> Response {
        self.button_with(Layout::default(), label, style)
    }

    /// Button with explicit layout properties. Auto width and height are
    /// derived from the label and the style.
    pub fn button_with(
        &mut self,
        mut layout: Layout,
        label: impl Into<String>,
        style: ButtonStyle,
    ) -> Response {
        let label = label.into();
        let text_size = self.draw.measure(&label, style.font_size);
        if matches!(layout.width, Size::Auto) {
            layout.width = Size::Fixed(
                (text_size.x + style.padding_x * 2.0)
                    .max(style.min_width)
                    .max(1.0),
            );
        }
        if matches!(layout.height, Size::Auto) {
            layout.height = Size::Fixed(style.height.max(1.0));
        }
        let id = self.add_leaf(
            WidgetKind::Button,
            layout,
            text_size,
            label,
            style.font_size,
            style.text_color,
            text::TextAlign::Center,
        );
        if let Some(node) = self.tree.node_mut(id) {
            node.background = Some(style.background);
            node.hover_color = Some(style.hovered);
            node.pressed_color = Some(style.pressed);
            node.border = Some(style.border);
            node.radius = style.radius;
            node.interactive = true;
            node.sense = Sense::CLICK;
        }
        self.interact_node(id, Sense::CLICK)
    }

    /// Compact text-only button that still meets touch-target height.
    pub fn text_button(&mut self, label: impl Into<String>) -> Response {
        let mut style = ButtonStyle::from_theme(self.theme());
        style.background[3] = 0.0;
        style.hovered[3] = 0.12;
        style.pressed[3] = 0.2;
        style.border[3] = 0.0;
        self.button_styled(label, style)
    }
}
