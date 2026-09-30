//! Desktop, Rust, step 2 of 2 - the same application in a native Windows window.
//! Before: 01-Terminal. The same app runs in Examples/Web/Rust and Examples/DOS/Rust.
//!
//! The window is not here. It is `lib/window`: CreateWindow, a message
//! loop and GDI, every cell drawn by hand - a screen, like the terminal
//! and the browser canvas. What is here is the whole of what a program
//! does to get one: say what its core is, say what to do after each key
//! or click, and call `run`.
//!
//! The application itself is `Examples/shared/app.rs`, the one the
//! browser and DOS builds link too.

#![windows_subsystem = "windows"]

extern crate alloc;

#[path = "../../../../shared/app.rs"]
mod app;

use owlosui_core::Ui;
use owlosui_window::{run, Options, Program};

/// The application, as the window sees it.
struct Demo(app::App);

impl Program for Demo {
    fn ui(&mut self) -> &mut Ui {
        &mut self.0.ui
    }

    /// Whatever the key or click pressed is done; the window closes when
    /// the application has answered Exit.
    fn after_input(&mut self) -> bool {
        self.0.after_input();
        !self.0.done
    }
}

fn main() {
    run(&Options { title: "OWLOSUI", ..Options::default() }, |_waker| Demo(app::App::new(80, 25)));
}
