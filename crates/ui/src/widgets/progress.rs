//! Progress bars.

use crate::frame::Ui;
use crate::layout::{Layout, Size};
use crate::theme::Theme;
use crate::tree::{Response, Sense, WidgetKind};

/// Visual style of a progress bar.
#[derive(Debug, Clone)]
pub struct ProgressStyle {
    pub height: f32,
    pub track: [f32; 4],
    pub fill: [f32; 4],
}

impl ProgressStyle {
    pub fn from_theme(theme: &Theme) -> Self {
        Self {
            height: 18.0,
            track: theme.track,
            fill: theme.track_fill,
        }
    }
}

impl Ui {
    /// Unlabelled progress bar.
    pub fn progress(&mut self, value: f32, max: f32) -> Response {
        self.progress_labeled("", value, max)
    }

    /// Progress bar with centred text such as `"Loading 42%"`.
    pub fn progress_labeled(&mut self, label: impl Into<String>, value: f32, max: f32) -> Response {
        let style = ProgressStyle::from_theme(self.theme());
        self.progress_styled(label, value, max, style)
    }

    pub fn progress_styled(
        &mut self,
        label: impl Into<String>,
        value: f32,
        max: f32,
        style: ProgressStyle,
    ) -> Response {
        self.progress_with(Layout::default(), label, value, max, style)
    }

    pub fn progress_with(
        &mut self,
        mut layout: Layout,
        label: impl Into<String>,
        value: f32,
        max: f32,
        style: ProgressStyle,
    ) -> Response {
        let label = label.into();
        let max = max.max(f32::EPSILON);
        if matches!(layout.width, Size::Auto) {
            layout.width = Size::Fill;
        }
        if matches!(layout.height, Size::Auto) {
            layout.height = Size::Fixed(style.height.max(1.0));
        }
        let text_size = self.draw.measure(&label, self.theme.small_font_size);
        let id = self.add_leaf(
            WidgetKind::Progress,
            layout,
            text_size,
            label,
            self.theme.small_font_size,
            self.theme.text,
            text::TextAlign::Center,
        );
        if let Some(node) = self.tree.node_mut(id) {
            node.value = value.clamp(0.0, max);
            node.min = 0.0;
            node.max_value = max;
            node.background = Some(style.track);
            node.fill_color = Some(style.fill);
        }
        self.interact_node(id, Sense::NONE)
    }
}
