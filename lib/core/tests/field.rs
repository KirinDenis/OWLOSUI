//! A one-line field edits the way every field a person has used does:
//! Shift with the arrows selects, Ctrl with them goes by words, typing
//! replaces the selection, Ctrl+A, Ctrl+Z, and the clipboard's three keys -
//! by the keyboard, by the mouse and by the right click's menu.

use owlosui_core::{glyphs, Buffer, Button, Event, InputLine, Key, KeyCode, Kind, Mods, Mouse, MouseKind, Rect, Ui, ViewId, Window};

fn key(ui: &mut Ui, code: KeyCode, mods: Mods) {
    ui.handle(Event::Key(Key { code, mods }));
}

fn plain(ui: &mut Ui, code: KeyCode) {
    key(ui, code, Mods::default());
}

fn ctrl(ui: &mut Ui, c: char) {
    key(ui, KeyCode::Char(c), Mods { ctrl: true, ..Mods::default() });
}

fn shift(ui: &mut Ui, code: KeyCode) {
    key(ui, code, Mods { shift: true, ..Mods::default() });
}

fn typed(ui: &mut Ui, s: &str) {
    for c in s.chars() {
        plain(ui, KeyCode::Char(c));
    }
}

fn mouse(ui: &mut Ui, x: i16, y: i16, kind: MouseKind) {
    ui.handle(Event::Mouse(Mouse { x, y, kind }));
}

fn frame(ui: &mut Ui) -> Buffer {
    let mut b = Buffer::new(80, 25);
    ui.draw(&mut b);
    b
}

fn row(b: &Buffer, y: i16) -> String {
    (0..80).map(|x| b.get(x, y).unwrap().to_char()).collect()
}

fn clip(ui: &Ui) -> String {
    ui.clipboard.iter().map(|l| l.iter().map(|&g| g as u8 as char).collect::<String>()).collect::<Vec<_>>().join("\n")
}

fn text(ui: &Ui, f: ViewId) -> String {
    match ui.kind(f) {
        Kind::Input(i) => i.text.clone(),
        _ => panic!("not a field"),
    }
}

fn selected(ui: &Ui, f: ViewId) -> String {
    match ui.kind(f) {
        Kind::Input(i) => i.selected().unwrap_or_default(),
        _ => panic!("not a field"),
    }
}

/// A dialog with one field holding `start`, focused, its caret at the end.
/// Returns the field and the screen cell of its first character.
fn field(ui: &mut Ui, start: &str) -> (ViewId, i16, i16) {
    let root = ui.root();
    let w = ui.insert(root, Rect::new(0, 1, 50, 6), Kind::Window(Window::new("Find")));
    let f = ui.insert(w, Rect::new(1, 1, 40, 1), Kind::Input(InputLine::new("Name", start)));
    ui.activate(w);
    ui.focus_on(w, f);
    let b = frame(ui);
    let y = (0..25).find(|&y| row(&b, y).contains("Name ")).expect("the field is drawn");
    let r = row(&b, y);
    let x = r[..r.find("Name ").unwrap()].chars().count() as i16 + 5;
    (f, x, y)
}

/// Choose the item of an open context menu whose words contain `label`.
fn choose(ui: &mut Ui, label: &str) {
    let b = frame(ui);
    let y = (0..25).find(|&y| row(&b, y).contains(label)).unwrap_or_else(|| panic!("no {label:?} on the screen"));
    let r = row(&b, y);
    let x = r[..r.find(label).unwrap()].chars().count() as i16;
    mouse(ui, x, y, MouseKind::Down(Button::Left));
    mouse(ui, x, y, MouseKind::Up(Button::Left));
    ui.complete_pick();
}

#[test]
fn shift_selects_ctrl_goes_by_words_and_typing_replaces_the_selection() {
    let mut ui = Ui::new(80, 25);
    let (f, _, _) = field(&mut ui, "hello big world");
    // Ctrl+Left from the end: the start of "world"; Shift+Ctrl+Left
    // takes "big " with it.
    key(&mut ui, KeyCode::Left, Mods { ctrl: true, ..Mods::default() });
    key(&mut ui, KeyCode::Left, Mods { ctrl: true, shift: true, ..Mods::default() });
    assert_eq!(selected(&ui, f), "big ");
    typed(&mut ui, "small ");
    assert_eq!(text(&ui, f), "hello small world");
    // Shift+Home, then Backspace: everything before the caret gone.
    shift(&mut ui, KeyCode::Home);
    assert_eq!(selected(&ui, f), "hello small ");
    plain(&mut ui, KeyCode::Backspace);
    assert_eq!(text(&ui, f), "world");
    // Ctrl+Z twice: back through both changes.
    ctrl(&mut ui, 'z');
    assert_eq!(text(&ui, f), "hello small world");
    ctrl(&mut ui, 'z');
    for _ in 0..5 {
        ctrl(&mut ui, 'z');
    }
    assert_eq!(text(&ui, f), "hello big world");
    // Left with a selection lands at its start, as everywhere else.
    ctrl(&mut ui, 'a');
    plain(&mut ui, KeyCode::Left);
    plain(&mut ui, KeyCode::Char('>'));
    assert_eq!(text(&ui, f), ">hello big world");
}

