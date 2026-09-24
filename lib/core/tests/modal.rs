//! Modal windows.
//!
//! A modal dialog is not a window that is drawn last. It is a window that is
//! the *only* window: while it is up, the menu bar is out of reach, a click on
//! the editor behind it does nothing at all, and the key that cycles windows
//! has nothing to cycle. Each of those is one line somewhere in `ui.rs`, and
//! each of them is a line somebody could delete next month without any of the
//! others complaining — which is what these tests are for.

use owlosui_core::cell::{glyphs, Glyph};
use owlosui_core::{
    Button, Event, Key, KeyCode, Kind, MenuBar, MenuItem, Mods, Mouse, MouseKind, PushButton,
    ButtonRow, Rect, TextView, Ui, ViewId, Window,
};

const CM_COPY: u16 = 20;
const CM_OK: u16 = 30;

/// A menu bar, an editor filling the screen, and nothing else.
fn app() -> (Ui, ViewId, ViewId) {
    let mut ui = Ui::new(60, 20);
    let root = ui.root();
    let bar = MenuBar::new(vec![MenuItem::sub(
        "~F~ile",
        vec![MenuItem::new("~C~opy", "", CM_COPY)],
    )]);
    ui.insert(root, Rect::new(0, 0, 60, 1), Kind::MenuBar(bar));

    let win = ui.insert(
        root,
        Rect::new(0, 1, 40, 15),
        Kind::Window(Window::new("EDIT.TXT")),
    );
    let text = ui.insert(
        win,
        Rect::default(),
        Kind::Text(TextView::new(vec![glyphs("hello"), glyphs("there")])),
    );
    settle(&mut ui);
    (ui, win, text)
}

fn box_of(ui: &mut Ui) -> ViewId {
    let row = ButtonRow::new(vec![PushButton::new("~O~K", CM_OK).default()]);
    let id = ui.message_box("Confirm", "Leave without saving?", row);
    settle(ui);
    id
}

/// Draw once and throw the picture away.
///
/// Child rectangles are worked out by the measure pass, which runs as part of
/// drawing — so a view that has never been drawn has no size, and a click at
/// any coordinate misses it. Tests that forget this pass by doing nothing,
/// which is the worst way for a test to pass.
fn settle(ui: &mut Ui) {
    let mut buf = owlosui_core::Buffer::new(60, 20);
    ui.draw(&mut buf);
}

fn click(ui: &mut Ui, x: i16, y: i16) {
    ui.handle(Event::Mouse(Mouse {
        x,
        y,
        kind: MouseKind::Down(Button::Left),
    }));
    ui.handle(Event::Mouse(Mouse {
        x,
        y,
        kind: MouseKind::Up(Button::Left),
    }));
}

fn key(ui: &mut Ui, code: KeyCode, mods: Mods) {
    ui.handle(Event::Key(Key { code, mods }));
}

#[test]
fn modal_is_the_active_window_however_deep_it_sits() {
    let (mut ui, win, _) = app();
    let m = box_of(&mut ui);
    assert_eq!(ui.modal(), Some(m));
    assert_eq!(ui.active_window(), Some(m));

    // Even after something else is explicitly raised: the modal did not come
    // to the front by being newest, and it does not leave by being old.
    ui.activate(win);
    assert_eq!(ui.active_window(), Some(m));
}

#[test]
fn a_modal_is_drawn_above_the_windows_behind_it() {
    let (mut ui, win, _) = app();
    let m = box_of(&mut ui);
    // The editor is raised to the front of the desktop's children. An
    // ordinary window would now be drawn last and cover the dialog; a modal
    // sits on a higher layer, so painter's order cannot reach it.
    ui.activate(win);

    let mut buf = owlosui_core::Buffer::new(60, 20);
    ui.draw(&mut buf);

    let r = ui.rect(m);
    let grey = owlosui_core::Palette::classic().gray.body;
    assert_eq!(
        buf.get(r.x + 2, r.y + 1).unwrap().attr,
        grey,
        "the editor was painted over the dialog"
    );
}

#[test]
fn a_modal_stays_below_the_menu_bar() {
    // Not a consolation prize for the layer above it: a dialog is clamped into
    // the work area like every other window, so it never reaches row 0 in the
    // first place. The layer matters for what it is drawn over, not for where
    // it is allowed to sit.
    let (mut ui, _, _) = app();
    let m = box_of(&mut ui);
    let r = ui.rect(m);
    ui.set_rect(m, Rect::new(4, 0, r.w, r.h));
    settle(&mut ui);
    assert!(ui.rect(m).y >= 1);
}

#[test]
fn a_click_behind_a_modal_does_nothing() {
    let (mut ui, win, text) = app();
    // Put the caret somewhere known first, and check that it went there -
    // otherwise the comparison below passes by both clicks doing nothing.
    click(&mut ui, 3, 3);
    let before = match ui.kind(text) {
        Kind::Text(t) => t.cur,
        _ => unreachable!(),
    };
    assert_eq!((before.x, before.y), (2, 1));

    let m = box_of(&mut ui);
    // Well clear of the centred dialog: the editor's own title bar.
    click(&mut ui, 2, 1);
    click(&mut ui, 5, 4);

    assert_eq!(ui.active_window(), Some(m), "the click activated the editor");
    let after = match ui.kind(text) {
        Kind::Text(t) => t.cur,
        _ => unreachable!(),
    };
    assert_eq!(before, after, "the click moved the caret behind the dialog");
    let _ = win;
}

