//! Widget tree structure and retained widget system.

use std::any::Any;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use glam::Vec2;

use crate::widget_system::core::*;

/// A node in the widget tree.
pub struct WidgetNode {
    pub id: WidgetId,
    pub widget: Box<dyn Widget>,
    pub state: WidgetState,
    pub layout: LayoutProps,
    pub computed: ComputedLayout,
    pub children: Vec<Rc<RefCell<WidgetNode>>>,
    pub parent: Option<Weak<RefCell<WidgetNode>>>,
    pub components: ComponentStore,
    pub dirty: bool,
    pub visible: bool,
}

impl WidgetNode {
    pub fn new(id: WidgetId, widget: Box<dyn Widget>) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            id,
            widget,
            state: WidgetState::new(),
            layout: LayoutProps::default(),
            computed: ComputedLayout {
                rect: Rect::new(0.0, 0.0, 0.0, 0.0),
                content_rect: Rect::new(0.0, 0.0, 0.0, 0.0),
                children: Vec::new(),
            },
            children: Vec::new(),
            parent: None,
            components: ComponentStore::new(),
            dirty: true,
            visible: true,
        }))
    }

    pub fn add_child(&mut self, child: Rc<RefCell<WidgetNode>>) {
        self.children.push(child);
        self.dirty = true;
    }

    /// Set parent relationship (call after both nodes are in Rc)
    pub fn set_parent(child: &Rc<RefCell<WidgetNode>>, parent: &Rc<RefCell<WidgetNode>>) {
        child.borrow_mut().parent = Some(Rc::downgrade(parent));
    }

    pub fn remove_child(&mut self, child_id: WidgetId) -> Option<Rc<RefCell<WidgetNode>>> {
        if let Some(pos) = self.children.iter().position(|c| c.borrow().id == child_id) {
            let child = self.children.remove(pos);
            child.borrow_mut().parent = None;
            self.dirty = true;
            Some(child)
        } else { None }
    }

    pub fn find_child(&self, id: WidgetId) -> Option<Rc<RefCell<WidgetNode>>> {
        for child in &self.children {
            if child.borrow().id == id { return Some(child.clone()); }
            if let Some(found) = child.borrow().find_child(id) { return Some(found); }
        }
        None
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
        if let Some(parent) = self.parent.as_ref().and_then(|p| p.upgrade()) {
            parent.borrow_mut().mark_dirty();
        }
    }
}

/// Trait for widgets in the retained tree.
pub trait Widget: Any + Send + Sync {
    fn id(&self) -> WidgetId;
    fn type_name(&self) -> &'static str { std::any::type_name::<Self>() }

    fn on_mount(&mut self, _node: &mut WidgetNode) {}
    fn on_unmount(&mut self, _node: &mut WidgetNode) {}

    fn update(&mut self, node: &mut WidgetNode, ctx: &mut UpdateContext);
    fn compute_layout(&mut self, node: &mut WidgetNode, constraints: LayoutConstraints) -> Vec2;
    fn render(&mut self, node: &WidgetNode, draw: &mut dyn DrawList);
    fn handle_event(&mut self, node: &mut WidgetNode, event: &InputEvent) -> EventPropagation;

    fn children(&self, node: &WidgetNode) -> Vec<WidgetId> {
        node.children.iter().map(|c| c.borrow().id).collect()
    }

    fn hit_test(&self, node: &WidgetNode, point: Vec2) -> bool {
        node.computed.rect.contains(point)
    }
}

/// Context passed to widgets during update.
pub struct UpdateContext<'a> {
    pub pointers: &'a [PointerState],
    pub primary_pointer: Option<&'a PointerState>,
    pub dt: f32,
    pub screen_size: Vec2,
    pub theme: &'a Theme,
}