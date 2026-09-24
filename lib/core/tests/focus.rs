//! The focus ring.
//!
//! Written because the ring was broken and nothing noticed. Tab went from the
//! field to the list and from the list to the buttons, and from the buttons
//! back to the list — so the field could not be reached again at all. Every
//! part worked; the two halves disagreed about where Tab went from the middle.
//!
//! That is the kind of fault only a person using the thing finds, which is
//! exactly the kind worth a test.

use owlosui_core::button::Align;
use owlosui_core::files::Focus;
use owlosui_core::{
    ButtonRow, Dock, Event, FileEntry, FileList, Key, KeyCode, Kind, Mods, PushButton, Rect, Ui,
    Window,
};

fn dialog() -> (Ui, owlosui_core::ViewId, owlosui_core::ViewId) {
    let mut ui = Ui::new(60, 20);
    let root = ui.root();
    let wid = ui.insert(root, Rect::new(0, 0, 60, 20), Kind::Window(Window::new("Open")));

    let entries = vec![FileEntry {
        name: "A.TXT".into(),
        size: 1,
        date: (2026, 1, 1, 0, 0),
        attrs: 0x20,
    }];
    let mut list = FileList::new(entries, "*.*");
    list.set_path("C:\\*.*");
    let fid = ui.insert(wid, Rect::default(), Kind::Files(list));

    let mut row = ButtonRow::new(vec![
        PushButton::new("~O~pen", 1).default(),
        PushButton::new("~C~ancel", 2),
    ]);
    row.align = Align::Right;
    let bid = ui.insert(wid, Rect::default(), Kind::Buttons(row));
    ui.set_dock(bid, Dock::BottomRight(24, 2));

    (ui, fid, bid)
}

/// 0 the path field, 1 the list, 2 the buttons.
fn where_is_focus(ui: &Ui, fid: owlosui_core::ViewId, bid: owlosui_core::ViewId) -> u8 {
    if matches!(ui.kind(bid), Kind::Buttons(b) if b.focused) {
        return 2;
    }
    match ui.kind(fid) {
        Kind::Files(f) if f.focus == Focus::Path => 0,
        _ => 1,
    }
}

fn tab(ui: &mut Ui) {
    ui.handle(Event::Key(Key::new(KeyCode::Tab, Mods::NONE)));
}

fn shift_tab(ui: &mut Ui) {
    ui.handle(Event::Key(Key::new(KeyCode::BackTab, Mods::NONE)));
}

#[test]
fn tab_goes_all_the_way_round() {
    let (mut ui, fid, bid) = dialog();
    if let Kind::Files(f) = ui.kind_mut(fid) {
        f.focus = Focus::Path;
        f.path.focused = true;
    }

    assert_eq!(where_is_focus(&ui, fid, bid), 0, "starts in the field");
    tab(&mut ui);
    assert_eq!(where_is_focus(&ui, fid, bid), 1, "the list");
    tab(&mut ui);
    assert_eq!(where_is_focus(&ui, fid, bid), 2, "the buttons");
    tab(&mut ui);
    assert_eq!(
        where_is_focus(&ui, fid, bid),
        0,
        "and back to the field, which is where it used to get stuck"
    );
}

#[test]
fn shift_tab_goes_the_other_way() {
    let (mut ui, fid, bid) = dialog();
    if let Kind::Files(f) = ui.kind_mut(fid) {
        f.focus = Focus::Path;
        f.path.focused = true;
    }

    shift_tab(&mut ui);
    assert_eq!(where_is_focus(&ui, fid, bid), 2);
    shift_tab(&mut ui);
    assert_eq!(where_is_focus(&ui, fid, bid), 1);
    shift_tab(&mut ui);
    assert_eq!(where_is_focus(&ui, fid, bid), 0);
}

/// A dialog with no buttons has a ring of two. It must not stop at a stop that
/// is not there.
#[test]
fn a_dialog_without_buttons_still_cycles() {
    let mut ui = Ui::new(60, 20);
    let root = ui.root();
    let wid = ui.insert(root, Rect::new(0, 0, 60, 20), Kind::Window(Window::new("Open")));
    let mut list = FileList::new(Vec::new(), "*.*");
    list.set_path("C:\\*.*");
    list.focus = Focus::Path;
    list.path.focused = true;
    let fid = ui.insert(wid, Rect::default(), Kind::Files(list));

    tab(&mut ui);
    assert!(matches!(ui.kind(fid), Kind::Files(f) if f.focus == Focus::List));
    tab(&mut ui);
    assert!(matches!(ui.kind(fid), Kind::Files(f) if f.focus == Focus::Path));
}

