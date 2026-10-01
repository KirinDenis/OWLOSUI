//! The desktop's own keys, the ones the classic DOS desktops had: numbered
//! windows and Alt+1..9, the window list on Alt+0, Shift+F6 for the
//! previous window, and Ctrl+F5 to move or resize from the keyboard.

use owlosui_core::{Buffer, Event, Key, KeyCode, Kind, Mods, Rect, Ui, ViewId, Window};

fn key(ui: &mut Ui, code: KeyCode, mods: Mods) {
    ui.handle(Event::Key(Key { code, mods }));
    while ui.pick_pending() {
        ui.complete_pick();
    }
}

fn alt(c: char) -> (KeyCode, Mods) {
    (KeyCode::Char(c), Mods { alt: true, ..Mods::default() })
}

fn doc(ui: &mut Ui, title: &str, x: i16) -> ViewId {
    let root = ui.root();
    ui.insert(root, Rect::new(x, 3, 24, 8), Kind::Window(Window::new(title)))
}

fn number(ui: &Ui, id: ViewId) -> Option<u8> {
    match ui.kind(id) {
        Kind::Window(w) => w.number,
        _ => None,
    }
}

fn frame_row(ui: &mut Ui, y: i16) -> String {
    let mut buf = Buffer::new(80, 25);
    ui.draw(&mut buf);
    (0..80).map(|x| buf.get(x, y).unwrap().to_char()).collect()
}

#[test]
fn windows_are_numbered_as_they_open_and_numbers_are_reused() {
    let mut ui = Ui::new(80, 25);
    // Side by side, so every frame shows.
    let a = doc(&mut ui, "A", 0);
    let b = doc(&mut ui, "B", 26);
    let c = doc(&mut ui, "C", 52);
    assert_eq!((number(&ui, a), number(&ui, b), number(&ui, c)), (Some(1), Some(2), Some(3)));
    // The number is in the frame, near the right end of the title row.
    let row = frame_row(&mut ui, 3);
    assert!(row.contains('1') && row.contains('2') && row.contains('3'), "{row:?}");
    ui.close(b);
    let d = doc(&mut ui, "D", 26);
    assert_eq!(number(&ui, d), Some(2), "a closed window's number is taken again");
    // A modal window takes no number.
    let root = ui.root();
    let mut m = Window::new("Ask");
    m.modal = true;
    let m = ui.insert(root, Rect::new(20, 8, 30, 6), Kind::Window(m));
    assert_eq!(number(&ui, m), None);
    ui.close(m);
    // Ten windows: the tenth has none.
    for i in 0..6 {
        doc(&mut ui, "X", i);
    }
    let tenth = doc(&mut ui, "Tenth", 30);
    assert_eq!(number(&ui, tenth), None);
}

#[test]
fn alt_and_a_digit_brings_that_window_to_the_front() {
    let mut ui = Ui::new(80, 25);
    let a = doc(&mut ui, "A", 0);
    let b = doc(&mut ui, "B", 5);
    let c = doc(&mut ui, "C", 10);
    assert_eq!(ui.active_window(), Some(c));
    let (code, mods) = alt('1');
    key(&mut ui, code, mods);
    assert_eq!(ui.active_window(), Some(a), "Alt+1 is window 1");
    let (code, mods) = alt('2');
    key(&mut ui, code, mods);
    assert_eq!(ui.active_window(), Some(b));
    let (code, mods) = alt('7');
    key(&mut ui, code, mods);
    assert_eq!(ui.active_window(), Some(b), "no window 7: nothing happens");
    // Shift+F6: the one at the back comes forward.
    key(&mut ui, KeyCode::F(6), Mods { shift: true, ..Mods::default() });
    assert_eq!(ui.active_window(), Some(c), "C was at the back");
    // Turned off, the desktop leaves the keys alone.
    ui.desktop_keys = false;
    let (code, mods) = alt('1');
    key(&mut ui, code, mods);
    assert_eq!(ui.active_window(), Some(c));
}

