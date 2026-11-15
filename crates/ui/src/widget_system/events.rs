//! Event system with hit-test and bubbling.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use glam::Vec2;

use crate::widget_system::core::*;
use crate::widget_system::tree::*;

/// Event dispatcher for the widget tree.
pub struct EventDispatcher {
    event_queue: VecDeque<InputEvent>,
    hover_stack: Vec<WidgetId>,
}

impl EventDispatcher {
    pub fn new() -> Self {
        Self { event_queue: VecDeque::new(), hover_stack: Vec::new() }
    }

    /// Process all pending input and dispatch events.
    pub fn dispatch(&mut self, root: &mut WidgetNode, pointers: &[PointerState]) {
        self.event_queue.clear();
        let mut new_hover = Vec::new();

        // For each pointer, find hit targets
        for pointer in pointers {
            if let Some(target) = Self::hit_test(root, pointer.position) {
                new_hover.push(target);

                // Generate events based on pointer state
                if pointer.just_pressed {
                    self.event_queue.push_back(InputEvent {
                        event: InteractionEvent::Press, target,
                        pointer: *pointer, propagation: EventPropagation::Continue,
                    });
                } else if pointer.just_released {
                    self.event_queue.push_back(InputEvent {
                        event: InteractionEvent::Release, target,
                        pointer: *pointer, propagation: EventPropagation::Continue,
                    });
                    self.event_queue.push_back(InputEvent {
                        event: InteractionEvent::Click, target,
                        pointer: *pointer, propagation: EventPropagation::Continue,
                    });
                } else if pointer.pressed && pointer.drag_delta().length() > 1.0 {
                    self.event_queue.push_back(InputEvent {
                        event: InteractionEvent::Drag, target,
                        pointer: *pointer, propagation: EventPropagation::Continue,
                    });
                } else if !pointer.pressed {
                    self.event_queue.push_back(InputEvent {
                        event: InteractionEvent::HoverEnter, target,
                        pointer: *pointer, propagation: EventPropagation::Continue,
                    });
                }
            }
        }

        // Handle hover exit for widgets no longer hovered
        for &old_id in &self.hover_stack {
            if !new_hover.contains(&old_id) {
                let pointer = pointers.first().copied().unwrap_or(PointerState {
                    position: Vec2::ZERO, pressed: false, just_pressed: false,
                    just_released: false, prev_position: Vec2::ZERO, id: 0,
                });
                self.event_queue.push_back(InputEvent {
                    event: InteractionEvent::HoverExit, target: old_id,
                    pointer, propagation: EventPropagation::Continue,
                });
            }
        }

        self.hover_stack = new_hover;

        // Process events with bubbling (capture phase top-down)
        while let Some(event) = self.event_queue.pop_front() {
            Self::dispatch_event(root, event);
        }
    }

    fn hit_test(node: &WidgetNode, point: Vec2) -> Option<WidgetId> {
        if !node.visible || !node.computed.rect.contains(point) { return None; }

        // Check children first (top-most wins)
        for child_rc in node.children.iter().rev() {
            let child = child_rc.borrow();
            if let Some(hit) = Self::hit_test(&child, point) { return Some(hit); }
        }

        // Check self
        if node.widget.hit_test(node, point) { Some(node.id) } else { None }
    }

    fn dispatch_event(root: &mut WidgetNode, mut event: InputEvent) {
        if let Some(path) = Self::find_path_to_root(root, event.target) {
            for node_rc in &path {
                let mut node = node_rc.borrow_mut();
                if !node.visible { continue; }
                let result = node.widget.handle_event(&mut node, &event);
                if result == EventPropagation::Stop { return; }
            }
        }
    }

    fn find_path_to_root(root: &WidgetNode, target: WidgetId) -> Option<Vec<Rc<RefCell<WidgetNode>>>> {
        let mut path = Vec::new();
        if Self::find_path_recursive(root, target, &mut path) {
            Some(path)
        } else { None }
    }

    fn find_path_recursive(node: &WidgetNode, target: WidgetId, path: &mut Vec<Rc<RefCell<WidgetNode>>>) -> bool {
        // We need to find the Rc for this node - this is a simplified approach
        // In practice, the tree traversal would need to track Rc references
        if node.id == target { return true; }
        for child_rc in &node.children {
            if Self::find_path_recursive(&child_rc.borrow(), target, path) {
                path.push(child_rc.clone());
                return true;
            }
        }
        false
    }
}