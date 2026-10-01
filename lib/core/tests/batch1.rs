//! The first five of the missing classic pieces: a status line that
//! binds keys, a label that names a control, a tick in a menu item, marks
//! in a list and a file panel, and a bar that fills up.

use owlosui_core::files::Focus;
use owlosui_core::{
    attr, Buffer, Button, Color, Dock, Event, FileEntry, FileList, InputLine, Key, KeyCode, Kind,
    Label, ListBox, MenuBar, MenuItem, Mods, Mouse, MouseKind, Palette, Progress, PushButton,
    ButtonRow, Rect, StatusItem, StatusLine, Ui, ViewId, Window,
};

const CM_HELP: u16 = 1;
const CM_QUIT: u16 = 2;
const CM_OK: u16 = 3;

fn draw(ui: &mut Ui) -> Buffer {
    let r = ui.rect(ui.root());
    let mut buf = Buffer::new(r.w, r.h);
    ui.draw(&mut buf);
    buf
}

fn row(buf: &Buffer, y: i16) -> String {
    (0..buf.width()).map(|x| buf.get(x, y).unwrap().to_char()).collect()
}

/// `n` cells of a row from `x`, as characters - a byte slice of `row` would
/// cut frame glyphs, which are two bytes each as UTF-8, in half.
fn cells(buf: &Buffer, x: i16, y: i16, n: i16) -> String {
    row(buf, y).chars().skip(x as usize).take(n as usize).collect()
}

fn click(ui: &mut Ui, x: i16, y: i16) {
    ui.handle(Event::Mouse(Mouse { x, y, kind: MouseKind::Down(Button::Left) }));
    ui.handle(Event::Mouse(Mouse { x, y, kind: MouseKind::Up(Button::Left) }));
}

fn key(ui: &mut Ui, code: KeyCode, mods: Mods) {
    ui.handle(Event::Key(Key { code, mods }));
}

fn with_status() -> (Ui, ViewId) {
    let mut ui = Ui::new(80, 25);
    let root = ui.root();
    let s = StatusLine::new(vec![
        StatusItem::new("~F1~ Help", Some(Key::new(KeyCode::F(1), Mods::default())), CM_HELP),
        StatusItem::new("~Alt-X~ Exit", Some(Key::new(KeyCode::Char('x'), Mods::alt())), CM_QUIT),
    ]);
    let sid = ui.insert(root, Rect::new(0, 24, 80, 1), Kind::Status(s));
    (ui, sid)
}

// ------------------------------------------------------------- status line

#[test]
fn the_status_line_takes_the_bottom_row_and_looks_the_part() {
    let (mut ui, _) = with_status();
    assert_eq!(ui.work_area(), Rect::new(0, 0, 80, 24), "the work area should stop above it");
    let buf = draw(&mut ui);
    let line = row(&buf, 24);
    assert!(line.starts_with(" F1 Help  Alt-X Exit"), "{line:?}");
    // The key's name in the key colour, the words in the text colour.
    assert_eq!(buf.get(1, 24).unwrap().attr, Palette::classic().status_key);
    assert_eq!(buf.get(4, 24).unwrap().attr, Palette::classic().status);
    assert_eq!(buf.get(60, 24).unwrap().attr, Palette::classic().status);
}

#[test]
fn the_status_line_binds_its_keys() {
    let (mut ui, _) = with_status();
    key(&mut ui, KeyCode::F(1), Mods::default());
    assert_eq!(ui.take_command(), Some(CM_HELP));
    key(&mut ui, KeyCode::Char('x'), Mods::alt());
    assert_eq!(ui.take_command(), Some(CM_QUIT));
    // A key it does not list is not its business.
    key(&mut ui, KeyCode::F(2), Mods::default());
    assert_eq!(ui.take_command(), None);
}

#[test]
fn the_status_line_answers_to_the_mouse() {
    let (mut ui, _) = with_status();
    draw(&mut ui);
    // " F1 Help  Alt-X Exit": "Exit" begins at column 17.
    click(&mut ui, 18, 24);
    assert_eq!(ui.take_command(), Some(CM_QUIT));
    click(&mut ui, 40, 24);
    assert_eq!(ui.take_command(), None, "the empty part of the line is not a button");
}

