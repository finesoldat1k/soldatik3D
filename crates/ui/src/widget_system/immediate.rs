//! Immediate-mode API for the widget system.
//!
//! This module provides a convenient immediate-mode interface that builds
//! the retained widget tree behind the scenes.

use std::cell::RefCell;
use std::rc::Rc;
use glam::Vec2;

use crate::widget_system::core::*;
use crate::widget_system::tree::*;
use crate::widget_system::layout::LayoutEngine;
use crate::widget_system::events::EventDispatcher;
use crate::widget_system::widgets::*;

/// Dummy widget for placeholder during updates
struct DummyWidget;
impl Widget for DummyWidget {
    fn id(&self) -> WidgetId { WidgetId(0) }
    fn update(&mut self, _node: &mut WidgetNode, _ctx: &mut UpdateContext) {}
    fn compute_layout(&mut self, _node: &mut WidgetNode, _constraints: LayoutConstraints) -> Vec2 { Vec2::ZERO }
    fn render(&mut self, _node: &WidgetNode, _draw: &mut dyn DrawList) {}
    fn handle_event(&mut self, _node: &mut WidgetNode, _event: &InputEvent) -> EventPropagation { EventPropagation::Continue }
}

/// High-level UI context for immediate-mode usage.
pub struct Ui {
    root_node: Option<Rc<RefCell<WidgetNode>>>,
    theme: Theme,
    layout_engine: LayoutEngine,
    event_dispatcher: EventDispatcher,
    pointers: Vec<PointerState>,
    screen_size: Vec2,
    dt: f32,
    widget_registry: std::collections::HashMap<WidgetId, Box<dyn Widget>>,
}

impl Ui {
    pub fn new(screen_size: Vec2, theme: Theme) -> Self {
        Self {
            root_node: None,
            theme,
            layout_engine: LayoutEngine,
            event_dispatcher: EventDispatcher::new(),
            pointers: Vec::new(),
            screen_size,
            dt: 0.0,
            widget_registry: std::collections::HashMap::new(),
        }
    }

    pub fn set_screen_size(&mut self, size: Vec2) { self.screen_size = size; }
    pub fn set_dt(&mut self, dt: f32) { self.dt = dt; }
    pub fn update_pointers(&mut self, pointers: &[PointerState]) { self.pointers = pointers.to_vec(); }
    pub fn theme(&self) -> &Theme { &self.theme }
    pub fn theme_mut(&mut self) -> &mut Theme { &mut self.theme }

    /// Begin a new frame
    pub fn begin(&mut self) -> UiFrame<'_> {
        UiFrame { ui: self }
    }

    /// Get the root widget node
    pub fn root_node(&self) -> Option<&Rc<RefCell<WidgetNode>>> { self.root_node.as_ref() }
}

/// Frame context for building UI
pub struct UiFrame<'a> {
    ui: &'a mut Ui,
}

impl<'a> UiFrame<'a> {
    /// Show a widget - adds to tree if new, updates if existing
    pub fn show(&mut self, widget: impl Into<Box<dyn Widget>>) -> WidgetId {
        let widget = widget.into();
        let id = widget.id();

        if self.ui.root_node.is_none() {
            let node = WidgetNode::new(id, widget);
            self.ui.root_node = Some(node);
        } else {
            self.update_or_add(self.ui.root_node.as_ref().unwrap(), widget);
        }
        id
    }

    fn update_or_add(&self, node: &Rc<RefCell<WidgetNode>>, widget: Box<dyn Widget>) {
        let target_id = widget.id();
        let widget_cell = RefCell::new(Some(widget));

        fn find_and_replace(node: &Rc<RefCell<WidgetNode>>, target_id: WidgetId, widget_cell: &RefCell<Option<Box<dyn Widget>>>) -> bool {
            let mut node_ref = node.borrow_mut();
            if node_ref.id == target_id {
                if let Some(w) = widget_cell.borrow_mut().take() {
                    node_ref.widget = w; node_ref.dirty = true; return true;
                }
            }
            for child in &node_ref.children {
                if child.borrow().id == target_id {
                    let mut child_ref = child.borrow_mut();
                    if let Some(w) = widget_cell.borrow_mut().take() {
                        child_ref.widget = w; child_ref.dirty = true; return true;
                    }
                }
                if find_and_replace(child, target_id, widget_cell) { return true; }
            }
            false
        }

        find_and_replace(node, target_id, &widget_cell);
    }

    // ========== Convenience widget builders ==========

