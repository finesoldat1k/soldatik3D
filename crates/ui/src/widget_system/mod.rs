//! Widget system module - immediate API with retained internals.
//!
//! This module provides a comprehensive UI widget system with:
//! - Hybrid immediate/retained mode: immediate API builds persistent widget tree
//! - Flexbox-style layout engine
//! - Component-based architecture (ECS-style)
//! - Hit-test + bubbling event system
//! - Built-in widgets: Button, Label, Container, Dropdown, VirtualJoystick, TouchCamera, ScrollPanel

pub mod core;
pub mod tree;
pub mod layout;
pub mod events;
pub mod widgets;
pub mod immediate;

// Re-export core types
pub use core::{
    WidgetId,
    Rect,
    LayoutProps,
    FlexDirection,
    AlignItems,
    JustifyContent,
    FlexBasis,
    ComputedLayout,
    PointerState,
    InteractionEvent,
    EventPropagation,
    Component,
    ComponentStore,
    WidgetState,
    LayoutConstraints,
    InputEvent,
    DrawList,
    Theme,
};
pub use glam::Vec2;

// Re-export tree types
pub use tree::{
    WidgetNode,
    Widget,
    UpdateContext,
};

// Re-export layout
pub use layout::LayoutEngine;

// Re-export events
pub use events::EventDispatcher;

// Re-export widgets
pub use widgets::{
    button::{Button, ButtonStyle, ButtonComponent},
    container::{Container, ContainerStyle},
    label::{Label, TextAlignment},
    dropdown::{Dropdown, DropdownStyle, DropdownComponent},
    virtual_joystick::{VirtualJoystick, VirtualJoystickStyle, VirtualJoystickComponent},
    touch_camera::{TouchCamera, TouchCameraStyle, TouchCameraComponent},
    scroll_panel::{ScrollPanel, ScrollPanelStyle, ScrollPanelComponent},
};

// Re-export immediate mode
pub use immediate::{
    Ui,
    UiFrame,
    ButtonBuilder,
    ContainerBuilder,
    LabelBuilder,
    DropdownBuilder,
    VirtualJoystickBuilder,
    TouchCameraBuilder,
    ScrollPanelBuilder,
};