//! Clipboard write and toast for composer mouse-selection copies.

use std::time::Duration;
use std::time::Instant;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Clear;
use ratatui::widgets::Widget;

use super::*;
use crate::clipboard_copy::ClipboardLease;
use crate::render::renderable::Renderable;
use crate::render::renderable::RenderableItem;

const TOAST_DURATION: Duration = Duration::from_millis(1800);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CopyToastKind {
    Success,
    Error,
}

#[derive(Debug, Clone)]
pub(super) struct CopyToast {
    pub(super) kind: CopyToastKind,
    pub(super) expires_at: Instant,
}

impl CopyToast {
    fn success() -> Self {
        Self {
            kind: CopyToastKind::Success,
            expires_at: Instant::now() + TOAST_DURATION,
        }
    }

    fn error() -> Self {
        Self {
            kind: CopyToastKind::Error,
            expires_at: Instant::now() + TOAST_DURATION,
        }
    }

    fn is_visible(&self, now: Instant) -> bool {
        now < self.expires_at
    }

    fn label(&self) -> &'static str {
        match self.kind {
            CopyToastKind::Success => "Copied",
            CopyToastKind::Error => "Copy failed",
        }
    }
}

struct CopyToastOverlay<'a> {
    child: RenderableItem<'a>,
    toast: Option<&'a CopyToast>,
}

impl Renderable for CopyToastOverlay<'_> {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.child.render(area, buf);
        if let Some(toast) = self.toast {
            render_copy_toast(toast, area, buf);
        }
    }

    fn desired_height(&self, width: u16) -> u16 {
        self.child.desired_height(width)
    }

    fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        self.child.cursor_pos(area)
    }

    fn cursor_style(&self, area: Rect) -> crossterm::cursor::SetCursorStyle {
        self.child.cursor_style(area)
    }
}

fn render_copy_toast(toast: &CopyToast, area: Rect, buf: &mut Buffer) {
    if area.width < 8 || area.height == 0 {
        return;
    }

    let label = toast.label();
    let styled = match toast.kind {
        CopyToastKind::Success => label.green().bold(),
        CopyToastKind::Error => label.red().bold(),
    };
    let line = Line::from(vec![" ".into(), styled, " ".into()]);
    let width = (line.width() as u16).min(area.width.saturating_sub(1).max(1));
    let x = area.right().saturating_sub(width);
    let toast_area = Rect::new(x, area.y, width, 1);
    Clear.render(toast_area, buf);
    line.render(toast_area, buf);
}

impl ChatWidget {
    pub(super) fn wrap_renderable_with_copy_toast<'a>(
        &'a self,
        child: RenderableItem<'a>,
    ) -> RenderableItem<'a> {
        let toast = self
            .copy_toast
            .as_ref()
            .filter(|toast| toast.is_visible(Instant::now()));
        RenderableItem::Owned(Box::new(CopyToastOverlay { child, toast }))
    }

    pub(super) fn expire_copy_toast(&mut self) {
        let now = Instant::now();
        let Some(toast) = self.copy_toast.as_ref() else {
            return;
        };
        if toast.is_visible(now) {
            self.frame_requester
                .schedule_frame_in(toast.expires_at.saturating_duration_since(now));
        } else {
            self.copy_toast = None;
            self.request_redraw();
        }
    }

    fn show_copy_success_toast(&mut self) {
        self.copy_toast = Some(CopyToast::success());
        self.frame_requester.schedule_frame_in(TOAST_DURATION);
        self.request_redraw();
    }

    fn show_copy_error_toast(&mut self) {
        self.copy_toast = Some(CopyToast::error());
        self.frame_requester.schedule_frame_in(TOAST_DURATION);
        self.request_redraw();
    }

    pub(super) fn copy_composer_text(&mut self, text: &str) {
        self.copy_composer_text_with(text, |text| {
            crate::clipboard_copy::copy_to_clipboard(
                text,
                crate::clipboard_copy::CopyFormat::PlainText,
            )
        });
    }

    pub(super) fn copy_composer_text_with(
        &mut self,
        text: &str,
        copy_fn: impl FnOnce(&str) -> Result<Option<ClipboardLease>, String>,
    ) {
        match copy_fn(text) {
            Ok(lease) => {
                self.clipboard_lease = lease;
                self.show_copy_success_toast();
            }
            Err(_) => self.show_copy_error_toast(),
        }
    }
}

#[cfg(test)]
#[path = "composer_copy_tests.rs"]
mod tests;
