use crossterm::event::MouseButton;
use crossterm::event::MouseEvent;
use crossterm::event::MouseEventKind;
use ratatui::layout::Position;

use super::ChatComposer;
use super::reset_mode_after_activity;
use crate::bottom_pane::textarea::MouseSelectionUpdate;

/// Result of routing a mouse event through the composer textarea.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ComposerMouseOutcome {
    Ignored,
    Handled,
    SelectionFinished(String),
}

impl ComposerMouseOutcome {
    pub(crate) fn is_handled(&self) -> bool {
        !matches!(self, Self::Ignored)
    }
}

impl ChatComposer {
    /// Selected composer text, if a non-empty range is highlighted.
    pub(crate) fn selected_text(&self) -> Option<String> {
        self.draft.textarea.selected_text().map(str::to_string)
    }

    #[cfg(test)]
    pub(crate) fn last_textarea_area(&self) -> Option<ratatui::layout::Rect> {
        self.last_textarea_area.get()
    }

    #[cfg(test)]
    pub(crate) fn select_byte_range(&mut self, range: std::ops::Range<usize>) {
        self.draft.textarea.select_byte_range(range);
    }

    fn finished_selection(&self) -> ComposerMouseOutcome {
        self.selected_text()
            .map(ComposerMouseOutcome::SelectionFinished)
            .unwrap_or(ComposerMouseOutcome::Handled)
    }

    /// Place the textarea cursor at a clicked terminal cell.
    ///
    /// Geometry comes from the most recent render, which keeps hit testing aligned with remote
    /// attachment rows, popups, right-side reservations, wrapping, and scrolling.
    pub(crate) fn handle_mouse_event(&mut self, mouse_event: MouseEvent) -> ComposerMouseOutcome {
        if !self.draft.input_enabled || self.history_search.is_some() {
            self.mouse_drag_active = false;
            return ComposerMouseOutcome::Ignored;
        }

        let Some(textarea_area) = self.last_textarea_area.get() else {
            self.mouse_drag_active = false;
            return ComposerMouseOutcome::Ignored;
        };
        if textarea_area.is_empty() {
            self.mouse_drag_active = false;
            return ComposerMouseOutcome::Ignored;
        }

        let raw_position = Position::new(mouse_event.column, mouse_event.row);
        let is_mouse_down = matches!(mouse_event.kind, MouseEventKind::Down(MouseButton::Left));
        let is_mouse_up = matches!(mouse_event.kind, MouseEventKind::Up(MouseButton::Left));
        let position = match mouse_event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.mouse_drag_active = false;
                if !textarea_area.contains(raw_position) {
                    // Clicking outside the textarea dismisses any active selection highlight.
                    let had_selection = self.draft.textarea.selection_range().is_some();
                    self.draft.textarea.clear_selection();
                    return if had_selection {
                        ComposerMouseOutcome::Handled
                    } else {
                        ComposerMouseOutcome::Ignored
                    };
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
                return ComposerMouseOutcome::Ignored;
            }
            MouseEventKind::Down(_)
            | MouseEventKind::Up(_)
            | MouseEventKind::Drag(_)
            | MouseEventKind::Moved
            | MouseEventKind::ScrollDown
            | MouseEventKind::ScrollUp
            | MouseEventKind::ScrollLeft
            | MouseEventKind::ScrollRight => return ComposerMouseOutcome::Ignored,
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
                return self.finished_selection();
            }
            return if did_flush_paste {
                ComposerMouseOutcome::Handled
            } else {
                ComposerMouseOutcome::Ignored
            };
        }

        if is_mouse_down {
            self.mouse_drag_active = true;
        } else if is_mouse_up {
            self.mouse_drag_active = false;
        }

        self.attachments.clear_remote_image_selection();
        self.footer.mode = reset_mode_after_activity(self.footer.mode);
        self.sync_popups();
        if is_mouse_up {
            self.finished_selection()
        } else {
            ComposerMouseOutcome::Handled
        }
    }
}
