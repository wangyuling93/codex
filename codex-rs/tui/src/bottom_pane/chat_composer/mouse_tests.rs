use codex_protocol::models::local_image_label_text;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use crossterm::event::MouseButton;
use crossterm::event::MouseEvent;
use crossterm::event::MouseEventKind;
use insta::assert_snapshot;
use pretty_assertions::assert_eq;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use std::path::PathBuf;
use std::time::Instant;
use tokio::sync::mpsc::unbounded_channel;

use super::ChatComposer;
use super::LARGE_PASTE_CHAR_THRESHOLD;
use crate::app_event::AppEvent;
use crate::app_event_sender::AppEventSender;
use crate::render::renderable::Renderable;

fn test_composer() -> ChatComposer {
    let (tx, _rx) = unbounded_channel::<AppEvent>();
    ChatComposer::new(
        /*has_input_focus*/ true,
        AppEventSender::new(tx),
        /*enhanced_keys_supported*/ false,
        "Ask Codex to do anything".to_string(),
        /*disable_paste_burst*/ true,
    )
}

fn mouse_event(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

fn left_click(column: u16, row: u16) -> MouseEvent {
    mouse_event(MouseEventKind::Down(MouseButton::Left), column, row)
}

fn modifiers_on_row(terminal: &Terminal<TestBackend>, x: u16, y: u16, width: u16) -> Vec<Modifier> {
    (0..width)
        .map(|offset| {
            terminal.backend().buffer()[(x + offset, y)]
                .style()
                .add_modifier
        })
        .collect()
}

fn drag_select_range(composer: &mut ChatComposer, anchor: usize, active: usize) {
    let width = 100;
    let height = composer.desired_height(width);
    let area = Rect::new(/*x*/ 0, /*y*/ 0, width, height);
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test terminal");
    terminal
        .draw(|frame| composer.render(frame.area(), frame.buffer_mut()))
        .expect("render before selection");
    let [_, _, textarea_area, _] = composer.layout_areas(area);
    let anchor = textarea_area.x + anchor as u16;
    let active = textarea_area.x + active as u16;

    assert!(composer.handle_mouse_event(mouse_event(
        MouseEventKind::Down(MouseButton::Left),
        anchor,
        textarea_area.y,
    )));
    assert!(composer.handle_mouse_event(mouse_event(
        MouseEventKind::Drag(MouseButton::Left),
        active,
        textarea_area.y,
    )));
    assert!(composer.handle_mouse_event(mouse_event(
        MouseEventKind::Up(MouseButton::Left),
        active,
        textarea_area.y,
    )));
}

#[test]
fn clicking_composer_supports_inserting_and_deleting_at_position() {
    let mut composer = test_composer();
    composer.insert_str("hello world");
    let width = 30;
    let height = composer.desired_height(width);
    let area = Rect::new(/*x*/ 0, /*y*/ 0, width, height);
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test terminal");
    terminal
        .draw(|frame| composer.render(frame.area(), frame.buffer_mut()))
        .expect("initial render");
    let [_, _, textarea_area, _] = composer.layout_areas(area);

    assert!(composer.handle_mouse_event(left_click(textarea_area.x + 5, textarea_area.y)));
    composer.handle_key_event(KeyEvent::new(KeyCode::Char(','), KeyModifiers::NONE));
    assert_eq!(composer.draft.textarea.text(), "hello, world");

    terminal
        .draw(|frame| composer.render(frame.area(), frame.buffer_mut()))
        .expect("render after insertion");
    assert!(composer.handle_mouse_event(left_click(textarea_area.x + 7, textarea_area.y)));
    composer.handle_key_event(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));
    assert_eq!(composer.draft.textarea.text(), "hello, orld");

    terminal
        .draw(|frame| composer.render(frame.area(), frame.buffer_mut()))
        .expect("render after deletion");
    insta::with_settings!({snapshot_path => "../snapshots"}, {
        assert_snapshot!(
            "composer_mouse_click_edit",
            format!("cursor: {:?}\n{}", composer.cursor_pos(area), terminal.backend())
        );
    });
}