#[test]
fn ctrl_c_ctrl_x_ctrl_v_and_the_old_keys_carry_the_selection() {
    let mut ui = Ui::new(80, 25);
    let (f, _, _) = field(&mut ui, "alpha beta");
    let before = ui.host_clip.copied;
    // Nothing selected: Ctrl+C copies nothing and the clipboard keeps
    // what it had.
    ui.clipboard = vec![glyphs("kept")];
    ctrl(&mut ui, 'c');
    assert_eq!(clip(&ui), "kept");
    assert_eq!(ui.host_clip.copied, before);
    // Select "beta", copy it, paste it at the start.
    key(&mut ui, KeyCode::Left, Mods { ctrl: true, shift: true, ..Mods::default() });
    ctrl(&mut ui, 'c');
    assert_eq!(clip(&ui), "beta");
    assert_eq!(ui.host_clip.copied, before + 1, "a copy is counted, for the host to carry out");
    plain(&mut ui, KeyCode::Home);
    ctrl(&mut ui, 'v');
    assert_eq!(text(&ui, f), "betaalpha beta");
    // Ctrl+X: out of the field and onto the clipboard.
    ctrl(&mut ui, 'a');
    ctrl(&mut ui, 'x');
    assert_eq!(text(&ui, f), "");
    assert_eq!(clip(&ui), "betaalpha beta");
    // Shift+Insert pastes, Shift+Delete cuts, Ctrl+Insert copies.
    shift(&mut ui, KeyCode::Insert);
    assert_eq!(text(&ui, f), "betaalpha beta");
    shift(&mut ui, KeyCode::Home);
    key(&mut ui, KeyCode::Insert, Mods { ctrl: true, ..Mods::default() });
    assert_eq!(clip(&ui), "betaalpha beta");
    ctrl(&mut ui, 'a');
    shift(&mut ui, KeyCode::Delete);
    assert_eq!(text(&ui, f), "");
}

#[test]
fn a_paste_of_several_lines_is_one_line_and_replaces_the_selection() {
    let mut ui = Ui::new(80, 25);
    let (f, _, _) = field(&mut ui, "old");
    ui.clipboard = vec![glyphs("one"), glyphs("two")];
    ctrl(&mut ui, 'a');
    ctrl(&mut ui, 'v');
    assert_eq!(text(&ui, f), "one two");
}

#[test]
fn the_host_clipboard_answers_a_paste_in_a_field() {
    let mut ui = Ui::new(80, 25);
    let (f, _, _) = field(&mut ui, "ab");
    ui.host_clip.on = true;
    ctrl(&mut ui, 'v');
    assert_eq!(ui.host_clip.paste_wanted, Some(f), "the paste waits for the host");
    assert!(ui.host_paste(Some(vec![glyphs("XY")]), false));
    assert_eq!(text(&ui, f), "abXY");
    // The browser's own paste event: into the focused field, over a selection.
    ctrl(&mut ui, 'a');
    assert!(ui.host_paste(Some(vec![glyphs("new")]), true));
    assert_eq!(text(&ui, f), "new");
}

#[test]
fn the_mouse_drags_a_selection_and_a_double_click_takes_a_word() {
    let mut ui = Ui::new(80, 25);
    let (f, x, y) = field(&mut ui, "copy this word");
    mouse(&mut ui, x + 5, y, MouseKind::Down(Button::Left));
    mouse(&mut ui, x + 7, y, MouseKind::Drag);
    mouse(&mut ui, x + 9, y, MouseKind::Drag);
    mouse(&mut ui, x + 9, y, MouseKind::Up(Button::Left));
    assert_eq!(selected(&ui, f), "this");
    // The selection is drawn: ink and paper turned round.
    let b = frame(&mut ui);
    let field_attr = b.get(x, y).unwrap().attr;
    assert_eq!(b.get(x + 5, y).unwrap().attr, field_attr.rotate_left(4) & 0x7F);
    assert_eq!(b.get(x + 9, y).unwrap().attr, field_attr, "the selection ends where the drag did");
    // A click that does not move selects nothing.
    mouse(&mut ui, x + 2, y, MouseKind::Down(Button::Left));
    mouse(&mut ui, x + 2, y, MouseKind::Up(Button::Left));
    assert_eq!(selected(&ui, f), "");
    mouse(&mut ui, x + 11, y, MouseKind::Double(Button::Left));
    mouse(&mut ui, x + 11, y, MouseKind::Up(Button::Left));
    assert_eq!(selected(&ui, f), "word");
}

