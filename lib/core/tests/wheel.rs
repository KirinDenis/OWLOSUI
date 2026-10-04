//! The mouse wheel and the scroll bar of lists and trees: what a person
//! reaches for first in any list, before reading a word of it. The wheel
//! and the bar move the view; the cursor stays on its item until a key or
//! a click moves it.

use owlosui_core::{Buffer, Button, Dock, Event, FileEntry, FileList, Key, KeyCode, Kind, ListBox, Mods, Mouse, MouseKind, Rect, TreeNode, TreeView, Ui, ViewId, Window};

fn frame(ui: &mut Ui) {
    let mut b = Buffer::new(80, 25);
    ui.draw(&mut b);
}

fn mouse(ui: &mut Ui, x: i16, y: i16, kind: MouseKind) {
    ui.handle(Event::Mouse(Mouse { x, y, kind }));
    frame(ui);
}

fn list_of(ui: &Ui, id: ViewId) -> (i16, usize) {
    match ui.kind(id) {
        Kind::List(l) => (l.top, l.current),
        _ => unreachable!(),
    }
}

/// A dialog at (0,0) 40x12 with a list of 20 at (1,1) 20x5 inside: the
/// list's cells are (2,2)..(21,6), its bar in column 21.
fn dialog_with_list(ui: &mut Ui) -> ViewId {
    let root = ui.root();
    let w = ui.insert(root, Rect::new(0, 0, 40, 12), Kind::Window(Window::new("Pick")));
    let items: Vec<String> = (0..20).map(|i| format!("item {i}")).collect();
    let refs: Vec<&str> = items.iter().map(|s| s.as_str()).collect();
    let l = ui.insert(w, Rect::new(1, 1, 20, 5), Kind::List(ListBox::new(&refs)));
    ui.set_dock(l, Dock::Manual);
    ui.activate(w);
    ui.focus_on(w, l);
    frame(ui);
    l
}

#[test]
fn the_wheel_scrolls_a_list_in_a_dialog_and_the_cursor_stays() {
    let mut ui = Ui::new(80, 25);
    let l = dialog_with_list(&mut ui);
    mouse(&mut ui, 5, 3, MouseKind::ScrollDown);
    assert_eq!(list_of(&ui, l), (3, 0), "the wheel did not move the view, or moved the cursor");
    // Drawn again and again, the view stays where the wheel put it.
    frame(&mut ui);
    assert_eq!(list_of(&ui, l).0, 3, "the view went back to the cursor by itself");
    mouse(&mut ui, 5, 3, MouseKind::ScrollUp);
    assert_eq!(list_of(&ui, l).0, 0);
    // Scrolled away, a key moves the cursor and the view comes to it.
    mouse(&mut ui, 5, 3, MouseKind::ScrollDown);
    mouse(&mut ui, 5, 3, MouseKind::ScrollDown);
    ui.handle(Event::Key(Key { code: KeyCode::Down, mods: Mods::default() }));
    frame(&mut ui);
    assert_eq!(list_of(&ui, l), (1, 1), "after Down the cursor is not in view");
}

#[test]
fn the_bar_of_a_list_answers_its_arrows_its_track_and_its_marker() {
    let mut ui = Ui::new(80, 25);
    let l = dialog_with_list(&mut ui);
    let click = |ui: &mut Ui, x: i16, y: i16| {
        mouse(ui, x, y, MouseKind::Down(Button::Left));
        mouse(ui, x, y, MouseKind::Up(Button::Left));
    };
    click(&mut ui, 21, 6);
    assert_eq!(list_of(&ui, l), (1, 0), "the down arrow did not scroll a row - or a click there chose a row");
    click(&mut ui, 21, 2);
    assert_eq!(list_of(&ui, l).0, 0, "the up arrow did not scroll back");
    // The track below the marker: a page (the rows less one).
    click(&mut ui, 21, 5);
    assert_eq!(list_of(&ui, l).0, 4, "the track did not page down");
    // The marker, taken hold of and dragged to the bottom: the end.
    let thumb = 3 + (4 * 2) / 15; // 1 + top*(track-1)/span, track 3, span 15
    mouse(&mut ui, 21, thumb, MouseKind::Down(Button::Left));
    mouse(&mut ui, 21, 5, MouseKind::Drag);
    mouse(&mut ui, 21, 5, MouseKind::Up(Button::Left));
    assert_eq!(list_of(&ui, l), (15, 0), "dragging the marker to the bottom did not reach the end");
}

