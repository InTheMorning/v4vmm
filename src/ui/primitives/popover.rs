//! Popover primitive — the canonical floating panel that points at its
//! trigger.
//!
//! This is the chrome of an Apple HIG popover (corner radius, arrow / tear
//! droplet on the edge facing the trigger, dismissal on outside-click and
//! Escape, focus trap), with no domain knowledge baked in. Composites such as
//! `crate::ui::playlist_popover::AddToPlaylistPopover` build on top of it by
//! providing the `trigger` and `content` callbacks.
//!
//! Internally, the primitive wraps `gpui_component::popover::Popover` for the
//! anchoring + dismissal infrastructure (which requires window-level overlay
//! plumbing we don't reimplement) and disables its built-in surface via
//! `.appearance(false)`. The user-facing chrome — surface, arrow, padding —
//! is rendered by us so it stays token-driven.
//!
//! ## API shape
//!
//! ```ignore
//! Popover::new("my-popover")
//!     .placement(PopoverPlacement::Below)
//!     .alignment(PopoverAlignment::Start)
//!     .trigger(Button::tinted("trig").label("Open"))
//!     .content(|_window, _cx| my_content_div())
//! ```
//!
//! ## Limitations (v1)
//!
//! * Placement is currently `Above` or `Below` only — gpui-component's
//!   `Anchor` enum does not yet expose left/right side anchoring.
//! * The arrow is rendered immediately adjacent to the popover surface, not
//!   floating in the gap between trigger and surface. This matches `NSPopover`
//!   well enough for v1.

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{
    canvas, div, point, prelude::*, AnyElement, App, Bounds, Corner, ElementId, IntoElement,
    PathBuilder, Pixels, RenderOnce, Window,
};
use gpui_component::{popover::Popover as ComponentPopover, Selectable};

use crate::ui::primitives::{Surface, SurfaceElevation};
use crate::ui::tokens::{Appearance, SemanticColor, Spacing};

// ---------------------------------------------------------------------------
// Placement / alignment
// ---------------------------------------------------------------------------

/// Which side of the trigger the popover hangs off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PopoverPlacement {
    /// Popover sits below the trigger; arrow points up.
    #[default]
    Below,
    /// Popover sits above the trigger; arrow points down.
    Above,
}

/// Where, along the trigger's edge, the popover anchors.
///
/// `Center` is supported in newer gpui-component versions but the currently
/// pinned `0.5.1` only exposes the four corners — `Center` therefore maps to
/// `Start` for now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PopoverAlignment {
    /// Anchor at the trigger's leading edge (left in LTR).
    #[default]
    Start,
    /// Anchor at the trigger's center. Currently treated as `Start` (see
    /// note above).
    Center,
    /// Anchor at the trigger's trailing edge (right in LTR).
    End,
}

impl PopoverPlacement {
    fn corner(self, alignment: PopoverAlignment) -> Corner {
        match (self, alignment) {
            (Self::Below, PopoverAlignment::Start | PopoverAlignment::Center) => Corner::TopLeft,
            (Self::Below, PopoverAlignment::End) => Corner::TopRight,
            (Self::Above, PopoverAlignment::Start | PopoverAlignment::Center) => Corner::BottomLeft,
            (Self::Above, PopoverAlignment::End) => Corner::BottomRight,
        }
    }
}

// ---------------------------------------------------------------------------
// Type aliases
// ---------------------------------------------------------------------------

type ContentBuilder = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement + 'static>;
type OpenChangeHandler = Rc<dyn Fn(&bool, &mut Window, &mut App) + 'static>;
type TriggerInjector = Box<dyn FnOnce(ComponentPopover) -> ComponentPopover + 'static>;

// ---------------------------------------------------------------------------
// Popover primitive
// ---------------------------------------------------------------------------

#[derive(IntoElement)]
#[must_use]
pub struct Popover {
    id: ElementId,
    placement: PopoverPlacement,
    alignment: PopoverAlignment,
    appearance: Appearance,
    surface_padding: Spacing,
    trigger: Option<TriggerInjector>,
    content: Option<ContentBuilder>,
    default_open: bool,
    open: Option<bool>,
    on_open_change: Option<OpenChangeHandler>,
    overlay_closable: bool,
}

