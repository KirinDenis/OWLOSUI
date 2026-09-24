//! A code review, turned into tests before anything was changed.
//!
//! Each of these reproduces one thing a reader of the source said did not
//! match its own comments. They were run first, to see which claims held;
//! the ones that did were fixed, and these stay so the fixes stay.

use owlosui_core::files::{matches, Focus};
use owlosui_core::cell::{glyphs, Glyph};
use owlosui_core::{
    Buffer, Button, ButtonRow, Cmd, Dock, Event, FileEntry, FileList, HexView, Html, InputLine, Key,
    KeyCode, Kind, MenuBar, MenuItem, Mods, Mouse, MouseKind, PushButton, Rect, TextView, Ui, ViewId,
    Window,
};

fn settle(ui: &mut Ui) {
    let r = ui.rect(ui.root());
    let mut buf = Buffer::new(r.w, r.h);
    ui.draw(&mut buf);
}

fn click(ui: &mut Ui, x: i16, y: i16) {
    ui.handle(Event::Mouse(Mouse { x, y, kind: MouseKind::Down(Button::Left) }));
    ui.handle(Event::Mouse(Mouse { x, y, kind: MouseKind::Up(Button::Left) }));
}

fn key(ui: &mut Ui, code: KeyCode) {
    ui.handle(Event::Key(Key { code, mods: Mods::default() }));
}

// ------------------------------------------------------------ the editor

#[test]
fn a_line_longer_than_i16_does_not_break_the_caret() {
    let mut t = TextView::new(vec![vec![b'x' as Glyph; 40_000]]);
    let mut clip = Vec::new();
    t.exec(Cmd::LineEnd, false, 20, &mut clip);
    assert!(t.cur.x > 0, "the caret went nowhere");
    t.exec(Cmd::CharRight, false, 20, &mut clip);
    t.exec(Cmd::CharLeft, false, 20, &mut clip);
    t.exec(Cmd::WordLeft, false, 20, &mut clip);
}

#[test]
fn more_lines_than_i16_do_not_overflow() {
    let lines: Vec<Vec<Glyph>> = (0..40_000).map(|i| glyphs(&format!("{i}"))).collect();
    let mut t = TextView::new(lines);
    let mut clip = Vec::new();
    t.exec(Cmd::TextEnd, false, 20, &mut clip);
    t.exec(Cmd::PageDown, false, 20, &mut clip);
    t.exec(Cmd::LineDown, false, 20, &mut clip);
    t.exec(Cmd::PageUp, false, 20, &mut clip);
    assert!(t.cur.y > 30_000, "the caret is at {}", t.cur.y);
}

#[test]
fn backspace_in_a_viewer_moves_nothing() {
    let mut t = TextView::new(vec![glyphs("abc")]);
    t.readonly = true;
    let mut clip = Vec::new();
    t.exec(Cmd::LineEnd, false, 20, &mut clip);
    assert_eq!(t.cur.x, 3);
    t.exec(Cmd::DeleteLeft, false, 20, &mut clip);
    assert_eq!(t.cur.x, 3, "Backspace moved the caret in a read-only view");
    t.exec(Cmd::Insert('z'), false, 20, &mut clip);
    t.exec(Cmd::NewLine, false, 20, &mut clip);
    t.exec(Cmd::DeleteLine, false, 20, &mut clip);
    assert_eq!(t.lines, vec![glyphs("abc")]);
    assert_eq!(t.cur.x, 3);
    // Movement still works; it is a viewer, not a wall.
    t.exec(Cmd::LineStart, false, 20, &mut clip);
    assert_eq!(t.cur.x, 0);
}

// ------------------------------------------------------------ windows

