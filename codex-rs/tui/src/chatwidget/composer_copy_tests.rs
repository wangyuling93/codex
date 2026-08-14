use pretty_assertions::assert_eq;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;

use super::CopyToast;
use super::CopyToastKind;
use super::render_copy_toast;

fn cell_text(buf: &Buffer, area: Rect) -> String {
    let mut out = String::new();
    for x in area.x..area.right() {
        out.push_str(buf[(x, area.y)].symbol());
    }
    out
}

#[test]
fn success_toast_renders_copied_in_the_top_right() {
    let toast = CopyToast {
        kind: CopyToastKind::Success,
        expires_at: std::time::Instant::now() + std::time::Duration::from_secs(1),
    };
    let area = Rect::new(
        /*x*/ 0, /*y*/ 0, /*width*/ 40, /*height*/ 5,
    );
    let mut buf = Buffer::empty(area);
    render_copy_toast(&toast, area, &mut buf);

    let row = cell_text(
        &buf,
        Rect::new(
            /*x*/ 0, /*y*/ 0, /*width*/ 40, /*height*/ 1,
        ),
    );
    assert!(
        row.contains("Copied") && !row.contains("hello"),
        "expected Copied-only toast, got {row:?}"
    );
    let copied_x = row.find("Copied").expect("Copied label");
    assert!(copied_x > 10, "expected top-right placement, got {row:?}");

    let label_x = u16::try_from(copied_x).expect("label x");
    assert_eq!(buf[(label_x, 0)].style().fg, Some(Color::Green));
    assert!(
        buf[(label_x, 0)]
            .style()
            .add_modifier
            .contains(Modifier::BOLD)
    );
}

#[test]
fn error_toast_renders_failure_label() {
    let toast = CopyToast {
        kind: CopyToastKind::Error,
        expires_at: std::time::Instant::now() + std::time::Duration::from_secs(1),
    };
    let area = Rect::new(
        /*x*/ 0, /*y*/ 0, /*width*/ 40, /*height*/ 3,
    );
    let mut buf = Buffer::empty(area);
    render_copy_toast(&toast, area, &mut buf);

    let row = cell_text(
        &buf,
        Rect::new(
            /*x*/ 0, /*y*/ 0, /*width*/ 40, /*height*/ 1,
        ),
    );
    assert!(
        row.contains("Copy failed") && !row.contains("unavailable"),
        "expected Copy failed label only, got {row:?}"
    );
}
