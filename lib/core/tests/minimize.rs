//! The boxes on a window's top edge - minimize, zoom, close, at the right,
//! the close box in the corner - and a window put away: it falls into a bar
//! in the bottom right corner, the bars stack upwards, and a click brings it
//! back where it was.

use owlosui_core::{Buffer, Button, Event, Kind, Mouse, MouseKind, Rect, StatusItem, StatusLine, Ui, ViewId, Window};

fn frame(ui: &mut Ui) -> Buffer {
    let mut b = Buffer::new(80, 25);
    ui.draw(&mut b);
    b
}

/// A row as text, the arrows and the square as themselves.
fn row(b: &Buffer, y: i16) -> String {
    (0..80)
        .map(|x| match b.get(x, y).unwrap().ch as u32 {
            0x18 => '↑',
            0x19 => '↓',
            0x12 => '↕',
            0xFE => '■',
            g if g < 128 => char::from_u32(g).unwrap(),
            _ => '#',
        })
        .collect()
}

fn click(ui: &mut Ui, x: i16, y: i16) {
    ui.handle(Event::Mouse(Mouse { x, y, kind: MouseKind::Down(Button::Left) }));
    ui.handle(Event::Mouse(Mouse { x, y, kind: MouseKind::Up(Button::Left) }));
}

/// Tick until nothing is held: the animation runs to its end.
fn settle(ui: &mut Ui) -> usize {
    let mut frames = 0;
    while ui.pick_pending() {
        frame(ui);
        ui.complete_pick();
        frames += 1;
        assert!(frames < 50, "the animation never ends");
    }
    frame(ui);
    frames
}

fn desk() -> Ui {
    let mut ui = Ui::new(80, 25);
    let root = ui.root();
    ui.insert(root, Rect::new(0, 24, 80, 1), Kind::Status(StatusLine::new(vec![StatusItem::new("~F1~ Help", None, 1)])));
    ui
}

fn window(ui: &mut Ui, title: &str, r: Rect) -> ViewId {
    let root = ui.root();
    let w = ui.insert(root, r, Kind::Window(Window::new(title)));
    ui.activate(w);
    w
}

#[test]
fn the_boxes_are_at_the_right_close_in_the_corner() {
    let mut ui = desk();
    window(&mut ui, "Doc", Rect::new(10, 2, 40, 10));
    let b = frame(&mut ui);
    let top = row(&b, 2);
    // right = 50: close at 45..47, zoom 42..44, minimize 39..41.
    assert_eq!(top.chars().skip(39).take(9).collect::<String>(), "[\u{2193}][\u{2191}][\u{25A0}]", "the top edge reads {top:?}");
    assert!(!top.chars().skip(10).take(10).any(|c| c == '['), "nothing is left at the left: {top:?}");
}

#[test]
fn minimize_falls_into_the_corner_and_a_click_brings_it_back() {
    let mut ui = desk();
    let a = window(&mut ui, "Behind", Rect::new(2, 2, 30, 8));
    let w = window(&mut ui, "NOTES.TXT", Rect::new(10, 4, 40, 12));
    frame(&mut ui);
    // The minimize box, [↓] at 39..41 of a window whose right edge is 50.
    click(&mut ui, 40, 4);
    assert!(ui.animating(), "it falls rather than vanishing");
    assert_eq!(ui.place(w).map(|p| p.1), Some(true), "nothing of it shows while it is away");
    let frames = settle(&mut ui);
    assert!(frames >= 4, "the fall took {frames} frames");
    assert_eq!(ui.active_window(), Some(a), "the window behind is the active one now");

    // Its bar: the bottom row above the status line, at the right.
    let b = frame(&mut ui);
    let bar = row(&b, 23);
    assert!(bar.trim_end().ends_with("[\u{2191}][\u{25A0}]") && bar.contains("NOTES.TXT"), "the bar reads {bar:?}");
    // A second one stacks above it.
    let c = window(&mut ui, "Third", Rect::new(5, 3, 30, 8));
    frame(&mut ui);
    ui.minimize(c);
    settle(&mut ui);
    let b = frame(&mut ui);
    assert!(row(&b, 22).contains("Third") && row(&b, 23).contains("NOTES.TXT"), "the bars do not stack:\n{}\n{}", row(&b, 22), row(&b, 23));

    // A click on the bar's words brings it back, where it was, in front.
    let x = row(&b, 23).chars().position(|c| c == 'N').unwrap() as i16;
    click(&mut ui, x, 23);
    settle(&mut ui);
    assert_eq!(ui.active_window(), Some(w));
    assert_eq!(ui.rect(w), Rect::new(10, 4, 40, 12));
    let b = frame(&mut ui);
    assert!(row(&b, 23).contains("Third") && !row(&b, 22).contains("Third"), "the bar above did not come down");

    // Activating a window put away brings it back too - Alt+its number.
    ui.activate(c);
    settle(&mut ui);
    assert_eq!(ui.active_window(), Some(c));
}

#[test]
fn the_square_on_a_bar_closes_its_window() {
    let mut ui = desk();
    let w = window(&mut ui, "Gone", Rect::new(10, 4, 40, 12));
    ui.minimize(w);
    settle(&mut ui);
    let b = frame(&mut ui);
    let x = row(&b, 23).chars().position(|c| c == '\u{25A0}').unwrap() as i16;
    click(&mut ui, x, 23);
    assert!(!ui.is_alive(w));
    assert!(!row(&frame(&mut ui), 23).contains("Gone"));
}

#[test]
fn a_dialog_has_no_minimize_box_and_a_modal_window_is_not_put_away() {
    let mut ui = desk();
    let root = ui.root();
    let mut d = Window::new("Dialog");
    d.zoomable = false;
    d.minimizable = false;
    ui.insert(root, Rect::new(10, 2, 40, 10), Kind::Window(d));
    let b = frame(&mut ui);
    assert!(!row(&b, 2).contains('\u{2193}'), "a dialog with a minimize box: {}", row(&b, 2));

    let mut m = Window::new("Modal");
    m.modal = true;
    let m = ui.insert(root, Rect::new(10, 2, 40, 10), Kind::Window(m));
    ui.minimize(m);
    assert!(!ui.animating());
    assert_eq!(ui.active_window(), Some(m));
}
