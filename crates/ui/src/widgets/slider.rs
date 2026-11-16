//! Sliders.

use crate::frame::Ui;
use crate::layout::{Layout, Size};
use crate::theme::Theme;
use crate::tree::{Response, Sense, WidgetKind};

/// Visual style of a slider row.
#[derive(Debug, Clone)]
pub struct SliderStyle {
    pub height: f32,
    pub font_size: f32,
    pub track: [f32; 4],
    pub fill: [f32; 4],
    pub text_color: [f32; 4],
    pub show_value: bool,
}

impl SliderStyle {
    pub fn from_theme(theme: &Theme) -> Self {
        Self {
            height: 52.0,
            font_size: theme.font_size,
            track: theme.track,
            fill: theme.track_fill,
            text_color: theme.text,
            show_value: true,
        }
    }
}

impl Ui {
    pub fn slider(
        &mut self,
        label: impl Into<String>,
        value: &mut f32,
        min: f32,
        max: f32,
    ) -> Response {
        let style = SliderStyle::from_theme(self.theme());
        self.slider_styled(label, value, min, max, style)
    }

    pub fn slider_styled(
        &mut self,
        label: impl Into<String>,
        value: &mut f32,
        min: f32,
        max: f32,
        style: SliderStyle,
    ) -> Response {
        self.slider_with(Layout::default(), label, value, min, max, style)
    }

    pub fn slider_with(
        &mut self,
        mut layout: Layout,
        label: impl Into<String>,
        value: &mut f32,
        min: f32,
        max: f32,
        style: SliderStyle,
    ) -> Response {
        let label = label.into();
        let (min, max) = if min <= max { (min, max) } else { (max, min) };
        let range = (max - min).max(f32::EPSILON);
        *value = value.clamp(min, max);
        if matches!(layout.width, Size::Auto) {
            layout.width = Size::Fill;
        }
        if matches!(layout.height, Size::Auto) {
            layout.height = Size::Fixed(style.height.max(1.0));
        }
        let text_size = self.draw.measure(&label, style.font_size);
        let id = self.add_leaf(
            WidgetKind::Slider,
            layout,
            text_size,
            label,
            style.font_size,
            style.text_color,
            text::TextAlign::Left,
        );
        if let Some(node) = self.tree.node_mut(id) {
            node.value = *value;
            node.min = min;
            node.max_value = max;
            node.show_value = style.show_value;
            node.background = Some(style.track);
            node.fill_color = Some(style.fill);
            node.interactive = true;
            node.sense = Sense::DRAG;
        }

        let mut response = self.interact_node(id, Sense::DRAG);
        if (response.pressed || response.active)
            && let Some(pointer) = self.interaction_pointer()
            && pointer.down
        {
            {
                let padding = self
                    .tree
                    .node(id)
                    .map(|node| node.layout.padding)
                    .unwrap_or_default();
                let rect = self
                    .tree
                    .node(id)
                    .map(|node| node.rect.inset_edges(padding))
                    .unwrap_or_default();
                if rect.w > 0.0 {
                    let t = ((pointer.position.x - rect.x) / rect.w).clamp(0.0, 1.0);
                    let new_value = min + t * range;
                    if (new_value - *value).abs() > f32::EPSILON {
                        *value = new_value;
                        response.changed = true;
                    }
                    if let Some(node) = self.tree.node_mut(id) {
                        node.value = *value;
                        node.changed = response.changed;
                    }
                }
            }
        }
        response
    }
}