/// What the selection looks like says what Enter will do.
///
/// A person edits the path, presses Enter, and that is not the Open button —
/// so the list must not go on looking as though it were the thing listening.
#[test]
fn the_selection_dims_when_the_list_is_not_listening() {
    use owlosui_core::Buffer;

    let bar = |ui: &mut Ui, fid: owlosui_core::ViewId| -> u8 {
        let mut buf = Buffer::new(60, 20);
        ui.draw(&mut buf);
        let r = ui.abs_rect(fid);
        // The first name sits two rows into the panel, one column in.
        buf.get(r.x + 1, r.y + 2).unwrap().attr
    };

    let (mut ui, fid, _bid) = dialog();
    if let Kind::Files(f) = ui.kind_mut(fid) {
        f.focus = Focus::List;
        f.path.focused = false;
    }
    let listening = bar(&mut ui, fid);

    if let Kind::Files(f) = ui.kind_mut(fid) {
        f.focus = Focus::Path;
        f.path.focused = true;
    }
    let not_listening = bar(&mut ui, fid);

    assert_ne!(
        listening, not_listening,
        "the same bar in both states tells the person nothing"
    );
}

/// A letter at a file list jumps to a name. It used to be pushed into the path
/// field along with the focus, which put the letter somewhere nobody was
/// looking and moved the next keystroke somewhere else again.
#[test]
fn a_letter_jumps_to_a_name() {
    let mut ui = Ui::new(60, 20);
    let root = ui.root();
    let wid = ui.insert(root, Rect::new(0, 0, 60, 20), Kind::Window(Window::new("Open")));
    let entries: Vec<FileEntry> = ["ALPHA.TXT", "BETA.TXT", "GAMMA.TXT"]
        .iter()
        .map(|n| FileEntry {
            name: (*n).into(),
            size: 1,
            date: (2026, 1, 1, 0, 0),
            attrs: 0x20,
        })
        .collect();
    let mut list = FileList::new(entries, "*.*");
    list.set_path("C:\\*.*");
    list.focus = Focus::List;
    let fid = ui.insert(wid, Rect::default(), Kind::Files(list));

    ui.handle(Event::Key(Key::new(KeyCode::Char('g'), Mods::NONE)));
    match ui.kind(fid) {
        Kind::Files(f) => {
            assert_eq!(f.selected().unwrap().name, "GAMMA.TXT");
            assert_eq!(f.path_text(), "C:\\*.*", "the path was not typed into");
            assert_eq!(f.focus, Focus::List, "and the focus did not move");
        }
        _ => panic!("no panel"),
    }
}

/// A window must never be able to hide under the menu bar.
///
/// Dragged up far enough its title bar goes behind the bar, and then there is
/// nothing left to take hold of: the window is still there, still on top, and
/// completely out of reach. The clamp lives in the measure pass so it holds
/// for a window dragged up, one created too high, and one that was fine until
/// a menu bar appeared above it.
#[test]
fn windows_stay_below_the_menu_bar() {
    use owlosui_core::{Buffer, MenuBar, MenuItem};

    let mut ui = Ui::new(40, 20);
    let root = ui.root();
    let bar = MenuBar::new(vec![MenuItem::sub(
        "~F~ile",
        vec![PushButton::new("x", 1)]
            .into_iter()
            .map(|b| MenuItem::new(&b.text, "", b.cmd))
            .collect(),
    )]);
    ui.insert(root, Rect::new(0, 0, 40, 1), Kind::MenuBar(bar));

    assert_eq!(ui.work_area().y, 1, "row zero belongs to the bar");

    // Put a window where it would be unreachable and draw a frame.
    let wid = ui.insert(root, Rect::new(5, 0, 20, 8), Kind::Window(Window::new("W")));
    let mut buf = Buffer::new(40, 20);
    ui.draw(&mut buf);

    assert_eq!(ui.rect(wid).y, 1, "pushed down to where it can be grabbed");

    // Zoomed is the work area too, not the screen.
    ui.toggle_zoom(wid);
    ui.draw(&mut buf);
    assert_eq!(ui.rect(wid).y, 1);
    assert_eq!(ui.rect(wid).h, 19);
}