#[test]
fn the_wheel_scrolls_a_tree_and_moves_a_file_panels_cursor() {
    let mut ui = Ui::new(80, 25);
    let root = ui.root();
    let w = ui.insert(root, Rect::new(0, 0, 30, 8), Kind::Window(Window::new("Tree")));
    let kids: Vec<TreeNode> = (0..20).map(|i| TreeNode::leaf(&format!("node {i}"))).collect();
    let mut top = TreeNode::leaf("C:");
    top.children = kids;
    top.open = true;
    let t = ui.insert(w, Rect::default(), Kind::Tree(TreeView::new(vec![top])));
    ui.activate(w);
    frame(&mut ui);
    mouse(&mut ui, 5, 3, MouseKind::ScrollDown);
    match ui.kind(t) {
        Kind::Tree(tv) => assert_eq!((tv.top, tv.current), (3, 0), "the tree did not scroll"),
        _ => unreachable!(),
    }

    let f = ui.insert(root, Rect::new(40, 0, 40, 12), Kind::Window(Window::new("Files")));
    let entries: Vec<FileEntry> = (0..30)
        .map(|i| FileEntry { name: format!("F{i:02}.TXT"), size: 1, date: (2026, 1, 1, 0, 0), attrs: 0 })
        .collect();
    let panel = ui.insert(f, Rect::default(), Kind::Files(FileList::new(entries, "*.*")));
    ui.activate(f);
    frame(&mut ui);
    let before = match ui.kind(panel) {
        Kind::Files(fl) => fl.current,
        _ => unreachable!(),
    };
    mouse(&mut ui, 50, 5, MouseKind::ScrollDown);
    match ui.kind(panel) {
        Kind::Files(fl) => assert_eq!(fl.current, before + 3, "the wheel did not move the panel's cursor"),
        _ => unreachable!(),
    }
}

#[test]
fn the_wheel_moves_the_cursor_of_a_list_that_shows_everything() {
    // The pilot's "What to see": eight choices in a list eight rows high.
    // Nothing to scroll, and a wheel that did nothing looked broken.
    let mut ui = Ui::new(80, 25);
    let root = ui.root();
    let w = ui.insert(root, Rect::new(0, 0, 40, 12), Kind::Window(Window::new("Pick")));
    let l = ui.insert(w, Rect::new(1, 1, 20, 5), Kind::List(ListBox::new(&["one", "two", "three"])));
    ui.set_dock(l, Dock::Manual);
    ui.activate(w);
    frame(&mut ui);
    mouse(&mut ui, 5, 3, MouseKind::ScrollDown);
    mouse(&mut ui, 5, 3, MouseKind::ScrollDown);
    assert_eq!(list_of(&ui, l), (0, 2), "the wheel did not move the cursor");
    mouse(&mut ui, 5, 3, MouseKind::ScrollUp);
    assert_eq!(list_of(&ui, l).1, 1);
}

