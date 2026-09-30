//! HELLO - a window, words, a button: the whole shape of a program.
//!
//!     RUN HELLO
//!
//! The same program as HELLO in `..\Asm`, `..\C` and `..\Pascal`, the
//! other way in: those ask the resident for their windows over INT 60h;
//! this one links the core and makes them itself (`kit.rs`). Then it
//! hands the core to `owlosui_dos::run`, the loop every DOS program here
//! has - draw, wait for a key or the mouse, act - until OK is pressed.

#![no_std]
#![no_main]

extern crate alloc;

mod kit;

use owlosui_core::Ui;
use owlosui_dos::{init, run, Program};

const CM_OK: u16 = 1; // the program's own command number

struct Hello {
    ui: Ui,
}

impl Program for Hello {
    fn ui(&mut self) -> &mut Ui {
        &mut self.ui
    }

    /// OK, pressed or chosen, ends it.
    fn after_input(&mut self) -> bool {
        let pressed = self.ui.take_pressed();
        let chosen = self.ui.take_command();
        pressed != Some(CM_OK) && chosen != Some(CM_OK)
    }
}

/// Where the loader jumps: `base` is where it put this program.
#[no_mangle]
#[link_section = ".text.start"]
pub extern "C" fn _start(base: u32) -> ! {
    init(base);
    let mut ui = Ui::new(80, 25); // the desktop: DOS's screen
    let win = kit::window(&mut ui, "Hello", -1, -1, 42, 9, kit::DIALOG, 0);
    kit::words(&mut ui, win, 2, 1, 36, 1, "Hello, world!");
    kit::words(&mut ui, win, 2, 3, 36, 2, "This window is drawn by a Rust core, linked into this Rust program.");
    kit::buttons(&mut ui, win, &[("~O~K", CM_OK, kit::DEFAULT)]);
    run(&mut Hello { ui })
}