    pub fn button(&mut self, label: impl Into<String>) -> ButtonBuilder<'_> {
        ButtonBuilder::new(self.ui, label)
    }

    pub fn label(&mut self, text: impl Into<String>) -> LabelBuilder<'_> {
        LabelBuilder::new(self.ui, text)
    }

    pub fn container(&mut self) -> ContainerBuilder<'_> {
        ContainerBuilder::new(self.ui)
    }

    pub fn dropdown(&mut self, label: impl Into<String>, items: Vec<String>) -> DropdownBuilder<'_> {
        DropdownBuilder::new(self.ui, label, items)
    }

    pub fn virtual_joystick(&mut self, area: Rect) -> VirtualJoystickBuilder<'_> {
        VirtualJoystickBuilder::new(self.ui, area)
    }

    pub fn touch_camera(&mut self, area: Rect) -> TouchCameraBuilder<'_> {
        TouchCameraBuilder::new(self.ui, area)
    }

    pub fn scroll_panel(&mut self) -> ScrollPanelBuilder<'_> {
        ScrollPanelBuilder::new(self.ui)
    }

    /// Layout and render the UI
    pub fn end(self) {
        if let Some(root) = &self.ui.root_node {
            LayoutEngine::compute(&mut *root.borrow_mut());
            { let mut root_ref = root.borrow_mut(); self.ui.event_dispatcher.dispatch(&mut *root_ref, &self.ui.pointers); }
            self.update_widgets(root);
            self.render_widgets(root);
        }
    }

    fn update_widgets(&self, node: &Rc<RefCell<WidgetNode>>) {
        let mut ctx = UpdateContext {
            pointers: &self.ui.pointers,
            primary_pointer: self.ui.pointers.iter().find(|p| p.id == 0),
            dt: self.ui.dt,
            screen_size: self.ui.screen_size,
            theme: &self.ui.theme,
        };

        { 
            let mut node_ref = node.borrow_mut();
            // Temporarily take widget out to avoid borrow conflicts
            let mut widget = std::mem::replace(&mut node_ref.widget, Box::new(DummyWidget));
            widget.update(&mut *node_ref, &mut ctx);
            node_ref.widget = widget;
        }

        let children = { let node_ref = node.borrow(); node_ref.children.clone() };
        for child in children { self.update_widgets(&child); }
    }

    fn render_widgets(&self, node: &Rc<RefCell<WidgetNode>>) {
        // Render would need DrawList implementation
        let node_ref = node.borrow();
        for child in &node_ref.children { self.render_widgets(child); }
    }
}

/// Button builder for immediate mode
pub struct ButtonBuilder<'a> {
    ui: &'a mut Ui,
    widget: Button,
}

impl<'a> ButtonBuilder<'a> {
    pub fn new(ui: &'a mut Ui, label: impl Into<String>) -> Self {
        Self { ui, widget: Button::new(label) }
    }
    pub fn id(mut self, id: WidgetId) -> Self { self.widget = self.widget.id(id); self }
    pub fn on_click<F>(mut self, f: F) -> Self where F: FnMut(&mut WidgetNode) + Send + Sync + 'static {
        self.widget = self.widget.on_click(f); self
    }
    pub fn layout(mut self, layout: LayoutProps) -> Self { self.widget = self.widget.layout(layout); self }
    pub fn style(mut self, style: ButtonStyle) -> Self { self.widget = self.widget.style(style); self }
    pub fn build(self) -> Box<dyn Widget> { self.widget.build() }
    pub fn show(self) -> WidgetId {
        let widget = self.widget.build();
        let id = widget.id();
        let mut frame = UiFrame { ui: self.ui };
        frame.show(widget);
        id
    }
}

/// Container builder for immediate mode
pub struct ContainerBuilder<'a> {
    ui: &'a mut Ui,
    widget: Container,
}

impl<'a> ContainerBuilder<'a> {
    pub fn new(ui: &'a mut Ui) -> Self { Self { ui, widget: Container::new() } }
    pub fn id(mut self, id: WidgetId) -> Self { self.widget = self.widget.id(id); self }
    pub fn layout(mut self, layout: LayoutProps) -> Self { self.widget = self.widget.layout(layout); self }
    pub fn style(mut self, style: ContainerStyle) -> Self { self.widget = self.widget.style(style); self }
    pub fn child(mut self, child: impl Into<Box<dyn Widget>>) -> Self { self.widget = self.widget.child(child); self }
    pub fn show(self) -> WidgetId {
        let widget = self.widget.build();
        let id = widget.id();
        let mut frame = UiFrame { ui: self.ui };
        frame.show(widget);
        id
    }
}

/// Label builder for immediate mode
pub struct LabelBuilder<'a> {
    ui: &'a mut Ui,
    widget: Label,
}