#[test]
fn dragging_selects_text_that_delete_removes() {
    let mut composer = test_composer();
    composer.insert_str("hello world");
    let width = 30;
    let height = composer.desired_height(width);
    let area = Rect::new(/*x*/ 0, /*y*/ 0, width, height);
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test terminal");
    terminal
        .draw(|frame| composer.render(frame.area(), frame.buffer_mut()))
        .expect("initial render");
    let [_, _, textarea_area, _] = composer.layout_areas(area);
    let selection_start = textarea_area.x + 5;
    let selection_end = textarea_area.x + 11;

    assert!(composer.handle_mouse_event(mouse_event(
        MouseEventKind::Down(MouseButton::Left),
        selection_start,
        textarea_area.y,
    )));
    assert!(composer.handle_mouse_event(mouse_event(
        MouseEventKind::Drag(MouseButton::Left),
        selection_end,
        textarea_area.y,
    )));
    assert!(composer.handle_mouse_event(mouse_event(
        MouseEventKind::Up(MouseButton::Left),
        selection_end,
        textarea_area.y,
    )));
    assert_eq!(composer.draft.textarea.selection_range(), Some(5..11));

    terminal
        .draw(|frame| composer.render(frame.area(), frame.buffer_mut()))
        .expect("render selection");
    let selection_modifiers = modifiers_on_row(
        &terminal,
        textarea_area.x,
        textarea_area.y,
        /*width*/ 11,
    );
    let selected = Modifier::REVERSED | Modifier::BOLD | Modifier::UNDERLINED;
    assert_eq!(
        selection_modifiers,
        vec![
            Modifier::empty(),
            Modifier::empty(),
            Modifier::empty(),
            Modifier::empty(),
            Modifier::empty(),
            selected,
            selected,
            selected,
            selected,
            selected,
            selected,
        ]
    );
    insta::with_settings!({snapshot_path => "../snapshots"}, {
        assert_snapshot!(
            "composer_mouse_drag_selection",
            format!(
                "selection: {:?}\nmodifiers: {selection_modifiers:?}\ncursor: {:?}\n{}",
                composer.draft.textarea.selection_range(),
                composer.cursor_pos(area),
                terminal.backend(),
            )
        );
    });

    composer.handle_key_event(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));
    assert_eq!(
        (
            composer.draft.textarea.text(),
            composer.cursor(),
            composer.draft.textarea.selection_range(),
        ),
        ("hello", 5, None)
    );
}

#[test]
fn dragging_past_right_edge_selects_the_final_character() {
    let mut composer = test_composer();
    let width = 30;
    let initial_height = composer.desired_height(width);
    let initial_area = Rect::new(/*x*/ 0, /*y*/ 0, width, initial_height);
    let [_, _, initial_textarea_area, _] = composer.layout_areas(initial_area);
    let text = "x".repeat(initial_textarea_area.width as usize);
    composer.insert_str(&text);
    let height = composer.desired_height(width);
    let area = Rect::new(/*x*/ 0, /*y*/ 0, width, height);
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test terminal");
    terminal
        .draw(|frame| composer.render(frame.area(), frame.buffer_mut()))
        .expect("initial render");
    let [_, _, textarea_area, _] = composer.layout_areas(area);

    assert!(composer.handle_mouse_event(mouse_event(
        MouseEventKind::Down(MouseButton::Left),
        textarea_area.x,
        textarea_area.y,
    )));
    assert!(composer.handle_mouse_event(mouse_event(
        MouseEventKind::Drag(MouseButton::Left),
        textarea_area.right(),
        textarea_area.y,
    )));
    assert!(composer.handle_mouse_event(mouse_event(
        MouseEventKind::Up(MouseButton::Left),
        textarea_area.right(),
        textarea_area.y,
    )));
    assert_eq!(
        composer.draft.textarea.selection_range(),
        Some(0..text.len())
    );

    composer.handle_key_event(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));
    assert_eq!(
        (
            composer.draft.textarea.text(),
            composer.cursor(),
            composer.draft.textarea.selection_range(),
        ),
        ("", 0, None)
    );
}

