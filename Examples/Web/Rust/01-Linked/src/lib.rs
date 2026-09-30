//! Web, Rust - the core and the application linked into one WebAssembly module.
//! The same app as Examples/Desktop/Rust/02-Window and Examples/DOS/Rust.
//!
//! The core in a browser.
//!
//! This is the whole of the browser backend on the Rust side: a handful of
//! `extern "C"` functions the page calls, and a frame it reads out of the
//! module's memory. No wasm-bindgen, no build step but `cargo build`
//! for the `wasm32-unknown-unknown` target: the core has no dependencies,
//! and a backend that needed a toolchain to say "here are the cells"
//! would be adding what the contract was written to leave out.
//!
//! The contract is the one every backend has: events go in
//! (`owl_key`, `owl_mouse`, `owl_resize`, `owl_tick`), a grid of cells
//! comes out (`owl_frame`, three bytes a cell: glyph low, glyph high,
//! attribute), and the page decides how to show it. The application
//! lives in `app.rs`, in Rust, inside the module; the page never sees a
//! window, only cells.
//!
//! One instance. A browser tab is one screen, and the functions keep the
//! `Ui` in a static rather than hand a pointer back and forth for the
//! page to lose. `static mut` is fine here: WebAssembly runs the module
//! on one thread and every call comes from that thread in order.

extern crate alloc;

#[path = "../../../../shared/app.rs"]
mod app;

use owlosui_core::{Buffer, Button, Event, Key, KeyCode, Mods, Mouse, MouseKind};

struct State {
    app: app::App,
    buf: Buffer,
    /// The frame as the page reads it: `w*h*3` bytes.
    frame: Vec<u8>,
    /// False once the application said it was done.
    running: bool,
}

static mut STATE: Option<State> = None;

#[allow(static_mut_refs)]
fn state() -> &'static mut State {
    // Safe on one thread; the page calls `owl_init` first.
    unsafe { STATE.as_mut().expect("owl_init first") }
}

/// Start, at a size in cells.
#[no_mangle]
pub extern "C" fn owl_init(w: i16, h: i16) {
    let app = app::App::new(w.max(20), h.max(8));
    #[allow(static_mut_refs)]
    unsafe {
        STATE = Some(State { app, buf: Buffer::new(w.max(20), h.max(8)), frame: Vec::new(), running: true });
    }
}

/// The page's size changed: the desktop follows, as it does on a console.
#[no_mangle]
pub extern "C" fn owl_resize(w: i16, h: i16) {
    let s = state();
    let (w, h) = (w.max(20), h.max(8));
    s.app.ui.handle(Event::Resize(w, h));
    s.buf = Buffer::new(w, h);
    s.app.after_input();
}

/// A key. `kind` 0 = a character (`value` is its code point), 1 = a
/// function key F`value`, 2 = a named key - the numbers the wire uses:
/// 0 Enter, 1 Esc, 2 Tab, 3 BackTab, 4 Backspace, 5 Delete, 6 Insert, 7
/// Home, 8 End, 9 PageUp, 10 PageDown, 11 Up, 12 Down, 13 Left, 14 Right.
/// `mods`: bit 0 shift, bit 1 ctrl, bit 2 alt.
#[no_mangle]
pub extern "C" fn owl_key(kind: u8, value: u32, mods: u8) {
    let code = match kind {
        0 => match char::from_u32(value) {
            Some(c) => KeyCode::Char(c),
            None => return,
        },
        1 => KeyCode::F(value as u8),
        _ => match value {
            0 => KeyCode::Enter,
            1 => KeyCode::Esc,
            2 => KeyCode::Tab,
            3 => KeyCode::BackTab,
            4 => KeyCode::Backspace,
            5 => KeyCode::Delete,
            6 => KeyCode::Insert,
            7 => KeyCode::Home,
            8 => KeyCode::End,
            9 => KeyCode::PageUp,
            10 => KeyCode::PageDown,
            11 => KeyCode::Up,
            12 => KeyCode::Down,
            13 => KeyCode::Left,
            14 => KeyCode::Right,
            _ => return,
        },
    };
    let mods = Mods { shift: mods & 1 != 0, ctrl: mods & 2 != 0, alt: mods & 4 != 0 };
    let s = state();
    s.app.ui.handle(Event::Key(Key { code, mods }));
    s.app.after_input();
}

/// The mouse. `kind`: 0 down, 1 up, 2 drag, 3 move, 4 wheel up, 5 wheel
/// down, 6 the second press of a double click. `button`: 0 left, 1 right,
/// 2 middle. `x`, `y` in cells.
#[no_mangle]
pub extern "C" fn owl_mouse(kind: u8, button: u8, x: i16, y: i16) {
    let b = match button {
        1 => Button::Right,
        2 => Button::Middle,
        _ => Button::Left,
    };
    let kind = match kind {
        0 => MouseKind::Down(b),
        1 => MouseKind::Up(b),
        2 => MouseKind::Drag,
        3 => MouseKind::Move,
        4 => MouseKind::ScrollUp,
        5 => MouseKind::ScrollDown,
        6 => MouseKind::Double(b),
        _ => return,
    };
    let s = state();
    s.app.ui.handle(Event::Mouse(Mouse { x, y, kind }));
    s.app.after_input();
}

/// Time passed: a button or menu item shown chosen is now delivered.
#[no_mangle]
pub extern "C" fn owl_tick() {
    let s = state();
    if s.app.ui.pick_pending() {
        s.app.ui.complete_pick();
    }
    s.app.after_input();
}

/// True while something is shown chosen and not yet delivered: draw this
/// frame, wait about 90 ms, then call `owl_tick`.
#[no_mangle]
pub extern "C" fn owl_hold() -> u8 {
    state().app.ui.pick_pending() as u8
}

/// False once the application has finished.
#[no_mangle]
pub extern "C" fn owl_running() -> u8 {
    state().running as u8
}

/// Draw, and hand back the frame: a pointer into the module's memory,
/// `owl_width() * owl_height() * 3` bytes.
#[no_mangle]
pub extern "C" fn owl_frame() -> *const u8 {
    let s = state();
    if s.app.done {
        s.running = false;
    }
    s.app.ui.draw(&mut s.buf);
    let (w, h) = (s.buf.width(), s.buf.height());
    s.frame.clear();
    s.frame.reserve((w as usize) * (h as usize) * 3);
    for y in 0..h {
        for x in 0..w {
            let c = s.buf.get(x, y).unwrap();
            s.frame.push((c.ch & 0xFF) as u8);
            s.frame.push((c.ch >> 8) as u8);
            s.frame.push(c.attr);
        }
    }
    s.frame.as_ptr()
}

#[no_mangle]
pub extern "C" fn owl_width() -> i16 {
    state().buf.width()
}

#[no_mangle]
pub extern "C" fn owl_height() -> i16 {
    state().buf.height()
}

/// Where the caret is, or -1: the page draws it, as a terminal would.
#[no_mangle]
pub extern "C" fn owl_cursor_x() -> i16 {
    state().app.ui.cursor().map_or(-1, |p| p.x)
}

#[no_mangle]
pub extern "C" fn owl_cursor_y() -> i16 {
    state().app.ui.cursor().map_or(-1, |p| p.y)
}
