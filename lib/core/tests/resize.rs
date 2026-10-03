//! The desktop changing size under the windows.
//!
//! Found by a person dragging the console window: everything vanished. The
//! old `resize` shrank every window to fit the new screen and never grew one
//! back, so a console taken down to a sliver and restored came back empty.
//! The next answer grew every window by the same number of cells, corners
//! where they were, and a browser made a little narrower squeezed a laid-out
//! page into its corners. Now a window keeps its proportions: its edges stay
//! at the same fractions of the work area.

use owlosui_core::{Buffer, Event, Kind, Rect, TextView, Ui, Window};
use owlosui_core::cell::{glyphs, Glyph};

fn settle(ui: &mut Ui) {
    let r = ui.rect(ui.root());
    let mut buf = Buffer::new(r.w, r.h);
    ui.draw(&mut buf);
}

fn editor(ui: &mut Ui, r: Rect) -> owlosui_core::ViewId {
    let root = ui.root();
    let w = ui.insert(root, r, Kind::Window(Window::new("NOTES.TXT")));
    ui.insert(w, Rect::default(), Kind::Text(TextView::new(vec![glyphs("x")])));
    w
}

#[test]
fn a_resizable_window_keeps_its_proportions() {
    let mut ui = Ui::new(80, 25);
    let w = editor(&mut ui, Rect::new(2, 1, 76, 23));
    settle(&mut ui);

    // Every edge is within two cells of the screen's: the margins are kept,
    // and the window grows and shrinks with the desktop.
    ui.handle(Event::Resize(100, 30));
    settle(&mut ui);
    assert_eq!(ui.rect(w), Rect::new(2, 1, 96, 28), "grew with the desktop");

    ui.handle(Event::Resize(60, 20));
    settle(&mut ui);
    assert_eq!(ui.rect(w), Rect::new(2, 1, 56, 18), "shrank with the desktop");

    // Worked out from where it was put, not from the last size: back to
    // 80 by 25 is back to exactly where it was.
    ui.handle(Event::Resize(80, 25));
    settle(&mut ui);
    assert_eq!(ui.rect(w), Rect::new(2, 1, 76, 23), "back where it was");
}

#[test]
fn a_laid_out_page_keeps_its_layout_when_the_browser_narrows() {
    // The pilot's page: an editor on the left two thirds, two windows
    // stacked on the right. Made narrower, it used to keep every corner and
    // shrink every window by the same amount - the right-hand ones down to
    // slivers in the corner, the middle of the screen empty.
    let mut ui = Ui::new(150, 40);
    let left = editor(&mut ui, Rect::new(0, 0, 100, 40));
    let top = editor(&mut ui, Rect::new(100, 0, 50, 20));
    let bottom = editor(&mut ui, Rect::new(100, 20, 50, 20));
    settle(&mut ui);

    ui.handle(Event::Resize(96, 30));
    settle(&mut ui);
    assert_eq!(ui.rect(left), Rect::new(0, 0, 64, 30));
    assert_eq!(ui.rect(top), Rect::new(64, 0, 32, 15));
    assert_eq!(ui.rect(bottom), Rect::new(64, 15, 32, 15));

    // And back: every window exactly where it was put.
    ui.handle(Event::Resize(150, 40));
    settle(&mut ui);
    assert_eq!(ui.rect(left), Rect::new(0, 0, 100, 40));
    assert_eq!(ui.rect(top), Rect::new(100, 0, 50, 20));
    assert_eq!(ui.rect(bottom), Rect::new(100, 20, 50, 20));
}

#[test]
fn a_sliver_and_back_loses_nothing_that_can_be_kept() {
    let mut ui = Ui::new(80, 25);
    let w = editor(&mut ui, Rect::new(2, 1, 76, 23));
    settle(&mut ui);

    ui.handle(Event::Resize(1, 1));
    settle(&mut ui);
    let r = ui.rect(w);
    assert!(r.w >= 6 && r.h >= 3, "went below the structural minimum: {r:?}");

    // Some size was lost at the floor, and that is accepted; what is not
    // accepted is the window being gone, or off the screen.
    ui.handle(Event::Resize(80, 25));
    settle(&mut ui);
    let r = ui.rect(w);
    assert!(r.w >= 6 && r.h >= 3);
    assert!(r.x < 80 && r.y < 25, "off screen: {r:?}");
    // It was pushed to the top-left corner by the sliver, which is where a
    // window that no longer fits goes; the title must be on screen somewhere.
    let mut buf = Buffer::new(80, 25);
    ui.draw(&mut buf);
    let rows: Vec<String> = (0..25)
        .map(|y| (0..80).map(|x| buf.get(x, y).unwrap().to_char()).collect())
        .collect();
    assert!(
        rows.iter().any(|r| r.contains("NOTES")),
        "the window is not on screen:\n{}",
        rows.join("\n")
    );
}