#[test]
fn selection_highlight_after_tab_uses_display_columns() {
    let mut composer = test_composer();
    composer.insert_str("a\tbc");
    let width = 30;
    let height = composer.desired_height(width);
    let area = Rect::new(/*x*/ 0, /*y*/ 0, width, height);
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test terminal");
    terminal
        .draw(|frame| composer.render(frame.area(), frame.buffer_mut()))
        .expect("initial render");
    let [_, _, textarea_area, _] = composer.layout_areas(area);

    assert!(composer.handle_mouse_event(mouse_event(
        MouseEventKind::Down(MouseButton::Left),
        textarea_area.x + 2,
        textarea_area.y,
    )));
    assert!(composer.handle_mouse_event(mouse_event(
        MouseEventKind::Drag(MouseButton::Left),
        textarea_area.x + 4,
        textarea_area.y,
    )));
    assert!(composer.handle_mouse_event(mouse_event(
        MouseEventKind::Up(MouseButton::Left),
        textarea_area.x + 4,
        textarea_area.y,
    )));
    terminal
        .draw(|frame| composer.render(frame.area(), frame.buffer_mut()))
        .expect("render selection");

    let selection_modifiers = modifiers_on_row(
        &terminal,
        textarea_area.x,
        textarea_area.y,
        /*width*/ 4,
    );
    let selected = Modifier::REVERSED | Modifier::BOLD | Modifier::UNDERLINED;
    assert_eq!(
        selection_modifiers,
        vec![Modifier::empty(), Modifier::empty(), selected, selected]
    );
    insta::with_settings!({snapshot_path => "../snapshots"}, {
        assert_snapshot!(
            "composer_mouse_selection_after_tab",
            format!(
                "selection: {:?}\nmodifiers: {selection_modifiers:?}\ncursor: {:?}\n{}",
                composer.draft.textarea.selection_range(),
                composer.cursor_pos(area),
                terminal.backend(),
            )
        );
    });
}

#[test]
fn newline_only_selection_highlights_the_end_of_line_cell() {
    let mut composer = test_composer();
    composer.insert_str("abc\ndef");
    let width = 30;
    let height = composer.desired_height(width);
    let area = Rect::new(/*x*/ 0, /*y*/ 0, width, height);
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test terminal");
    terminal
        .draw(|frame| composer.render(frame.area(), frame.buffer_mut()))
        .expect("initial render");
    let [_, _, textarea_area, _] = composer.layout_areas(area);

    assert!(composer.handle_mouse_event(mouse_event(
        MouseEventKind::Down(MouseButton::Left),
        textarea_area.x,
        textarea_area.y + 1,
    )));
    assert!(composer.handle_mouse_event(mouse_event(
        MouseEventKind::Drag(MouseButton::Left),
        textarea_area.x + 3,
        textarea_area.y,
    )));
    assert!(composer.handle_mouse_event(mouse_event(
        MouseEventKind::Up(MouseButton::Left),
        textarea_area.x + 3,
        textarea_area.y,
    )));
    terminal
        .draw(|frame| composer.render(frame.area(), frame.buffer_mut()))
        .expect("render selection");

    let selection_modifiers = modifiers_on_row(
        &terminal,
        textarea_area.x,
        textarea_area.y,
        /*width*/ 4,
    );
    let selected = Modifier::REVERSED | Modifier::BOLD | Modifier::UNDERLINED;
    assert_eq!(
        (
            composer.draft.textarea.selection_range(),
            selection_modifiers.clone(),
        ),
        (
            Some(3..4),
            vec![
                Modifier::empty(),
                Modifier::empty(),
                Modifier::empty(),
                selected,
            ],
        )
    );
    insta::with_settings!({snapshot_path => "../snapshots"}, {
        assert_snapshot!(
            "composer_mouse_newline_selection",
            format!(
                "selection: {:?}\nmodifiers: {selection_modifiers:?}\ncursor: {:?}\n{}",
                composer.draft.textarea.selection_range(),
                composer.cursor_pos(area),
                terminal.backend(),
            )
        );
    });
}

