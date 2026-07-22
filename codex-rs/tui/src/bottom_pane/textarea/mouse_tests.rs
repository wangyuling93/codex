use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use pretty_assertions::assert_eq;
use ratatui::layout::Position;
use ratatui::layout::Rect;

use super::MouseSelectionUpdate;
use super::TextArea;
use super::TextAreaState;

type SelectionKillCommand = (&'static str, fn(&mut TextArea));

fn textarea_with_text(text: &str) -> TextArea {
    let mut textarea = TextArea::new();
    textarea.set_text_clearing_elements(text);
    textarea
}

fn delete_forward_kill_one(textarea: &mut TextArea) {
    textarea.delete_forward_kill(/*n*/ 1);
}

#[test]
fn click_maps_terminal_columns_to_text_positions() {
    let area = Rect::new(
        /*x*/ 2, /*y*/ 3, /*width*/ 20, /*height*/ 1,
    );
    let positions = [
        Position::new(area.x, area.y),
        Position::new(area.x + 5, area.y),
        Position::new(area.x + 11, area.y),
        Position::new(area.x + 15, area.y),
    ];
    let actual = positions
        .into_iter()
        .map(|position| {
            let mut textarea = textarea_with_text("hello world");
            let handled = textarea.set_cursor_from_screen_position(
                area,
                TextAreaState::default(),
                position,
                MouseSelectionUpdate::Begin,
            );
            (handled, textarea.cursor())
        })
        .collect::<Vec<_>>();

    assert_eq!(actual, vec![(true, 0), (true, 5), (true, 11), (true, 11)]);
}

#[test]
fn click_uses_wrapping_unicode_width_and_scroll_state() {
    let text = "abcd日本ef";
    let area = Rect::new(
        /*x*/ 4, /*y*/ 7, /*width*/ 4, /*height*/ 2,
    );
    let mut textarea = textarea_with_text(text);
    textarea.set_cursor(text.len());

    let first_visible_line_handled = textarea.set_cursor_from_screen_position(
        area,
        TextAreaState::default(),
        Position::new(area.x, area.y),
        MouseSelectionUpdate::Begin,
    );
    let first_visible_line_cursor = textarea.cursor();

    let second_wide_cell_handled = textarea.set_cursor_from_screen_position(
        area,
        TextAreaState { scroll: 1 },
        Position::new(area.x + 1, area.y),
        MouseSelectionUpdate::Begin,
    );
    let second_wide_cell_cursor = textarea.cursor();

    let after_wide_character_handled = textarea.set_cursor_from_screen_position(
        area,
        TextAreaState { scroll: 1 },
        Position::new(area.x + 2, area.y),
        MouseSelectionUpdate::Begin,
    );
    let after_wide_character_cursor = textarea.cursor();

    assert_eq!(
        (
            first_visible_line_handled,
            first_visible_line_cursor,
            second_wide_cell_handled,
            second_wide_cell_cursor,
            after_wide_character_handled,
            after_wide_character_cursor,
        ),
        (true, 4, true, 4, true, 7)
    );
}

#[test]
fn click_uses_the_single_column_tab_representation() {
    let area = Rect::new(
        /*x*/ 2, /*y*/ 3, /*width*/ 10, /*height*/ 1,
    );
    let positions = [
        Position::new(area.x + 1, area.y),
        Position::new(area.x + 2, area.y),
    ];
    let actual = positions
        .into_iter()
        .map(|position| {
            let mut textarea = textarea_with_text("a\tb");
            let handled = textarea.set_cursor_from_screen_position(
                area,
                TextAreaState::default(),
                position,
                MouseSelectionUpdate::Begin,
            );
            (handled, textarea.cursor())
        })
        .collect::<Vec<_>>();

    assert_eq!(actual, vec![(true, 1), (true, 2)]);
}

#[test]
fn click_clamps_to_atomic_element_edges_and_ignores_outside_positions() {
    let mut textarea = textarea_with_text("go[image]now");
    textarea.add_element_range(2..9).expect("element range");
    let area = Rect::new(
        /*x*/ 1, /*y*/ 1, /*width*/ 20, /*height*/ 1,
    );

    let near_start = textarea.set_cursor_from_screen_position(
        area,
        TextAreaState::default(),
        Position::new(area.x + 4, area.y),
        MouseSelectionUpdate::Begin,
    );
    let near_start_cursor = textarea.cursor();
    let near_end = textarea.set_cursor_from_screen_position(
        area,
        TextAreaState::default(),
        Position::new(area.x + 8, area.y),
        MouseSelectionUpdate::Begin,
    );
    let near_end_cursor = textarea.cursor();
    let outside = textarea.set_cursor_from_screen_position(
        area,
        TextAreaState::default(),
        Position::new(area.right(), area.y),
        MouseSelectionUpdate::Begin,
    );

    assert_eq!(
        (
            near_start,
            near_start_cursor,
            near_end,
            near_end_cursor,
            outside,
            textarea.cursor(),
        ),
        (true, 2, true, 9, false, 9)
    );
}

#[test]
fn clicking_or_dragging_within_atomic_element_selects_the_whole_element() {
    let prefix = "before ";
    let placeholder = "[Pasted Content 1100 chars]";
    let element_range = prefix.len()..prefix.len() + placeholder.len();
    let text = format!("{prefix}{placeholder} after");
    let mut clicked = textarea_with_text(&text);
    clicked
        .add_element_range(element_range.clone())
        .expect("element range");
    let mut dragged = textarea_with_text(&text);
    dragged
        .add_element_range(element_range.clone())
        .expect("element range");
    let area = Rect::new(
        /*x*/ 2, /*y*/ 3, /*width*/ 60, /*height*/ 1,
    );

    for selection_update in [MouseSelectionUpdate::Begin, MouseSelectionUpdate::Release] {
        assert!(clicked.set_cursor_from_screen_position(
            area,
            TextAreaState::default(),
            Position::new(area.x + prefix.len() as u16 + 2, area.y),
            selection_update,
        ));
    }

    for selection_update in [
        MouseSelectionUpdate::Begin,
        MouseSelectionUpdate::Drag,
        MouseSelectionUpdate::Release,
    ] {
        let element_offset = if selection_update == MouseSelectionUpdate::Begin {
            2
        } else {
            4
        };
        assert!(dragged.set_cursor_from_screen_position(
            area,
            TextAreaState::default(),
            Position::new(area.x + prefix.len() as u16 + element_offset, area.y),
            selection_update,
        ));
    }

    assert_eq!(
        (clicked.selection_range(), dragged.selection_range()),
        (Some(element_range.clone()), Some(element_range))
    );
}

#[test]
fn mouse_selection_is_deleted_or_replaced_by_editing_keys() {
    let area = Rect::new(
        /*x*/ 2, /*y*/ 3, /*width*/ 20, /*height*/ 1,
    );
    let edits = [
        (KeyCode::Backspace, "hello"),
        (KeyCode::Delete, "hello"),
        (KeyCode::Char('!'), "hello!"),
    ];
    let mut actual = Vec::new();

    for (key_code, expected_text) in edits {
        let mut textarea = textarea_with_text("hello world");
        assert!(textarea.set_cursor_from_screen_position(
            area,
            TextAreaState::default(),
            Position::new(area.x + 11, area.y),
            MouseSelectionUpdate::Begin,
        ));
        assert!(textarea.set_cursor_from_screen_position(
            area,
            TextAreaState::default(),
            Position::new(area.x + 5, area.y),
            MouseSelectionUpdate::Drag,
        ));
        assert!(textarea.set_cursor_from_screen_position(
            area,
            TextAreaState::default(),
            Position::new(area.x + 5, area.y),
            MouseSelectionUpdate::Release,
        ));
        let selection_before_edit = textarea.selection_range();

        textarea.input(KeyEvent::new(key_code, KeyModifiers::NONE));
        actual.push((
            selection_before_edit,
            textarea.text().to_string(),
            textarea.cursor(),
            textarea.selection_range(),
        ));

        assert_eq!(textarea.text(), expected_text);
    }

    assert_eq!(
        actual,
        vec![
            (Some(5..11), "hello".to_string(), 5, None),
            (Some(5..11), "hello".to_string(), 5, None),
            (Some(5..11), "hello!".to_string(), 6, None),
        ]
    );
}

#[test]
fn mouse_selection_is_used_by_kill_commands_and_yank() {
    let area = Rect::new(
        /*x*/ 2, /*y*/ 3, /*width*/ 20, /*height*/ 1,
    );
    let commands: [SelectionKillCommand; 5] = [
        ("delete_forward_kill", delete_forward_kill_one),
        ("delete_backward_word", TextArea::delete_backward_word),
        ("delete_forward_word", TextArea::delete_forward_word),
        ("kill_to_end_of_line", TextArea::kill_to_end_of_line),
        (
            "kill_to_beginning_of_line",
            TextArea::kill_to_beginning_of_line,
        ),
    ];
    let mut actual = Vec::new();

    for (name, command) in commands {
        let mut textarea = textarea_with_text("hello world");
        assert!(textarea.set_cursor_from_screen_position(
            area,
            TextAreaState::default(),
            Position::new(area.x + 11, area.y),
            MouseSelectionUpdate::Begin,
        ));
        assert!(textarea.set_cursor_from_screen_position(
            area,
            TextAreaState::default(),
            Position::new(area.x + 5, area.y),
            MouseSelectionUpdate::Release,
        ));

        command(&mut textarea);
        let after_kill = (
            textarea.text().to_string(),
            textarea.cursor(),
            textarea.selection_range(),
        );
        textarea.yank();
        actual.push((
            name,
            after_kill,
            textarea.text().to_string(),
            textarea.cursor(),
            textarea.selection_range(),
        ));
    }

    assert_eq!(
        actual,
        [
            "delete_forward_kill",
            "delete_backward_word",
            "delete_forward_word",
            "kill_to_end_of_line",
            "kill_to_beginning_of_line",
        ]
        .map(|name| {
            (
                name,
                ("hello".to_string(), 5, None),
                "hello world".to_string(),
                11,
                None,
            )
        })
    );
}

#[test]
fn release_position_completes_selection_without_an_intermediate_drag_event() {
    let area = Rect::new(
        /*x*/ 2, /*y*/ 3, /*width*/ 20, /*height*/ 1,
    );
    let mut textarea = textarea_with_text("hello world");

    assert!(textarea.set_cursor_from_screen_position(
        area,
        TextAreaState::default(),
        Position::new(area.x, area.y),
        MouseSelectionUpdate::Begin,
    ));
    assert!(textarea.set_cursor_from_screen_position(
        area,
        TextAreaState::default(),
        Position::new(area.x + 5, area.y),
        MouseSelectionUpdate::Release,
    ));

    assert_eq!(textarea.selection_range(), Some(0..5));
}

#[test]
fn arrow_keys_collapse_mouse_selection_to_the_requested_edge() {
    let area = Rect::new(
        /*x*/ 2, /*y*/ 3, /*width*/ 20, /*height*/ 1,
    );
    let cases = [
        (5, 11, KeyCode::Left, 5),
        (11, 5, KeyCode::Left, 5),
        (5, 11, KeyCode::Right, 11),
        (11, 5, KeyCode::Right, 11),
    ];
    let mut actual = Vec::new();

    for (anchor, active, key_code, expected_cursor) in cases {
        let mut textarea = textarea_with_text("hello world");
        assert!(textarea.set_cursor_from_screen_position(
            area,
            TextAreaState::default(),
            Position::new(area.x + anchor, area.y),
            MouseSelectionUpdate::Begin,
        ));
        assert!(textarea.set_cursor_from_screen_position(
            area,
            TextAreaState::default(),
            Position::new(area.x + active, area.y),
            MouseSelectionUpdate::Drag,
        ));
        assert!(textarea.set_cursor_from_screen_position(
            area,
            TextAreaState::default(),
            Position::new(area.x + active, area.y),
            MouseSelectionUpdate::Release,
        ));

        textarea.input(KeyEvent::new(key_code, KeyModifiers::NONE));
        actual.push((textarea.cursor(), textarea.selection_range()));

        assert_eq!(textarea.cursor(), expected_cursor);
    }

    assert_eq!(actual, vec![(5, None), (5, None), (11, None), (11, None)]);
}

#[test]
fn vim_normal_mode_click_after_line_clamps_before_deleting() {
    let area = Rect::new(
        /*x*/ 2, /*y*/ 3, /*width*/ 20, /*height*/ 2,
    );
    let cases = [(0, 1, "a\ncd"), (1, 4, "ab\nc")];
    let mut actual = Vec::new();

    for (row, expected_cursor, expected_text) in cases {
        let mut textarea = textarea_with_text("ab\ncd");
        textarea.set_vim_enabled(/*enabled*/ true);
        let position = Position::new(area.x + 10, area.y + row);
        assert!(textarea.set_cursor_from_screen_position(
            area,
            TextAreaState::default(),
            position,
            MouseSelectionUpdate::Begin,
        ));
        assert!(textarea.set_cursor_from_screen_position(
            area,
            TextAreaState::default(),
            position,
            MouseSelectionUpdate::Release,
        ));
        let before_delete = (textarea.cursor(), textarea.selection_range());

        textarea.input(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        actual.push((before_delete, textarea.text().to_string()));

        assert_eq!(
            (textarea.cursor(), textarea.text()),
            (expected_cursor, expected_text)
        );
    }

    assert_eq!(
        actual,
        vec![
            ((1, None), "a\ncd".to_string()),
            ((4, None), "ab\nc".to_string()),
        ]
    );
}

#[test]
fn vim_normal_mode_drag_past_right_edge_selects_the_final_character() {
    let area = Rect::new(
        /*x*/ 2, /*y*/ 3, /*width*/ 3, /*height*/ 1,
    );
    let mut textarea = textarea_with_text("abc");
    textarea.set_vim_enabled(/*enabled*/ true);

    for (position, selection_update) in [
        (Position::new(area.x, area.y), MouseSelectionUpdate::Begin),
        (
            Position::new(area.right(), area.y),
            MouseSelectionUpdate::Drag,
        ),
        (
            Position::new(area.right(), area.y),
            MouseSelectionUpdate::Release,
        ),
    ] {
        assert!(textarea.set_cursor_from_screen_position(
            area,
            TextAreaState::default(),
            position,
            selection_update,
        ));
    }

    assert_eq!(textarea.selection_range(), Some(0..3));
    textarea.input(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
    assert_eq!((textarea.text(), textarea.cursor()), ("", 0));
}

#[test]
fn vim_normal_mode_reverse_drag_from_trailing_blank_includes_final_character() {
    let area = Rect::new(
        /*x*/ 2, /*y*/ 3, /*width*/ 20, /*height*/ 1,
    );
    let mut textarea = textarea_with_text("abc");
    textarea.set_vim_enabled(/*enabled*/ true);

    for (position, selection_update) in [
        (
            Position::new(area.x + 10, area.y),
            MouseSelectionUpdate::Begin,
        ),
        (Position::new(area.x, area.y), MouseSelectionUpdate::Drag),
        (Position::new(area.x, area.y), MouseSelectionUpdate::Release),
    ] {
        assert!(textarea.set_cursor_from_screen_position(
            area,
            TextAreaState::default(),
            position,
            selection_update,
        ));
    }

    assert_eq!(textarea.selection_range(), Some(0..3));
    textarea.input(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
    assert_eq!((textarea.text(), textarea.cursor()), ("", 0));
}

#[test]
fn vim_left_collapse_of_newline_selection_keeps_cursor_on_character() {
    let area = Rect::new(
        /*x*/ 2, /*y*/ 3, /*width*/ 20, /*height*/ 2,
    );
    let mut textarea = textarea_with_text("abc\ndef");
    textarea.set_vim_enabled(/*enabled*/ true);

    for (position, selection_update) in [
        (
            Position::new(area.x, area.y + 1),
            MouseSelectionUpdate::Begin,
        ),
        (
            Position::new(area.x + 10, area.y),
            MouseSelectionUpdate::Drag,
        ),
        (
            Position::new(area.x + 10, area.y),
            MouseSelectionUpdate::Release,
        ),
    ] {
        assert!(textarea.set_cursor_from_screen_position(
            area,
            TextAreaState::default(),
            position,
            selection_update,
        ));
    }

    assert_eq!(textarea.selection_range(), Some(3..4));
    textarea.input(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
    let collapsed_cursor = textarea.cursor();
    textarea.input(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));

    assert_eq!((collapsed_cursor, textarea.text()), (2, "ab\ndef"));
}

#[test]
fn vim_newline_selection_replacement_places_cursor_after_inserted_text() {
    let area = Rect::new(
        /*x*/ 2, /*y*/ 3, /*width*/ 20, /*height*/ 2,
    );
    let mut textarea = textarea_with_text("abc\ndef");
    textarea.set_vim_enabled(/*enabled*/ true);

    for (position, selection_update) in [
        (
            Position::new(area.x, area.y + 1),
            MouseSelectionUpdate::Begin,
        ),
        (
            Position::new(area.x + 10, area.y),
            MouseSelectionUpdate::Release,
        ),
    ] {
        assert!(textarea.set_cursor_from_screen_position(
            area,
            TextAreaState::default(),
            position,
            selection_update,
        ));
    }

    assert_eq!(
        (textarea.selection_range(), textarea.cursor()),
        (Some(3..4), 2)
    );
    textarea.insert_str(" pasted ");

    assert_eq!(
        (
            textarea.text(),
            textarea.cursor(),
            textarea.selection_range()
        ),
        ("abc pasted def", 11, None)
    );
}

#[test]
fn vim_right_collapse_of_end_of_line_element_keeps_cursor_off_newline() {
    let placeholder = "[Pasted Content 1200 chars]";
    let text = format!("a{placeholder}\nnext");
    let element_range = 1..1 + placeholder.len();
    let area = Rect::new(
        /*x*/ 2, /*y*/ 3, /*width*/ 40, /*height*/ 2,
    );
    let mut textarea = textarea_with_text(&text);
    textarea
        .add_element_range(element_range)
        .expect("element range");
    textarea.set_vim_enabled(/*enabled*/ true);

    for selection_update in [MouseSelectionUpdate::Begin, MouseSelectionUpdate::Release] {
        assert!(textarea.set_cursor_from_screen_position(
            area,
            TextAreaState::default(),
            Position::new(area.x + 4, area.y),
            selection_update,
        ));
    }
    textarea.input(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    let collapsed_cursor = textarea.cursor();
    textarea.input(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));

    assert_eq!((collapsed_cursor, textarea.text()), (1, "a\nnext"));
}

#[test]
fn replacing_selected_element_payload_clears_captured_ranges() {
    let old = "[Pasted Content 1200 chars]";
    let new = "[Pasted Content 2 chars]";
    let area = Rect::new(
        /*x*/ 2, /*y*/ 3, /*width*/ 40, /*height*/ 1,
    );
    let mut textarea = textarea_with_text(old);
    textarea
        .add_element_range(0..old.len())
        .expect("element range");
    for selection_update in [MouseSelectionUpdate::Begin, MouseSelectionUpdate::Release] {
        assert!(textarea.set_cursor_from_screen_position(
            area,
            TextAreaState::default(),
            Position::new(area.x + 3, area.y),
            selection_update,
        ));
    }

    assert!(textarea.replace_element_payload(old, new));

    assert_eq!((textarea.text(), textarea.selection_range()), (new, None));
}