#[test]
fn the_boxes_of_an_inactive_window_only_activate_it() {
    let mut ui = Ui::new(80, 25);
    let root = ui.root();
    let a = ui.insert(root, Rect::new(0, 0, 40, 10), Kind::Window(Window::new("A")));
    let b = ui.insert(root, Rect::new(20, 5, 40, 10), Kind::Window(Window::new("B")));
    settle(&mut ui);
    assert_eq!(ui.active_window(), Some(b));

    // A's close box would be at (3,0) if it were active. It is not, so the
    // click activates A and that is all.
    click(&mut ui, 3, 0);
    assert!(ui.is_alive(a), "clicking where an inactive window's [■] would be closed it");
    assert_eq!(ui.active_window(), Some(a));

    // Now it is active and the box is there.
    click(&mut ui, 3, 0);
    assert!(!ui.is_alive(a));

    // Same for the zoom box: (40+20-5+1, 5) is B's [↑] cell, B inactive.
    let c = ui.insert(root, Rect::new(0, 0, 10, 4), Kind::Window(Window::new("C")));
    settle(&mut ui);
    assert_eq!(ui.active_window(), Some(c));
    click(&mut ui, 56, 5);
    assert_eq!(ui.rect(b), Rect::new(20, 5, 40, 10), "an inactive window was zoomed by a click");
    assert_eq!(ui.active_window(), Some(b));
}

#[test]
fn a_fixed_window_does_not_grow_with_the_desktop() {
    let mut ui = Ui::new(80, 25);
    let root = ui.root();
    let mut w = Window::new("Fixed");
    w.resizable = false;
    let id = ui.insert(root, Rect::new(10, 3, 20, 5), Kind::Window(w));
    settle(&mut ui);
    ui.handle(Event::Resize(100, 30));
    settle(&mut ui);
    assert_eq!(ui.rect(id), Rect::new(10, 3, 20, 5));
}

#[test]
fn closing_a_window_closes_what_is_in_it() {
    let mut ui = Ui::new(80, 25);
    let root = ui.root();
    let w = ui.insert(root, Rect::new(0, 0, 40, 10), Kind::Window(Window::new("W")));
    let t = ui.insert(w, Rect::default(), Kind::Text(TextView::new(vec![])));
    let row = ui.insert(w, Rect::default(), Kind::Buttons(ButtonRow::new(vec![PushButton::new("~O~K", 1)])));
    ui.close(w);
    assert!(!ui.is_alive(w));
    assert!(!ui.is_alive(t), "the editor outlived its window");
    assert!(!ui.is_alive(row));
}

// ---------------------------------------------------------- the mouse

fn files_dialog() -> (Ui, ViewId, ViewId) {
    let mut ui = Ui::new(60, 20);
    let root = ui.root();
    let wid = ui.insert(root, Rect::new(0, 0, 60, 20), Kind::Window(Window::new("Open")));
    let entries = ["A.TXT", "B.TXT", "C.TXT"]
        .iter()
        .map(|n| FileEntry { name: n.to_string(), size: 1, date: (2026, 1, 1, 0, 0), attrs: 0x20 })
        .collect();
    let mut list = FileList::new(entries, "*.*");
    list.set_path("C:\\*.*");
    let fid = ui.insert(wid, Rect::default(), Kind::Files(list));
    let bid = ui.insert(wid, Rect::default(), Kind::Buttons(ButtonRow::ok_cancel("~O~pen", 1, 2)));
    ui.set_dock(bid, Dock::BottomRight(24, 2));
    settle(&mut ui);
    (ui, fid, bid)
}

#[test]
fn a_click_on_a_name_selects_it() {
    let (mut ui, fid, _) = files_dialog();
    let panel = ui.abs_rect(fid);
    // Row 0 is the path, row 1 the gap, row 2 the first name, row 3 the second.
    click(&mut ui, panel.x + 3, panel.y + 3);
    let Kind::Files(f) = ui.kind(fid) else { unreachable!() };
    assert_eq!(f.current, 1, "the click did not select the name under it");
    assert_eq!(f.focus, Focus::List);
}

#[test]
fn a_click_on_the_path_focuses_it() {
    let (mut ui, fid, _) = files_dialog();
    let panel = ui.abs_rect(fid);
    // First make sure the focus is on the list.
    click(&mut ui, panel.x + 3, panel.y + 2);
    click(&mut ui, panel.x + 10, panel.y);
    let Kind::Files(f) = ui.kind(fid) else { unreachable!() };
    assert_eq!(f.focus, Focus::Path, "the click on the path line did not focus it");
    assert!(f.path.focused);
}

