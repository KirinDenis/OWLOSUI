//! Rows of buttons placed by hand: a keypad.
//!
//! One docked row at the bottom right is a dialog's own buttons - Enter,
//! Escape, the default - and the toolkit used to know about that row alone.
//! A calculator has six rows of them, none docked, and the display above
//! must keep the caret while they are clicked.

use owlosui_core::{
    attr, Align, Buffer, Button, ButtonRow, ButtonStyle, Color, Dock, Event, InputLine, Key, KeyCode, Kind, Mods,
    Mouse, MouseKind, PushButton, Rect, Ui, Window,
};

const CM_SEVEN: u16 = 7;
const CM_PLUS: u16 = 20;
const CM_EQ: u16 = 21;
const CM_CLEAR: u16 = 22;
const CM_OK: u16 = 30;

struct Pad {
    ui: Ui,
    row1: owlosui_core::ViewId,
    row2: owlosui_core::ViewId,
    input: owlosui_core::ViewId,
}

/// A window with a display, two placed rows and one docked row.
fn pad() -> Pad {
    pad_with(false)
}

/// `cancel`: C is the Escape button.
fn pad_with(cancel: bool) -> Pad {
    let mut ui = Ui::new(60, 20);
    let root = ui.root();
    let w = ui.insert(root, Rect::new(0, 0, 50, 14), Kind::Window(Window::new("Calc")));
    let input = ui.insert(w, Rect::new(1, 0, 30, 1), Kind::Input(InputLine::new("", "")));
    let mut c_key = PushButton::new(" C ", CM_CLEAR).style(ButtonStyle::Danger);
    if cancel {
        c_key = c_key.cancel();
    }
    let mut r1 = ButtonRow::new(vec![
        c_key,
        PushButton::new(" 7 ", CM_SEVEN),
        PushButton::new(" A ", 8).disabled(),
    ]);
    r1.align = Align::Left;
    r1.selectable = false;
    let row1 = ui.insert(w, Rect::new(1, 2, r1.width(), 2), Kind::Buttons(r1));
    ui.set_dock(row1, Dock::Manual);
    let mut r2 = ButtonRow::new(vec![
        PushButton::new(" + ", CM_PLUS).style(ButtonStyle::Accent),
        PushButton::new(" = ", CM_EQ).default(),
    ]);
    r2.align = Align::Left;
    r2.selectable = false;
    let row2 = ui.insert(w, Rect::new(1, 4, r2.width(), 2), Kind::Buttons(r2));
    ui.set_dock(row2, Dock::Manual);
    let ok = ui.insert(w, Rect::default(), Kind::Buttons(ButtonRow::new(vec![PushButton::new("~O~K", CM_OK)])));
    ui.set_dock(ok, Dock::BottomRight(12, 2));
    ui.focus_first();
    Pad { ui, row1, row2, input }
}

fn draw(ui: &mut Ui) -> Buffer {
    let mut buf = Buffer::new(60, 20);
    ui.draw(&mut buf);
    buf
}

/// Where a label starts on screen.
fn find(buf: &Buffer, s: &str) -> (i16, i16) {
    let want: Vec<char> = s.chars().collect();
    for y in 0..20 {
        let row: Vec<char> = (0..60).map(|x| buf.get(x, y).unwrap().to_char()).collect();
        for x in 0..=60 - want.len() {
            if row[x..x + want.len()] == want[..] {
                return (x as i16, y);
            }
        }
    }
    panic!("{s:?} not on screen");
}

fn click(ui: &mut Ui, x: i16, y: i16) {
    ui.handle(Event::Mouse(Mouse { x, y, kind: MouseKind::Down(Button::Left) }));
    ui.handle(Event::Mouse(Mouse { x, y, kind: MouseKind::Up(Button::Left) }));
}

fn key(ui: &mut Ui, code: KeyCode) {
    ui.handle(Event::Key(Key { code, mods: Mods::default() }));
    while ui.pick_pending() {
        ui.complete_pick();
    }
}

#[test]
fn a_placed_row_starts_at_its_left_edge() {
    let mut p = pad();
    let buf = draw(&mut p.ui);
    let (x, y) = find(&buf, "  C  ");
    assert_eq!((x, y), (3, 3), "C: frame, one cell in, one cell of padding; row 2 of the body");
    let (x7, _) = find(&buf, "  7  ");
    assert_eq!(x7, x + 7 + 2, "a gap of two between keys");
}

