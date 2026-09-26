//! A double click: the click it is, and then the second meaning of what
//! is under the pointer.

use owlosui_core::files::ATTR_DIR;
use owlosui_core::{
    Button, ButtonRow, Dock, Event, FileEntry, FileList, Kind, ListBox, Mouse, MouseKind, PushButton, Rect, Ui,
    Window,
};

fn entry(name: &str, size: u32, attrs: u8) -> FileEntry {
    FileEntry { name: name.into(), size, date: (0, 0, 0, 0, 0), attrs }
}

fn double(ui: &mut Ui, x: i16, y: i16) {
    ui.handle(Event::Mouse(Mouse { x, y, kind: MouseKind::Down(Button::Left) }));
    ui.handle(Event::Mouse(Mouse { x, y, kind: MouseKind::Up(Button::Left) }));
    ui.handle(Event::Mouse(Mouse { x, y, kind: MouseKind::Double(Button::Left) }));
    ui.handle(Event::Mouse(Mouse { x, y, kind: MouseKind::Up(Button::Left) }));
    // A button pressed by a key (which is what a double click amounts to
    // for a default button) is shown down first and delivered after.
    while ui.pick_pending() {
        ui.complete_pick();
    }
}

#[test]
fn on_the_title_bar_it_zooms_and_zooms_back() {
    let mut ui = Ui::new(80, 25);
    let root = ui.root();
    let w = ui.insert(root, Rect::new(10, 5, 40, 10), Kind::Window(Window::new("Doc")));
    double(&mut ui, 20, 5);
    assert_eq!(ui.rect(w), Rect::new(0, 0, 80, 25), "a double click on the title zooms");
    double(&mut ui, 20, 0);
    assert_eq!(ui.rect(w), Rect::new(10, 5, 40, 10), "and zooms back");
    // A window that cannot zoom stays where it is.
    let mut fixed = Window::new("Fixed");
    fixed.zoomable = false;
    let f = ui.insert(root, Rect::new(5, 5, 30, 8), Kind::Window(fixed));
    double(&mut ui, 15, 5);
    assert_eq!(ui.rect(f), Rect::new(5, 5, 30, 8));
}

#[test]
fn on_a_file_name_it_chooses_as_enter_would() {
    let mut ui = Ui::new(80, 25);
    let root = ui.root();
    let w = ui.insert(root, Rect::new(0, 0, 60, 20), Kind::Window(Window::new("Open")));
    let entries = vec![entry("sub", 0, ATTR_DIR), entry("a.txt", 3, 0)];
    let mut f = FileList::new(entries, "*.*");
    f.path_line = false;
    let files = ui.insert(w, Rect::default(), Kind::Files(f));
    ui.set_dock(files, Dock::Fill);
    let ok = ui.insert(w, Rect::default(), Kind::Buttons(ButtonRow::new(vec![PushButton::new("~O~pen", 7).default()])));
    ui.set_dock(ok, Dock::BottomRight(12, 2));
    ui.focus_first();
    // Laid out by a draw: rows and columns exist only after one.
    let mut buf = owlosui_core::Buffer::new(80, 25);
    ui.draw(&mut buf);
    // The names start at row 1 (the frame), one column in.
    let (fx, fy) = (2, 1);
    double(&mut ui, fx + 1, fy + 1); // a.txt, the second row
    let Kind::Files(f) = ui.kind(files) else { panic!() };
    assert_eq!(f.chosen.as_deref(), Some("a.txt"), "the file is chosen");
    assert_eq!(ui.take_pressed(), Some(7), "and Open is pressed, as Enter would press it");
    // A folder is only entered: chosen, and no button.
    if let Kind::Files(f) = ui.kind_mut(files) {
        f.chosen = None;
    }
    double(&mut ui, fx + 1, fy);
    let Kind::Files(f) = ui.kind(files) else { panic!() };
    assert_eq!(f.chosen.as_deref(), Some("sub"));
    assert_eq!(ui.take_pressed(), None, "a folder does not press the button");
}

#[test]
fn on_a_list_item_it_presses_the_default_button() {
    let mut ui = Ui::new(80, 25);
    let root = ui.root();
    let w = ui.insert(root, Rect::new(0, 0, 40, 12), Kind::Window(Window::new("Levels")));
    let list = ui.insert(w, Rect::new(1, 1, 20, 5), Kind::List(ListBox::new(&["one", "two", "three"])));
    ui.set_dock(list, Dock::Manual);
    let play = ui.insert(w, Rect::default(), Kind::Buttons(ButtonRow::new(vec![PushButton::new("~P~lay", 9).default()])));
    ui.set_dock(play, Dock::BottomRight(12, 2));
    ui.focus_first();
    let mut buf = owlosui_core::Buffer::new(80, 25);
    ui.draw(&mut buf);
    double(&mut ui, 3, 3); // "two"
    let Kind::List(l) = ui.kind(list) else { panic!() };
    assert_eq!(l.current, 1, "the click put the cursor on two");
    assert_eq!(ui.take_pressed(), Some(9), "and the double pressed Play");
}