#[test]
fn a_click_in_a_scrolled_field_lands_on_the_character_under_it() {
    let mut ui = Ui::new(30, 5);
    let root = ui.root();
    let w = ui.insert(root, Rect::new(0, 0, 30, 5), Kind::Window(Window::new("W")));
    let mut i = InputLine::new("Path:", "0123456789ABCDEFGHIJKLMNOP");
    i.set_cursor(26);
    let id = ui.insert(w, Rect::new(0, 0, 20, 1), Kind::Input(i));
    ui.set_dock(id, Dock::Manual);
    settle(&mut ui);
    // Field is 14 wide (20 - "Path: "), caret at 26, so the first character
    // showing is number 13. Column 2 of the field is character 15.
    let r = ui.abs_rect(id);
    click(&mut ui, r.x + 6 + 2, r.y);
    let Kind::Input(i) = ui.kind(id) else { unreachable!() };
    assert_eq!(i.cursor, 15, "the caret went to the visible column, not the character");
}

#[test]
fn the_hex_view_scrolls_with_the_wheel() {
    let mut ui = Ui::new(80, 25);
    let root = ui.root();
    let w = ui.insert(root, Rect::new(0, 0, 80, 20), Kind::Window(Window::new("BIN")));
    let h = ui.insert(w, Rect::default(), Kind::Hex(HexView::new(vec![0u8; 4096])));
    settle(&mut ui);
    ui.handle(Event::Mouse(Mouse { x: 10, y: 10, kind: MouseKind::ScrollDown }));
    let Kind::Hex(hv) = ui.kind(h) else { unreachable!() };
    assert!(hv.top > 0, "the wheel did not scroll the hex view");
}

#[test]
fn the_hex_view_scrolls_from_its_bar() {
    let mut ui = Ui::new(80, 25);
    let root = ui.root();
    let w = ui.insert(root, Rect::new(0, 0, 80, 20), Kind::Window(Window::new("BIN")));
    let h = ui.insert(w, Rect::default(), Kind::Hex(HexView::new(vec![0u8; 4096])));
    settle(&mut ui);
    // The down arrow of the vertical bar: the frame's right column, one
    // above the bottom-right corner.
    click(&mut ui, 79, 18);
    let Kind::Hex(hv) = ui.kind(h) else { unreachable!() };
    assert!(hv.top > 0, "the scroll bar's arrow did nothing");
}

#[test]
fn a_help_page_cannot_be_scrolled_past_its_end() {
    let mut ui = Ui::new(40, 17);
    let root = ui.root();
    let w = ui.insert(root, Rect::new(0, 0, 40, 17), Kind::Window(Window::new("Help")));
    let body: String = (1..=59).map(|i| format!("line {i}\n")).collect();
    let h = ui.insert(w, Rect::default(), Kind::Html(Html::new(&format!("<pre>{body}</pre>"))));
    settle(&mut ui);
    for _ in 0..100 {
        key(&mut ui, KeyCode::Down);
    }
    // Also from the bar's arrow, which is a different door.
    for _ in 0..100 {
        click(&mut ui, 39, 15);
    }
    let Kind::Html(hv) = ui.kind(h) else { unreachable!() };
    let page = 15;
    assert!(hv.top <= 61 - page, "top is {} on a page of {page}", hv.top);
}

// ------------------------------------------------------------- menus

