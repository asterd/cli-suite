//! Fixture crate for axt-outline.

use std::fmt::Display;

/// Public widget data.
pub struct Widget {
    id: u64,
}

impl Widget {
    /// Create a widget.
    pub fn new(id: u64) -> Self {
        Self { id }
    }

    fn id(&self) -> u64 {
        self.id
    }
}

/// Render values.
pub trait Render {
    /// Render to a string.
    fn render(&self) -> String;
}

pub(crate) enum Mode {
    Fast,
    Slow,
}

type WidgetId = u64;

const DEFAULT_ID: u64 = 1;

pub mod nested {
    /// Nested public function.
    pub fn nested_fn<T: ToString>(value: T) -> String {
        value.to_string()
    }
}

macro_rules! fixture_macro {
    () => {};
}

pub fn display<T: Display>(value: T) -> String {
    value.to_string()
}
