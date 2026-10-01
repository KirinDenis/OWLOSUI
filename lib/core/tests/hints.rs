//! A menu item's hint on the status line, and an input line's history.
//! Two small classic things: a menu hint, and an input history.

use owlosui_core::{
    Button, Event, InputLine, Key, KeyCode, Kind, MenuBar, MenuItem, Mods, Mouse, MouseKind, Rect, StatusItem,
    StatusLine, Ui, Window,
};

fn key(ui: &mut Ui, code: KeyCode) {
    ui.handle(Event::Key(Key { code, mods: Mods::default() }));
    while ui.pick_pending() {
        ui.complete_pick();
    }
}

fn row(ui: &mut Ui, y: i16) -> String {
    let mut buf = owlosui_core::Buffer::new(80, 25);
    ui.draw(&mut buf);
    (0..80).map(|x| buf.get(x, y).unwrap().to_char()).collect()
}

#[test]
fn the_status_line_shows_the_hint_of_the_item_under_the_cursor() {
    let mut ui = Ui::new(80, 25);
    let root = ui.root();
    ui.insert(
        root,
        Rect::new(0, 0, 80, 1),
        Kind::MenuBar(MenuBar::new(vec![MenuItem::sub(
            "~F~ile",
            vec![
                MenuItem::new("~O~pen", "F3", 1).hint("Open a file in a window of its own"),
                MenuItem::new("E~x~it", "Alt+X", 2),
            ],
        )])),
    );
    ui.insert(root, Rect::new(0, 24, 80, 1), Kind::Status(StatusLine::new(vec![StatusItem::new("~F1~ Help", None, 9)])));
    assert!(row(&mut ui, 24).contains("F1 Help"));
    ui.open_menu(0);
    let r = row(&mut ui, 24);
    assert!(r.contains("Open a file in a window of its own") && !r.contains("F1 Help"), "{r:?}");
    // Down to Exit, which has no hint: the keys are back.
    key(&mut ui, KeyCode::Down);
    let r = row(&mut ui, 24);
    assert!(r.contains("F1 Help"), "{r:?}");
    ui.close_menu();
    assert!(row(&mut ui, 24).contains("F1 Help"));
}

#[test]
fn an_input_line_remembers_what_was_entered_and_lists_it_on_down() {
    let mut ui = Ui::new(80, 25);
    let root = ui.root();
    let w = ui.insert(root, Rect::new(5, 5, 50, 8), Kind::Window(Window::new("Go")));
    let input = ui.insert(w, Rect::new(1, 1, 40, 1), Kind::Input(InputLine::new("Path:", "")));
    ui.set_dock(input, owlosui_core::Dock::Manual);
    ui.focus_first();
    // Nothing yet: no ▼, and Down does nothing.
    assert!(!row(&mut ui, 7).contains('\u{1F}'));
    key(&mut ui, KeyCode::Down);
    assert!(ui.menu_open().is_none());

    for c in "C:\\A".chars() {
        key(&mut ui, KeyCode::Char(c));
    }
    key(&mut ui, KeyCode::Enter);
    assert_eq!(ui.history(input), vec!["C:\\A".to_string()]);
    // Entered twice, kept once; a new one goes on top.
    key(&mut ui, KeyCode::Enter);
    assert_eq!(ui.history(input).len(), 1);
    if let Kind::Input(i) = ui.kind_mut(input) {
        i.set_text("D:\\B");
    }
    key(&mut ui, KeyCode::Enter);
    assert_eq!(ui.history(input), vec!["D:\\B".to_string(), "C:\\A".to_string()]);
    let r = row(&mut ui, 7);
    assert!(r.contains('\u{1F}'), "the ▼ at the field's end: {r:?}");

    // Down: a panel under the field, newest first; Down, Enter picks C:\A.
    key(&mut ui, KeyCode::Down);
    let mb = ui.menu_open().expect("the history panel");
    assert!(ui.rect(mb).y == 8, "under the field");
    let rows: Vec<String> = (8..12).map(|y| row(&mut ui, y)).collect();
    assert!(rows.iter().any(|r| r.contains("D:\\B")) && rows.iter().any(|r| r.contains("C:\\A")), "{rows:#?}");
    key(&mut ui, KeyCode::Down);
    key(&mut ui, KeyCode::Enter);
    assert_eq!(ui.take_command(), None, "a history row is the toolkit's, not a command");
    let Kind::Input(i) = ui.kind(input) else { panic!() };
    assert_eq!(i.text, "C:\\A");
    assert!(ui.menu_open().is_none());

    // The ▼ clicked opens it too; Escape closes it.
    let abs = ui.abs_rect(input);
    let (x, y) = (abs.x + abs.w - 1, abs.y);
    ui.handle(Event::Mouse(Mouse { x, y, kind: MouseKind::Down(Button::Left) }));
    ui.handle(Event::Mouse(Mouse { x, y, kind: MouseKind::Up(Button::Left) }));
    assert!(ui.menu_open().is_some());
    key(&mut ui, KeyCode::Esc);
    assert!(ui.menu_open().is_none());
    let Kind::Input(i) = ui.kind(input) else { panic!() };
    assert_eq!(i.text, "C:\\A", "Escape changed nothing");

    // Set from outside, read back.
    assert!(ui.set_history(input, vec!["x".into(), "y".into()]));
    assert_eq!(ui.history(input), vec!["x".to_string(), "y".to_string()]);
    assert!(!ui.set_history(w, vec![]), "not an input line");
}
