//! A hole: a window's inside left for a picture the core does not draw - a
//! web page's emulator behind its canvas. Its cells are holes exactly where
//! nothing is drawn over them, cell by cell: a menu, a dialog, a shadow
//! takes away only the cells it covers.

use owlosui_core::{Buffer, Button, Dock, Event, Key, KeyCode, Kind, MenuBar, MenuItem, Mods, Mouse, MouseKind, Palette, Rect, Ui, Window};

fn desk() -> (Ui, owlosui_core::ViewId, owlosui_core::ViewId) {
    let mut ui = Ui::new(80, 25);
    let root = ui.root();
    ui.insert(
        root,
        Rect::new(0, 0, 80, 1),
        Kind::MenuBar(MenuBar::new(vec![MenuItem::sub("~F~ile", vec![MenuItem::new("~O~pen", "", 1)])])),
    );
    let dos = ui.insert(root, Rect::new(10, 3, 42, 14), Kind::Window(Window::new("DOS")));
    let hole = ui.insert(dos, Rect::default(), Kind::Hole);
    ui.set_dock(hole, Dock::Fill);
    (ui, dos, hole)
}

fn frame(ui: &mut Ui) -> Buffer {
    let mut buf = Buffer::new(80, 25);
    ui.draw(&mut buf);
    buf
}

/// The hole cells of a frame, counted inside a rectangle.
fn holes_in(buf: &Buffer, r: Rect) -> usize {
    let mut n = 0;
    for y in r.y..r.bottom() {
        for x in r.x..r.right() {
            n += buf.is_hole(x, y) as usize;
        }
    }
    n
}

const INSIDE: Rect = Rect { x: 11, y: 4, w: 40, h: 12 };

#[test]
fn the_inside_is_a_hole_and_the_frame_is_not() {
    let (mut ui, _, _) = desk();
    let buf = frame(&mut ui);
    assert_eq!(holes_in(&buf, INSIDE), 40 * 12);
    assert_eq!(holes_in(&buf, Rect::new(0, 0, 80, 25)), 40 * 12, "only the inside");
    // Drawn as the window's background, for a screen with nothing under it.
    let body = Palette::default().blue.body;
    let c = buf.get(20, 8).unwrap();
    assert_eq!((c.ch, c.attr), (b' ' as _, body));
}

#[test]
fn a_menu_takes_away_only_the_cells_it_covers() {
    let (mut ui, dos, _) = desk();
    ui.activate(dos);
    ui.handle(Event::Key(Key { code: KeyCode::Char('f'), mods: Mods { alt: true, ..Mods::default() } }));
    let buf = frame(&mut ui);
    // The File menu drops from column 0 or so: the cells it and its shadow
    // cover are the menu's, and the rest of the inside is still a hole.
    let covered = 40 * 12 - holes_in(&buf, INSIDE);
    assert!(covered > 0 && covered < 40 * 6, "{covered} cells covered by a small menu");
    assert!(buf.is_hole(50, 15), "the far corner is still a hole");
    // Closed: the whole inside again.
    ui.handle(Event::Key(Key { code: KeyCode::Esc, mods: Mods::default() }));
    let buf = frame(&mut ui);
    assert_eq!(holes_in(&buf, INSIDE), 40 * 12);
}

#[test]
fn a_window_half_over_it_and_a_shadow_take_their_own_cells() {
    let (mut ui, _, _) = desk();
    let root = ui.root();
    // Over the right half: from column 31, rows 6 to 11 - and its shadow two
    // columns right of it and a row below, inside the hole too.
    let other = ui.insert(root, Rect::new(31, 6, 10, 5), Kind::Window(Window::new("Other")));
    let buf = frame(&mut ui);
    for y in 6..11 {
        for x in 31..41 {
            assert!(!buf.is_hole(x, y), "under the window at {x},{y}");
        }
    }
    // The shadow: right of it, from its second row; below it, from its third column.
    assert!(!buf.is_hole(41, 7) && !buf.is_hole(42, 10), "the shadow on the right");
    assert!(!buf.is_hole(33, 11) && !buf.is_hole(42, 11), "the shadow below");
    assert!(buf.is_hole(41, 6) && buf.is_hole(31, 11) && buf.is_hole(32, 11), "a shadow leaves its corners");
    assert!(buf.is_hole(30, 8) && buf.is_hole(43, 8), "beside it, a hole");
    let shadowed = 2 * 4 + 10;
    assert_eq!(holes_in(&buf, INSIDE), 40 * 12 - 10 * 5 - shadowed);
    // Moved away: nothing to fix.
    ui.set_rect(other, Rect::new(60, 18, 10, 5));
    let buf = frame(&mut ui);
    assert_eq!(holes_in(&buf, INSIDE), 40 * 12);
}

#[test]
fn a_click_in_the_hole_brings_its_window_to_the_front() {
    let (mut ui, dos, hole) = desk();
    let root = ui.root();
    let other = ui.insert(root, Rect::new(40, 10, 20, 8), Kind::Window(Window::new("Other")));
    ui.activate(other);
    assert_eq!(ui.active_window(), Some(other));
    for kind in [MouseKind::Down(Button::Left), MouseKind::Up(Button::Left)] {
        ui.handle(Event::Mouse(Mouse { x: 15, y: 6, kind }));
    }
    assert_eq!(ui.active_window(), Some(dos));
    let buf = frame(&mut ui);
    assert_eq!(holes_in(&buf, INSIDE), 40 * 12, "in front, nothing covers it");
    // The hole takes no focus: it is nothing to type into.
    assert_ne!(ui.focused(), Some(hole));
}