#[test]
fn a_fixed_centred_window_is_centred_again() {
    let mut ui = Ui::new(80, 25);
    let root = ui.root();
    let mut win = Window::new("Confirm");
    win.resizable = false;
    win.min_w = 40;
    win.max_w = 40;
    win.min_h = 9;
    win.max_h = 9;
    win.centred = true;
    let w = ui.insert(root, Rect::new(20, 8, 40, 9), Kind::Window(win));
    settle(&mut ui);
    assert_eq!(ui.rect(w), Rect::new(20, 8, 40, 9));

    ui.handle(Event::Resize(120, 40));
    settle(&mut ui);
    assert_eq!(ui.rect(w), Rect::new(40, 15, 40, 9), "still 40x9, and in the middle");

    ui.handle(Event::Resize(30, 8));
    settle(&mut ui);
    let r = ui.rect(w);
    assert_eq!((r.w, r.h), (40, 9), "a fixed window does not shrink");
    assert!(r.x < 30 && r.y < 8, "some of it is on the screen: {r:?}");
}

#[test]
fn dragging_a_centred_window_makes_it_stay_put() {
    use owlosui_core::{Button, Mouse, MouseKind};
    let mut ui = Ui::new(80, 25);
    let root = ui.root();
    let mut win = Window::new("Confirm");
    win.centred = true;
    win.resizable = false;
    win.min_w = 40;
    win.max_w = 40;
    win.min_h = 9;
    win.max_h = 9;
    let w = ui.insert(root, Rect::new(20, 8, 40, 9), Kind::Window(win));
    settle(&mut ui);

    let r = ui.rect(w);
    let grab = (r.x + r.w / 2, r.y);
    ui.handle(Event::Mouse(Mouse { x: grab.0, y: grab.1, kind: MouseKind::Down(Button::Left) }));
    ui.handle(Event::Mouse(Mouse { x: grab.0 - 10, y: grab.1 + 3, kind: MouseKind::Drag }));
    ui.handle(Event::Mouse(Mouse { x: grab.0 - 10, y: grab.1 + 3, kind: MouseKind::Up(Button::Left) }));
    settle(&mut ui);
    assert_eq!(ui.rect(w), Rect::new(10, 11, 40, 9));

    ui.handle(Event::Resize(120, 40));
    settle(&mut ui);
    // Its middle was at 30/80 and 15/25; it stays there, not in the middle.
    assert_eq!(ui.rect(w), Rect::new(25, 20, 40, 9), "where the hand put it, not the middle");
}

#[test]
fn the_close_box_can_be_a_command() {
    use owlosui_core::{Button, Mouse, MouseKind};
    let mut ui = Ui::new(80, 25);
    let root = ui.root();
    let mut win = Window::new("NOTES.TXT");
    win.close_cmd = 42;
    let w = ui.insert(root, Rect::new(2, 1, 76, 23), Kind::Window(win));
    settle(&mut ui);

    // The box is in the right-hand corner of the title row: right - 5 .. right - 3.
    ui.handle(Event::Mouse(Mouse { x: 74, y: 1, kind: MouseKind::Down(Button::Left) }));
    ui.handle(Event::Mouse(Mouse { x: 74, y: 1, kind: MouseKind::Up(Button::Left) }));
    assert!(ui.is_alive(w), "closed instead of asking");
    assert_eq!(ui.take_command(), Some(42));

    // And with no command, it just closes.
    let w2 = ui.insert(root, Rect::new(2, 1, 76, 23), Kind::Window(Window::new("Other")));
    settle(&mut ui);
    ui.handle(Event::Mouse(Mouse { x: 74, y: 1, kind: MouseKind::Down(Button::Left) }));
    ui.handle(Event::Mouse(Mouse { x: 74, y: 1, kind: MouseKind::Up(Button::Left) }));
    assert!(!ui.is_alive(w2));
    assert_eq!(ui.take_command(), None);
}