#[test]
fn styles_are_colours_and_disabled_is_grey() {
    let mut p = pad();
    let buf = draw(&mut p.ui);
    let (x, y) = find(&buf, "  C  ");
    assert_eq!(buf.get(x + 2, y).unwrap().attr, attr(Color::White, Color::Red), "Danger is white on red");
    let (x, y) = find(&buf, "  +  ");
    assert_eq!(buf.get(x + 2, y).unwrap().attr, attr(Color::Black, Color::Cyan), "Accent is black on cyan");
    let (x, y) = find(&buf, "  7  ");
    assert_eq!(buf.get(x + 2, y).unwrap().attr, attr(Color::Black, Color::Green), "Normal is black on green");
    let (x, y) = find(&buf, "  A  ");
    assert_eq!(buf.get(x + 2, y).unwrap().attr, attr(Color::DarkGray, Color::LightGray), "disabled is grey");
}

#[test]
fn clicking_a_key_presses_it_and_leaves_the_caret_in_the_display() {
    let mut p = pad();
    assert!(p.ui.focused() == Some(p.input), "the display has the focus to begin with");
    let buf = draw(&mut p.ui);
    let (x, y) = find(&buf, "  7  ");
    click(&mut p.ui, x + 2, y);
    assert_eq!(p.ui.take_pressed(), Some(CM_SEVEN));
    assert_eq!(p.ui.focused(), Some(p.input), "a click on a key must not take the focus");
    // The second row too.
    let (x, y) = find(&buf, "  +  ");
    click(&mut p.ui, x + 2, y);
    assert_eq!(p.ui.take_pressed(), Some(CM_PLUS));
    // A disabled key does nothing.
    let (x, y) = find(&buf, "  A  ");
    click(&mut p.ui, x + 2, y);
    assert_eq!(p.ui.take_pressed(), None, "a disabled key must not press");
}

#[test]
fn enter_finds_the_default_in_a_placed_row_and_escape_the_docked_one() {
    let mut p = pad();
    key(&mut p.ui, KeyCode::Enter);
    assert_eq!(p.ui.take_pressed(), Some(CM_EQ), "Enter presses = although its row is not docked");
    key(&mut p.ui, KeyCode::Esc);
    assert_eq!(p.ui.take_pressed(), Some(CM_OK), "Escape presses the last button of the docked row, not of a keypad");
}

#[test]
fn escape_presses_the_cancel_button_wherever_it_is() {
    let mut p = pad_with(true);
    key(&mut p.ui, KeyCode::Esc);
    assert_eq!(p.ui.take_pressed(), Some(CM_CLEAR), "Escape is C when C says so");
}

#[test]
fn a_keypad_is_not_a_tab_stop() {
    let mut p = pad();
    assert_eq!(p.ui.focused(), Some(p.input));
    key(&mut p.ui, KeyCode::Tab);
    let f = p.ui.focused().expect("something has the focus");
    assert!(f != p.row1 && f != p.row2, "Tab stopped at a keypad row");
    key(&mut p.ui, KeyCode::Tab);
    assert_eq!(p.ui.focused(), Some(p.input), "display, OK, display: two stops");
    // The keys still press with the mouse and answer Enter.
    let buf = draw(&mut p.ui);
    let (x, y) = find(&buf, "  7  ");
    click(&mut p.ui, x + 2, y);
    assert_eq!(p.ui.take_pressed(), Some(CM_SEVEN));
    // And the display keeps the keys typed: '5' lands in it.
    key(&mut p.ui, KeyCode::Char('5'));
    let Kind::Input(i) = p.ui.kind(p.input) else { panic!() };
    assert_eq!(i.text, "5");
}

#[test]
fn a_key_can_be_turned_on_and_off_by_its_place() {
    let mut p = pad();
    assert!(p.ui.set_button_enabled(p.row1, 2, true));
    let buf = draw(&mut p.ui);
    let (x, y) = find(&buf, "  A  ");
    assert_eq!(buf.get(x + 2, y).unwrap().attr, attr(Color::Black, Color::Green));
    click(&mut p.ui, x + 2, y);
    assert_eq!(p.ui.take_pressed(), Some(8));
    assert!(!p.ui.set_button_enabled(p.row1, 9, true), "no such button");
    assert!(!p.ui.set_button_enabled(p.input, 0, true), "not a row");
    let _ = p.row2;
}
