//! The terminal backend.
//!
//! Everything the core is not allowed to know lives here: how to put a cell
//! on a screen (`term`), what a CP437 byte looks like in Unicode (`cp437`),
//! and how to read a directory (`dir`). An application takes the core and one
//! of these; the examples under `Examples/Rust` are what that looks like.
//!
//! `scenes` and `compare` are not for applications. They are the half of the
//! reference factory that lives on this side: scenes built here and again in
//! `REFGEN.PAS` with the real Turbo Vision units, and the comparison between
//! the two. See `src/bin/match.rs`.

pub mod codepage;
pub mod compare;
pub mod cp437;
pub mod dir;
pub mod scenes;
pub mod term;

pub use term::Term;
