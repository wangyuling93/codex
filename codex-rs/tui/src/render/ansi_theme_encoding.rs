//! Shared bat/syntect ANSI palette encoding.
//!
//! Themes such as two-face `ansi` / `base16` / `base16-256` store palette
//! semantics in the syntect `Color` alpha channel rather than true RGB:
//!
//! - `a = 0x00` — `r` is an ANSI palette index (0–255)
//! - `a = 0x01` — use the terminal default foreground/background
//! - `a = 0xFF` — ordinary opaque RGB

/// Alpha marker: `r` holds an ANSI palette index, not a red channel.
pub(super) const ANSI_ALPHA_INDEX: u8 = 0x00;
/// Alpha marker: use the terminal default color.
pub(super) const ANSI_ALPHA_DEFAULT: u8 = 0x01;
/// Alpha for ordinary opaque RGB colors.
pub(super) const OPAQUE_ALPHA: u8 = 0xFF;
