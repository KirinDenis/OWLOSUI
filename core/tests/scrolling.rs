//! What a scrollbar does when you press it.
//!
//! These check the thing that was broken: the bar was drawn from one
//! calculation and clicked against another, so it looked right and did
//! nothing. Both now come from `Ui::bars`, and these tests press the actual
//! cells the drawing puts the arrows and the marker in.

use owlosui_core::{Button, Event, Kind, Mouse, MouseKind, Rect, TextView, Ui, Window};

/// A window at (5,4) 55x15 holding 100 lines of 100 columns.
///
/// Its vertical bar therefore runs down column 59 from row 5 to row 17, and
/// its horizontal bar along row 18 from column 7 to column 57. The tests press
/// those cells by number on purpose: a test that asks the code where its own
/// buttons are cannot catch the code putting them in the wrong place.
fn window() -> Ui {
    let mut ui = Ui::new(80, 25);
    let root = ui.root();
    let mut w = Window::new("Scrolling");
    w.footer = String::new();
    let wid = ui.insert(root, Rect::new(5, 4, 55, 15), Kind::Window(w));
    let lines: Vec<Vec<u8>> = (0..100).map(|_| vec![b'x'; 100]).collect();
    ui.insert(wid, Rect::new(0, 0, 53, 13), Kind::Text(TextView::new(lines)));
    ui
}

fn click(ui: &mut Ui, x: i16, y: i16) {
    ui.handle(Event::Mouse(Mouse {
        x,
        y,
        kind: MouseKind::Down(Button::Left),
    }));
}

fn drag_to(ui: &mut Ui, x: i16, y: i16) {
    ui.handle(Event::Mouse(Mouse {
        x,
        y,
        kind: MouseKind::Drag,
    }));
}

fn scroll(ui: &Ui) -> (i16, i16) {
    let win = ui.active_window().unwrap();
    let text = ui.children(win)[0];
    match ui.kind(text) {
        Kind::Text(t) => (t.top, t.left),
        _ => panic!("not a text view"),
    }
}

#[test]
fn arrows_step_one() {
    let mut ui = window();

    click(&mut ui, 59, 17); // ▼
    assert_eq!(scroll(&ui).0, 1);

    click(&mut ui, 59, 5); // ▲
    assert_eq!(scroll(&ui).0, 0);

    click(&mut ui, 57, 18); // ►
    assert_eq!(scroll(&ui).1, 1);

    click(&mut ui, 7, 18); // ◄
    assert_eq!(scroll(&ui).1, 0);
}

#[test]
fn the_track_steps_a_page() {
    let mut ui = window();

    // The marker starts at the top of the track, row 6. Pressing below it
    // moves on by one view's worth — thirteen rows here.
    click(&mut ui, 59, 12);
    assert_eq!(scroll(&ui).0, 13);

    click(&mut ui, 59, 6);
    assert_eq!(scroll(&ui).0, 0);
}

#[test]
fn dragging_the_marker_moves_the_content() {
    let mut ui = window();

    // Take hold of the marker where it is, then pull it to the far end of the
    // track. 100 lines in a 13-row view can travel 87.
    click(&mut ui, 59, 6);
    drag_to(&mut ui, 59, 16);
    assert_eq!(scroll(&ui).0, 87);

    // And back.
    drag_to(&mut ui, 59, 6);
    assert_eq!(scroll(&ui).0, 0);
}

/// The bug that started this: the horizontal bar responded to nothing at all.
#[test]
fn the_horizontal_bar_is_not_decoration() {
    let mut ui = window();

    click(&mut ui, 30, 18);
    let after_page = scroll(&ui).1;
    assert!(after_page > 0, "pressing the track did nothing");

    click(&mut ui, 7, 18); // ◄ steps back one
    assert_eq!(scroll(&ui).1, after_page - 1);
}

#[test]
fn the_horizontal_marker_drags_too() {
    // A window of its own, because the marker only sits at the left end of the
    // track while the content is unscrolled — and grabbing where it *was* is
    // the mistake this test was written wrong to begin with.
    let mut ui = window();

    click(&mut ui, 8, 18); // the marker, at rest
    drag_to(&mut ui, 99, 18); // far beyond the end of the bar

    // 100 columns in a 53-wide view can travel 47, and no further however far
    // the pointer goes.
    assert_eq!(scroll(&ui).1, 47);
}
