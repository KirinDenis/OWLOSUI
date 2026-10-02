//! Where a window's inside is, and whether anything is drawn over it: what a
//! program needs to lay a picture of its own - a web page's emulator - over
//! a window and take it away while a menu or another window covers it.

use owlosui_core::{Buffer, Event, Key, KeyCode, Kind, MenuBar, MenuItem, Mods, Rect, Ui, Window};

fn desk() -> Ui {
    let mut ui = Ui::new(80, 25);
    let root = ui.root();
    ui.insert(
        root,
        Rect::new(0, 0, 80, 1),
        Kind::MenuBar(MenuBar::new(vec![MenuItem::sub("~F~ile", vec![MenuItem::new("~O~pen", "", 1)])])),
    );
    ui
}

fn frame(ui: &mut Ui) {
    let mut buf = Buffer::new(80, 25);
    ui.draw(&mut buf);
}

#[test]
fn the_inside_of_a_window_and_what_covers_it() {
    let mut ui = desk();
    let root = ui.root();
    let dos = ui.insert(root, Rect::new(10, 3, 42, 14), Kind::Window(Window::new("DOS")));
    frame(&mut ui);
    // The frame is one cell all round.
    assert_eq!(ui.place(dos), Some((Rect::new(11, 4, 40, 12), false)));

    // A window put over a corner of it covers it; moved clear, it does not -
    // its shadow included, two cells right and one down.
    let other = ui.insert(root, Rect::new(45, 10, 20, 8), Kind::Window(Window::new("Other")));
    frame(&mut ui);
    assert_eq!(ui.place(dos).map(|p| p.1), Some(true));
    ui.set_rect(other, Rect::new(0, 3, 10, 8));
    frame(&mut ui);
    assert_eq!(ui.place(dos).map(|p| p.1), Some(true), "the shadow of a window just left of it falls on it");
    ui.set_rect(other, Rect::new(0, 3, 9, 8));
    frame(&mut ui);
    assert_eq!(ui.place(dos).map(|p| p.1), Some(false));

    // The window brought to the front is covered by nothing; a menu dropped
    // over it covers it again.
    ui.activate(dos);
    frame(&mut ui);
    assert_eq!(ui.place(dos).map(|p| p.1), Some(false));
    ui.handle(Event::Key(Key { code: KeyCode::Char('f'), mods: Mods { alt: true, ..Mods::default() } }));
    frame(&mut ui);
    assert_eq!(ui.place(dos).map(|p| p.1), Some(true), "the File menu is open over the window");

    // Not a window: no place.
    assert_eq!(ui.place(root), None);
}

#[test]
fn a_window_carries_the_word_its_program_gives_it() {
    let mut ui = desk();
    let root = ui.root();
    let mut w = Window::new("DOS");
    w.tag = "awake".into();
    let dos = ui.insert(root, Rect::new(10, 3, 42, 14), Kind::Window(w));
    let mut buf = Buffer::new(80, 25);
    ui.draw(&mut buf);
    let top: String = (10..52).map(|x| buf.get(x, 3).unwrap().to_char()).collect();
    assert!(top.contains("DOS [awake]"), "the top edge reads {top:?}");
    if let Kind::Window(w) = ui.kind_mut(dos) {
        w.tag.clear();
    }
    ui.draw(&mut buf);
    let top: String = (10..52).map(|x| buf.get(x, 3).unwrap().to_char()).collect();
    assert!(!top.contains("awake"), "the word stayed: {top:?}");
}

#[test]
fn a_windows_own_words_at_the_right_of_the_status_line_with_a_lit_part() {
    use owlosui_core::{StatusItem, StatusLine};
    let mut ui = desk();
    let root = ui.root();
    ui.insert(root, Rect::new(0, 24, 80, 1), Kind::Status(StatusLine::new(vec![StatusItem::new("~F1~ Help", None, 1)])));
    let mut w = Window::new("DOS");
    w.indicator = "[~A:~ B: C:]".into();
    ui.insert(root, Rect::new(10, 3, 42, 14), Kind::Window(w));
    let mut buf = Buffer::new(80, 25);
    ui.draw(&mut buf);
    let row: String = (0..80).map(|x| buf.get(x, 24).unwrap().to_char()).collect();
    assert!(row.ends_with("[A: B: C:]"), "the status line reads {row:?}");
    let a = row.find("A:").unwrap() as i16;
    let b = row.find("B:").unwrap() as i16;
    assert_ne!(buf.get(a, 24).unwrap().attr, buf.get(b, 24).unwrap().attr, "A: is not lit apart from B:");
}