#[test]
fn the_status_line_keeps_quiet_under_a_modal_dialog() {
    let (mut ui, _) = with_status();
    ui.message_box("Confirm", "Really?", ButtonRow::new(vec![PushButton::new("~O~K", CM_OK).default()]));
    key(&mut ui, KeyCode::F(1), Mods::default());
    assert_eq!(ui.take_command(), None, "F1 reached the status line through a modal box");
}

#[test]
fn the_status_line_follows_a_resize() {
    let (mut ui, sid) = with_status();
    ui.handle(Event::Resize(100, 30));
    draw(&mut ui);
    assert_eq!(ui.rect(sid), Rect::new(0, 29, 100, 1));
    assert_eq!(ui.work_area(), Rect::new(0, 0, 100, 29));
}

// ------------------------------------------------------------------ labels

fn labelled_dialog() -> (Ui, ViewId, ViewId, ViewId) {
    let mut ui = Ui::new(60, 20);
    let root = ui.root();
    let w = ui.insert(root, Rect::new(0, 0, 60, 20), Kind::Window(Window::new("Dialog")));
    let name = ui.insert(w, Rect::new(8, 1, 30, 1), Kind::Input(InputLine::new("", "")));
    ui.set_dock(name, Dock::Manual);
    let list = ui.insert(w, Rect::new(2, 4, 20, 5), Kind::List(ListBox::new(&["one", "two", "three"])));
    ui.set_dock(list, Dock::Manual);
    let label = ui.insert(w, Rect::new(2, 1, 6, 1), Kind::Label(Label::new("~N~ame:", Some(name))));
    ui.set_dock(label, Dock::Manual);
    ui.focus_on(w, list);
    draw(&mut ui);
    (ui, name, list, label)
}

#[test]
fn a_label_is_drawn_with_its_letter_lit() {
    let (mut ui, _, _, label) = labelled_dialog();
    let buf = draw(&mut ui);
    let r = ui.abs_rect(label);
    assert_eq!(cells(&buf, r.x, r.y, 5), "Name:");
    assert_eq!(buf.get(r.x, r.y).unwrap().attr, Palette::classic().label_key);
    assert_eq!(buf.get(r.x + 1, r.y).unwrap().attr, Palette::classic().label);
}

#[test]
fn alt_and_the_letter_focus_the_labelled_control() {
    let (mut ui, name, list, label) = labelled_dialog();
    assert_eq!(ui.focused(), Some(list));
    key(&mut ui, KeyCode::Char('n'), Mods::alt());
    assert_eq!(ui.focused(), Some(name), "Alt+N did not move the focus to the field");
    // And the label lights up now that its control is current.
    let buf = draw(&mut ui);
    let r = ui.abs_rect(label);
    assert_eq!(buf.get(r.x + 1, r.y).unwrap().attr, Palette::classic().label_active);
}

#[test]
fn a_click_on_a_label_focuses_its_control() {
    let (mut ui, name, list, label) = labelled_dialog();
    assert_eq!(ui.focused(), Some(list));
    let r = ui.abs_rect(label);
    click(&mut ui, r.x + 2, r.y);
    assert_eq!(ui.focused(), Some(name));
}

// ------------------------------------------------------------------- menus

#[test]
fn a_checked_item_wears_a_tick() {
    let mut ui = Ui::new(60, 20);
    let root = ui.root();
    let bar = MenuBar::new(vec![MenuItem::sub(
        "~O~ptions",
        vec![
            MenuItem::new("~A~uto indent", "", 5).checked(true),
            MenuItem::new("~W~ord wrap", "", 6),
        ],
    )]);
    ui.insert(root, Rect::new(0, 0, 60, 1), Kind::MenuBar(bar));
    ui.open_menu(0);
    let buf = draw(&mut ui);
    // The panel's first item is on row 2; the tick sits in the column
    // before the label, which is the first column inside the frame.
    let ticks: Vec<(i16, i16)> = (0..20)
        .flat_map(|y| (0..60).map(move |x| (x, y)))
        .filter(|&(x, y)| buf.get(x, y).unwrap().ch == 0xFB)
        .collect();
    assert_eq!(ticks.len(), 1, "one tick, on the checked item");
    assert_eq!(ticks[0].1, 2);
    let line = row(&buf, 2);
    assert!(line.contains("Auto indent"));
    assert!(!row(&buf, 3).contains('\u{FB}'));
}

