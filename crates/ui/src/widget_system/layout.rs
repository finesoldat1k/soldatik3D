//! Flexbox-style layout engine for widget tree.

use std::any::Any;
use glam::Vec2;

use crate::widget_system::core::*;
use crate::widget_system::tree::*;

/// Dummy widget for placeholder during layout computation.
struct DummyWidget;
impl Widget for DummyWidget {
    fn id(&self) -> WidgetId { WidgetId(0) }
    fn update(&mut self, _node: &mut WidgetNode, _ctx: &mut UpdateContext) {}
    fn compute_layout(&mut self, _node: &mut WidgetNode, _constraints: LayoutConstraints) -> Vec2 { Vec2::ZERO }
    fn render(&mut self, _node: &WidgetNode, _draw: &mut dyn DrawList) {}
    fn handle_event(&mut self, _node: &mut WidgetNode, _event: &InputEvent) -> EventPropagation { EventPropagation::Continue }
}

/// Layout engine that computes positions for the widget tree.
pub struct LayoutEngine;

impl LayoutEngine {
    pub fn compute(root: &mut WidgetNode) {
        Self::compute_node(root, LayoutConstraints::default());
    }

    fn compute_node(node: &mut WidgetNode, constraints: LayoutConstraints) {
        if !node.visible {
            node.computed = ComputedLayout {
                rect: Rect::new(0.0, 0.0, 0.0, 0.0),
                content_rect: Rect::new(0.0, 0.0, 0.0, 0.0),
                children: Vec::new(),
            };
            return;
        }

        // Compute widget's own desired size - use mem::replace to avoid borrow conflict
        let mut widget = std::mem::replace(&mut node.widget, Box::new(DummyWidget));
        let desired = widget.compute_layout(node, constraints);
        node.widget = widget;

        // Compute children layout first
        let mut child_layouts = Vec::new();
        for child_rc in &node.children {
            let mut child = child_rc.borrow_mut();
            Self::compute_node(&mut child, LayoutConstraints {
                min_width: 0.0, max_width: constraints.max_width,
                min_height: 0.0, max_height: constraints.max_height,
            });
            child_layouts.push(child.computed.clone());
        }

        // Apply flexbox layout
        let (rect, content_rect, children_rects) = Self::flex_layout(
            &node.layout, desired, &child_layouts, constraints,
        );

        node.computed.rect = rect;
        node.computed.content_rect = content_rect;
        node.computed.children = children_rects;

        // Update children with final positions
        for (i, child_rc) in node.children.iter().enumerate() {
            let mut child = child_rc.borrow_mut();
            if i < node.computed.children.len() {
                child.computed.rect = node.computed.children[i].rect;
            }
        }

        node.dirty = false;
    }

    fn flex_layout(
        layout: &LayoutProps,
        desired_size: Vec2,
        children: &[ComputedLayout],
        constraints: LayoutConstraints,
    ) -> (Rect, Rect, Vec<ComputedLayout>) {
        let is_row = matches!(layout.flex_direction, FlexDirection::Row | FlexDirection::RowReverse);
        let main_reverse = matches!(layout.flex_direction, FlexDirection::RowReverse | FlexDirection::ColumnReverse);

        let main_constraint = if is_row { constraints.max_width } else { constraints.max_height };
        let _cross_constraint = if is_row { constraints.max_height } else { constraints.max_width };

        // Determine container size
        let mut main_size = desired_size.max(Vec2::new(
            layout.min_size.x.max(layout.padding.x + layout.padding.w),
            layout.min_size.y.max(layout.padding.y + layout.padding.h),
        ));

        main_size.x = main_size.x.clamp(constraints.min_width, constraints.max_width);
        main_size.y = main_size.y.clamp(constraints.min_height, constraints.max_height);

        let content_w = (main_size.x - layout.padding.x - layout.padding.w).max(0.0);
        let content_h = (main_size.y - layout.padding.y - layout.padding.h).max(0.0);

        let container_rect = Rect::new(0.0, 0.0, main_size.x, main_size.y);
        let content_rect = Rect::new(layout.padding.x, layout.padding.y, content_w, content_h);

        if children.is_empty() {
            return (container_rect, content_rect, Vec::new());
        }

        // Flexbox algorithm
        let (child_main_sizes, child_cross_sizes, child_main_positions, child_cross_positions) =
            Self::compute_flex_children(layout, children, content_w, content_h, is_row, main_reverse);

        // Build child rects
        let mut child_layouts = Vec::new();
        for i in 0..children.len() {
            let (x, y, w, h) = if is_row {
                (child_main_positions[i], child_cross_positions[i], child_main_sizes[i], child_cross_sizes[i])
            } else {
                (child_cross_positions[i], child_main_positions[i], child_cross_sizes[i], child_main_sizes[i])
            };
            let rect = Rect::new(
                x + layout.padding.x, y + layout.padding.y, w, h,
            );
            child_layouts.push(ComputedLayout {
                rect,
                content_rect: rect.inset(0.0),
                children: Vec::new(),
            });
        }

        (container_rect, content_rect, child_layouts)
    }

