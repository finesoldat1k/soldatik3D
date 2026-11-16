//! Input abstraction shared by widgets and games.
//!
//! Widgets never touch winit or `platform::Input` directly. The runtime builds
//! a [`UiInput`] snapshot once per frame; tests and alternative platforms can
//! provide their own [`TouchInputProvider`].

use glam::Vec2;

/// Kind of device that produced a pointer event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PointerKind {
    Mouse,
    Touch,
}

/// One pointer (mouse cursor or finger) during a frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiPointer {
    /// Unique pointer id. Touch ids have the high bit set so they can never
    /// collide with the mouse pointer (id 0).
    pub id: u64,
    pub kind: PointerKind,
    pub position: Vec2,
    pub prev_position: Vec2,
    pub down: bool,
    pub pressed: bool,
    pub released: bool,
}

impl UiPointer {
    pub fn delta(&self) -> Vec2 {
        self.position - self.prev_position
    }

    pub fn drag_delta(&self) -> Vec2 {
        self.delta()
    }

    pub fn touch(&self) -> bool {
        matches!(self.kind, PointerKind::Touch)
    }
}

/// Input snapshot for one UI frame.
#[derive(Debug, Clone, Default)]
pub struct UiInput {
    pub pointers: Vec<UiPointer>,
    pub scroll_delta: f32,
    /// True once the platform has reported a touch. Games use it to show
    /// touch-only controls such as the virtual joystick.
    pub touch_mode: bool,
}

impl UiInput {
    /// Builds a snapshot from any platform input source.
    pub fn from_provider<I: TouchInputProvider>(input: &I) -> Self {
        let touches = input.touches();
        let touch_mode = input.touch_mode() || !touches.is_empty();
        let mut pointers = Vec::with_capacity(touches.len() + 1);

        for touch in touches {
            pointers.push(UiPointer {
                id: touch.id() | TOUCH_ID_BIT,
                kind: PointerKind::Touch,
                position: touch.position(),
                prev_position: touch.prev_position(),
                down: !touch.ended(),
                pressed: touch.started(),
                released: touch.ended(),
            });
        }

        // A mouse exists on desktop, and on Android it is only reported while
        // a real pointer is connected. Touch events take priority over the
        // stale (0, 0) cursor position.
        if !touch_mode || pointers.is_empty() {
            let position = input.cursor_position();
            let delta = input.mouse_delta();
            let down = input.primary_down();
            pointers.push(UiPointer {
                id: 0,
                kind: PointerKind::Mouse,
                position,
                prev_position: position - delta,
                down,
                pressed: input.primary_clicked(),
                released: input.primary_released(),
            });
        }

        Self {
            pointers,
            scroll_delta: input.scroll_delta(),
            touch_mode,
        }
    }

    pub fn pointer(&self, id: u64) -> Option<&UiPointer> {
        self.pointers.iter().find(|pointer| pointer.id == id)
    }

    pub fn primary(&self) -> Option<&UiPointer> {
        self.pointers
            .iter()
            .find(|pointer| pointer.kind == PointerKind::Touch)
            .or_else(|| self.pointers.first())
    }

    pub fn is_touch(&self) -> bool {
        self.touch_mode
    }

    pub fn any_pressed(&self) -> bool {
        self.pointers.iter().any(|pointer| pointer.pressed)
    }
}

/// Touch ids are remapped into a separate namespace.
pub const TOUCH_ID_BIT: u64 = 1 << 63;

/// A single touch sample. Implemented for `platform::TouchState`.
pub trait TouchStateProvider {
    fn position(&self) -> Vec2;
    fn prev_position(&self) -> Vec2;
    fn id(&self) -> u64;
    fn started(&self) -> bool;
    fn ended(&self) -> bool;
}

/// Minimal input interface required by the UI.
pub trait TouchInputProvider {
    type Touch: TouchStateProvider;

    fn first_touch(&self) -> Option<&Self::Touch>;
    fn touch(&self, id: u64) -> Option<&Self::Touch>;
    fn touches(&self) -> &[Self::Touch];
    fn primary_clicked(&self) -> bool;
    fn mouse_delta(&self) -> Vec2;
    fn cursor_position(&self) -> Vec2;

    /// Whether the left mouse button is currently held.
    fn primary_down(&self) -> bool {
        self.primary_clicked()
    }

    /// Whether the left mouse button was released this frame.
    fn primary_released(&self) -> bool {
        false
    }

    fn scroll_delta(&self) -> f32 {
        0.0
    }

    /// Persistent hint that the current session uses touch controls.
    fn touch_mode(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct TestTouch {
        id: u64,
        position: Vec2,
        prev: Vec2,
        started: bool,
        ended: bool,
    }

    impl TouchStateProvider for TestTouch {
        fn position(&self) -> Vec2 {
            self.position
        }
        fn prev_position(&self) -> Vec2 {
            self.prev
        }
        fn id(&self) -> u64 {
            self.id
        }
        fn started(&self) -> bool {
            self.started
        }
        fn ended(&self) -> bool {
            self.ended
        }
    }

    #[derive(Default)]
    struct TestInput {
        touches: Vec<TestTouch>,
    }

    impl TouchInputProvider for TestInput {
        type Touch = TestTouch;
        fn first_touch(&self) -> Option<&Self::Touch> {
            self.touches.first()
        }
        fn touch(&self, id: u64) -> Option<&Self::Touch> {
            self.touches.iter().find(|touch| touch.id == id)
        }
        fn touches(&self) -> &[Self::Touch] {
            &self.touches
        }
        fn primary_clicked(&self) -> bool {
            false
        }
        fn mouse_delta(&self) -> Vec2 {
            Vec2::ZERO
        }
        fn cursor_position(&self) -> Vec2 {
            Vec2::ZERO
        }
        fn touch_mode(&self) -> bool {
            true
        }
    }

    #[test]
    fn touch_ids_do_not_collide_with_mouse() {
        let input = TestInput {
            touches: vec![TestTouch {
                id: 0,
                position: Vec2::new(10.0, 20.0),
                prev: Vec2::new(8.0, 20.0),
                started: true,
                ..TestTouch::default()
            }],
        };
        let snapshot = UiInput::from_provider(&input);
        assert_eq!(snapshot.pointers.len(), 1);
        assert_eq!(snapshot.pointers[0].id, TOUCH_ID_BIT);
        assert_eq!(snapshot.pointers[0].kind, PointerKind::Touch);
        assert!(snapshot.pointers[0].pressed);
        assert_eq!(snapshot.pointers[0].delta(), Vec2::new(2.0, 0.0));
    }
}
