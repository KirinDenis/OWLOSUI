//! An editor's offers: what it brings to the menu bar and the status line
//! when a program switches them on - Edit menu, Find and Replace with the
//! core's own dialogs, word wrap, read only, the hex view, the two keymaps.

use owlosui_core::edit::{offer, row_starts, state};
use owlosui_core::{
    glyphs, Buffer, Event, Key, KeyCode, Kind, MenuBar, MenuItem, Mods, Rect, StatusItem, StatusLine, TextView, Ui,
    ViewId, Window,
};

fn settle(ui: &mut Ui) {
    // What an application's loop does after every event: let a pick show,
    // then collect. The core's own commands are done while collecting.
    for _ in 0..8 {
        while ui.pick_pending() {
            ui.complete_pick();
        }
        let a = ui.take_pressed();
        let b = ui.take_command();
        if a.is_none() && b.is_none() && !ui.pick_pending() {
            break;
        }
    }
}

fn press(ui: &mut Ui, code: KeyCode, mods: Mods) {
    ui.handle(Event::Key(Key { code, mods }));
    settle(ui);
}

fn key(ui: &mut Ui, code: KeyCode) {
    press(ui, code, Mods::default());
}

fn ctrl(ui: &mut Ui, c: char) {
    press(ui, KeyCode::Char(c), Mods { ctrl: true, ..Mods::default() });
}

fn alt(ui: &mut Ui, c: char) {
    press(ui, KeyCode::Char(c), Mods { alt: true, ..Mods::default() });
}

fn typed(ui: &mut Ui, s: &str) {
    for c in s.chars() {
        key(ui, KeyCode::Char(c));
    }
}

/// Columns `a..b` of a row, counted in characters: the frame is not ASCII.
fn cols(row: &str, a: usize, b: usize) -> String {
    row.chars().skip(a).take(b - a).collect()
}

fn screen(ui: &mut Ui) -> Vec<String> {
    let mut buf = Buffer::new(80, 25);
    ui.draw(&mut buf);
    (0..25).map(|y| (0..80).map(|x| buf.get(x, y).unwrap().to_char()).collect()).collect()
}

/// A desktop with a File menu (or none) and a status line.
fn desk(bar: bool) -> Ui {
    let mut ui = Ui::new(80, 25);
    let root = ui.root();
    if bar {
        ui.insert(
            root,
            Rect::new(0, 0, 80, 1),
            Kind::MenuBar(MenuBar::new(vec![
                MenuItem::sub("~F~ile", vec![MenuItem::new("~N~ew", "F3", 1)]),
                MenuItem::sub("~H~elp", vec![MenuItem::new("~A~bout", "", 2)]),
            ])),
        );
    }
    ui.insert(root, Rect::new(0, 24, 80, 1), Kind::Status(StatusLine::new(vec![StatusItem::new("~F1~ Help", None, 9)])));
    ui
}

/// A window of `w` by `h` holding a text; its inside is two less each way.
fn editor(ui: &mut Ui, text: &str, w: i16, h: i16) -> (ViewId, ViewId) {
    let root = ui.root();
    let win = ui.insert(root, Rect::new(0, 1, w, h), Kind::Window(Window::new("Doc")));
    let t = ui.insert(win, Rect::default(), Kind::Text(TextView::new(text.split('\n').map(glyphs).collect())));
    ui.activate(win);
    (win, t)
}

fn text_of(ui: &Ui, t: ViewId) -> String {
    match ui.kind(t) {
        Kind::Text(t) => t.text(),
        _ => panic!("not a text"),
    }
}

#[test]
fn a_line_folds_after_the_last_space_that_fits() {
    let l = glyphs("the quick brown fox");
    // Width 10: "the quick " then "brown fox".
    assert_eq!(row_starts(&l, 10), vec![0, 10]);
    // A word longer than the row is cut where the row ends.
    assert_eq!(row_starts(&glyphs("abcdefghijkl"), 5), vec![0, 5, 10]);
    // A line that exactly fills the row gets an empty row for the caret.
    assert_eq!(row_starts(&glyphs("abcde"), 5), vec![0, 5]);
    assert_eq!(row_starts(&glyphs("abcd"), 5), vec![0]);
    assert_eq!(row_starts(&[], 5), vec![0]);
}

