//! Scrollable panels with a draggable scrollbar.

use crate::frame::Ui;
use crate::layout::Layout;
use crate::tree::{Node, Response, WidgetKind};

/// Decoration of a scroll panel.
#[derive(Debug, Clone)]
pub struct ScrollPanelStyle {
    pub background: Option<[f32; 4]>,
    pub border: Option<[f32; 4]>,
    pub radius: f32,
}

impl ScrollPanelStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            background: Some(theme.panel),
            border: Some(theme.panel_border),
            radius: theme.radius,
        }
    }

    pub fn bare() -> Self {
        Self {
            background: None,
            border: None,
            radius: 0.0,
        }
    }
}

impl Ui {
    /// Scrollable container. Children are clipped to the panel and laid out in
    /// a column; the wheel and the scrollbar change the scroll offset.
    pub fn scroll_panel(&mut self, layout: Layout, body: impl FnOnce(&mut Self)) -> Response {
        let style = ScrollPanelStyle::from_theme(self.theme());
        self.scroll_panel_styled(layout, style, body)
    }

    pub fn scroll_panel_styled(
        &mut self,
        layout: Layout,
        style: ScrollPanelStyle,
        body: impl FnOnce(&mut Self),
    ) -> Response {
        let id = self.begin_node(WidgetKind::ScrollPanel, layout.scrollable());
        if let Some(node) = self.tree.node_mut(id) {
            node.background = style.background;
            node.border = style.border;
            node.radius = style.radius;
        }

        self.handle_scroll_input(id);
        body(self);
        self.end_node();

        let Some(node) = self.tree.node(id).cloned() else {
            return Response::default();
        };
        let mut response = node.response();
        response.hovered = self.pointer_inside(node.rect);
        response
    }

    fn handle_scroll_input(&mut self, id: crate::layout::UiId) {
        let Some(node) = self.tree.node(id).cloned() else {
            return;
        };
        if node.rect.is_empty() {
            return;
        }

        // Mouse wheel over the panel.
        if self.pointer_inside(node.rect) && self.input.scroll_delta != 0.0 {
            self.tree.request_scroll(id, self.input.scroll_delta);
        }

        // Scrollbar dragging. The thumb rectangle is produced by the previous
        // layout pass; using it for hit testing avoids a second layout pass.
        if node.scrollbar_rect.is_empty() || node.scroll_max <= 0.0 {
            return;
        }
        let Some(pointer) = self.primary_pointer() else {
            return;
        };
        if self.active_scrollbar.is_none()
            && pointer.pressed
            && node.scrollbar_rect.contains(pointer.position)
        {
            self.active_scrollbar = Some((pointer.id, id));
        }
        if self.active_scrollbar == Some((pointer.id, id)) {
            let content = node.rect.inset_edges(node.layout.padding);
            let travel = (content.h - node.scrollbar_rect.h).max(0.0);
            if travel > 0.0 {
                let t = ((pointer.position.y - content.y - node.scrollbar_rect.h * 0.5) / travel)
                    .clamp(0.0, 1.0);
                if let Some(target) = self.tree.node_mut(id) {
                    target.scroll = t * target.scroll_max;
                }
            }
            if pointer.released {
                self.active_scrollbar = None;
            }
        }
    }

    /// Total scrollable height of a panel, useful for custom scrollbars.
    pub fn scroll_info(&self, id: crate::layout::UiId) -> Option<(f32, f32)> {
        self.tree
            .node(id)
            .map(|node: &Node| (node.scroll, node.scroll_max))
    }
}
