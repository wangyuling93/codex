//! Policy for fully transparent TUI backgrounds.
//!
//! When enabled, every painted cell uses the terminal's default background so
//! terminal-level transparency shows through. Reverse video and other modifiers
//! are left unchanged so selection and emphasis styles keep working.
//!
//! Call sites:
//! - live viewport: [`apply_to_buffer`] from `custom_terminal::Terminal::try_draw`
//! - scrollback history: [`strip_line_backgrounds`] from `insert_history` before write

use ratatui::buffer::Buffer;
use ratatui::style::Color;
use ratatui::text::Line;

/// Replace every cell background in a ratatui buffer with the terminal default.
pub(crate) fn apply_to_buffer(buffer: &mut Buffer) {
    for cell in &mut buffer.content {
        cell.set_bg(Color::Reset);
    }
}

/// Strip explicit backgrounds from a ratatui line and its spans.
pub(crate) fn strip_line_backgrounds(mut line: Line<'static>) -> Line<'static> {
    line.style.bg = Some(Color::Reset);
    for span in &mut line.spans {
        span.style.bg = Some(Color::Reset);
    }
    line
}

#[cfg(test)]
#[path = "transparent_background_tests.rs"]
mod tests;
