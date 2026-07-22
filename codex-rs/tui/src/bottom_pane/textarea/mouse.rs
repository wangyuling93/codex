use ratatui::layout::Position;
use ratatui::layout::Rect;

use super::TextArea;
use super::TextAreaState;
use super::TextSelection;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MouseSelectionUpdate {
    Begin,
    Drag,
    Release,
}

impl TextArea {
    fn begin_selection(&mut self, initial_hit: usize, element: Option<std::ops::Range<usize>>) {
        self.selection = Some(TextSelection {
            anchor: initial_hit,
            active: initial_hit,
            initial_hit,
            anchor_element: element.clone(),
            active_element: element,
        });
    }

    fn set_selection_endpoint(&mut self, active: usize, element: Option<std::ops::Range<usize>>) {
        if let Some(selection) = self.selection.as_mut() {
            selection.active = active;
            selection.active_element = element;
        }
    }

    pub(crate) fn finish_selection(&mut self) {
        if self.selection_range().is_none() {
            self.selection = None;
        }
    }

    pub(crate) fn clear_selection(&mut self) {
        self.selection = None;
    }

    pub(crate) fn selection_range(&self) -> Option<std::ops::Range<usize>> {
        let selection = self.selection.as_ref()?;
        let mut start = selection.anchor.min(selection.active);
        let mut end = selection.anchor.max(selection.active);

        for element in [
            selection.anchor_element.as_ref(),
            selection.active_element.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            start = start.min(element.start);
            end = end.max(element.end);
        }

        (start < end).then_some(start..end)
    }

    pub(super) fn delete_selection(&mut self) -> bool {
        let Some(range) = self.selection_range() else {
            return false;
        };
        self.replace_range(range, "");
        true
    }

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
                if let Some(selection) = self.selection.as_mut() {
                    if selection_position == selection.initial_hit {
                        selection.active = selection.anchor;
                        selection
                            .active_element
                            .clone_from(&selection.anchor_element);
                    } else {
                        selection.active = selection_position;
                        selection.active_element = element;
                    }
                }
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