#[test]
fn a_submenu_opens_and_its_items_can_be_chosen() {
    let mut ui = Ui::new(60, 20);
    let root = ui.root();
    let bar = MenuBar::new(vec![MenuItem::sub(
        "~F~ile",
        vec![
            MenuItem::new("~O~pen", "F3", 1),
            MenuItem::sub("~R~ecent", vec![MenuItem::new("~A~", "", 11), MenuItem::new("~B~", "", 12)]),
        ],
    )]);
    ui.insert(root, Rect::new(0, 0, 60, 1), Kind::MenuBar(bar));
    settle(&mut ui);
    ui.open_menu(0);
    key(&mut ui, KeyCode::Down); // Recent
    key(&mut ui, KeyCode::Enter);
    assert_eq!(ui.take_command(), None, "a submenu is not a command");
    let boxes: Vec<ViewId> = ui
        .children(root)
        .iter()
        .copied()
        .filter(|id| matches!(ui.kind(*id), Kind::MenuBox(_)))
        .collect();
    assert_eq!(boxes.len(), 2, "no submenu panel opened");
    let Kind::MenuBox(m) = ui.kind(boxes[1]) else { unreachable!() };
    assert_eq!(m.items[0].label(), "A");

    // Down to B, Enter: the command, and every panel gone.
    key(&mut ui, KeyCode::Down);
    key(&mut ui, KeyCode::Enter);
    assert_eq!(ui.take_command(), Some(12));
    assert!(ui.menu_open().is_none());

    // Left in a submenu closes just the submenu.
    ui.open_menu(0);
    key(&mut ui, KeyCode::Down);
    key(&mut ui, KeyCode::Right);
    let n = ui.children(root).iter().filter(|id| matches!(ui.kind(**id), Kind::MenuBox(_))).count();
    assert_eq!(n, 2);
    key(&mut ui, KeyCode::Left);
    let n = ui.children(root).iter().filter(|id| matches!(ui.kind(**id), Kind::MenuBox(_))).count();
    assert_eq!(n, 1, "Left should close only the submenu");
    key(&mut ui, KeyCode::Esc);
    assert!(ui.menu_open().is_none());
}

// ------------------------------------------------------------- files

#[test]
fn star_dot_star_means_everything() {
    assert!(matches("readme", "*.*"), "a name without a dot is a file too");
    assert!(matches("readme.txt", "*.*"));
    assert!(matches("README", "*.*"));
    assert!(!matches("readme", "*.txt"));
    assert!(matches("a.b.c", "*.*"));
    assert!(matches("readme", "read*"));
    assert!(matches("readme", "*."), "DOS: `*.` is the files with no extension");
    assert!(!matches("readme.txt", "*."));
}

// -------------------------------------------------------------- html

#[test]
fn an_empty_link_is_not_a_link() {
    let mut h = Html::new("<a href=\"x\"></a>text <a href=\"y\">real</a>");
    h.layout(40);
    assert_eq!(h.links.len(), 1, "an empty link stayed in the list");
    assert_eq!(h.links[0].href, "y");
    // And the cell that carries the surviving link still says which it is.
    let cell = h.lines[0].iter().find(|c| c.ch == b'r' as Glyph).expect("the r of real");
    assert_eq!(cell.link, 0);
}

#[test]
fn non_ascii_is_one_glyph_index_not_mojibake() {
    // A char up to U+00FF is a glyph index the backend's code page put
    // there; it goes through as one cell. Beyond that there is no glyph
    // and it becomes `?`. Neither case is the bytes of UTF-8 shown one by
    // one, which is what `cafÃƒÂ©` was.
    let mut h = Html::new("caf\u{e9} ok");
    h.layout(40);
    assert_eq!(h.text(), "caf\u{e9} ok");
    assert_eq!(h.lines[0][3].ch, 0xE9);
    // A Cyrillic word: with 16-bit glyphs its chars are glyph indices like
    // any other and go through; with the DOS build's byte glyphs there is
    // no such index and it becomes `?`.
    let word = "\u{41f}\u{440}\u{438}\u{432}\u{435}\u{442}";
    let mut h = Html::new(word);
    h.layout(40);
    if owlosui_core::cell::GLYPH_MAX >= 0x442 {
        assert_eq!(h.text(), word);
    } else {
        assert_eq!(h.text(), "??????");
    }
}

#[test]
fn closing_a_link_restores_the_style_around_it() {
    use owlosui_core::html::Style;
    let mut h = Html::new("<b><a href=\"x\">go</a> on</b>");
    h.layout(40);
    let line = &h.lines[0];
    let on = line.iter().position(|c| c.ch == b'o' as Glyph && c.link == owlosui_core::html::NO_LINK).expect("the o of on");
    assert_eq!(line[on].style, Style::Bold, "`</a>` threw away the bold around it");
}