// ------------------------------------------------------------------- marks

#[test]
fn insert_marks_a_run_of_items() {
    let mut ui = Ui::new(40, 12);
    let root = ui.root();
    let w = ui.insert(root, Rect::new(0, 0, 40, 12), Kind::Window(Window::new("Pick")));
    let mut l = ListBox::new(&["a", "b", "c", "d"]);
    l.multi = true;
    let list = ui.insert(w, Rect::new(1, 1, 20, 6), Kind::List(l));
    ui.set_dock(list, Dock::Manual);
    ui.focus_on(w, list);
    draw(&mut ui);

    key(&mut ui, KeyCode::Insert, Mods::default());
    key(&mut ui, KeyCode::Insert, Mods::default());
    let Kind::List(l) = ui.kind(list) else { unreachable!() };
    assert_eq!(l.marked(), vec![0, 1]);
    assert_eq!(l.current, 2, "Insert should step down after marking");

    // Drawn yellow, and the one under the cursor stays yellow.
    let buf = draw(&mut ui);
    let r = ui.abs_rect(list);
    assert_eq!(buf.get(r.x, r.y).unwrap().attr, Palette::classic().list_marked);
    assert_ne!(buf.get(r.x, r.y + 2).unwrap().attr, Palette::classic().list_marked);
    key(&mut ui, KeyCode::Up, Mods::default());
    let buf = draw(&mut ui);
    let cur = buf.get(r.x, r.y + 1).unwrap().attr;
    assert_eq!(cur & 0x0F, Color::Yellow as u8, "a marked item under the cursor lost its mark colour");

    // Insert again on b unmarks it.
    key(&mut ui, KeyCode::Insert, Mods::default());
    let Kind::List(l) = ui.kind(list) else { unreachable!() };
    assert_eq!(l.marked(), vec![0]);
}

#[test]
fn a_single_choice_list_ignores_insert() {
    let mut l = ListBox::new(&["a", "b"]);
    l.toggle_mark();
    assert!(l.marked().is_empty());
    assert_eq!(l.current, 0);
}

#[test]
fn insert_marks_files_but_not_folders() {
    let mut ui = Ui::new(60, 20);
    let root = ui.root();
    let w = ui.insert(root, Rect::new(0, 0, 60, 20), Kind::Window(Window::new("Open")));
    let entries = vec![
        FileEntry { name: "DOCS".into(), size: 0, date: (2026, 1, 1, 0, 0), attrs: 0x10 },
        FileEntry { name: "A.TXT".into(), size: 1, date: (2026, 1, 1, 0, 0), attrs: 0x20 },
        FileEntry { name: "B.TXT".into(), size: 1, date: (2026, 1, 1, 0, 0), attrs: 0x20 },
    ];
    let mut list = FileList::new(entries, "*.*");
    list.set_path("C:\\*.*");
    list.multi = true;
    list.focus = Focus::List;
    let fid = ui.insert(w, Rect::default(), Kind::Files(list));
    draw(&mut ui);

    // The folder comes first and is skipped over rather than marked.
    key(&mut ui, KeyCode::Insert, Mods::default());
    key(&mut ui, KeyCode::Insert, Mods::default());
    let Kind::Files(f) = ui.kind(fid) else { unreachable!() };
    assert_eq!(f.marked_names(), vec!["A.TXT".to_string()]);
    assert_eq!(f.current, 2);

    // A new mask keeps the mark: it belongs to the file, not to the row.
    let Kind::Files(f) = ui.kind_mut(fid) else { unreachable!() };
    f.set_mask("*.TXT");
    assert_eq!(f.marked_names(), vec!["A.TXT".to_string()]);
}

// ---------------------------------------------------------- two panels

