//! Text labels and headings.

use glam::Vec2;
use text::{TextAlign, TextStyle};

use crate::frame::Ui;
use crate::layout::Layout;
use crate::tree::{Response, Sense, WidgetKind};

/// Visual style of a label.
#[derive(Debug, Clone)]
pub struct LabelStyle {
    pub font_size: f32,
    pub color: [f32; 4],
    pub align: TextAlign,
    /// Soft-wrap width in pixels. `None` keeps the text on one line.
    pub wrap: Option<f32>,
}

impl LabelStyle {
    pub fn new(font_size: f32) -> Self {
        Self {
            font_size,
            color: [0.93, 0.94, 0.96, 1.0],
            align: TextAlign::Left,
            wrap: None,
        }
    }

    pub fn color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    pub fn align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    pub fn wrap(mut self, width: f32) -> Self {
        self.wrap = Some(width.max(1.0));
        self
    }

    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            font_size: theme.font_size,
            color: theme.text,
            align: TextAlign::Left,
            wrap: None,
        }
    }
}

impl Ui {
    /// Single-line body text.
    pub fn label(&mut self, text: impl Into<String>) -> Response {
        let style = LabelStyle::from_theme(self.theme());
        self.label_styled(text, style)
    }

    /// Body text with an explicit style.
    pub fn label_styled(&mut self, text: impl Into<String>, style: LabelStyle) -> Response {
        let layout = Layout::default();
        self.label_with(layout, text, style)
    }

    /// Label with layout overrides.
    pub fn label_with(
        &mut self,
        mut layout: Layout,
        text: impl Into<String>,
        style: LabelStyle,
    ) -> Response {
        let text = text.into();
        let mut text_style = TextStyle::new(style.font_size)
            .with_color(style.color)
            .align(style.align);
        if let Some(width) = style.wrap {
            text_style = text_style.wrap(width);
        }
        let size = self.draw.measure_styled(&text, &text_style);
        if matches!(layout.width, crate::layout::Size::Auto) {
            layout.width = crate::layout::Size::Fixed(size.x.max(1.0));
        }
        if matches!(layout.height, crate::layout::Size::Auto) {
            layout.height = crate::layout::Size::Fixed(size.y.max(1.0));
        }
        let id = self.add_leaf(
            WidgetKind::Label,
            layout,
            size,
            text,
            style.font_size,
            style.color,
            style.align,
        );
        if let Some(node) = self.tree.node_mut(id) {
            node.text_wrap = style.wrap;
        }
        self.interact_node(id, Sense::NONE)
    }

    /// Section heading using the theme's title size.
    pub fn heading(&mut self, text: impl Into<String>) -> Response {
        let style = LabelStyle {
            font_size: self.theme.title_font_size,
            color: self.theme.text,
            align: TextAlign::Left,
            wrap: None,
        };
        self.label_styled(text, style)
    }

    /// Secondary / hint text.
    pub fn hint(&mut self, text: impl Into<String>) -> Response {
        let style = LabelStyle {
            font_size: self.theme.small_font_size,
            color: self.theme.text_muted,
            align: TextAlign::Left,
            wrap: None,
        };
        self.label_styled(text, style)
    }

    /// Free-form text that uses the parent's full width and wraps.
    pub fn paragraph(&mut self, text: impl Into<String>) -> Response {
        let width = self.content_width_hint();
        let style = LabelStyle {
            font_size: self.theme.font_size,
            color: self.theme.text,
            align: TextAlign::Left,
            wrap: Some(width),
        };
        self.label_styled(text, style)
    }

    fn content_width_hint(&self) -> f32 {
        // Labels wrap against the current viewport by default; callers that
        // need a precise width can use `label_with(..., style.wrap(width))`.
        self.viewport().w.max(1.0)
    }

    /// Measures a string using the same font stack as labels.
    pub fn measure_label(&self, text: &str, style: &LabelStyle) -> Vec2 {
        let mut text_style = TextStyle::new(style.font_size)
            .with_color(style.color)
            .align(style.align);
        if let Some(width) = style.wrap {
            text_style = text_style.wrap(width);
        }
        self.draw.measure_styled(text, &text_style)
    }
}