#[test]
fn words_longer_than_their_box_scroll_with_the_wheel_and_say_so() {
    let mut ui = Ui::new(80, 25);
    let root = ui.root();
    let w = ui.insert(root, Rect::new(0, 0, 30, 8), Kind::Window(Window::new("About")));
    let s = ui.insert(w, Rect::new(1, 1, 20, 2), Kind::Static(owlosui_core::StaticText::new("one two three four five six seven eight nine ten eleven twelve")));
    ui.set_dock(s, Dock::Manual);
    ui.activate(w);
    let mut b = Buffer::new(80, 25);
    ui.draw(&mut b);
    // Its cells are (2,2)..(21,3): the last column says there is more below.
    assert_eq!(b.get(21, 3).unwrap().to_char(), '\u{1F}', "no arrow saying there is more");
    let first = |b: &Buffer| (2..21).map(|x| b.get(x, 2).unwrap().to_char()).collect::<String>();
    let before = first(&b);
    mouse(&mut ui, 5, 2, MouseKind::ScrollDown);
    let mut b = Buffer::new(80, 25);
    ui.draw(&mut b);
    assert_ne!(first(&b), before, "the wheel did not scroll the words");
    assert_eq!(b.get(21, 2).unwrap().to_char(), '\u{1E}', "no arrow saying there is more above");
}

#[test]
fn folded_text_gets_a_bar_that_counts_rows() {
    // One long line, folded into many rows in a small window: counted by
    // lines it was one line, and there was no bar at all.
    let mut ui = Ui::new(80, 25);
    let root = ui.root();
    let w = ui.insert(root, Rect::new(0, 0, 20, 8), Kind::Window(Window::new("Notes")));
    let line = owlosui_core::cell::glyphs(&"word ".repeat(40));
    let t = ui.insert(w, Rect::default(), Kind::Text(owlosui_core::TextView::new(vec![line])));
    ui.activate(w);
    ui.set_editor(t, 0, owlosui_core::edit::state::WRAP);
    let mut b = Buffer::new(80, 25);
    ui.draw(&mut b);
    // Inside 18 by 6; the bar in column 19, its arrows at rows 1 and 6.
    assert_eq!(b.get(19, 1).unwrap().to_char(), '\u{1E}', "no bar on folded text");
    assert_eq!(b.get(19, 6).unwrap().to_char(), '\u{1F}');
    // Its down arrow scrolls a row.
    mouse(&mut ui, 19, 6, MouseKind::Down(Button::Left));
    mouse(&mut ui, 19, 6, MouseKind::Up(Button::Left));
    match ui.kind(t) {
        Kind::Text(t) => assert_eq!((t.top, t.top_row), (0, 1), "the arrow did not scroll one row"),
        _ => unreachable!(),
    }
}

#[test]
fn the_wheel_walks_an_open_menu_as_the_arrows_do() {
    use owlosui_core::{MenuBar, MenuItem};
    let mut ui = Ui::new(80, 25);
    let root = ui.root();
    ui.insert(
        root,
        Rect::new(0, 0, 80, 1),
        Kind::MenuBar(MenuBar::new(vec![MenuItem::sub(
            "~F~ile",
            vec![MenuItem::new("~N~ew", "", 1), MenuItem::new("~O~pen", "", 2), MenuItem::line(), MenuItem::new("E~x~it", "", 3)],
        )])),
    );
    let w = ui.insert(root, Rect::new(10, 5, 40, 10), Kind::Window(Window::new("Behind")));
    ui.activate(w);
    frame(&mut ui);
    ui.handle(Event::Key(Key { code: KeyCode::Char('f'), mods: Mods { alt: true, ..Mods::default() } }));
    frame(&mut ui);
    let current = |ui: &Ui| match ui.menu_open().map(|m| ui.kind(m)) {
        Some(Kind::MenuBox(m)) => m.current,
        _ => usize::MAX,
    };
    assert_eq!(current(&ui), 0);
    mouse(&mut ui, 30, 10, MouseKind::ScrollDown);
    assert_eq!(current(&ui), 1, "the wheel did not move to Open");
    // Over the line, to Exit, as Down does; and back up.
    mouse(&mut ui, 30, 10, MouseKind::ScrollDown);
    assert_eq!(current(&ui), 3, "the wheel did not step over the line");
    mouse(&mut ui, 30, 10, MouseKind::ScrollUp);
    assert_eq!(current(&ui), 1);
}