/// A button's hotkey is Alt and its letter, and it works wherever the focus
/// happens to be. A shortcut that only works when you are already standing on
/// the button is not a shortcut.
#[test]
fn alt_and_a_letter_press_a_button_from_anywhere() {
    let (mut ui, fid, _bid) = dialog();

    // From the path field, which is the case that matters: somebody has just
    // typed a name and wants to open it without reaching for the mouse.
    if let Kind::Files(f) = ui.kind_mut(fid) {
        f.focus = Focus::Path;
        f.path.focused = true;
    }
    ui.handle(Event::Key(Key::new(KeyCode::Char('o'), Mods::alt())));
    assert_eq!(ui.take_pressed(), Some(1));

    // And from the list.
    if let Kind::Files(f) = ui.kind_mut(fid) {
        f.focus = Focus::List;
    }
    ui.handle(Event::Key(Key::new(KeyCode::Char('c'), Mods::alt())));
    assert_eq!(ui.take_pressed(), Some(2));
}

/// A bare letter is not a button's business. It belongs to the path field and
/// to jumping about the list, and a button answering to one steals it from
/// both.
#[test]
fn a_bare_letter_does_not_press_a_button() {
    let (mut ui, fid, _bid) = dialog();
    if let Kind::Files(f) = ui.kind_mut(fid) {
        f.focus = Focus::Path;
        f.path.focused = true;
    }

    ui.handle(Event::Key(Key::new(KeyCode::Char('o'), Mods::NONE)));
    assert_eq!(ui.take_pressed(), None, "nothing was pressed");
    match ui.kind(fid) {
        Kind::Files(f) => assert!(
            f.path_text().ends_with('o'),
            "the letter was typed, got {:?}",
            f.path_text()
        ),
        _ => panic!("no panel"),
    }
}

/// A click both moves the focus and does whatever a click means there. Those
/// are one action to the hand and two to the program — and doing only the
/// second is how a dialog ends up answering the keyboard somewhere the eye is
/// not looking.
#[test]
fn clicking_a_control_gives_it_the_focus() {
    use owlosui_core::{Cluster, ListBox, Mouse, MouseKind, StaticText};
    use owlosui_core::Button as MouseButton;

    let mut ui = Ui::new(60, 20);
    let root = ui.root();
    let wid = ui.insert(root, Rect::new(0, 0, 60, 20), Kind::Window(Window::new("D")));

    let lab = ui.insert(wid, Rect::new(1, 1, 30, 1), Kind::Static(StaticText::new("Pick:")));
    ui.set_dock(lab, Dock::Manual);
    let list = ui.insert(
        wid,
        Rect::new(1, 3, 20, 4),
        Kind::List(ListBox::new(&["one", "two", "three", "four"])),
    );
    ui.set_dock(list, Dock::Manual);
    let cluster = ui.insert(
        wid,
        Rect::new(25, 3, 20, 2),
        Kind::Cluster(Cluster::checks(&["~A~lpha", "~B~eta"])),
    );
    ui.set_dock(cluster, Dock::Manual);

    // Draw once first. The measure pass is what tells a list how many rows it
    // has, and a program always draws a frame before it handles an event.
    let mut buf = owlosui_core::Buffer::new(60, 20);
    ui.draw(&mut buf);

    ui.focus_first();
    assert_eq!(ui.focused(), Some(list), "the first thing with anything to do");

    let click = |ui: &mut Ui, x, y| {
        ui.handle(Event::Mouse(Mouse {
            x,
            y,
            kind: MouseKind::Down(MouseButton::Left),
        }))
    };

    // The list's third row, which is both a focus change and a choice.
    click(&mut ui, 4, 6);
    assert_eq!(ui.focused(), Some(list));
    match ui.kind(list) {
        Kind::List(l) => assert_eq!(l.current, 2),
        _ => panic!(),
    }

    // The cluster's second row, on the bracket.
    click(&mut ui, 26, 5);
    assert_eq!(ui.focused(), Some(cluster));
    match ui.kind(cluster) {
        Kind::Cluster(c) => {
            assert_eq!(c.current, 1);
            assert_eq!(c.chosen(), vec![1], "the bracket was hit, so it toggled");
        }
        _ => panic!(),
    }

    // On the label, not the bracket: reading the words of an option should not
    // change the answer.
    click(&mut ui, 31, 4);
    match ui.kind(cluster) {
        Kind::Cluster(c) => {
            assert_eq!(c.current, 0);
            assert_eq!(c.chosen(), vec![1], "still only the one");
        }
        _ => panic!(),
    }

    // Static text is not in the ring, so clicking it takes the focus nowhere.
    click(&mut ui, 3, 2);
    assert_eq!(ui.focused(), Some(cluster));
}
