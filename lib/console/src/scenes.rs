//! Scenes that exist in two places at once.
//!
//! Each one here is built again, by hand, in `TOOLS/REFGEN/REFGEN.PAS` in the
//! wire-city repository — same coordinates, same titles, same numbers — and
//! compiled against the classic DOS toolkit. Running that produces a dump
//! of what that toolkit actually drew; `--match` renders the scene of the same
//! name here and says which cells disagree.
//!
//! Keeping the two definitions in step is the price of the method, and it is
//! worth paying: a disagreement then means a real difference in behaviour and
//! not a difference in what we asked for.

use owlosui_core::{Kind, Rect, TextView, Ui, Window};

pub const NAMES: &[&str] = &["desktop", "one-window", "two-windows", "scrollbars"];

/// Build a named scene at 80x25. Returns `None` for a name we do not know.
pub fn build(name: &str) -> Option<Ui> {
    let mut ui = Ui::new(80, 25);
    let root = ui.root();

    // Every rectangle here is one row lower than the one in REFGEN.PAS, and
    // deliberately so: the reference's coordinates are the desktop's, and the
    // desktop begins below the menu bar. We have no menu bar yet, so our
    // desktop starts at row 0. When MenuBar becomes a real view this offset
    // comes back out — and the comparison will say so the moment it does.
    match name {
        // REF00 - nothing but the background.
        "desktop" => {}

        // REF01 - TRect.Assign(5, 3, 60, 18), 'One window', number 1.
        "one-window" => {
            let mut w = Window::new("One window");
            w.number = Some(1);
            ui.insert(root, Rect::new(5, 4, 55, 15), Kind::Window(w));
        }

        // REF02 - two windows, the second in front.
        "two-windows" => {
            let mut a = Window::new("Behind");
            a.number = Some(1);
            ui.insert(root, Rect::new(3, 3, 45, 12), Kind::Window(a));
            let mut b = Window::new("In front");
            b.number = Some(2);
            ui.insert(root, Rect::new(18, 8, 54, 13), Kind::Window(b));
        }

        // REF03 - a window whose content scrolls in both directions. The
        // reference was given the scrollbar positions directly; we get them by
        // giving the view more content than fits, which is the same thing
        // arrived at from the other end.
        //
        // This scene does not reach zero, and the reason is worth keeping.
        // The reference window is *empty* — the reference toolkit needs no content to
        // own a scrollbar — while ours must hold a view, because our window
        // asks its first child how far it has scrolled. So 728 cells of blank
        // content come out 0x1E where the reference has 0x1F. Both are blue on
        // screen; a space shows only its background. It goes to zero when
        // scrollbars stop depending on a content view, which is a change worth
        // making for its own sake.
        "scrollbars" => {
            let mut w = Window::new("Scrollbars");
            w.number = Some(3);
            let wid = ui.insert(root, Rect::new(8, 5, 58, 15), Kind::Window(w));
            let long: Vec<Vec<owlosui_core::Glyph>> = (0..100).map(|_| vec![b' ' as owlosui_core::Glyph; 100]).collect();
            let mut t = TextView::new(long);
            t.top = 30;
            t.left = 20;
            ui.insert(wid, Rect::new(0, 0, 56, 13), Kind::Text(t));
        }

        _ => return None,
    }

    Some(ui)
}
