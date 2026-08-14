use super::*;
use crate::bottom_pane::ComposerMouseOutcome;
use crate::clipboard_copy::ClipboardLease;
use crate::render::renderable::Renderable;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use crossterm::event::MouseButton;
use crossterm::event::MouseEvent;
use crossterm::event::MouseEventKind;
use pretty_assertions::assert_eq;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use std::time::Duration;
use std::time::Instant;

fn mouse_event(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

#[tokio::test]
async fn cmd_c_does_not_copy_or_replace_a_composer_selection() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.bottom_pane
        .set_composer_text("hello world".to_string(), Vec::new(), Vec::new());
    chat.bottom_pane.select_composer_byte_range(6..11);

    chat.handle_key_event(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::SUPER));

    assert_eq!(chat.bottom_pane.composer_text(), "hello world");
    assert_eq!(
        chat.bottom_pane.composer_selected_text().as_deref(),
        Some("world")
    );
    assert!(chat.copy_toast.is_none());
}

#[tokio::test]
async fn ctrl_c_clears_the_composer_even_when_text_is_selected() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.thread_id = Some(ThreadId::new());
    chat.bottom_pane
        .set_composer_text("keep this draft".to_string(), Vec::new(), Vec::new());
    chat.bottom_pane.select_composer_byte_range(5..9);

    chat.handle_key_event(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));

    assert!(chat.bottom_pane.composer_is_empty());
}

#[tokio::test]
async fn ctrl_c_without_a_selection_still_clears_the_composer() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.thread_id = Some(ThreadId::new());
    chat.bottom_pane
        .set_composer_text("clear me".to_string(), Vec::new(), Vec::new());

    chat.handle_key_event(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));

    assert!(chat.bottom_pane.composer_is_empty());
}

#[tokio::test]
async fn finishing_a_mouse_selection_auto_copies_and_shows_a_toast() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.bottom_pane
        .set_composer_text("hello world".to_string(), Vec::new(), Vec::new());

    let width = 40;
    let height = 12;
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test terminal");
    terminal
        .draw(|frame| {
            chat.as_renderable()
                .render(frame.area(), frame.buffer_mut());
        })
        .expect("render composer");
    let textarea_area = chat
        .bottom_pane
        .composer_textarea_area()
        .expect("textarea geometry after render");

    assert!(
        chat.bottom_pane
            .handle_mouse_event(mouse_event(
                MouseEventKind::Down(MouseButton::Left),
                textarea_area.x,
                textarea_area.y,
            ))
            .is_handled()
    );
    assert!(
        chat.bottom_pane
            .handle_mouse_event(mouse_event(
                MouseEventKind::Drag(MouseButton::Left),
                textarea_area.x + 5,
                textarea_area.y,
            ))
            .is_handled()
    );
    let outcome = chat.bottom_pane.handle_mouse_event(mouse_event(
        MouseEventKind::Up(MouseButton::Left),
        textarea_area.x + 5,
        textarea_area.y,
    ));
    let ComposerMouseOutcome::SelectionFinished(text) = outcome else {
        panic!("expected a finished selection, got {outcome:?}");
    };
    assert!(!text.is_empty(), "expected selected text, got {text:?}");
    assert_eq!(chat.bottom_pane.composer_text(), "hello world");

    chat.copy_composer_text_with(&text, |_| Ok(Some(ClipboardLease::test())));
    assert!(chat.copy_toast.is_some());

    terminal
        .draw(|frame| {
            chat.as_renderable()
                .render(frame.area(), frame.buffer_mut());
        })
        .expect("render toast");
    let top_row: String = (0..width)
        .map(|x| terminal.backend().buffer()[(x, 0)].symbol().to_string())
        .collect();
    assert!(
        top_row.contains("Copied"),
        "expected top-right toast, got {top_row:?}"
    );
}

#[tokio::test]
async fn copy_failure_shows_an_error_toast() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.copy_composer_text_with("hello", |_| Err("clipboard unavailable".to_string()));

    let toast = chat.copy_toast.as_ref().expect("error toast");
    assert_eq!(
        toast.kind,
        super::super::composer_copy::CopyToastKind::Error
    );
}

#[tokio::test]
async fn copy_toast_expires_on_pre_draw_tick() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.bottom_pane
        .set_composer_text("hello".to_string(), Vec::new(), Vec::new());
    chat.bottom_pane.select_composer_byte_range(0..5);
    chat.copy_composer_text_with("hello", |_| Ok(Some(ClipboardLease::test())));
    assert!(chat.copy_toast.is_some());

    if let Some(toast) = chat.copy_toast.as_mut() {
        toast.expires_at = Instant::now()
            .checked_sub(Duration::from_secs(1))
            .unwrap_or_else(Instant::now);
    }
    chat.pre_draw_tick();
    assert!(chat.copy_toast.is_none());
}