fn panel(ui: &mut Ui, r: Rect) -> (ViewId, ViewId) {
    let root = ui.root();
    let w = ui.insert(root, r, Kind::Window(Window::new("P")));
    let entries = vec![
        FileEntry { name: "A.TXT".into(), size: 1, date: (2026, 1, 1, 0, 0), attrs: 0x20 },
        FileEntry { name: "B.TXT".into(), size: 1, date: (2026, 1, 1, 0, 0), attrs: 0x20 },
    ];
    let mut list = FileList::new(entries, "*.*");
    list.set_path("C:\\*.*");
    list.focus = Focus::List;
    let f = ui.insert(w, Rect::default(), Kind::Files(list));
    (w, f)
}

fn cells_with(buf: &Buffer, r: Rect, a: u8) -> usize {
    (r.y..r.bottom())
        .flat_map(|y| (r.x..r.right()).map(move |x| (x, y)))
        .filter(|&(x, y)| buf.get(x, y).map_or(false, |c| c.attr == a))
        .count()
}

#[test]
fn only_the_active_panel_shows_a_cursor_bar() {
    let mut ui = Ui::new(80, 25);
    let (lw, _) = panel(&mut ui, Rect::new(0, 0, 40, 24));
    let (rw, _) = panel(&mut ui, Rect::new(40, 0, 40, 24));
    let buf = draw(&mut ui);
    let p = Palette::classic();
    // The name rows only: the path line above them is yellow on blue as
    // well, and a count over the whole window would see it.
    let left = Rect::new(1, 3, 38, 8);
    let right = Rect::new(41, 3, 38, 8);
    // The right window is the newest, so the active one.
    assert_eq!(ui.active_window(), Some(rw));
    assert!(cells_with(&buf, right, p.file_selected) > 0, "no cursor bar in the active panel");
    assert_eq!(cells_with(&buf, left, p.file_selected), 0, "a cursor bar in the inactive panel");
    assert_eq!(cells_with(&buf, left, p.file_selected_passive), 0, "a dimmed bar in the inactive panel");

    ui.activate(lw);
    let buf = draw(&mut ui);
    assert!(cells_with(&buf, left, p.file_selected) > 0);
    assert_eq!(cells_with(&buf, right, p.file_selected), 0);
}

#[test]
fn the_foot_of_a_panel_takes_the_window_colour() {
    // On a blue document window the details are white on blue; on a grey
    // dialog they stay the measured black on grey.
    let mut ui = Ui::new(80, 25);
    let (_, f) = panel(&mut ui, Rect::new(0, 0, 40, 24));
    let buf = draw(&mut ui);
    let r = ui.abs_rect(f);
    let foot = buf.get(r.x + 1, r.bottom() - 1).unwrap().attr;
    assert_eq!(foot, attr(Color::White, Color::Blue), "{foot:02X}");

    let mut ui = Ui::new(80, 25);
    let root = ui.root();
    let mut win = Window::new("Open");
    win.palette = owlosui_core::WinPalette::Gray;
    let w = ui.insert(root, Rect::new(0, 0, 40, 24), Kind::Window(win));
    let mut list = FileList::new(vec![], "*.*");
    list.set_path("C:\\*.*");
    let f = ui.insert(w, Rect::default(), Kind::Files(list));
    let buf = draw(&mut ui);
    let r = ui.abs_rect(f);
    assert_eq!(buf.get(r.x + 1, r.bottom() - 1).unwrap().attr, Palette::classic().file_info);
}

#[test]
fn a_panel_without_a_path_line_starts_with_the_names() {
    let mut ui = Ui::new(80, 25);
    let root = ui.root();
    let w = ui.insert(root, Rect::new(0, 0, 40, 24), Kind::Window(Window::new("P")));
    let entries = vec![
        FileEntry { name: "A.TXT".into(), size: 1, date: (2026, 1, 1, 0, 0), attrs: 0x20 },
        FileEntry { name: "B.TXT".into(), size: 1, date: (2026, 1, 1, 0, 0), attrs: 0x20 },
    ];
    let mut list = FileList::new(entries, "*.*");
    list.set_path("C:\\*.*");
    list.focus = Focus::List;
    list.path_line = false;
    let f = ui.insert(w, Rect::default(), Kind::Files(list));
    let buf = draw(&mut ui);
    let r = ui.abs_rect(f);

    // No `*.*` above the names; the first name is on the panel's first row.
    assert!(!row(&buf, r.y).contains("*.*"), "{:?}", row(&buf, r.y));
    assert!(row(&buf, r.y).contains("A.TXT"), "{:?}", row(&buf, r.y));
    assert!(row(&buf, r.y + 1).contains("B.TXT"));
    // The foot still says where you are.
    assert!(row(&buf, r.bottom() - 2).contains("C:\\"), "{:?}", row(&buf, r.bottom() - 2));

    // A click on the second row selects the second name, and Tab has no
    // path line to go to.
    click(&mut ui, r.x + 3, r.y + 1);
    let Kind::Files(fl) = ui.kind(f) else { unreachable!() };
    assert_eq!(fl.current, 1);
    key(&mut ui, KeyCode::Tab, Mods::default());
    let Kind::Files(fl) = ui.kind(f) else { unreachable!() };
    assert_eq!(fl.focus, Focus::List, "Tab reached a path line that is not there");
}