#[test]
fn a_right_click_on_a_field_opens_cut_copy_paste() {
    let mut ui = Ui::new(80, 25);
    let (f, x, y) = field(&mut ui, "one two");
    // Select "two" and right-click inside it: the selection stays.
    mouse(&mut ui, x + 4, y, MouseKind::Down(Button::Left));
    mouse(&mut ui, x + 7, y, MouseKind::Drag);
    mouse(&mut ui, x + 7, y, MouseKind::Up(Button::Left));
    mouse(&mut ui, x + 5, y, MouseKind::Down(Button::Right));
    let b = frame(&mut ui);
    let menu: String = (0..25).map(|r| row(&b, r)).collect();
    assert!(menu.contains("Cut") && menu.contains("Copy") && menu.contains("Paste") && menu.contains("Select all"), "no context menu:\n{menu}");
    choose(&mut ui, "Cut");
    assert_eq!(text(&ui, f), "one ");
    assert_eq!(clip(&ui), "two");
    // At the start, outside any selection: Paste goes there.
    mouse(&mut ui, x, y, MouseKind::Down(Button::Right));
    choose(&mut ui, "Paste");
    assert_eq!(text(&ui, f), "twoone ");
    mouse(&mut ui, x, y, MouseKind::Down(Button::Right));
    choose(&mut ui, "Undo");
    assert_eq!(text(&ui, f), "one ");
}

#[test]
fn shift_click_stretches_the_selection_in_a_field_and_in_a_text() {
    let mut ui = Ui::new(80, 25);
    let (f, x, y) = field(&mut ui, "one two three");
    let shift = Mods { shift: true, ..Mods::default() };
    // The caret at the start of "two", then Shift+click after it.
    mouse(&mut ui, x + 4, y, MouseKind::Down(Button::Left));
    mouse(&mut ui, x + 4, y, MouseKind::Up(Button::Left));
    ui.mouse_with(Mouse { x: x + 7, y, kind: MouseKind::Down(Button::Left) }, shift);
    mouse(&mut ui, x + 7, y, MouseKind::Up(Button::Left));
    assert_eq!(selected(&ui, f), "two");
    // Again, further: the far end stays where it was.
    ui.mouse_with(Mouse { x: x + 13, y, kind: MouseKind::Down(Button::Left) }, shift);
    mouse(&mut ui, x + 13, y, MouseKind::Up(Button::Left));
    assert_eq!(selected(&ui, f), "two three");
    // Without Shift a click lets it go.
    mouse(&mut ui, x + 1, y, MouseKind::Down(Button::Left));
    mouse(&mut ui, x + 1, y, MouseKind::Up(Button::Left));
    assert_eq!(selected(&ui, f), "");

    // The same in a text: a window at (0,1), its first cell (1,2).
    let mut ui = Ui::new(80, 25);
    let root = ui.root();
    let win = ui.insert(root, Rect::new(0, 1, 40, 10), Kind::Window(Window::new("Doc")));
    let t = ui.insert(win, Rect::default(), Kind::Text(owlosui_core::TextView::new(vec![glyphs("hello world"), glyphs("second line")])));
    ui.activate(win);
    frame(&mut ui);
    mouse(&mut ui, 7, 2, MouseKind::Down(Button::Left));
    mouse(&mut ui, 7, 2, MouseKind::Up(Button::Left));
    ui.mouse_with(Mouse { x: 7, y: 3, kind: MouseKind::Down(Button::Left) }, shift);
    mouse(&mut ui, 7, 3, MouseKind::Up(Button::Left));
    let sel = match ui.kind(t) {
        Kind::Text(tv) => tv.selected_text().iter().map(|l| l.iter().map(|&g| g as u8 as char).collect::<String>()).collect::<Vec<_>>().join("\n"),
        _ => unreachable!(),
    };
    assert_eq!(sel, "world\nsecond");
}
