//! A viewer is an editor that does not take typing, and the title says
//! so: `[view]`, in the same brackets a modal window wears. A hex dump
//! says `[hex]`. The colour stays the editor's blue.

use owlosui_core::{glyphs, Buffer, Event, HexView, Key, KeyCode, Kind, Mods, Rect, TextView, Ui, Window};

fn title_row(ui: &mut Ui) -> String {
    let mut buf = Buffer::new(60, 12);
    ui.draw(&mut buf);
    (0..60).map(|x| buf.get(x, 2).unwrap().to_char()).collect()
}

#[test]
fn a_read_only_text_window_says_view_and_an_editor_says_nothing() {
    let mut ui = Ui::new(60, 12);
    let root = ui.root();
    let w = ui.insert(root, Rect::new(5, 2, 40, 8), Kind::Window(Window::new("A.TXT")));
    let mut t = TextView::new(vec![glyphs("hello")]);
    t.readonly = true;
    let text = ui.insert(w, Rect::default(), Kind::Text(t));
    ui.focus_first();
    assert!(title_row(&mut ui).contains(" A.TXT [view] "));
    assert!(ui.cursor().is_none(), "a viewer shows no caret");

    // Off: the tag goes, the caret comes, typing lands.
    assert!(ui.set_readonly(text, false));
    ui.focus_first();
    let row = title_row(&mut ui);
    assert!(row.contains(" A.TXT ") && !row.contains("view"), "{row:?}");
    ui.handle(Event::Key(Key { code: KeyCode::Char('x'), mods: Mods::default() }));
    let Kind::Text(t) = ui.kind(text) else { panic!() };
    assert_eq!(t.lines[0], glyphs("xhello"));

    // On again: the tag is back and typing does nothing.
    assert!(ui.set_readonly(text, true));
    assert!(title_row(&mut ui).contains("[view]"));
    ui.handle(Event::Key(Key { code: KeyCode::Char('y'), mods: Mods::default() }));
    let Kind::Text(t) = ui.kind(text) else { panic!() };
    assert_eq!(t.lines[0], glyphs("xhello"), "a viewer takes no typing");
    assert!(!ui.set_readonly(w, true), "not a text");
}

#[test]
fn a_hex_window_says_hex() {
    let mut ui = Ui::new(60, 12);
    let root = ui.root();
    let w = ui.insert(root, Rect::new(5, 2, 40, 8), Kind::Window(Window::new("A.BIN")));
    ui.insert(w, Rect::default(), Kind::Hex(HexView::new(b"Hello".to_vec())));
    let row = title_row(&mut ui);
    assert!(row.contains(" A.BIN [hex] "), "{row:?}");
}