#[test]
fn alt_zero_lists_the_windows_and_enter_picks_one() {
    let mut ui = Ui::new(80, 25);
    let a = doc(&mut ui, "Alpha", 0);
    let _b = doc(&mut ui, "Beta", 5);
    let c = doc(&mut ui, "Gamma", 10);
    let (code, mods) = alt('0');
    key(&mut ui, code, mods);
    let m = ui.modal().expect("the list is a modal dialog");
    let rows: Vec<String> = (0..25).map(|y| frame_row(&mut ui, y)).collect();
    assert!(rows.iter().any(|r| r.contains("Windows [modal]")));
    assert!(rows.iter().any(|r| r.contains("1  Alpha")) && rows.iter().any(|r| r.contains("3  Gamma")), "{rows:#?}");
    // Front to back: Gamma first. Down twice is Alpha; Enter picks it.
    key(&mut ui, KeyCode::Down, Mods::default());
    key(&mut ui, KeyCode::Down, Mods::default());
    key(&mut ui, KeyCode::Enter, Mods::default());
    assert_eq!(ui.take_pressed(), None, "the list's OK is the toolkit's, not the program's");
    assert!(!ui.is_alive(m), "the dialog closed");
    assert_eq!(ui.active_window(), Some(a), "Alpha came to the front");
    // Escape cancels and changes nothing.
    let (code, mods) = alt('0');
    key(&mut ui, code, mods);
    key(&mut ui, KeyCode::Esc, Mods::default());
    assert_eq!(ui.take_pressed(), None);
    assert_eq!(ui.active_window(), Some(a));
    let _ = c;
}

#[test]
fn ctrl_f5_moves_and_resizes_from_the_keyboard() {
    let mut ui = Ui::new(80, 25);
    let a = doc(&mut ui, "A", 10);
    key(&mut ui, KeyCode::F(5), Mods { ctrl: true, ..Mods::default() });
    assert_eq!(ui.sizing(), Some(a));
    key(&mut ui, KeyCode::Right, Mods::default());
    key(&mut ui, KeyCode::Right, Mods::default());
    key(&mut ui, KeyCode::Down, Mods::default());
    assert_eq!(ui.rect(a), Rect::new(12, 4, 24, 8), "arrows move");
    let shift = Mods { shift: true, ..Mods::default() };
    key(&mut ui, KeyCode::Right, shift);
    key(&mut ui, KeyCode::Up, shift);
    assert_eq!(ui.rect(a), Rect::new(12, 4, 25, 7), "Shift+arrows resize");
    key(&mut ui, KeyCode::Enter, Mods::default());
    assert_eq!(ui.sizing(), None, "Enter keeps it");
    assert_eq!(ui.rect(a), Rect::new(12, 4, 25, 7));
    // Escape puts it back where it was when Ctrl+F5 was pressed.
    key(&mut ui, KeyCode::F(5), Mods { ctrl: true, ..Mods::default() });
    key(&mut ui, KeyCode::Left, Mods::default());
    key(&mut ui, KeyCode::Esc, Mods::default());
    assert_eq!(ui.rect(a), Rect::new(12, 4, 25, 7));
    assert_eq!(ui.sizing(), None);
    // A fixed window moves but does not resize.
    let root = ui.root();
    let mut d = Window::new("Dialog");
    d.resizable = false;
    let d = ui.insert(root, Rect::new(20, 6, 24, 6), Kind::Window(d));
    key(&mut ui, KeyCode::F(5), Mods { ctrl: true, ..Mods::default() });
    key(&mut ui, KeyCode::Right, shift);
    key(&mut ui, KeyCode::Down, Mods::default());
    key(&mut ui, KeyCode::Enter, Mods::default());
    assert_eq!(ui.rect(d), Rect::new(20, 7, 24, 6));
}