#[test]
fn wrapped_text_shows_every_word_and_the_arrows_go_by_rows() {
    let mut ui = desk(true);
    // Inside 20 wide.
    let (_, t) = editor(&mut ui, "one two three four five six seven\nend", 22, 10);
    ui.set_editor(t, 0, state::WRAP);
    let s = screen(&mut ui);
    assert_eq!(cols(&s[2], 1, 21), "one two three four  ", "{s:#?}");
    assert_eq!(cols(&s[3], 1, 21), "five six seven      ");
    assert_eq!(cols(&s[4], 1, 21), "end                 ");
    // Down goes to the next row of the same line, at the same column.
    key(&mut ui, KeyCode::Right);
    key(&mut ui, KeyCode::Right);
    key(&mut ui, KeyCode::Down);
    let (_, _, y, x) = ui.editor_state(t).unwrap();
    assert_eq!((y, x), (0, 21), "row 2 of line 1, column 2: 'v' of five");
    assert_eq!(ui.cursor().map(|p| (p.x, p.y)), Some((3, 3)));
    key(&mut ui, KeyCode::Down);
    assert_eq!(ui.editor_state(t).map(|s| (s.2, s.3)), Some((1, 2)));
    // End goes to the end of the row, Home to its start.
    key(&mut ui, KeyCode::Up);
    key(&mut ui, KeyCode::End);
    assert_eq!(ui.editor_state(t).map(|s| (s.2, s.3)), Some((0, 33)), "end of the last row is the line's end");
    key(&mut ui, KeyCode::Home);
    assert_eq!(ui.editor_state(t).map(|s| (s.2, s.3)), Some((0, 19)), "start of row 2");
    // The file is what it was: only the picture folds.
    assert_eq!(text_of(&ui, t), "one two three four five six seven\nend");
    // Off again: one long line, cut at the edge.
    ui.set_editor(t, 0, 0);
    let s = screen(&mut ui);
    assert_eq!(cols(&s[2], 1, 21), "one two three four f");
}

#[test]
fn typing_at_the_end_of_a_folded_line_folds_it_further() {
    let mut ui = desk(true);
    let (_, t) = editor(&mut ui, "", 12, 8);
    ui.set_editor(t, 0, state::WRAP);
    typed(&mut ui, "aaa bbb ccc ddd");
    let s = screen(&mut ui);
    assert_eq!(cols(&s[2], 1, 11), "aaa bbb   ");
    assert_eq!(cols(&s[3], 1, 11), "ccc ddd   ");
    assert_eq!(ui.cursor().map(|p| (p.x, p.y)), Some((8, 3)));
}

#[test]
fn a_folded_view_scrolls_by_rows_and_follows_the_caret() {
    let mut ui = desk(true);
    // Inside 10 wide and 3 high; eight rows of text.
    let long = "aaaa bbbb cccc dddd eeee ffff gggg hhhh";
    let (_, t) = editor(&mut ui, long, 12, 5);
    ui.set_editor(t, 0, state::WRAP);
    screen(&mut ui);
    ctrl_end(&mut ui);
    let s = screen(&mut ui);
    assert_eq!(cols(&s[4], 1, 11), "gggg hhhh ", "the last row at the bottom: {s:#?}");
    assert_eq!(ui.cursor().map(|p| p.y), Some(4));
    press(&mut ui, KeyCode::Home, Mods { ctrl: true, ..Mods::default() });
    assert_eq!(cols(&screen(&mut ui)[2], 1, 11), "aaaa bbbb ");
    let _ = t;
}

fn ctrl_end(ui: &mut Ui) {
    press(ui, KeyCode::End, Mods { ctrl: true, ..Mods::default() });
}

#[test]
fn an_editor_that_offers_things_brings_an_edit_menu_after_file() {
    let mut ui = desk(true);
    let (_, t) = editor(&mut ui, "hello", 40, 10);
    assert!(!screen(&mut ui)[0].contains("Edit"), "nothing offered, nothing added");
    ui.set_editor(t, offer::ALL, 0);
    let bar = &screen(&mut ui)[0];
    assert!(bar.starts_with("  File  Edit  Help"), "{bar:?}");
    // Its items, with the keys of the keymap in force.
    alt(&mut ui, 'e');
    let s = screen(&mut ui).join("\n");
    for words in ["Undo", "Ctrl+Z", "Paste", "Find...", "Ctrl+F", "Replace...", "Word wrap", "Read only", "Hex view", "Classic keys"] {
        assert!(s.contains(words), "{words} in {s}");
    }
    key(&mut ui, KeyCode::Esc);
    // Another window in front: its Edit goes with it.
    let root = ui.root();
    let other = ui.insert(root, Rect::new(30, 5, 20, 6), Kind::Window(Window::new("Other")));
    ui.activate(other);
    assert!(!screen(&mut ui)[0].contains("Edit"));
}