#[test]
fn the_menu_cannot_be_opened_while_a_modal_is_up() {
    let (mut ui, _, _) = app();
    box_of(&mut ui);

    key(&mut ui, KeyCode::F(10), Mods::default());
    assert!(ui.menu_open().is_none(), "F10 opened the menu");

    key(&mut ui, KeyCode::Char('f'), Mods { alt: true, ..Mods::default() });
    assert!(ui.menu_open().is_none(), "Alt+F opened the menu");

    // Nor by clicking it.
    click(&mut ui, 3, 0);
    assert!(ui.menu_open().is_none(), "a click opened the menu");
}

#[test]
fn window_cycling_stops_while_a_modal_is_up() {
    let (mut ui, _, _) = app();
    let m = box_of(&mut ui);
    ui.cycle_windows();
    assert_eq!(ui.active_window(), Some(m));
}

#[test]
fn a_modal_can_still_be_moved() {
    let (mut ui, _, _) = app();
    let m = box_of(&mut ui);
    let r = ui.rect(m);

    // Grab the title bar and drag it two right, one down. A dialog that covers
    // the thing it is asking about is a dialog you cannot answer.
    let grab = r.x + r.w / 2;
    ui.handle(Event::Mouse(Mouse {
        x: grab,
        y: r.y,
        kind: MouseKind::Down(Button::Left),
    }));
    ui.handle(Event::Mouse(Mouse {
        x: grab + 2,
        y: r.y + 1,
        kind: MouseKind::Drag,
    }));
    ui.handle(Event::Mouse(Mouse {
        x: grab + 2,
        y: r.y + 1,
        kind: MouseKind::Up(Button::Left),
    }));

    let moved = ui.rect(m);
    assert_eq!((moved.x, moved.y), (r.x + 2, r.y + 1));
    assert_eq!((moved.w, moved.h), (r.w, r.h));
}

#[test]
fn closing_it_hands_everything_back() {
    let (mut ui, win, text) = app();
    let m = box_of(&mut ui);
    ui.close(m);

    assert!(ui.modal().is_none());
    assert_eq!(ui.active_window(), Some(win));

    key(&mut ui, KeyCode::F(10), Mods::default());
    assert!(ui.menu_open().is_some(), "the menu is still unreachable");
    key(&mut ui, KeyCode::Esc, Mods::default());

    click(&mut ui, 3, 3);
    let cur = match ui.kind(text) {
        Kind::Text(t) => t.cur,
        _ => unreachable!(),
    };
    assert_eq!((cur.x, cur.y), (2, 1));
}

#[test]
fn the_box_is_as_tall_as_its_words() {
    let mut ui = Ui::new(60, 20);
    let row = ButtonRow::new(vec![PushButton::new("~O~K", CM_OK).default()]);
    let one = ui.message_box("Confirm", "Short.", row);
    let short = ui.rect(one);
    ui.close(one);

    let row = ButtonRow::new(vec![PushButton::new("~O~K", CM_OK).default()]);
    let long = ui.message_box(
        "Confirm",
        "A sentence quite long enough that it cannot fit on one line of a \
         dialog this narrow, and so has to be broken across several.",
        row,
    );
    let tall = ui.rect(long);

    assert!(tall.h > short.h);
    assert_eq!(tall.w, short.w, "the width is fixed; only the height grows");
}

#[test]
fn the_default_button_answers_it() {
    let (mut ui, _, _) = app();
    box_of(&mut ui);
    key(&mut ui, KeyCode::Enter, Mods::default());
    // Down first, drawn without its shadow; the command comes on the tick.
    assert!(ui.pick_pending());
    assert_eq!(ui.take_pressed(), None);
    ui.complete_pick();
    assert_eq!(ui.take_pressed(), Some(CM_OK));
}

#[test]
fn enter_presses_the_default_even_when_it_is_not_first() {
    // "Yes / No" with No the default: the cursor must start on No, or the
    // first Enter answers Yes to a question built to be safe.
    let (mut ui, _, _) = app();
    let row = ButtonRow::new(vec![
        PushButton::new("~Y~es", 5),
        PushButton::new("~N~o", 6).default(),
    ]);
    ui.message_box("Confirm", "Delete everything?", row);
    settle(&mut ui);
    key(&mut ui, KeyCode::Enter, Mods::default());
    ui.complete_pick();
    assert_eq!(ui.take_pressed(), Some(6), "Enter pressed Yes");
}

#[test]
fn a_key_press_is_seen_before_it_happens() {
    let (mut ui, _, _) = app();
    let m = box_of(&mut ui);
    let mut buf = owlosui_core::Buffer::new(60, 20);
    ui.draw(&mut buf);
    // Standing up: a `▀` shadow somewhere in the box.
    let r = ui.rect(m);
    let shadow = |buf: &owlosui_core::Buffer| {
        (r.y..r.bottom()).any(|y| (r.x..r.right()).any(|x| buf.get(x, y).unwrap().ch == 0xDF))
    };
    assert!(shadow(&buf), "no shadow under the button to begin with");

    key(&mut ui, KeyCode::Enter, Mods::default());
    ui.draw(&mut buf);
    assert!(!shadow(&buf), "the pressed button still casts a shadow");

    ui.complete_pick();
    ui.draw(&mut buf);
    assert!(shadow(&buf), "the button did not come back up");
    assert_eq!(ui.take_pressed(), Some(CM_OK));
}
