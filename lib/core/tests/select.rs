//! Selecting with the mouse, the context menu a right click opens, and the
//! clipboard a host program shares with the core: what a person who has
//! used any program since expects to find in a text and in a console.

use owlosui_core::{glyphs, Buffer, Button, Console, Dock, Event, Glyph, Key, KeyCode, Kind, Mods, Mouse, MouseKind, Rect, TextView, Ui, ViewId, Window};

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

/// A window at (0,1) 40x10 with a text in it; the text's first cell is (1,2).
fn editor(ui: &mut Ui, text: &str) -> ViewId {
    let root = ui.root();
    let win = ui.insert(root, Rect::new(0, 1, 40, 10), Kind::Window(Window::new("Doc")));
    let t = ui.insert(win, Rect::default(), Kind::Text(TextView::new(text.split('\n').map(glyphs).collect())));
    ui.activate(win);
    t
}

fn selected(ui: &Ui, t: ViewId) -> String {
    match ui.kind(t) {
        Kind::Text(t) => t.selected_text().iter().map(|l| l.iter().map(|&g| g as u8 as char).collect::<String>()).collect::<Vec<_>>().join("\n"),
        _ => panic!("not a text"),
    }
}

fn clip(ui: &Ui) -> String {
    ui.clipboard.iter().map(|l| l.iter().map(|&g| g as u8 as char).collect::<String>()).collect::<Vec<_>>().join("\n")
}

/// Choose the item of an open context menu whose words contain `label`.
fn choose(ui: &mut Ui, label: &str) {
    let b = frame(ui);
    let y = (0..25).find(|&y| row(&b, y).contains(label)).unwrap_or_else(|| panic!("no {label:?} on the screen"));
    let x = row(&b, y).find(label).unwrap() as i16;
    mouse(ui, x, y, MouseKind::Down(Button::Left));
    mouse(ui, x, y, MouseKind::Up(Button::Left));
    ui.complete_pick();
}

#[test]
fn dragging_with_the_button_down_selects_and_a_click_selects_nothing() {
    let mut ui = Ui::new(80, 25);
    let t = editor(&mut ui, "hello world\nsecond line");
    frame(&mut ui);
    // From the "w" of world to the "c" of second: across a line.
    mouse(&mut ui, 7, 2, MouseKind::Down(Button::Left));
    mouse(&mut ui, 5, 3, MouseKind::Drag);
    mouse(&mut ui, 4, 3, MouseKind::Drag);
    mouse(&mut ui, 4, 3, MouseKind::Up(Button::Left));
    assert_eq!(selected(&ui, t), "world\nsec");
    // A click without moving: the caret moves, nothing is selected.
    mouse(&mut ui, 3, 2, MouseKind::Down(Button::Left));
    mouse(&mut ui, 3, 2, MouseKind::Up(Button::Left));
    assert_eq!(selected(&ui, t), "");
}

#[test]
fn a_right_click_opens_cut_copy_paste_and_they_work() {
    let mut ui = Ui::new(80, 25);
    let t = editor(&mut ui, "one two three");
    frame(&mut ui);
    // Select "two", then right-click inside it: the selection stays.
    mouse(&mut ui, 5, 2, MouseKind::Down(Button::Left));
    mouse(&mut ui, 8, 2, MouseKind::Drag);
    mouse(&mut ui, 8, 2, MouseKind::Up(Button::Left));
    mouse(&mut ui, 6, 2, MouseKind::Down(Button::Right));
    let b = frame(&mut ui);
    let menu: String = (3..12).map(|y| row(&b, y)).collect();
    assert!(menu.contains("Cut") && menu.contains("Copy") && menu.contains("Paste") && menu.contains("Select all"), "no context menu:\n{menu}");
    let before = ui.host_clip.copied;
    choose(&mut ui, "Copy");
    assert_eq!(clip(&ui), "two");
    assert_eq!(ui.host_clip.copied, before + 1, "a copy is counted, for the host to carry out");

    // Right-click at the end, outside the selection: the caret goes there.
    mouse(&mut ui, 14, 2, MouseKind::Down(Button::Right));
    choose(&mut ui, "Paste");
    match ui.kind(t) {
        Kind::Text(tv) => assert_eq!(tv.text(), "one two threetwo"),
        _ => unreachable!(),
    }
    // Cut is greyed out with nothing selected - it is still on the menu.
    mouse(&mut ui, 3, 2, MouseKind::Down(Button::Right));
    let b = frame(&mut ui);
    assert!((3..12).any(|y| row(&b, y).contains("Cut")));
}