// ------------------------------------------------------------------ canvas

#[test]
fn a_canvas_is_clear_until_drawn_on_and_words_take_the_window_colour() {
    use owlosui_core::{Canvas, Cell, StaticText};
    let mut ui = Ui::new(40, 10);
    let root = ui.root();
    // A blue document window: body 0x1F, text 0x1E.
    let w = ui.insert(root, Rect::new(0, 0, 40, 10), Kind::Window(Window::new("W")));
    let canvas = ui.insert(w, Rect::new(1, 1, 10, 3), Kind::Canvas(Canvas::new(10, 3)));
    ui.set_dock(canvas, Dock::Manual);
    let words = ui.insert(w, Rect::new(1, 5, 20, 1), Kind::Static(StaticText::new("moves 0")));
    ui.set_dock(words, Dock::Manual);

    let buf = draw(&mut ui);
    let body = Palette::classic().blue.body;
    let r = ui.abs_rect(canvas);
    assert_eq!(buf.get(r.x, r.y).unwrap().attr, body, "a new canvas should show the window through");
    let wr = ui.abs_rect(words);
    assert_eq!(buf.get(wr.x, wr.y).unwrap().attr, Palette::classic().blue.text, "words on a blue window are yellow on blue, not a grey bar");

    // Draw two cells, one of them clear again: the clear one stays window.
    if let Kind::Canvas(c) = ui.kind_mut(canvas) {
        c.blit(2, 1, 2, 1, &[Cell::new(b'#' as owlosui_core::Glyph, 0x4E), Canvas::CLEAR]);
    }
    let buf = draw(&mut ui);
    assert_eq!(buf.get(r.x + 2, r.y + 1).unwrap().attr, 0x4E);
    assert_eq!(buf.get(r.x + 3, r.y + 1).unwrap().attr, body);
}

// ---------------------------------------------------------------- progress

#[test]
fn a_progress_bar_fills_in_proportion() {
    let mut p = Progress::new(200);
    p.set(50);
    assert_eq!(p.filled(20), 5);
    assert_eq!(p.percent_text(), "25%");
    p.set(1000);
    assert_eq!(p.value, 200, "clamped to max");
    assert_eq!(p.filled(20), 20);
}

#[test]
fn a_progress_bar_is_drawn_in_blocks() {
    let mut ui = Ui::new(40, 6);
    let root = ui.root();
    let mut win = Window::new("Copy");
    win.palette = owlosui_core::WinPalette::Gray;
    let w = ui.insert(root, Rect::new(0, 0, 40, 6), Kind::Window(win));
    let mut p = Progress::new(100);
    p.set(50);
    let id = ui.insert(w, Rect::new(1, 1, 25, 1), Kind::Progress(p));
    ui.set_dock(id, Dock::Manual);
    let buf = draw(&mut ui);
    let r = ui.abs_rect(id);
    let line = cells(&buf, r.x, r.y, r.w);
    // 25 cells less " 50%" (5 with its gap) leaves a 20-cell bar, half full.
    let bar: String = line.chars().take(20).collect();
    assert_eq!(bar, "\u{DB}".repeat(10) + &"\u{B0}".repeat(10), "{line:?}");
    assert!(line.ends_with(" 50%"), "{line:?}");
    assert_eq!(buf.get(r.x, r.y).unwrap().attr, attr(Color::Blue, Color::LightGray));
}
