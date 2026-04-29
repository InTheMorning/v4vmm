//! Token-driven UI primitives — the **second** layer of the design system,
//! sitting directly above [`crate::ui::tokens`] and below composite
//! components.
//!
//! A primitive is the smallest reusable, presentational building block in the
//! UI. It must:
//!
//! * Resolve every color through [`crate::ui::tokens::SemanticColor`] — never
//!   accept raw `Rgba` from callers.
//! * Be Apple HIG-compliant by default (corner radii, hit targets, type
//!   weights) — callers shouldn't need to know HIG to produce a correct UI.
//! * Carry no domain logic (no DB calls, no service references, no GPUI
//!   `Entity<T>` state). All event handlers are pure callbacks the consumer
//!   wires up.
//! * Be implemented as `RenderOnce` builders so they can be cheaply composed
//!   inside a parent's `render()` call without owning state.
//!
//! See `docs/architecture-diagrams.md` § 2.3 for where this layer fits in the
//! ideal architecture pyramid.

#![warn(clippy::pedantic)]

pub mod button;
pub mod divider;
pub mod image;
pub mod label;
pub mod popover;
pub mod stack;
pub mod surface;

pub use button::{Button, ButtonSize, ButtonVariant};
pub use divider::Divider;
pub use image::{Image, ImageSize};
pub use label::{Label, LabelVariant};
pub use popover::{Popover, PopoverAlignment, PopoverPlacement};
pub use stack::{HStack, Spacer, StackAlignment, VStack, ZStack};
pub use surface::{Surface, SurfaceElevation};
