//! Layout primitives for the retained widget tree.
//!
//! The engine deliberately implements a compact subset of flexbox: row/column
//! flow, auto/fixed/percent/fill sizing, gaps, padding, alignment and absolute
//! placement. That is enough for game HUDs and settings screens while staying
//! easy to reason about and cheap to compute every frame.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::geometry::{EdgeInsets, Rect};

/// Stable identifier of a UI node.
pub type UiId = u64;

/// Identifier of the implicit root node.
pub const UI_ROOT_ID: UiId = 0x5D01_D123_0000_0001;

/// Computes a stable 64-bit id from arbitrary data.
pub fn stable_id(value: impl Hash) -> UiId {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

/// Size of a node along one axis.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Size {
    /// Fit the widget's measured content.
    #[default]
    Auto,
    /// Exact size in pixels.
    Fixed(f32),
    /// Percentage of the parent's content box (`100.0` = full parent).
    Percent(f32),
    /// Share the remaining space with other `Fill` siblings.
    Fill,
}

impl Size {
    pub fn fixed(value: f32) -> Self {
        Self::Fixed(value.max(0.0))
    }

    pub fn percent(value: f32) -> Self {
        Self::Percent(value.max(0.0))
    }

    pub fn is_fill(self) -> bool {
        matches!(self, Self::Fill)
    }

    /// Resolves this size against an axis with the given available space and
    /// the node's intrinsic content size.
    pub fn resolve(self, available: f32, intrinsic: f32) -> f32 {
        let value = match self {
            Self::Auto => intrinsic,
            Self::Fixed(value) => value,
            Self::Percent(value) => available * value / 100.0,
            Self::Fill => available,
        };
        value.max(0.0)
    }
}

/// Main flow direction of a container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlexDirection {
    Row,
    #[default]
    Column,
}

/// Cross-axis alignment of children.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlignItems {
    #[default]
    Start,
    Center,
    End,
    Stretch,
}

/// Main-axis distribution of children.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JustifyContent {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
}

/// Declarative layout properties attached to a node.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    /// Optional explicit id. Without it the node gets a stable auto id derived
    /// from its parent, child index and widget kind.
    pub id: Option<UiId>,
    pub direction: FlexDirection,
    pub align_items: AlignItems,
    pub justify_content: JustifyContent,
    pub padding: EdgeInsets,
    pub margin: EdgeInsets,
    pub width: Size,
    pub height: Size,
    pub gap: f32,
    /// Relative weight for `Size::Fill`; all fill siblings share leftover
    /// space proportionally to this value.
    pub grow: f32,
    /// Absolute placement relative to the parent's content box. Absolute
    /// children do not participate in the normal flow.
    pub absolute: Option<Rect>,
    /// Enables vertical scrolling for the container's children.
    pub scrollable: bool,
    /// Clips children to the node bounds.
    pub clip_contents: bool,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            id: None,
            direction: FlexDirection::Column,
            align_items: AlignItems::Start,
            justify_content: JustifyContent::Start,
            padding: EdgeInsets::ZERO,
            margin: EdgeInsets::ZERO,
            width: Size::Auto,
            height: Size::Auto,
            gap: 0.0,
            grow: 1.0,
            absolute: None,
            scrollable: false,
            clip_contents: false,
        }
    }
}

impl Layout {
    pub fn row() -> Self {
        Self {
            direction: FlexDirection::Row,
            ..Self::default()
        }
    }

    pub fn column() -> Self {
        Self::default()
    }

    pub fn id(mut self, id: impl Hash) -> Self {
        self.id = Some(stable_id(id));
        self
    }

    pub fn padding(mut self, value: f32) -> Self {
        self.padding = EdgeInsets::all(value);
        self
    }

    pub fn padding_edges(mut self, edges: EdgeInsets) -> Self {
        self.padding = edges;
        self
    }

    pub fn padding_horizontal(mut self, value: f32) -> Self {
        self.padding.left = value;
        self.padding.right = value;
        self
    }

    pub fn padding_vertical(mut self, value: f32) -> Self {
        self.padding.top = value;
        self.padding.bottom = value;
        self
    }

    pub fn margin(mut self, value: f32) -> Self {
        self.margin = EdgeInsets::all(value);
        self
    }

    pub fn margin_edges(mut self, edges: EdgeInsets) -> Self {
        self.margin = edges;
        self
    }

    pub fn gap(mut self, value: f32) -> Self {
        self.gap = value.max(0.0);
        self
    }

    pub fn width(mut self, value: f32) -> Self {
        self.width = Size::Fixed(value);
        self
    }

    pub fn height(mut self, value: f32) -> Self {
        self.height = Size::Fixed(value);
        self
    }

    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = Size::Fixed(width);
        self.height = Size::Fixed(height);
        self
    }

    pub fn width_size(mut self, size: Size) -> Self {
        self.width = size;
        self
    }

    pub fn height_size(mut self, size: Size) -> Self {
        self.height = size;
        self
    }

    pub fn fill_width(mut self) -> Self {
        self.width = Size::Fill;
        self
    }

    pub fn fill_height(mut self) -> Self {
        self.height = Size::Fill;
        self
    }

    pub fn fill(mut self) -> Self {
        self.width = Size::Fill;
        self.height = Size::Fill;
        self
    }

    pub fn align_items(mut self, align: AlignItems) -> Self {
        self.align_items = align;
        self
    }

    pub fn justify_content(mut self, justify: JustifyContent) -> Self {
        self.justify_content = justify;
        self
    }

    pub fn center(mut self) -> Self {
        self.align_items = AlignItems::Center;
        self.justify_content = JustifyContent::Center;
        self
    }

    pub fn grow(mut self, grow: f32) -> Self {
        self.grow = grow.max(0.0);
        self
    }

    pub fn absolute(mut self, rect: Rect) -> Self {
        self.absolute = Some(rect);
        self
    }

    pub fn scrollable(mut self) -> Self {
        self.scrollable = true;
        self.clip_contents = true;
        self
    }

    pub fn clip(mut self) -> Self {
        self.clip_contents = true;
        self
    }

    pub fn main_axis(&self) -> FlexDirection {
        self.direction
    }

    pub fn is_row(&self) -> bool {
        matches!(self.direction, FlexDirection::Row)
    }
}

impl PartialEq<FlexDirection> for Layout {
    fn eq(&self, other: &FlexDirection) -> bool {
        self.direction == *other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_resolve_against_available_space() {
        assert_eq!(Size::Auto.resolve(100.0, 30.0), 30.0);
        assert_eq!(Size::Fixed(42.0).resolve(100.0, 30.0), 42.0);
        assert_eq!(Size::Percent(50.0).resolve(100.0, 30.0), 50.0);
        assert_eq!(Size::Fill.resolve(100.0, 30.0), 100.0);
        assert_eq!(Size::Fixed(-4.0).resolve(100.0, 30.0), 0.0);
    }

    #[test]
    fn explicit_ids_are_stable() {
        let a = stable_id("settings");
        let b = stable_id("settings");
        let c = stable_id("other");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(Layout::default().id("x").id, Some(stable_id("x")));
    }
}