#[test]
fn clicking_large_paste_placeholder_selects_and_deletes_the_paste() {
    let mut composer = test_composer();
    let words = "many words ";
    let paste = words.repeat(LARGE_PASTE_CHAR_THRESHOLD / words.len() + 2);
    assert!(composer.handle_paste(paste));
    let placeholder = composer.draft.textarea.text().to_string();
    assert_eq!(composer.draft.pending_pastes.len(), 1);

    let width = 60;
    let height = composer.desired_height(width);
    let area = Rect::new(/*x*/ 0, /*y*/ 0, width, height);
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test terminal");
    terminal
        .draw(|frame| composer.render(frame.area(), frame.buffer_mut()))
        .expect("initial render");
    let [_, _, textarea_area, _] = composer.layout_areas(area);

    let clicked_column = textarea_area.x + 2;
    assert!(composer.handle_mouse_event(left_click(clicked_column, textarea_area.y)));
    assert!(composer.handle_mouse_event(mouse_event(
        MouseEventKind::Up(MouseButton::Left),
        clicked_column,
        textarea_area.y,
    )));
    assert_eq!(
        composer.draft.textarea.selection_range(),
        Some(0..placeholder.len())
    );

    terminal
        .draw(|frame| composer.render(frame.area(), frame.buffer_mut()))
        .expect("render selection");
    let selection_modifiers = modifiers_on_row(
        &terminal,
        textarea_area.x,
        textarea_area.y,
        placeholder.len() as u16,
    );
    let selected = Modifier::REVERSED | Modifier::BOLD | Modifier::UNDERLINED;
    assert!(
        selection_modifiers
            .iter()
            .all(|modifier| *modifier == selected)
    );
    insta::with_settings!({snapshot_path => "../snapshots"}, {
        assert_snapshot!(
            "composer_mouse_click_large_paste_selection",
            format!(
                "selection: {:?}\nmodifiers: {selection_modifiers:?}\ncursor: {:?}\n{}",
                composer.draft.textarea.selection_range(),
                composer.cursor_pos(area),
                terminal.backend(),
            )
        );
    });

    composer.handle_key_event(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));
    assert_eq!(
        (
            composer.draft.textarea.text(),
            composer.cursor(),
            composer.draft.textarea.selection_range(),
            composer.draft.pending_pastes.len(),
        ),
        ("", 0, None, 0)
    );
}

#[test]
fn large_paste_replaces_mouse_selection_in_both_directions() {
    let prefix = "before ";
    let selected = "selected";
    let suffix = " after";
    let pasted = "x".repeat(LARGE_PASTE_CHAR_THRESHOLD + 1);
    let placeholder = format!("[Pasted Content {} chars]", pasted.chars().count());
    let selection = prefix.len()..prefix.len() + selected.len();

    for (anchor, active) in [
        (selection.start, selection.end),
        (selection.end, selection.start),
    ] {
        let mut composer = test_composer();
        composer.insert_str(&format!("{prefix}{selected}{suffix}"));
        drag_select_range(&mut composer, anchor, active);
        assert_eq!(
            composer.draft.textarea.selection_range(),
            Some(selection.clone())
        );

        assert!(composer.handle_paste(pasted.clone()));

        let expected_text = format!("{prefix}{placeholder}{suffix}");
        assert_eq!(
            (
                composer.draft.textarea.text().to_string(),
                composer.cursor(),
                composer.draft.textarea.selection_range(),
                composer.draft.textarea.element_payloads(),
                composer.draft.pending_pastes.clone(),
            ),
            (
                expected_text,
                prefix.len() + placeholder.len(),
                None,
                vec![placeholder.clone()],
                vec![(placeholder.clone(), pasted.clone())],
            )
        );
    }
}

#[test]
fn paste_replacing_large_placeholder_reconciles_pending_payloads() {
    let old_paste = "o".repeat(LARGE_PASTE_CHAR_THRESHOLD + 1);
    let replacements = [
        "small replacement".to_string(),
        "n".repeat(LARGE_PASTE_CHAR_THRESHOLD + 2),
    ];

    for replacement in replacements {
        let mut composer = test_composer();
        assert!(composer.handle_paste(old_paste.clone()));
        let old_placeholder = composer.draft.textarea.text().to_string();
        drag_select_range(&mut composer, /*anchor*/ 2, /*active*/ 2);
        assert_eq!(
            composer.draft.textarea.selection_range(),
            Some(0..old_placeholder.len())
        );

        assert!(composer.handle_paste(replacement.clone()));

        if replacement.chars().count() > LARGE_PASTE_CHAR_THRESHOLD {
            let new_placeholder = format!("[Pasted Content {} chars]", replacement.chars().count());
            assert_eq!(
                (
                    composer.draft.textarea.text().to_string(),
                    composer.draft.textarea.element_payloads(),
                    composer.draft.pending_pastes.clone(),
                ),
                (
                    new_placeholder.clone(),
                    vec![new_placeholder.clone()],
                    vec![(new_placeholder, replacement)],
                )
            );
        } else {
            assert_eq!(
                (
                    composer.draft.textarea.text().to_string(),
                    composer.draft.textarea.element_payloads(),
                    composer.draft.pending_pastes.clone(),
                ),
                (
                    replacement,
                    Vec::<String>::new(),
                    Vec::<(String, String)>::new(),
                )
            );
        }
    }
}

