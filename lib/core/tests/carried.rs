//! A window carries its keys and its menu items: they are on the status
//! line and the bar, and bound, only while the window is the active one.
//!
//! The classic toolkits changed the status line by help context; here the window
//! simply says what it brings, and the desktop composes the bars before
//! every event and every frame.

use owlosui_core::{
    Buffer, Event, Key, KeyCode, Kind, MenuBar, MenuItem, Mods, Rect, StatusItem, StatusLine, Ui, ViewId, Window,
};

const CM_OPEN: u16 = 1;
const CM_MOUSE: u16 = 2;
const CM_EDIT: u16 = 10;
const CM_TICK: u16 = 11;

fn f4() -> Key {
    Key { code: KeyCode::F(4), mods: Mods::default() }
}

fn desk() -> Ui {
    let mut ui = Ui::new(80, 25);
    let root = ui.root();
    ui.insert(
        root,
        Rect::new(0, 0, 80, 1),
        Kind::MenuBar(MenuBar::new(vec![
            MenuItem::sub("~F~ile", vec![MenuItem::new("~O~pen", "F3", CM_OPEN)]),
            MenuItem::sub("~O~ptions", vec![MenuItem::new("~M~ouse...", "", CM_MOUSE)]),
        ])),
    );
    ui.insert(
        root,
        Rect::new(0, 24, 80, 1),
        Kind::Status(StatusLine::new(vec![StatusItem::new("~F3~ Open", None, CM_OPEN)])),
    );
    ui
}

/// A file window that brings F4 and an Options item.
fn file_window(ui: &mut Ui) -> ViewId {
    let root = ui.root();
    let mut w = Window::new("A.TXT");
    w.status = vec![StatusItem::new("~F4~ Edit", Some(f4()), CM_EDIT)];
    w.menu = vec![MenuItem::sub(
        "~O~ptions",
        vec![MenuItem::new("~E~dit / view", "F4", CM_EDIT), MenuItem::new("~T~icked", "", CM_TICK)],
    )];
    ui.insert(root, Rect::new(2, 2, 40, 10), Kind::Window(w))
}

fn status_row(ui: &mut Ui) -> String {
    let mut buf = Buffer::new(80, 25);
    ui.draw(&mut buf);
    (0..80).map(|x| buf.get(x, 24).unwrap().to_char()).collect()
}

#[test]
fn the_keys_come_and_go_with_the_window() {
    let mut ui = desk();
    assert!(!status_row(&mut ui).contains("F4"), "nothing brought F4 yet");
    let a = file_window(&mut ui);
    let row = status_row(&mut ui);
    assert!(row.contains("F3 Open") && row.contains("F4 Edit"), "{row:?}");
    // Bound: F4 is the window's command.
    ui.handle(Event::Key(f4()));
    assert_eq!(ui.take_command(), Some(CM_EDIT));

    // Another window in front: F4 is gone from the line and from the keys.
    let root = ui.root();
    let b = ui.insert(root, Rect::new(10, 5, 40, 10), Kind::Window(Window::new("B")));
    assert!(!status_row(&mut ui).contains("F4"), "B in front, and F4 still shows");
    ui.handle(Event::Key(f4()));
    assert_eq!(ui.take_command(), None, "F4 bound with B in front");

    // Back in front, back on the line.
    ui.activate(a);
    assert!(status_row(&mut ui).contains("F4 Edit"));
    let _ = b;
}

#[test]
fn the_menu_items_go_into_the_menu_of_the_same_name() {
    let mut ui = desk();
    let a = file_window(&mut ui);
    let mut buf = Buffer::new(80, 25);
    ui.draw(&mut buf);
    ui.open_menu(1);
    ui.draw(&mut buf);
    let screen: Vec<String> = (0..25).map(|y| (0..80).map(|x| buf.get(x, y).unwrap().to_char()).collect()).collect();
    let has = |s: &str| screen.iter().any(|r| r.contains(s));
    assert!(has("Mouse...") && has("Edit / view") && has("Ticked"), "Options should hold its own and the window's items");
    // The bar itself did not grow: Options took the items in.
    assert!(!screen[0].contains("Options  Options"));
    ui.close_menu();

    // A tick set on a window's item survives the next composition.
    assert!(ui.set_menu_checked(CM_TICK, true));
    let mut buf = Buffer::new(80, 25);
    ui.draw(&mut buf);
    ui.open_menu(1);
    ui.draw(&mut buf);
    let row = (0..25)
        .map(|y| (0..80).map(|x| buf.get(x, y).unwrap().to_char()).collect::<String>())
        .find(|r| r.contains("Ticked"))
        .unwrap();
    assert!(row.contains('\u{221A}') || row.contains('\u{FB}') || row.contains('√'), "no tick: {row:?}");
    ui.close_menu();

    // Without the window, Options is its own again.
    ui.close(a);
    let mut buf = Buffer::new(80, 25);
    ui.draw(&mut buf);
    ui.open_menu(1);
    ui.draw(&mut buf);
    let screen: Vec<String> = (0..25).map(|y| (0..80).map(|x| buf.get(x, y).unwrap().to_char()).collect()).collect();
    assert!(screen.iter().any(|r| r.contains("Mouse...")) && !screen.iter().any(|r| r.contains("Edit / view")));
}

#[test]
fn a_menu_with_a_new_name_goes_on_the_end_of_the_bar() {
    let mut ui = desk();
    let root = ui.root();
    let mut w = Window::new("Board");
    w.menu = vec![MenuItem::sub("~G~ame", vec![MenuItem::new("~S~cramble", "", 20)])];
    ui.insert(root, Rect::new(2, 2, 40, 10), Kind::Window(w));
    let mut buf = Buffer::new(80, 25);
    ui.draw(&mut buf);
    let bar: String = (0..80).map(|x| buf.get(x, 0).unwrap().to_char()).collect();
    let file = bar.find("File").unwrap();
    let game = bar.find("Game").unwrap();
    assert!(game > file, "{bar:?}");
}