#[test]
fn with_a_host_clipboard_paste_waits_for_the_host() {
    let mut ui = Ui::new(80, 25);
    let t = editor(&mut ui, "abc");
    ui.host_clip.on = true;
    frame(&mut ui);
    // Nothing on the core's clipboard, and still Paste is offered: the
    // host's may have something.
    mouse(&mut ui, 4, 2, MouseKind::Down(Button::Right));
    choose(&mut ui, "Paste");
    assert_eq!(ui.host_clip.paste_wanted, Some(t), "the paste waits for the host");
    let pasted = ui.host_paste(Some(vec![glyphs("XY")]), false);
    assert!(pasted);
    match ui.kind(t) {
        Kind::Text(tv) => assert_eq!(tv.text(), "abcXY"),
        _ => unreachable!(),
    }
    // The host's own paste key: into the text with the focus, nothing waiting.
    assert!(ui.host_paste(Some(vec![glyphs("!")]), true));
    match ui.kind(t) {
        Kind::Text(tv) => assert_eq!(tv.text(), "abcXY!"),
        _ => unreachable!(),
    }
}

fn ascii(c: char) -> Glyph {
    c as u32 as Glyph
}

#[test]
fn a_console_selects_with_the_mouse_copies_with_ctrl_c_and_copies_all_without_a_selection() {
    let mut ui = Ui::new(80, 25);
    let root = ui.root();
    let w = ui.insert(root, Rect::new(0, 1, 40, 10), Kind::Window(Window::new("Console")));
    let mut con = Console::new(100);
    con.write("\x1b[31mfirst line\x1b[0m\nsecond line\nthird\n", &mut ascii);
    let id = ui.insert(w, Rect::default(), Kind::Console(con));
    ui.set_dock(id, Dock::Fill);
    ui.activate(w);
    ui.focus_on(w, id);
    frame(&mut ui);
    let ctrl = |c| Event::Key(Key { code: KeyCode::Char(c), mods: Mods { ctrl: true, ..Mods::default() } });

    // "line" of the first line to "sec" of the second.
    mouse(&mut ui, 7, 2, MouseKind::Down(Button::Left));
    mouse(&mut ui, 4, 3, MouseKind::Drag);
    mouse(&mut ui, 4, 3, MouseKind::Up(Button::Left));
    let b = frame(&mut ui);
    assert_eq!(b.get(7, 2).unwrap().attr, 0x70, "selected text is black on grey, whatever its colour");
    ui.handle(ctrl('c'));
    assert_eq!(clip(&ui), "line\nsec");

    // Escape lets go of it; Ctrl+C then copies everything.
    ui.handle(Event::Key(Key { code: KeyCode::Esc, mods: Mods::default() }));
    ui.handle(ctrl('c'));
    assert_eq!(clip(&ui), "first line\nsecond line\nthird");

    // The right click offers Copy all, and Select all then makes it Copy.
    mouse(&mut ui, 5, 4, MouseKind::Down(Button::Right));
    let b = frame(&mut ui);
    assert!((3..12).any(|y| row(&b, y).contains("Copy all")), "no Copy all");
    choose(&mut ui, "Select all");
    mouse(&mut ui, 5, 4, MouseKind::Down(Button::Right));
    let b = frame(&mut ui);
    assert!(!(3..12).any(|y| row(&b, y).contains("Copy all")) && (3..12).any(|y| row(&b, y).contains("Copy")));
}
