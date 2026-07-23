//! Text selection state and edit policy for [`TextArea`].
//!
//! Hit-testing lives in [`super::mouse`]; this module owns the selection model and the shared
//! kill / delete / collapse / replace operations used by both mouse and keyboard paths.

use std::ops::Range;

use super::TextArea;

/// Active mouse or keyboard selection endpoints as UTF-8 caret indices.
///
/// Atomic elements are absorbed into these endpoints at hit-test time. The effective byte range is
/// expanded through live element metadata in [`TextArea::selection_range`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TextSelection {
    /// Selection origin as a UTF-8 caret index.
    pub(super) anchor: usize,
    /// Free end as a UTF-8 caret index.
    ///
    /// Together with `anchor`, this forms a half-open range after sorting: bytes in
    /// `min(anchor, active)..max(anchor, active)` (exclusive end), then expanded for elements.
    pub(super) active: usize,
}

/// How an edit or navigation path consumes an active selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SelectionCollapse {
    /// Place the cursor at the start of the selection (e.g. Left).
    Start,
    /// Place the cursor at the end of the selection (e.g. Right).
    End,
    /// Clear the selection without moving the cursor (e.g. Up / Down / set_cursor).
    Discard,
}

impl TextArea {
    pub(super) fn begin_selection(&mut self, pos: usize, element: Option<Range<usize>>) {
        // Atomic elements are selected as a whole on press so a pure click does not need stored
        // element snapshots later.
        let (anchor, active) = match element {
            Some(element) => (element.start, element.end),
            None => (pos, pos),
        };
        self.selection = Some(TextSelection { anchor, active });
    }

    pub(super) fn set_selection_endpoint(&mut self, active: usize, element: Option<Range<usize>>) {
        let Some(selection) = self.selection.as_mut() else {
            return;
        };
        selection.active = match element {
            // Dragging onto an atomic element absorbs the whole element relative to the anchor.
            Some(element) if selection.anchor <= element.start => element.end,
            Some(element) => element.start,
            None => active,
        };
    }

    pub(crate) fn finish_selection(&mut self) {
        if self.selection_range().is_none() {
            self.selection = None;
        }
    }

    pub(crate) fn clear_selection(&mut self) {
        self.selection = None;
    }

    /// Byte range of the active selection, expanded through any intersecting atomic elements.
    pub(crate) fn selection_range(&self) -> Option<Range<usize>> {
        let selection = self.selection.as_ref()?;
        let start = selection.anchor.min(selection.active);
        let end = selection.anchor.max(selection.active);
        let expanded = self.expand_range_to_element_boundaries(start..end);
        (expanded.start < expanded.end).then_some(expanded)
    }

    /// Delete the active selection without recording it in the kill buffer.
    pub(super) fn delete_selection(&mut self) -> bool {
        let Some(range) = self.selection_range() else {
            return false;
        };
        self.replace_range(range, "");
        true
    }

    /// Kill the active selection into the kill buffer. Returns `true` when a selection was present.
    pub(super) fn kill_selection(&mut self) -> bool {
        let Some(range) = self.selection_range() else {
            return false;
        };
        self.kill_range(range);
        true
    }

    /// Replace the active selection with `text`.
    ///
    /// The editable Vim cursor may sit before a selected newline; it is moved to the selection
    /// start first so the caret lands after the inserted text. Returns the byte start of the
    /// replacement when a selection was present.
    pub(super) fn replace_selection_with(&mut self, text: &str) -> Option<usize> {
        let range = self.selection_range()?;
        let start = range.start;
        self.cursor_pos = start;
        self.replace_range(range, text);
        Some(start)
    }

    /// Consume an active selection for navigation.
    ///
    /// Returns `true` when a non-empty selection was present. For [`SelectionCollapse::Start`] /
    /// [`SelectionCollapse::End`], the cursor is moved to that edge (with Vim end-of-line clamp).
    /// [`SelectionCollapse::Discard`] only clears the selection.
    pub(super) fn collapse_selection(&mut self, collapse: SelectionCollapse) -> bool {
        let Some(range) = self.selection_range() else {
            self.clear_selection();
            return false;
        };
        match collapse {
            SelectionCollapse::Start => {
                self.cursor_pos = range.start;
                if self.is_vim_normal_mode() {
                    self.cursor_pos = self.cursor_pos.min(self.vim_line_end_cursor());
                }
                self.preferred_col = None;
            }
            SelectionCollapse::End => {
                self.cursor_pos = range.end;
                if self.is_vim_normal_mode() {
                    self.cursor_pos = self.cursor_pos.min(self.vim_line_end_cursor());
                }
                self.preferred_col = None;
            }
            SelectionCollapse::Discard => {}
        }
        self.clear_selection();
        true
    }
}