#[test]
fn a_program_with_its_own_edit_menu_gets_the_editors_items_in_it() {
    let mut ui = Ui::new(80, 25);
    let root = ui.root();
    ui.insert(
        root,
        Rect::new(0, 0, 80, 1),
        Kind::MenuBar(MenuBar::new(vec![
            MenuItem::sub("~F~ile", vec![MenuItem::new("~N~ew", "", 1)]),
            MenuItem::sub("~E~dit", vec![MenuItem::new("~I~nsert date", "", 3)]),
        ])),
    );
    let (_, t) = editor(&mut ui, "x", 40, 10);
    ui.set_editor(t, offer::FIND, 0);
    let bar = &screen(&mut ui)[0];
    assert_eq!(bar.matches("Edit").count(), 1, "{bar:?}");
    alt(&mut ui, 'e');
    let s = screen(&mut ui).join("\n");
    assert!(s.contains("Insert date") && s.contains("Find..."), "{s}");
}

#[test]
fn word_wrap_from_the_menu_ticks_and_folds() {
    let mut ui = desk(true);
    let (_, t) = editor(&mut ui, "one two three four five six seven", 22, 10);
    ui.set_editor(t, offer::WRAP, 0);
    alt(&mut ui, 'e');
    key(&mut ui, KeyCode::Char('w'));
    assert_eq!(ui.editor_state(t).map(|s| s.1 & state::WRAP), Some(state::WRAP));
    assert_eq!(cols(&screen(&mut ui)[3], 1, 15), "five six seven");
    alt(&mut ui, 'e');
    let s = screen(&mut ui).join("\n");
    assert!(s.contains("\u{FB}Word wrap"), "ticked (the tick is glyph FB): {s}");
}

#[test]
fn find_opens_the_cores_own_dialog_and_selects_the_match() {
    let mut ui = desk(true);
    let (win, t) = editor(&mut ui, "alpha beta\ngamma beta", 40, 10);
    ui.set_editor(t, offer::FIND, 0);
    ctrl(&mut ui, 'f');
    let s = screen(&mut ui).join("\n");
    assert!(s.contains("Find") && s.contains("Text to find") && s.contains("Whole words only"), "{s}");
    assert_ne!(ui.active_window(), Some(win), "the dialog is in front");
    typed(&mut ui, "beta");
    key(&mut ui, KeyCode::Enter);
    assert_eq!(ui.active_window(), Some(win), "the dialog is gone");
    let sel = match ui.kind(t) {
        Kind::Text(tv) => tv.selection().map(|(a, b)| (a.y, a.x, b.x)),
        _ => None,
    };
    assert_eq!(sel, Some((0, 6, 10)));
    // Ctrl+L: the next one, without the dialog.
    ctrl(&mut ui, 'l');
    assert_eq!(ui.editor_state(t).map(|s| (s.2, s.3)), Some((1, 10)));
    // And round from the top again: finding means anywhere in the text.
    ctrl(&mut ui, 'l');
    assert_eq!(ui.editor_state(t).map(|s| (s.2, s.3)), Some((0, 10)));
    // A word that is not there says so, and OK takes the box away.
    ctrl(&mut ui, 'f');
    // The field starts with the selected word; replace it.
    for _ in 0..4 {
        key(&mut ui, KeyCode::Backspace);
    }
    typed(&mut ui, "delta");
    key(&mut ui, KeyCode::Enter);
    let s = screen(&mut ui).join("\n");
    assert!(s.contains("\"delta\" is not in this text."), "{s}");
    key(&mut ui, KeyCode::Enter);
    assert_eq!(ui.active_window(), Some(win));
}

#[test]
fn replace_all_counts_and_escape_leaves_the_dialog() {
    let mut ui = desk(true);
    let (win, t) = editor(&mut ui, "a cat, a cat, a dog", 40, 10);
    ui.set_editor(t, offer::REPLACE, 0);
    ctrl(&mut ui, 'h');
    typed(&mut ui, "cat");
    key(&mut ui, KeyCode::Tab);
    typed(&mut ui, "fox");
    alt(&mut ui, 'a');
    assert_eq!(text_of(&ui, t), "a fox, a fox, a dog");
    assert!(screen(&mut ui).join("\n").contains("2 replaced."));
    key(&mut ui, KeyCode::Enter);
    ctrl(&mut ui, 'h');
    key(&mut ui, KeyCode::Esc);
    assert_eq!(ui.active_window(), Some(win));
}

#[test]
fn read_only_from_the_menu_stops_typing() {
    let mut ui = desk(true);
    let (_, t) = editor(&mut ui, "fixed", 40, 10);
    ui.set_editor(t, offer::READONLY, 0);
    alt(&mut ui, 'e');
    key(&mut ui, KeyCode::Char('o'));
    typed(&mut ui, "xyz");
    assert_eq!(text_of(&ui, t), "fixed");
    assert!(screen(&mut ui)[1].contains("[view]"));
    alt(&mut ui, 'e');
    key(&mut ui, KeyCode::Char('o'));
    typed(&mut ui, "ab");
    assert_eq!(text_of(&ui, t), "abfixed");
}