#[test]
fn direct_insert_replacing_large_placeholder_reconciles_pending_payload() {
    let mut composer = test_composer();
    let paste = "x".repeat(LARGE_PASTE_CHAR_THRESHOLD + 1);
    assert!(composer.handle_paste(paste));
    drag_select_range(&mut composer, /*anchor*/ 2, /*active*/ 2);

    composer.insert_str("replacement");

    assert_eq!(
        (
            composer.draft.textarea.text(),
            composer.draft.textarea.element_payloads(),
            composer.draft.pending_pastes.clone(),
        ),
        (
            "replacement",
            Vec::<String>::new(),
            Vec::<(String, String)>::new(),
        )
    );
}

#[test]
fn paste_replacing_local_image_placeholder_removes_attachment() {
    let mut composer = test_composer();
    composer.attach_image(PathBuf::from("/tmp/selected.png"));
    let placeholder = composer.draft.textarea.text().to_string();
    drag_select_range(&mut composer, /*anchor*/ 2, /*active*/ 2);
    assert_eq!(
        composer.draft.textarea.selection_range(),
        Some(0..placeholder.len())
    );

    assert!(composer.handle_paste("replacement".to_string()));

    assert_eq!(
        (
            composer.draft.textarea.text(),
            composer.draft.textarea.element_payloads(),
            composer.local_image_paths(),
        ),
        ("replacement", Vec::<String>::new(), Vec::<PathBuf>::new(),)
    );
}

#[test]
fn attach_image_replacing_local_image_placeholder_reconciles_attachment() {
    let mut composer = test_composer();
    composer.attach_image(PathBuf::from("/tmp/selected.png"));
    drag_select_range(&mut composer, /*anchor*/ 2, /*active*/ 2);
    let replacement_path = PathBuf::from("/tmp/replacement.png");

    composer.attach_image(replacement_path.clone());

    let replacement_placeholder = local_image_label_text(/*number*/ 1);
    assert_eq!(
        (
            composer.draft.textarea.text().to_string(),
            composer.draft.textarea.element_payloads(),
            composer.local_image_paths(),
        ),
        (
            replacement_placeholder.clone(),
            vec![replacement_placeholder],
            vec![replacement_path],
        )
    );
}

#[test]
fn attach_image_replacing_large_placeholder_reconciles_pending_payload() {
    let mut composer = test_composer();
    let paste = "x".repeat(LARGE_PASTE_CHAR_THRESHOLD + 1);
    assert!(composer.handle_paste(paste));
    drag_select_range(&mut composer, /*anchor*/ 2, /*active*/ 2);
    let replacement_path = PathBuf::from("/tmp/replacement.png");

    composer.attach_image(replacement_path.clone());

    let replacement_placeholder = local_image_label_text(/*number*/ 1);
    assert_eq!(
        (
            composer.draft.textarea.text().to_string(),
            composer.draft.textarea.element_payloads(),
            composer.draft.pending_pastes.clone(),
            composer.local_image_paths(),
        ),
        (
            replacement_placeholder.clone(),
            vec![replacement_placeholder],
            Vec::<(String, String)>::new(),
            vec![replacement_path],
        )
    );
}

#[test]
fn non_ascii_input_replacing_local_image_placeholder_removes_attachment() {
    let mut composer = test_composer();
    composer.attach_image(PathBuf::from("/tmp/selected.png"));
    drag_select_range(&mut composer, /*anchor*/ 2, /*active*/ 2);

    composer.handle_key_event(KeyEvent::new(KeyCode::Char('界'), KeyModifiers::NONE));

    assert_eq!(
        (
            composer.draft.textarea.text(),
            composer.draft.textarea.element_payloads(),
            composer.local_image_paths(),
        ),
        ("界", Vec::<String>::new(), Vec::<PathBuf>::new())
    );
}

