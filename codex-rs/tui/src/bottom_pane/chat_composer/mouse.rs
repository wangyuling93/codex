use crossterm::event::MouseButton;
use crossterm::event::MouseEvent;
use crossterm::event::MouseEventKind;
use ratatui::layout::Position;

use super::ChatComposer;
use super::reset_mode_after_activity;
use crate::bottom_pane::textarea::MouseSelectionUpdate;

impl ChatComposer {
    /// Place the textarea cursor at a clicked terminal cell.
    ///
    /// Geometry comes from the most recent render, which keeps hit testing aligned with remote
    /// attachment rows, popups, right-side reservations, wrapping, and scrolling.
    pub(crate) fn handle_mouse_event(&mut self, mouse_event: MouseEvent) -> bool {
        if !self.draft.input_enabled || self.history_search.is_some() {
            self.mouse_drag_active = false;
            return false;
        }

        let Some(textarea_area) = self.last_textarea_area.get() else {
            self.mouse_drag_active = false;
            return false;
        };
        if textarea_area.is_empty() {
            self.mouse_drag_active = false;
            return false;
        }

        let raw_position = Position::new(mouse_event.column, mouse_event.row);
        let is_mouse_down = matches!(mouse_event.kind, MouseEventKind::Down(MouseButton::Left));
        let is_mouse_up = matches!(mouse_event.kind, MouseEventKind::Up(MouseButton::Left));
        let position = match mouse_event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.mouse_drag_active = false;
                if !textarea_area.contains(raw_position) {
                    return false;
                }
                raw_position
            }
            MouseEventKind::Drag(MouseButton::Left) | MouseEventKind::Up(MouseButton::Left)
                if self.mouse_drag_active =>
            {
                Position::new(
                    raw_position.x.clamp(textarea_area.x, textarea_area.right()),
                    raw_position
                        .y
                        .clamp(textarea_area.y, textarea_area.bottom().saturating_sub(1)),
                )
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.mouse_drag_active = false;
                return false;
            }
            MouseEventKind::Down(_)
            | MouseEventKind::Up(_)
            | MouseEventKind::Drag(_)
            | MouseEventKind::Moved
            | MouseEventKind::ScrollDown
            | MouseEventKind::ScrollUp
            | MouseEventKind::ScrollLeft
            | MouseEventKind::ScrollRight => return false,
        };

        let did_flush_paste = if is_mouse_down {
            // A click is a cursor-moving input, so preserve any text still held by the paste-burst
            // detector at the cursor where the user entered it before moving elsewhere.
            let flushed_paste = self.draft.paste_burst.flush_before_modified_input();
            let did_flush_paste = flushed_paste.is_some();
            if let Some(pasted) = flushed_paste {
                self.handle_paste(pasted);
            }
            self.draft.textarea.clear_selection();
            did_flush_paste
        } else {
            false
        };

        let state = *self.draft.textarea_state.borrow();
        let selection_update = if is_mouse_down {
            MouseSelectionUpdate::Begin
        } else if is_mouse_up {
            MouseSelectionUpdate::Release
        } else {
            MouseSelectionUpdate::Drag
        };
        if !self.draft.textarea.set_cursor_from_screen_position(
            textarea_area,
            state,
            position,
            selection_update,
        ) {
            if is_mouse_up {
                self.mouse_drag_active = false;
                self.draft.textarea.finish_selection();
            }
            return did_flush_paste;
        }

        if is_mouse_down {
            self.mouse_drag_active = true;
        } else if is_mouse_up {
            self.mouse_drag_active = false;
        }

        self.attachments.clear_remote_image_selection();
        self.footer.mode = reset_mode_after_activity(self.footer.mode);
        self.sync_popups();
        true
    }
}
