//! Terminal hit-testing for mouse cursor placement and drag selection.

use ratatui::layout::Position;
use ratatui::layout::Rect;

use super::TextArea;
use super::TextAreaState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MouseSelectionUpdate {
    Begin,
    Drag,
    Release,
}

impl TextArea {
    /// Move the cursor to the editable position represented by a rendered terminal cell.
    ///
    /// Returns `true` when `position` addresses a visible wrapped line in `area`. The mapping uses
    /// the same wrap cache and effective scroll offset as rendering, then applies the textarea's
    /// atomic-element clamping so clicks cannot place the cursor inside a placeholder.
    pub(crate) fn set_cursor_from_screen_position(
        &mut self,
        area: Rect,
        state: TextAreaState,
        position: Position,
        selection_update: MouseSelectionUpdate,
    ) -> bool {
        let is_right_edge_selection = matches!(
            selection_update,
            MouseSelectionUpdate::Drag | MouseSelectionUpdate::Release
        ) && position.x == area.right()
            && position.y >= area.top()
            && position.y < area.bottom();
        if area.is_empty() || (!area.contains(position) && !is_right_edge_selection) {
            return false;
        }

        let Some((line_start, line_end)) =
            self.line_range_at_screen_position(area, state, position)
        else {
            return false;
        };
        let target_col = usize::from(position.x.saturating_sub(area.x));
        let raw_position = self.position_at_display_col_on_line(line_start, line_end, target_col);
        let element = self
            .elements
            .iter()
            .find(|element| raw_position >= element.range.start && raw_position < element.range.end)
            .map(|element| element.range.clone());
        let selection_position = self.clamp_pos_to_nearest_boundary(raw_position);
        self.cursor_pos = selection_position;
        if self.is_vim_normal_mode() && line_start < line_end {
            self.cursor_pos = self.cursor_pos.min(self.prev_atomic_boundary(line_end));
        }
        self.preferred_col = None;

        match selection_update {
            MouseSelectionUpdate::Begin => self.begin_selection(selection_position, element),
            MouseSelectionUpdate::Drag => self.set_selection_endpoint(selection_position, element),
            MouseSelectionUpdate::Release => {
                self.set_selection_endpoint(selection_position, element);
                self.finish_selection();
            }
        }
        true
    }

    fn line_range_at_screen_position(
        &self,
        area: Rect,
        state: TextAreaState,
        position: Position,
    ) -> Option<(usize, usize)> {
        let lines = self.wrapped_lines(area.width);
        let scroll = self.effective_scroll(area.height, &lines, state.scroll);
        let visible_row = position.y.saturating_sub(area.y);
        let line = lines.get(usize::from(scroll.saturating_add(visible_row)))?;
        let line_end = line.end.saturating_sub(1).min(self.text.len());
        Some((line.start.min(line_end), line_end))
    }
}
