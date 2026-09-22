//! Menus: what they look like and what they do when pressed.
//!
//! The picture in `panel_layout` is the one Turbo Vision draws, measured off
//! video memory rather than copied from a screenshot — label at the left,
//! shortcut against the right edge, the widest item setting the width for all
//! of them, and a separator that meets the frame with tee pieces.

use owlosui_core::{Button, Event, Kind, MenuBar, MenuBox, MenuItem, Mouse, MouseKind, Rect, Ui};
use owlosui_core::{Key, KeyCode, Mods};

const CM_OPEN: u16 = 10;
const CM_SAVE: u16 = 11;
const CM_EXIT: u16 = 12;

fn file_menu() -> Vec<MenuItem> {
    vec![
        MenuItem::new("~O~pen", "F3", CM_OPEN),
        MenuItem::new("~S~ave", "F2", CM_SAVE),
        MenuItem::line(),
        MenuItem::new("~P~rint", "", 13).disabled(),
        MenuItem::new("E~x~it", "Alt+X", CM_EXIT),
    ]
}

fn app() -> Ui {
    let mut ui = Ui::new(60, 20);
    let root = ui.root();
    let bar = MenuBar::new(vec![
        MenuItem::sub("~F~ile", file_menu()),
        MenuItem::sub("~E~dit", vec![MenuItem::new("~C~opy", "Ctrl+Ins", 20)]),
    ]);
    ui.insert(root, Rect::new(0, 0, 60, 1), Kind::MenuBar(bar));
    ui
}

fn picture(ui: &mut Ui, r: Rect) -> String {
    let mut buf = owlosui_core::Buffer::new(60, 20);
    ui.draw(&mut buf);
    let mut s = String::new();
    for y in r.y..r.bottom() {
        for x in r.x..r.right() {
            let c = buf.get(x, y).unwrap().ch;
            s.push(match c {
                0xB0 => '.',
                0xC4 => '-',
                0xB3 => '|',
                0xDA => '+',
                0xBF => '+',
                0xC0 => '+',
                0xD9 => '+',
                0xC3 => '[',
                0xB4 => ']',
                b if b.is_ascii_graphic() || b == b' ' => b as char,
                _ => '?',
            });
        }
        s.push('\n');
    }
    s
}

fn key(ui: &mut Ui, code: KeyCode, mods: Mods) {
    ui.handle(Event::Key(Key::new(code, mods)));
}

fn click(ui: &mut Ui, x: i16, y: i16) {
    ui.handle(Event::Mouse(Mouse {
        x,
        y,
        kind: MouseKind::Down(Button::Left),
    }));
}

#[test]
fn the_bar_is_spaced_the_way_the_original_was() {
    let mut ui = app();
    // Two spaces in front of the first label, two between each pair. Measured.
    assert_eq!(picture(&mut ui, Rect::new(0, 0, 18, 1)), "  File  Edit      \n");
}

#[test]
fn panel_layout() {
    let mut ui = app();
    ui.open_menu(0);

    // Label at the left, shortcut against the right edge, and the widest item
    // — `Exit  Alt+X` — setting the width for every other line.
    assert_eq!(
        picture(&mut ui, Rect::new(0, 1, 17, 7)),
        concat!(
            " +-------------+ \n",
            " | Open     F3 | \n",
            " | Save     F2 | \n",
            " [-------------] \n",
            " | Print       | \n",
            " | Exit  Alt+X | \n",
            " +-------------+ \n",
        )
    );
}

#[test]
fn walking_the_panel_skips_what_cannot_be_chosen() {
    let mut ui = app();
    ui.open_menu(0);

    let cur = |ui: &Ui| match ui.kind(ui.children(ui.root())[1]) {
        Kind::MenuBox(m) => m.current,
        _ => panic!("no panel"),
    };
    assert_eq!(cur(&ui), 0);

    key(&mut ui, KeyCode::Down, Mods::NONE);
    assert_eq!(cur(&ui), 1);

    // The separator and the disabled Print are both stepped over.
    key(&mut ui, KeyCode::Down, Mods::NONE);
    assert_eq!(cur(&ui), 4, "should have skipped the line and the disabled item");

    // And it wraps.
    key(&mut ui, KeyCode::Down, Mods::NONE);
    assert_eq!(cur(&ui), 0);
}

#[test]
fn choosing_an_item_hands_out_a_command_and_closes() {
    let mut ui = app();
    ui.open_menu(0);
    key(&mut ui, KeyCode::Down, Mods::NONE); // Save
    key(&mut ui, KeyCode::Enter, Mods::NONE);

    assert_eq!(ui.take_command(), Some(CM_SAVE));
    assert_eq!(ui.take_command(), None, "a command is collected once");
    assert_eq!(ui.children(ui.root()).len(), 1, "the panel should be gone");
}

#[test]
fn hotkeys() {
    // Alt with a letter opens from the bar.
    let mut ui = app();
    key(&mut ui, KeyCode::Char('f'), Mods::alt());
    assert_eq!(ui.children(ui.root()).len(), 2);

    // A bare letter inside the panel picks an item outright.
    key(&mut ui, KeyCode::Char('x'), Mods::NONE);
    assert_eq!(ui.take_command(), Some(CM_EXIT));
}

#[test]
fn escape_closes_and_chooses_nothing() {
    let mut ui = app();
    ui.open_menu(0);
    key(&mut ui, KeyCode::Esc, Mods::NONE);
    assert_eq!(ui.take_command(), None);
    assert_eq!(ui.children(ui.root()).len(), 1);
}

/// A click outside an open panel puts it away and does nothing else. One click
/// that both dismissed a menu and acted on what was behind it would be one
/// press doing two things the user only asked for once.
#[test]
fn clicking_away_only_closes() {
    let mut ui = app();
    click(&mut ui, 3, 0); // on "File"
    assert_eq!(ui.children(ui.root()).len(), 2, "the panel should be open");

    click(&mut ui, 40, 15); // far away
    assert_eq!(ui.children(ui.root()).len(), 1);
    assert_eq!(ui.take_command(), None);
}

#[test]
fn a_panel_can_stand_on_its_own() {
    // No bar at all: this is what a local menu is, and it must not need one.
    let mut ui = Ui::new(60, 20);
    let root = ui.root();
    let b = MenuBox::new(file_menu());
    let (w, h) = (b.width(), b.height());
    ui.insert(root, Rect::new(4, 6, w, h), Kind::MenuBox(b));

    key(&mut ui, KeyCode::Down, Mods::NONE);
    key(&mut ui, KeyCode::Enter, Mods::NONE);
    assert_eq!(ui.take_command(), Some(CM_SAVE));

    // Left and Right have no bar to walk and must not reach for one.
    let mut ui = Ui::new(60, 20);
    let root = ui.root();
    let b = MenuBox::new(file_menu());
    let (w, h) = (b.width(), b.height());
    ui.insert(root, Rect::new(4, 6, w, h), Kind::MenuBox(b));
    key(&mut ui, KeyCode::Left, Mods::NONE);
    assert_eq!(ui.children(ui.root()).len(), 1, "still there, unchanged");
}
