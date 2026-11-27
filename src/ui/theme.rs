//! Colours come only from GPUI's built-in palette (`black`, `white`,
//! `opaque_grey`, `blue`, `green`, `red`); these names just say where each is used.

use gpui::{Hsla, black, blue, green, opaque_grey, red, white};

pub const MONO: &str = "DejaVu Sans Mono";

pub fn bg() -> Hsla { white() }
pub fn panel() -> Hsla { white() }
pub fn field() -> Hsla { white() }
pub fn border() -> Hsla { opaque_grey(0.85, 1.0) }
pub fn text() -> Hsla { black() }
pub fn muted() -> Hsla { opaque_grey(0.45, 1.0) }
pub fn accent() -> Hsla { blue() }
pub fn accent_dim() -> Hsla { blue().opacity(0.12) }
pub fn ok() -> Hsla { green() }
pub fn bad() -> Hsla { red() }
