use gpui::{px, rgb, FontWeight, Pixels, Rgba, Styled};

pub mod color {
    use super::*;

    pub fn bg_canvas() -> Rgba { rgb(0x0f1117) }
    pub fn bg_surface() -> Rgba { rgb(0x1a1d27) }
    pub fn bg_surface_hi() -> Rgba { rgb(0x232735) }
    pub fn bg_selected() -> Rgba { rgb(0x2a3352) }
    
    pub fn border_subtle() -> Rgba { rgb(0x2a2d3a) }
    pub fn border_strong() -> Rgba { rgb(0x3d4153) }
    
    pub fn text_primary() -> Rgba { rgb(0xeceef5) }
    pub fn text_secondary() -> Rgba { rgb(0xb4bacb) }
    pub fn text_muted() -> Rgba { rgb(0x8a90a4) }
    pub fn text_on_accent() -> Rgba { rgb(0x0b0d13) }
    
    pub fn accent() -> Rgba { rgb(0x8b9bff) }
    pub fn accent_hover() -> Rgba { rgb(0xa5b2ff) }
    pub fn accent_pressed() -> Rgba { rgb(0x7486f5) }
    
    pub fn focus_ring() -> Rgba { rgb(0xa8b6ff) }
    
    pub fn status_success() -> Rgba { rgb(0x7dd67d) }
    pub fn status_warning() -> Rgba { rgb(0xffd666) }
    pub fn status_danger() -> Rgba { rgb(0xff8585) }
    
    pub fn diff_match() -> Rgba { rgb(0x6fd4a3) }
    pub fn diff_different() -> Rgba { rgb(0xffd27a) }
    pub fn diff_missing() -> Rgba { rgb(0xffa07f) }
}

pub mod badges {
    use super::*;

    pub fn type_color(entity_type: &str) -> Rgba {
        match entity_type {
            "feed" => rgb(0xe8943a),
            "track" => rgb(0x3ac4c4),
            "publisher" => rgb(0xe84393),
            "artist" => rgb(0x4caf82),
            "release" => rgb(0x6c7cff),
            "recording" => rgb(0xb06cf4),
            _ => color::accent(),
        }
    }

    pub fn text_color(entity_type: &str) -> Rgba {
        match entity_type {
            "feed" | "track" => rgb(0x111318),
            _ => rgb(0xffffff),
        }
    }

    pub fn emoji(entity_type: &str) -> &'static str {
        match entity_type {
            "feed" => "\u{1F4E1}",
            "track" => "\u{1F3B6}",
            "publisher" => "\u{1F3E2}",
            _ => "\u{1F3B5}",
        }
    }
}

pub mod layout {
    use super::*;
    pub const TAB_BAR_HEIGHT: Pixels = px(44.0);
    pub const ROW_HEIGHT: Pixels = px(36.0);
    pub const HIT_TARGET_MIN: Pixels = px(28.0);
    pub const INSPECTOR_WIDTH: Pixels = px(360.0);
}

pub mod glyphs {
    pub const DIFF_MATCH: &str = "=";
    pub const DIFF_DIFFERENT: &str = "\u{2260}";
    pub const DIFF_MISSING: &str = "\u{2205}";
    pub const STATUS_SUCCESS: &str = "\u{2713}";
    pub const STATUS_WARNING: &str = "\u{26A0}";
    pub const STATUS_DANGER: &str = "\u{2717}";
}

pub mod spacing {
    use super::*;
    pub const XXS: Pixels = px(2.0);
    pub const XS: Pixels = px(4.0);
    pub const SM: Pixels = px(8.0);
    pub const MD: Pixels = px(12.0);
    pub const LG: Pixels = px(16.0);
    pub const XL: Pixels = px(24.0);
    pub const XXL: Pixels = px(32.0);
}

pub mod radius {
    use super::*;
    pub const SM: Pixels = px(4.0);
    pub const MD: Pixels = px(6.0);
    pub const LG: Pixels = px(10.0);
}

pub mod typography {
    use super::*;
    
    pub const SIZE_TITLE: Pixels = px(20.0);
    pub const SIZE_HEADLINE: Pixels = px(15.0);
    pub const SIZE_BODY: Pixels = px(13.0);
    pub const SIZE_CAPTION: Pixels = px(12.0);
    pub const SIZE_MICRO: Pixels = px(11.0);

    pub fn type_title<T: Styled>(el: T) -> T {
        el.text_size(SIZE_TITLE).font_weight(FontWeight::SEMIBOLD)
    }

    pub fn type_headline<T: Styled>(el: T) -> T {
        el.text_size(SIZE_HEADLINE).font_weight(FontWeight::SEMIBOLD)
    }

    pub fn type_body<T: Styled>(el: T) -> T {
        el.text_size(SIZE_BODY).font_weight(FontWeight::NORMAL)
    }

    pub fn type_caption<T: Styled>(el: T) -> T {
        el.text_size(SIZE_CAPTION).font_weight(FontWeight::NORMAL)
    }

    pub fn type_caption_strong<T: Styled>(el: T) -> T {
        el.text_size(SIZE_CAPTION).font_weight(FontWeight::MEDIUM)
    }

    pub fn type_micro<T: Styled>(el: T) -> T {
        el.text_size(SIZE_MICRO).font_weight(FontWeight::MEDIUM)
    }
}
