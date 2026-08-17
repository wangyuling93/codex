//! Rounded outline geometry for the composer input box.

use crate::render::Insets;
use crate::ui_consts::LIVE_PREFIX_COLS;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::widgets::Block;
use ratatui::widgets::BorderType;
use ratatui::widgets::Borders;
use ratatui::widgets::Widget;

pub(super) const FRAME: u16 = 1;
pub(super) const PAD: u16 = 1;
const LEFT_CHROME: u16 = FRAME + PAD + LIVE_PREFIX_COLS;
const RIGHT_CHROME: u16 = PAD + FRAME;

pub(super) fn content_insets() -> Insets {
    Insets::tlbr(
        /*top*/ FRAME,
        LEFT_CHROME,
        /*bottom*/ FRAME,
        RIGHT_CHROME,
    )
}

pub(super) fn inner_width(width: u16, right_reserve: u16) -> u16 {
    width.saturating_sub(LEFT_CHROME + RIGHT_CHROME + right_reserve)
}

pub(super) fn vertical_chrome() -> u16 {
    FRAME.saturating_mul(2)
}

pub(super) fn render(area: Rect, buf: &mut Buffer, style: Style) {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().add_modifier(Modifier::DIM))
        .style(style)
        .render(area, buf);
}
