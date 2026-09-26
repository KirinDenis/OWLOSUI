//! Cascade and tile: the desktop arranging its windows.
//!
//! Turbo Vision put both on the desktop and not in the application, and
//! so does this: the desktop knows which windows there are, which may
//! move and which one is modal.

use owlosui_core::{Kind, MenuBar, Rect, StatusLine, Ui, ViewId, Window};

fn desk() -> Ui {
    let mut ui = Ui::new(80, 25);
    let root = ui.root();
    ui.insert(root, Rect::new(0, 0, 80, 1), Kind::MenuBar(MenuBar::new(vec![])));
    ui.insert(root, Rect::new(0, 24, 80, 1), Kind::Status(StatusLine::new(vec![])));
    ui
}

fn doc(ui: &mut Ui, title: &str) -> ViewId {
    let root = ui.root();
    ui.insert(root, Rect::new(10, 5, 30, 8), Kind::Window(Window::new(title)))
}

fn dialog(ui: &mut Ui, title: &str) -> ViewId {
    let root = ui.root();
    let mut w = Window::new(title);
    w.resizable = false;
    ui.insert(root, Rect::new(20, 6, 24, 6), Kind::Window(w))
}

#[test]
fn tile_shares_the_work_area_without_overlap() {
    let mut ui = desk();
    let a = doc(&mut ui, "A");
    let b = doc(&mut ui, "B");
    let c = doc(&mut ui, "C");
    let d = doc(&mut ui, "D");
    ui.tile();
    // Four windows: two columns of two, over rows 1..23 and columns 0..79.
    assert_eq!(ui.rect(a), Rect::new(0, 1, 40, 11));
    assert_eq!(ui.rect(b), Rect::new(0, 12, 40, 12));
    assert_eq!(ui.rect(c), Rect::new(40, 1, 40, 11));
    assert_eq!(ui.rect(d), Rect::new(40, 12, 40, 12));
}

#[test]
fn tile_puts_the_odd_window_in_the_right_hand_column() {
    let mut ui = desk();
    let a = doc(&mut ui, "A");
    let b = doc(&mut ui, "B");
    let c = doc(&mut ui, "C");
    ui.tile();
    // Three: one column (the square root of three, rounded down), so three
    // rows. Five would be two columns, the right one three tall.
    assert_eq!(ui.rect(a).w, 80);
    assert_eq!(ui.rect(a).y, 1);
    assert_eq!(ui.rect(b).y, ui.rect(a).y + ui.rect(a).h);
    assert_eq!(ui.rect(c).y + ui.rect(c).h, 24);
    let ids: Vec<ViewId> = (0..2).map(|i| doc(&mut ui, &format!("{i}"))).collect();
    ui.tile();
    assert_eq!(ui.rect(a).w, 40, "five windows: two columns");
    assert_eq!(ui.rect(a).h + ui.rect(b).h, 23, "two on the left");
    assert_eq!(ui.rect(c).x, 40, "three on the right");
    assert_eq!(ui.rect(c).h + ui.rect(ids[0]).h + ui.rect(ids[1]).h, 23);
}

#[test]
fn a_fixed_window_keeps_its_size_in_its_cell_and_on_the_staircase() {
    let mut ui = desk();
    let a = doc(&mut ui, "A");
    let d = dialog(&mut ui, "Fixed");
    ui.tile();
    // Two windows: one column, two rows. The dialog takes the lower cell's
    // corner at its own size; the document fills the upper cell.
    assert_eq!(ui.rect(a), Rect::new(0, 1, 80, 11));
    assert_eq!(ui.rect(d), Rect::new(0, 12, 24, 6), "the dialog stands in its cell, its own size");
    ui.cascade();
    assert_eq!(ui.rect(a), Rect::new(0, 1, 80, 23), "the back window fills the work area");
    assert_eq!(ui.rect(d), Rect::new(1, 2, 24, 6), "the dialog moved one step down the diagonal, same size");
    // A dialog alone on the desktop: Tile puts it in the corner.
    let mut ui = desk();
    let d = dialog(&mut ui, "Only");
    ui.tile();
    assert_eq!(ui.rect(d), Rect::new(0, 1, 24, 6));
}

#[test]
fn cascade_is_a_staircase_from_the_back() {
    let mut ui = desk();
    let a = doc(&mut ui, "A");
    let b = doc(&mut ui, "B");
    let c = doc(&mut ui, "C");
    ui.cascade();
    assert_eq!(ui.rect(a), Rect::new(0, 1, 80, 23));
    assert_eq!(ui.rect(b), Rect::new(1, 2, 79, 22));
    assert_eq!(ui.rect(c), Rect::new(2, 3, 78, 21));
    // Zoom afterwards zooms from the cascaded place, not from before it.
    ui.toggle_zoom(c);
    assert_eq!(ui.rect(c), Rect::new(0, 1, 80, 23));
    ui.toggle_zoom(c);
    assert_eq!(ui.rect(c), Rect::new(2, 3, 78, 21));
}

#[test]
fn nothing_moves_under_a_modal_window() {
    let mut ui = desk();
    let a = doc(&mut ui, "A");
    let root = ui.root();
    let mut m = Window::new("Ask");
    m.modal = true;
    ui.insert(root, Rect::new(30, 8, 20, 6), Kind::Window(m));
    ui.tile();
    ui.cascade();
    assert_eq!(ui.rect(a), Rect::new(10, 5, 30, 8));
}