#[test]
fn shell_mode_backspace_deletes_reverse_selection_at_buffer_start() {
    let mut composer = test_composer();
    composer.insert_str("shell");
    drag_select_range(&mut composer, /*anchor*/ 5, /*active*/ 0);
    assert_eq!(
        (composer.cursor(), composer.draft.textarea.selection_range()),
        (0, Some(0..5))
    );
    composer.draft.is_bash_mode = true;

    composer.handle_key_event(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));

    assert_eq!(
        (
            composer.draft.textarea.text(),
            composer.draft.is_bash_mode,
            composer.draft.textarea.selection_range(),
        ),
        ("", true, None)
    );
}

#[test]
fn composer_ignores_clicks_before_render_and_outside_textarea() {
    let mut composer = test_composer();
    composer.insert_str("hello");
    let before_render = composer.handle_mouse_event(left_click(/*column*/ 0, /*row*/ 0));

    let width = 30;
    let height = composer.desired_height(width);
    let area = Rect::new(/*x*/ 0, /*y*/ 0, width, height);
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test terminal");
    terminal
        .draw(|frame| composer.render(frame.area(), frame.buffer_mut()))
        .expect("render");
    let outside_textarea = composer.handle_mouse_event(left_click(area.x, area.y));

    assert_eq!(
        (before_render, outside_textarea, composer.cursor()),
        (false, false, 5)
    );
}

#[test]
fn clicking_outside_textarea_clears_active_selection() {
    let mut composer = test_composer();
    composer.insert_str("hello world");
    drag_select_range(&mut composer, /*anchor*/ 0, /*active*/ 5);
    assert_eq!(composer.draft.textarea.selection_range(), Some(0..5));

    let width = 30;
    let height = composer.desired_height(width);
    let area = Rect::new(/*x*/ 0, /*y*/ 0, width, height);
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test terminal");
    terminal
        .draw(|frame| composer.render(frame.area(), frame.buffer_mut()))
        .expect("render");
    let [_, _, textarea_area, _] = composer.layout_areas(area);
    // Exclusive right edge is outside `Rect::contains` but still a realistic pointer position.
    let outside = left_click(textarea_area.right(), textarea_area.y);

    let handled = composer.handle_mouse_event(outside);

    assert_eq!(
        (
            handled,
            composer.draft.textarea.selection_range(),
            composer.cursor()
        ),
        (true, None, 5)
    );
}

#[test]
fn clicking_inside_image_placeholder_clamps_to_an_editable_edge() {
    let mut composer = test_composer();
    let placeholder = local_image_label_text(/*number*/ 1);
    composer.draft.textarea.insert_element(&placeholder);
    let width = 40;
    let height = composer.desired_height(width);
    let area = Rect::new(/*x*/ 0, /*y*/ 0, width, height);
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test terminal");
    terminal
        .draw(|frame| composer.render(frame.area(), frame.buffer_mut()))
        .expect("render");
    let [_, _, textarea_area, _] = composer.layout_areas(area);

    assert!(composer.handle_mouse_event(left_click(textarea_area.x + 2, textarea_area.y)));
    assert_eq!(composer.cursor(), 0);
    assert!(composer.handle_mouse_event(left_click(
        textarea_area.x + placeholder.len() as u16 - 1,
        textarea_area.y,
    )));
    assert_eq!(composer.cursor(), placeholder.len());
}

#[test]
fn clicking_flushes_buffered_input_before_moving_the_cursor() {
    let (tx, _rx) = unbounded_channel::<AppEvent>();
    let mut composer = ChatComposer::new(
        /*has_input_focus*/ true,
        AppEventSender::new(tx),
        /*enhanced_keys_supported*/ false,
        "Ask Codex to do anything".to_string(),
        /*disable_paste_burst*/ false,
    );
    composer.insert_str("world");
    composer
        .draft
        .paste_burst
        .begin_with_retro_grabbed("hello ".to_string(), Instant::now());
    let width = 30;
    let height = composer.desired_height(width);
    let area = Rect::new(/*x*/ 0, /*y*/ 0, width, height);
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test terminal");
    terminal
        .draw(|frame| composer.render(frame.area(), frame.buffer_mut()))
        .expect("render");
    let [_, _, textarea_area, _] = composer.layout_areas(area);

    assert!(composer.handle_mouse_event(left_click(textarea_area.x, textarea_area.y)));
    assert_eq!(
        (
            composer.draft.textarea.text(),
            composer.cursor(),
            composer.is_in_paste_burst(),
        ),
        ("worldhello ", 0, false)
    );
}