impl<'a> LabelBuilder<'a> {
    pub fn new(ui: &'a mut Ui, text: impl Into<String>) -> Self { Self { ui, widget: Label::new(text) } }
    pub fn id(mut self, id: WidgetId) -> Self { self.widget = self.widget.id(id); self }
    pub fn scale(mut self, scale: f32) -> Self { self.widget = self.widget.scale(scale); self }
    pub fn color(mut self, color: [f32; 4]) -> Self { self.widget = self.widget.color(color); self }
    pub fn layout(mut self, layout: LayoutProps) -> Self { self.widget = self.widget.layout(layout); self }
    pub fn alignment(mut self, alignment: TextAlignment) -> Self { self.widget = self.widget.alignment(alignment); self }
    pub fn show(self) -> WidgetId {
        let widget = self.widget.build();
        let id = widget.id();
        let mut frame = UiFrame { ui: self.ui };
        frame.show(widget);
        id
    }
}

/// Dropdown builder for immediate mode
pub struct DropdownBuilder<'a> {
    ui: &'a mut Ui,
    widget: Dropdown,
}

impl<'a> DropdownBuilder<'a> {
    pub fn new(ui: &'a mut Ui, label: impl Into<String>, items: Vec<String>) -> Self {
        Self { ui, widget: Dropdown::new(label, items) }
    }
    pub fn id(mut self, id: WidgetId) -> Self { self.widget = self.widget.id(id); self }
    pub fn selected(mut self, index: usize) -> Self { self.widget = self.widget.selected(index); self }
    pub fn on_change<F>(mut self, f: F) -> Self where F: FnMut(usize, &mut WidgetNode) + Send + Sync + 'static {
        self.widget = self.widget.on_change(f); self
    }
    pub fn layout(mut self, layout: LayoutProps) -> Self { self.widget = self.widget.layout(layout); self }
    pub fn style(mut self, style: DropdownStyle) -> Self { self.widget = self.widget.style(style); self }
    pub fn show(self) -> WidgetId {
        let widget = self.widget.build();
        let id = widget.id();
        let mut frame = UiFrame { ui: self.ui };
        frame.show(widget);
        id
    }
}

/// Virtual joystick builder for immediate mode
pub struct VirtualJoystickBuilder<'a> {
    ui: &'a mut Ui,
    widget: VirtualJoystick,
}

impl<'a> VirtualJoystickBuilder<'a> {
    pub fn new(ui: &'a mut Ui, area: Rect) -> Self { Self { ui, widget: VirtualJoystick::new(area) } }
    pub fn id(mut self, id: WidgetId) -> Self { self.widget = self.widget.id(id); self }
    pub fn on_change<F>(mut self, f: F) -> Self where F: FnMut(Vec2, &mut WidgetNode) + Send + Sync + 'static {
        self.widget = self.widget.on_change(f); self
    }
    pub fn style(mut self, style: VirtualJoystickStyle) -> Self { self.widget = self.widget.style(style); self }
    pub fn show(self) -> WidgetId {
        let widget = self.widget.build();
        let id = widget.id();
        let mut frame = UiFrame { ui: self.ui };
        frame.show(widget);
        id
    }
}

/// Touch camera builder for immediate mode
pub struct TouchCameraBuilder<'a> {
    ui: &'a mut Ui,
    widget: TouchCamera,
}

impl<'a> TouchCameraBuilder<'a> {
    pub fn new(ui: &'a mut Ui, area: Rect) -> Self { Self { ui, widget: TouchCamera::new(area) } }
    pub fn id(mut self, id: WidgetId) -> Self { self.widget = self.widget.id(id); self }
    pub fn sensitivity(mut self, sensitivity: f32) -> Self { self.widget = self.widget.sensitivity(sensitivity); self }
    pub fn on_delta<F>(mut self, f: F) -> Self where F: FnMut(Vec2, &mut WidgetNode) + Send + Sync + 'static {
        self.widget = self.widget.on_delta(f); self
    }
    pub fn style(mut self, style: TouchCameraStyle) -> Self { self.widget = self.widget.style(style); self }
    pub fn show(self) -> WidgetId {
        let widget = self.widget.build();
        let id = widget.id();
        let mut frame = UiFrame { ui: self.ui };
        frame.show(widget);
        id
    }
}

/// Scroll panel builder for immediate mode
pub struct ScrollPanelBuilder<'a> {
    ui: &'a mut Ui,
    widget: ScrollPanel,
}

impl<'a> ScrollPanelBuilder<'a> {
    pub fn new(ui: &'a mut Ui) -> Self { Self { ui, widget: ScrollPanel::new() } }
    pub fn id(mut self, id: WidgetId) -> Self { self.widget = self.widget.id(id); self }
    pub fn layout(mut self, layout: LayoutProps) -> Self { self.widget = self.widget.layout(layout); self }
    pub fn style(mut self, style: ScrollPanelStyle) -> Self { self.widget = self.widget.style(style); self }
    pub fn child(mut self, child: impl Into<Box<dyn Widget>>) -> Self { self.widget = self.widget.child(child); self }
    pub fn show(self) -> WidgetId {
        let widget = self.widget.build();
        let id = widget.id();
        let mut frame = UiFrame { ui: self.ui };
        frame.show(widget);
        id
    }
}