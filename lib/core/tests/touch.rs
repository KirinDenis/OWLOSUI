//! What a finger dragged from a cell would do, as the core tells a touch
//! screen's host: move a window by its title, resize it by a corner it may
//! take from a little way off, scroll a text a row at a time, or - for
//! anything else - be pressed and dragged as the mouse would.

use owlosui_core::{glyphs, Buffer, Kind, Point, Rect, TextView, Touch, Ui, Window};

fn frame(ui: &mut Ui) {
    let mut b = Buffer::new(80, 25);
    ui.draw(&mut b);
}

#[test]
fn a_finger_moves_by_the_title_resizes_by_the_corner_and_scrolls_a_text() {
    let mut ui = Ui::new(80, 25);
    let root = ui.root();
    let back = ui.insert(root, Rect::new(40, 12, 30, 10), Kind::Window(Window::new("Back")));
    let win = ui.insert(root, Rect::new(5, 3, 30, 12), Kind::Window(Window::new("Notes")));
    let lines = (0..60).map(|i| glyphs(&format!("line {i}"))).collect();
    let t = ui.insert(win, Rect::default(), Kind::Text(TextView::new(lines)));
    ui.activate(win);
    frame(&mut ui);

    // The title, away from its boxes: the window moves.
    assert_eq!(ui.touch_at(Point::new(12, 3)).0, Touch::Move);
    // The close box on the title is tapped, not dragged.
    assert_eq!(ui.touch_at(Point::new(31, 3)).0, Touch::Mouse);
    // A finger near the corner - not on it - takes the corner itself.
    let (what, at) = ui.touch_at(Point::new(31, 13));
    assert_eq!((what, at), (Touch::Resize, Point::new(34, 14)));
    // Inside the text: it scrolls.
    assert_eq!(ui.touch_at(Point::new(10, 6)).0, Touch::Scroll);
    // The desktop, and the frame's left edge: the mouse's.
    assert_eq!(ui.touch_at(Point::new(2, 20)).0, Touch::Mouse);
    assert_eq!(ui.touch_at(Point::new(5, 6)).0, Touch::Mouse);
    // A window that is not active has no grip drawn, so none to take.
    assert_ne!(ui.touch_at(Point::new(68, 21)).0, Touch::Resize);
    let _ = back;

    // A row at a time, both ways, and never past the ends.
    ui.scroll_rows_at(Point::new(10, 6), 1);
    ui.scroll_rows_at(Point::new(10, 6), 1);
    let top = |ui: &Ui| match ui.kind(t) {
        Kind::Text(tv) => tv.top,
        _ => unreachable!(),
    };
    assert_eq!(top(&ui), 2);
    ui.scroll_rows_at(Point::new(10, 6), -5);
    assert_eq!(top(&ui), 0);
}
