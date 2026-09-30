//! DOS, Rust - a flat binary under DPMI, drawing into B800.
//! The app is Examples/shared/app.rs, the same as on the desktop window and the web.
//!
//! The core on DOS, linked in.
//!
//! This is the shortest of the screens, because the machine is the one the
//! core was shaped for: a frame is a `Vec<Cell>` of character-and-attribute
//! pairs, and video memory at B800 is exactly that, so drawing is a copy.
//! Keys come out of the BIOS keyboard buffer at 0040:001A, the mouse from
//! INT 33h, the clock is the BIOS tick at 0040:006C. All of that is
//! `lib/dos` (the crate `owlosui-dos`), which the resident OWLOSRES stands
//! on too; what is here is the program: the loop, and two ways a test
//! drives it.
//!
//! The program is a flat 32-bit binary in extended memory. `OWLOS.COM`
//! (OWLOS.ASM, and lib/dos/LOADER.INC) asks DPMI for the memory, loads the
//! file into it, makes a code and a data selector whose base is the block,
//! and jumps to `_start` with where the block is and the flags.

#![no_std]
#![no_main]

extern crate alloc;

#[path = "../../../shared/app.rs"]
mod app;

use alloc::collections::VecDeque;
use owlosui_core::{Buffer, Button, Event, Key, KeyCode, Mods, Mouse, MouseKind};
use owlosui_dos::*;

// ------------------------------------------------------------------- entry

/// Where the loader jumps: `base` is the block's linear address, `flags`
/// bit 0 /SHOT, bit 1 /QUIT, bit 2 /STRESS. On the stack as a C call
/// would put them, under a return address nobody uses - this never
/// returns; it leaves through DOS.
#[no_mangle]
#[link_section = ".text.start"]
pub extern "C" fn _start(base: u32, flags: u32) -> ! {
    init(base);
    trace(b'A');
    let mut app = app::App::new(80, 25);
    trace(b'B');
    let mut buf = Buffer::new(80, 25);
    let mut mouse = MouseState::detect();

    if flags & 4 != 0 {
        // /STRESS: what a person does to a window with a mouse for a
        // while, done in a second - drag it about, zoom it by double
        // click, open and close the dialog - so the heap is proven to
        // hold up before anyone drags anything. Then the frame goes to
        // SHOT.BIN and the code says whether it lived.
        trace(b'S');
        let title = Mouse { x: 30, y: 2, kind: MouseKind::Down(Button::Left) };
        for round in 0..20 {
            app.ui.handle(Event::Mouse(title));
            for step in 0..60i16 {
                let x = 30 + if round % 2 == 0 { step } else { 60 - step };
                app.ui.handle(Event::Mouse(Mouse { x, y: 2 + step / 20, kind: MouseKind::Drag }));
                app.after_input();
                app.ui.draw(&mut buf);
                let bytes = frame_bytes(&buf, Some((x, 2)));
                show(&bytes);
            }
            app.ui.handle(Event::Mouse(Mouse { x: 60, y: 4, kind: MouseKind::Up(Button::Left) }));
            // A double click on the title bar where the window is now:
            // zoomed, and on the next round zoomed back.
            if let Some(w) = app.ui.active_window() {
                let r = app.ui.rect(w);
                let (tx, ty) = (r.x + 8, r.y);
                app.ui.handle(Event::Mouse(Mouse { x: tx, y: ty, kind: MouseKind::Down(Button::Left) }));
                app.ui.handle(Event::Mouse(Mouse { x: tx, y: ty, kind: MouseKind::Up(Button::Left) }));
                app.ui.handle(Event::Mouse(Mouse { x: tx, y: ty, kind: MouseKind::Double(Button::Left) }));
                app.ui.handle(Event::Mouse(Mouse { x: tx, y: ty, kind: MouseKind::Up(Button::Left) }));
            }
            app.after_input();
            app.ui.handle(Event::Key(Key { code: KeyCode::F(4), mods: Mods::default() }));
            app.after_input();
            app.ui.draw(&mut buf);
            app.ui.handle(Event::Key(Key { code: KeyCode::Esc, mods: Mods::default() }));
            while app.ui.pick_pending() {
                app.ui.complete_pick();
            }
            app.after_input();
            app.ui.draw(&mut buf);
            // The window back where the drag can find it.
            app.ui.cascade();
            app.ui.draw(&mut buf);
        }
        app.ui.draw(&mut buf);
        let bytes = frame_bytes(&buf, None);
        show(&bytes);
        shot(&bytes);
        trace(b'T');
        dos_exit(0);
    }

    if flags & 1 != 0 {
        app.ui.draw(&mut buf);
        trace(b'C');
        let bytes = frame_bytes(&buf, mouse.pointer());
        show(&bytes);
        trace(b'D');
        shot(&bytes);
        trace(b'E');
        dos_exit(0);
    }

    loop {
        app.ui.draw(&mut buf);
        let bytes = frame_bytes(&buf, mouse.pointer());
        show(&bytes);
        cursor(app.ui.cursor().map(|p| (p.x, p.y)));
        if app.done {
            break;
        }
        // Something shown chosen: hold it a moment, then deliver.
        if app.ui.pick_pending() {
            hold();
            app.ui.complete_pick();
            app.after_input();
            continue;
        }
        // Wait for a key or the mouse; everything that came, into the core.
        let mut events = VecDeque::new();
        wait_input(&mut mouse, &mut events, &bytes);
        for ev in events {
            app.ui.handle(ev);
            app.after_input();
        }
    }
    // A clean screen for DOS to come back to.
    clear_screen();
    dos_exit(0)
}