    fn compute_flex_children(
        layout: &LayoutProps,
        children: &[ComputedLayout],
        content_main: f32,
        content_cross: f32,
        is_row: bool,
        main_reverse: bool,
    ) -> (Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>) {
        let n = children.len();
        let mut main_sizes = vec![0.0; n];
        let mut cross_sizes = vec![0.0; n];
        let mut flex_grow_total = 0.0;
        let mut fixed_main = 0.0;

        // First pass: determine sizes
        for (i, child) in children.iter().enumerate() {
            let child_main = if is_row { child.rect.w } else { child.rect.h };
            let child_cross = if is_row { child.rect.h } else { child.rect.w };

            // Use parent's flex props for simplicity
            let grow = layout.flex_grow;

            if grow > 0.0 {
                flex_grow_total += grow;
                main_sizes[i] = 0.0;
            } else {
                main_sizes[i] = child_main;
                fixed_main += child_main;
            }
            cross_sizes[i] = child_cross;
        }

        // Second pass: distribute remaining space for flex children
        let gap_total = layout.gap * (n - 1).max(0) as f32;
        let remaining = (content_main - fixed_main - gap_total).max(0.0);

        for i in 0..n {
            if layout.flex_grow > 0.0 && flex_grow_total > 0.0 {
                main_sizes[i] = remaining * (layout.flex_grow / flex_grow_total);
            }
        }

        // Cross axis: align items
        for i in 0..n {
            cross_sizes[i] = match layout.align_items {
                AlignItems::Stretch => content_cross,
                _ => cross_sizes[i].min(content_cross),
            };
        }

        // Calculate positions
        let mut main_positions = vec![0.0; n];
        let mut cross_positions = vec![0.0; n];

        let start_pos = match layout.justify_content {
            JustifyContent::Start => 0.0,
            JustifyContent::Center => remaining * 0.5,
            JustifyContent::End => remaining,
            JustifyContent::SpaceBetween => if n > 1 { remaining / (n - 1) as f32 } else { 0.0 },
            JustifyContent::SpaceAround => if n > 0 { remaining / n as f32 * 0.5 } else { 0.0 },
            JustifyContent::SpaceEvenly => remaining / (n + 1) as f32,
        };

        let mut current_main = if main_reverse { content_main } else { start_pos };

        for i in 0..n {
            let idx = if main_reverse { n - 1 - i } else { i };

            if main_reverse {
                current_main -= main_sizes[idx];
                main_positions[idx] = current_main;
                if i < n - 1 { current_main -= layout.gap; }
            } else {
                main_positions[idx] = current_main;
                current_main += main_sizes[idx] + layout.gap;
            }

            cross_positions[idx] = match layout.align_items {
                AlignItems::Start => 0.0,
                AlignItems::Center => (content_cross - cross_sizes[idx]) * 0.5,
                AlignItems::End => content_cross - cross_sizes[idx],
                AlignItems::Stretch => 0.0,
                AlignItems::Baseline => 0.0,
            };
        }

        (main_sizes, cross_sizes, main_positions, cross_positions)
    }
}