#[test]
fn the_hex_view_stands_in_for_the_text_and_gives_it_back() {
    let mut ui = desk(true);
    let (win, t) = editor(&mut ui, "AB\nC", 70, 10);
    ui.set_editor(t, offer::HEX, 0);
    alt(&mut ui, 'e');
    key(&mut ui, KeyCode::Char('h'));
    let s = screen(&mut ui).join("\n");
    assert!(s.contains("41 42 0A 43"), "the bytes, the line break as 0A: {s}");
    assert!(s.contains("[hex]"));
    assert_eq!(ui.editor_state(t).map(|s| s.1 & state::HEX), Some(state::HEX));
    // The program's handle to the text still works while the bytes show.
    assert_eq!(text_of(&ui, t), "AB\nC");
    alt(&mut ui, 'e');
    key(&mut ui, KeyCode::Char('h'));
    assert_eq!(ui.children(win).first(), Some(&t), "the text is back in its place");
    typed(&mut ui, "z");
    assert_eq!(text_of(&ui, t), "zAB\nC");
    // Closing the window with the bytes showing closes the text as well.
    ui.set_editor(t, offer::HEX, state::HEX);
    ui.close(win);
    assert!(!ui.is_alive(t));
}

#[test]
fn classic_keys_from_the_menu_and_the_ctrl_q_f_chord() {
    let mut ui = desk(true);
    let (_, t) = editor(&mut ui, "abc def", 40, 10);
    ui.set_editor(t, offer::KEYS | offer::FIND, 0);
    alt(&mut ui, 'e');
    key(&mut ui, KeyCode::Char('l'));
    assert_eq!(ui.editor_state(t).map(|s| s.1 & state::CLASSIC), Some(state::CLASSIC));
    // Ctrl+D is a character right in WordStar's diamond.
    ctrl(&mut ui, 'd');
    assert_eq!(ui.editor_state(t).map(|s| s.3), Some(1));
    ctrl(&mut ui, 'q');
    key(&mut ui, KeyCode::Char('f'));
    assert!(screen(&mut ui).join("\n").contains("Text to find"));
    key(&mut ui, KeyCode::Esc);
    alt(&mut ui, 'e');
    let s = screen(&mut ui).join("\n");
    assert!(s.contains("Ctrl+Q F"), "the menu shows the classic key: {s}");
}

#[test]
fn the_status_line_says_where_the_caret_is() {
    let mut ui = desk(true);
    let (_, t) = editor(&mut ui, "one\ntwo", 40, 10);
    assert!(!screen(&mut ui)[24].contains("Ln "), "nothing offered, nothing said");
    ui.set_editor(t, offer::WRAP, state::WRAP);
    key(&mut ui, KeyCode::Down);
    key(&mut ui, KeyCode::Right);
    let line = &screen(&mut ui)[24];
    assert!(line.starts_with(" F1 Help"), "{line:?}");
    assert!(line.trim_end().ends_with("Ln 2 Col 2  Wrap"), "{line:?}");
}

#[test]
fn without_a_menu_bar_the_keys_go_on_the_status_line() {
    let mut ui = desk(false);
    let (_, t) = editor(&mut ui, "x", 40, 10);
    ui.set_editor(t, offer::FIND | offer::REPLACE, 0);
    let line = &screen(&mut ui)[24];
    assert!(line.contains("Ctrl+F Find") && line.contains("Ctrl+H Replace"), "{line:?}");
    // With a bar they are in the menu instead, and the line keeps its room.
    let mut ui = desk(true);
    let (_, t) = editor(&mut ui, "x", 40, 10);
    ui.set_editor(t, offer::FIND, 0);
    assert!(!screen(&mut ui)[24].contains("Ctrl+F"));
}

#[test]
fn a_menu_dropped_over_the_caret_hides_it() {
    // The card's cursor is above every cell: left on, it blinked through the menu.
    let mut ui = desk(true);
    let (_, t) = editor(&mut ui, "hello", 60, 10);
    ui.set_editor(t, 0, 0);
    screen(&mut ui);
    assert_eq!(ui.cursor().map(|p| (p.x, p.y)), Some((1, 2)));
    alt(&mut ui, 'f');
    screen(&mut ui);
    assert!(ui.menu_open().is_some());
    assert_eq!(ui.cursor(), None, "the caret is under the File menu");
    key(&mut ui, KeyCode::Esc);
    key(&mut ui, KeyCode::Esc);
    assert!(ui.menu_open().is_none());
    screen(&mut ui);
    assert_eq!(ui.cursor().map(|p| (p.x, p.y)), Some((1, 2)), "and back when it closes");
    // A caret the menu does not reach stays where it is.
    typed(&mut ui, &" ".repeat(40));
    alt(&mut ui, 'f');
    screen(&mut ui);
    assert!(ui.menu_open().is_some());
    assert_eq!(ui.cursor().map(|p| (p.x, p.y)), Some((41, 2)));
}
