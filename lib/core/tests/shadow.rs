//! A button's shadow is the surface it stands on, darker.
//!
//! It used to be one colour - dark grey on light grey, right for a dialog -
//! and on a blue document window it came out as a light grey smear under
//! each button, as if the buttons had brought a piece of dialog with them.
//! A shadow has no colour of its own.

use owlosui_core::{attr, Buffer, ButtonRow, Color, Dock, Kind, Palette, PushButton, Rect, Ui, WinPalette, Window};
use owlosui_core::cell::{glyphs, Glyph};

fn with_buttons(palette: WinPalette) -> Buffer {
    let mut ui = Ui::new(40, 10);
    let root = ui.root();
    let mut win = Window::new("W");
    win.palette = palette;
    let w = ui.insert(root, Rect::new(0, 0, 40, 10), Kind::Window(win));
    let row = ui.insert(
        w,
        Rect::default(),
        Kind::Buttons(ButtonRow::new(vec![PushButton::new("~O~K", 1)])),
    );
    ui.set_dock(row, Dock::BottomRight(12, 2));
    let mut buf = Buffer::new(40, 10);
    ui.draw(&mut buf);
    buf
}

/// The cell under the button's first label column: the `▀` of its shadow.
fn shadow_cell(buf: &Buffer) -> (Glyph, u8) {
    for y in 0..10 {
        for x in 0..40 {
            let c = buf.get(x, y).unwrap();
            if c.ch == 0xDF {
                return (c.ch, c.attr);
            }
        }
    }
    panic!("no shadow drawn");
}

#[test]
fn on_a_grey_dialog_the_shadow_is_dark_grey_on_grey() {
    let (_, a) = shadow_cell(&with_buttons(WinPalette::Gray));
    assert_eq!(a, attr(Color::DarkGray, Color::LightGray), "{a:02X}");
}

#[test]
fn on_a_blue_window_the_shadow_is_black_on_blue() {
    let (_, a) = shadow_cell(&with_buttons(WinPalette::Blue));
    assert_eq!(a, attr(Color::Black, Color::Blue), "{a:02X}");
}

#[test]
fn the_rule_itself() {
    assert_eq!(Palette::shadow_on(attr(Color::Black, Color::LightGray)), attr(Color::DarkGray, Color::LightGray));
    assert_eq!(Palette::shadow_on(attr(Color::Yellow, Color::Blue)), attr(Color::Black, Color::Blue));
    assert_eq!(Palette::shadow_on(attr(Color::Black, Color::Cyan)), attr(Color::Black, Color::Cyan));
    // Nothing is darker than black; the shadow has to be lighter to exist.
    assert_eq!(Palette::shadow_on(attr(Color::White, Color::Black)), attr(Color::DarkGray, Color::Black));
    // A bright surface shadows to its own dark half.
    assert_eq!(Palette::shadow_on(attr(Color::Black, Color::LightCyan)), attr(Color::Cyan, Color::LightCyan));
}
