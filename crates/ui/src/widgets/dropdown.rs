//! Dropdown / select widget.

use crate::frame::Ui;
use crate::geometry::Rect;
use crate::layout::{Layout, Size};
use crate::theme::Theme;
use crate::tree::{Response, Sense, WidgetKind};

/// Visual style of a dropdown.
#[derive(Debug, Clone)]
pub struct DropdownStyle {
    pub height: f32,
    pub item_height: f32,
    pub font_size: f32,
    pub min_width: f32,
    pub background: [f32; 4],
    pub hovered: [f32; 4],
    pub pressed: [f32; 4],
    pub border: [f32; 4],
    pub text_color: [f32; 4],
    pub radius: f32,
}

impl DropdownStyle {
    pub fn from_theme(theme: &Theme) -> Self {
        Self {
            height: theme.control_height,
            item_height: (theme.control_height - 4.0).max(28.0),
            font_size: theme.font_size,
            min_width: 140.0,
            background: theme.button,
            hovered: theme.button_hovered,
            pressed: theme.button_pressed,
            border: theme.button_border,
            text_color: theme.text,
            radius: theme.radius,
        }
    }
}

impl Ui {
    /// Dropdown that displays the selected item.
    ///
    /// `label` is used as the header text when the item list is empty. Pass an
    /// empty label to always show the current selection.
    pub fn dropdown(
        &mut self,
        label: impl Into<String>,
        items: &[impl AsRef<str>],
        selected: &mut usize,
    ) -> Response {
        let style = DropdownStyle::from_theme(self.theme());
        self.dropdown_styled(label, items, selected, style)
    }

    pub fn dropdown_styled(
        &mut self,
        label: impl Into<String>,
        items: &[impl AsRef<str>],
        selected: &mut usize,
        style: DropdownStyle,
    ) -> Response {
        self.dropdown_with(Layout::default(), label, items, selected, style)
    }

    pub fn dropdown_with(
        &mut self,
        mut layout: Layout,
        label: impl Into<String>,
        items: &[impl AsRef<str>],
        selected: &mut usize,
        style: DropdownStyle,
    ) -> Response {
        let label = label.into();
        let items: Vec<String> = items.iter().map(|item| item.as_ref().to_owned()).collect();
        if items.is_empty() {
            *selected = 0;
        } else {
            *selected = (*selected).min(items.len() - 1);
        }
        let header = if label.is_empty() {
            items
                .get(*selected)
                .cloned()
                .unwrap_or_else(|| "Select...".to_owned())
        } else {
            label
        };
        let mut widest = self.draw.measure(&header, style.font_size).x;
        for item in &items {
            widest = widest.max(self.draw.measure(item, style.font_size).x);
        }
        let arrow_space = style.font_size + 14.0;
        if matches!(layout.width, Size::Auto) {
            layout.width = Size::Fixed((widest + arrow_space + 28.0).max(style.min_width).max(1.0));
        }
        if matches!(layout.height, Size::Auto) {
            layout.height = Size::Fixed(style.height.max(1.0));
        }

        let text_size = self.draw.measure(&header, style.font_size);
        let id = self.add_leaf(
            WidgetKind::Dropdown,
            layout,
            text_size,
            header,
            style.font_size,
            style.text_color,
            text::TextAlign::Left,
        );
        if let Some(node) = self.tree.node_mut(id) {
            node.background = Some(style.background);
            node.hover_color = Some(style.hovered);
            node.pressed_color = Some(style.pressed);
            node.border = Some(style.border);
            node.radius = style.radius;
            node.interactive = true;
            node.sense = Sense::CLICK;
            node.items = items.clone();
            node.selected = *selected;
        }

        let mut response = self.interact_node(id, Sense::CLICK);
        let (mut open, header_rect) = self
            .tree
            .node(id)
            .map(|node| (node.open, node.rect))
            .unwrap_or((false, Rect::ZERO));
        if response.clicked {
            open = !open;
        }

        if open && !items.is_empty() && !header_rect.is_empty() {
            let popup_height = style.item_height * items.len() as f32 + 4.0;
            let viewport = self.viewport();
            let below = header_rect.bottom() + 2.0;
            let y = if below + popup_height <= viewport.bottom() {
                below
            } else {
                (header_rect.y - popup_height - 2.0).max(viewport.y)
            };
            let popup = Rect::new(header_rect.x, y, header_rect.w, popup_height);

            let mut hovered = None;
            if let Some(pointer) = self.primary_pointer() {
                if popup.contains(pointer.position) {
                    let index = ((pointer.position.y - popup.y - 2.0) / style.item_height)
                        .floor()
                        .max(0.0) as usize;
                    if index < items.len() {
                        hovered = Some(index);
                        if pointer.pressed {
                            *selected = index;
                            response.changed = true;
                            open = false;
                        }
                    }
                } else if pointer.pressed && !header_rect.contains(pointer.position) {
                    open = false;
                }
            }

            if let Some(node) = self.tree.node_mut(id) {
                node.popup_rect = popup;
                node.hovered_item = hovered;
            }
        }

        if let Some(node) = self.tree.node_mut(id) {
            node.open = open;
            node.selected = *selected;
            node.changed = response.changed;
        }
        response
    }
}
