use super::*;
use crate::custom_terminal::Terminal;
use crate::test_backend::VT100Backend;
use pretty_assertions::assert_eq;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Span;

#[test]
fn apply_to_buffer_resets_backgrounds_and_preserves_reverse() {
    let area = Rect::new(0, 0, 2, 1);
    let mut buffer = ratatui::buffer::Buffer::empty(area);
    for cell in &mut buffer.content {
        cell.set_bg(Color::Red);
        cell.modifier = Modifier::REVERSED | Modifier::BOLD;
    }

    apply_to_buffer(&mut buffer);

    for cell in &buffer.content {
        assert_eq!(cell.bg, Color::Reset);
        assert!(cell.modifier.contains(Modifier::REVERSED));
        assert!(cell.modifier.contains(Modifier::BOLD));
        assert!(!cell.modifier.contains(Modifier::UNDERLINED));
    }
}

#[test]
fn strip_line_backgrounds_resets_line_and_span_backgrounds() {
    let line = Line::from(Span::styled(
        "A",
        Style::default().fg(Color::Cyan).bg(Color::Red),
    ))
    .style(Style::default().bg(Color::Blue));

    let stripped = strip_line_backgrounds(line);

    assert_eq!(stripped.style.bg, Some(Color::Reset));
    assert_eq!(stripped.spans[0].style.bg, Some(Color::Reset));
    assert_eq!(stripped.spans[0].style.fg, Some(Color::Cyan));
}

#[test]
fn terminal_full_transparency_repaints_backgrounds_preserves_reverse_and_can_be_disabled() {
    let area = Rect::new(0, 0, 3, 1);
    let mut terminal =
        Terminal::with_options(VT100Backend::new(area.width, area.height)).expect("terminal");
    terminal.set_viewport_area(area);

    terminal
        .draw(|frame| {
            frame
                .buffer_mut()
                .set_style(area, Style::default().bg(Color::Red));
            frame.buffer_mut().set_string(
                0,
                0,
                "X",
                Style::default()
                    .fg(Color::Cyan)
                    .bg(Color::Red)
                    .add_modifier(Modifier::REVERSED),
            );
        })
        .expect("opaque draw");

    for column in 0..area.width {
        assert_ne!(
            terminal
                .backend()
                .vt100()
                .screen()
                .cell(/*row*/ 0, column)
                .expect("opaque screen cell")
                .bgcolor(),
            vt100::Color::Default
        );
    }

    terminal.set_full_transparency(/*enabled*/ true);
    terminal
        .draw(|frame| {
            frame
                .buffer_mut()
                .set_style(area, Style::default().bg(Color::Red));
            frame.buffer_mut().set_string(
                0,
                0,
                "X",
                Style::default()
                    .fg(Color::Cyan)
                    .bg(Color::Red)
                    .add_modifier(Modifier::REVERSED),
            );
        })
        .expect("transparent draw");

    for column in 0..area.width {
        assert_eq!(
            terminal
                .backend()
                .vt100()
                .screen()
                .cell(/*row*/ 0, column)
                .expect("transparent screen cell")
                .bgcolor(),
            vt100::Color::Default
        );
    }
    let transparent_cell = terminal
        .backend()
        .vt100()
        .screen()
        .cell(/*row*/ 0, /*col*/ 0)
        .expect("transparent styled cell");
    assert!(transparent_cell.inverse());
    assert!(!transparent_cell.underline());

    terminal.set_full_transparency(/*enabled*/ false);
    terminal
        .draw(|frame| {
            frame
                .buffer_mut()
                .set_style(area, Style::default().bg(Color::Red));
            frame.buffer_mut().set_string(
                0,
                0,
                "X",
                Style::default()
                    .fg(Color::Cyan)
                    .bg(Color::Red)
                    .add_modifier(Modifier::REVERSED),
            );
        })
        .expect("opaque draw");

    let opaque_cell = terminal
        .backend()
        .vt100()
        .screen()
        .cell(/*row*/ 0, /*col*/ 0)
        .expect("opaque styled cell");
    assert_ne!(opaque_cell.bgcolor(), vt100::Color::Default);
    assert!(opaque_cell.inverse());
}