impl Popover {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            placement: PopoverPlacement::default(),
            alignment: PopoverAlignment::default(),
            appearance: Appearance::Dark,
            surface_padding: Spacing::XS,
            trigger: None,
            content: None,
            default_open: false,
            open: None,
            on_open_change: None,
            overlay_closable: true,
        }
    }

    pub fn placement(mut self, placement: PopoverPlacement) -> Self {
        self.placement = placement;
        self
    }

    pub fn alignment(mut self, alignment: PopoverAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn appearance(mut self, appearance: Appearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// Inner padding applied to the surface body (around the user's content).
    pub fn surface_padding(mut self, padding: Spacing) -> Self {
        self.surface_padding = padding;
        self
    }

    pub fn trigger<T>(mut self, trigger: T) -> Self
    where
        T: Selectable + IntoElement + 'static,
    {
        self.trigger = Some(Box::new(move |popover| popover.trigger(trigger)));
        self
    }

    /// Set the content builder. Called every render — avoid creating new
    /// entities inside.
    pub fn content<F, E>(mut self, content: F) -> Self
    where
        E: IntoElement,
        F: Fn(&mut Window, &mut App) -> E + 'static,
    {
        self.content = Some(Rc::new(move |window, cx| {
            content(window, cx).into_any_element()
        }));
        self
    }

    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    /// Force the open state. Must be paired with [`Self::on_open_change`] so
    /// the caller can react to user-driven state changes.
    pub fn open(mut self, open: bool) -> Self {
        self.open = Some(open);
        self
    }

    pub fn on_open_change<F>(mut self, callback: F) -> Self
    where
        F: Fn(&bool, &mut Window, &mut App) + 'static,
    {
        self.on_open_change = Some(Rc::new(callback));
        self
    }

    pub fn overlay_closable(mut self, closable: bool) -> Self {
        self.overlay_closable = closable;
        self
    }
}

// Arrow droplet dimensions are sourced from the spacing scale so they
// automatically inherit any future UI scale factor — no hard-coded points.
// `Spacing::LG` (16) for width and `Spacing::SM` (8) for height matches the
// macOS `NSPopover` tip's ~2:1 base-to-height ratio.
const ARROW_WIDTH_TOKEN: Spacing = Spacing::LG;
const ARROW_HEIGHT_TOKEN: Spacing = Spacing::SM;

impl RenderOnce for Popover {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let placement = self.placement;
        let alignment = self.alignment;
        let appearance = self.appearance;
        let surface_padding = self.surface_padding;
        let content_builder = self.content;
        let arrow_color = SemanticColor::TertiarySystemBackground.resolve(appearance);

        let mut popover = ComponentPopover::new(self.id)
            .anchor(placement.corner(alignment))
            .appearance(false)
            .overlay_closable(self.overlay_closable)
            .default_open(self.default_open);

        if let Some(open) = self.open {
            popover = popover.open(open);
        }
        if let Some(handler) = self.on_open_change {
            popover = popover.on_open_change(move |is_open, window, cx| {
                handler(is_open, window, cx);
            });
        }
        if let Some(inject) = self.trigger {
            popover = inject(popover);
        }
        if let Some(builder) = content_builder {
            popover = popover.content(move |_state, window, cx| {
                let arrow = arrow_div(placement, arrow_color);
                let body = Surface::new(SurfaceElevation::Floating)
                    .padding(surface_padding)
                    .appearance(appearance)
                    .child(builder(window, cx));

                let mut wrapper = div().flex().flex_col();
                wrapper = match alignment {
                    PopoverAlignment::Start => wrapper.items_start(),
                    PopoverAlignment::Center => wrapper.items_center(),
                    PopoverAlignment::End => wrapper.items_end(),
                };
                // Indent the arrow slightly from the alignment edge so it
                // sits *over* the trigger rather than at the very corner —
                // matches HIG's centered tip on a flush-aligned popover.
                let arrow_inset = match alignment {
                    PopoverAlignment::Start => div().pl(Spacing::SM.px()).child(arrow),
                    PopoverAlignment::Center => div().child(arrow),
                    PopoverAlignment::End => div().pr(Spacing::SM.px()).child(arrow),
                };

                if placement == PopoverPlacement::Below {
                    wrapper.child(arrow_inset).child(body)
                } else {
                    wrapper.child(body).child(arrow_inset)
                }
            });
        }

        popover
    }
}

// ---------------------------------------------------------------------------
// Arrow rendering
// ---------------------------------------------------------------------------

/// Render the arrow droplet as a fixed-size `canvas` element. The triangle
/// is filled with `color` (which the caller picks to match the popover
/// surface background).
fn arrow_div(placement: PopoverPlacement, color: gpui::Rgba) -> impl IntoElement {
    div()
        .w(ARROW_WIDTH_TOKEN.px())
        .h(ARROW_HEIGHT_TOKEN.px())
        .child(canvas(
            |_bounds, _window, _cx| {},
            move |bounds, (), window, _cx| {
                paint_arrow(bounds, placement, color, window);
            },
        ))
}

fn paint_arrow(
    bounds: Bounds<Pixels>,
    placement: PopoverPlacement,
    color: gpui::Rgba,
    window: &mut Window,
) {
    let origin = bounds.origin;
    let w = bounds.size.width;
    let h = bounds.size.height;

    let (apex, left, right) = match placement {
        // Tip points up at the trigger above; base sits flush with the
        // popover surface below.
        PopoverPlacement::Below => (
            point(origin.x + w / 2.0, origin.y),
            point(origin.x, origin.y + h),
            point(origin.x + w, origin.y + h),
        ),
        // Tip points down at the trigger below; base sits flush above.
        PopoverPlacement::Above => (
            point(origin.x + w / 2.0, origin.y + h),
            point(origin.x, origin.y),
            point(origin.x + w, origin.y),
        ),
    };

    let mut builder = PathBuilder::fill();
    builder.move_to(apex);
    builder.line_to(right);
    builder.line_to(left);
    builder.close();
    if let Ok(path) = builder.build() {
        window.paint_path(path, color);
    }
}
