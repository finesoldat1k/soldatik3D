//! Fonts compiled into the engine so text works with zero configuration.

/// Open Sans Regular, Apache-2.0. Latin, Cyrillic and Greek coverage.
pub const OPEN_SANS_REGULAR: &[u8] = include_bytes!("../../assets/OpenSans-Regular.ttf");

/// Open Sans Bold, Apache-2.0.
pub const OPEN_SANS_BOLD: &[u8] = include_bytes!("../../assets/OpenSans-Bold.ttf");

pub const OPEN_SANS_LICENSE: &str = include_str!("../../assets/LICENSE-OpenSans.txt");
