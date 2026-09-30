//! The terminal backend.
//!
//! Everything the core is not allowed to know lives here: how to put a cell
//! on a screen (`term`), what a CP437 byte looks like in Unicode (`cp437`),
//! and how to read a directory (`dir`). An application takes the core and one
//! of these; `Examples/Desktop/Rust/01-Terminal` is what that looks like.
//!
//! `scenes` and `compare` are not for applications. They are the half of the
//! reference factory that lives on this side: scenes built here and again in
//! `REFGEN.PAS` with the real Turbo Vision units, and the comparison between
//! the two. See `src/bin/match.rs`.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod codepage;
#[cfg(feature = "std")]
pub mod compare;
pub mod cp437;
#[cfg(feature = "std")]
pub mod dir;
#[cfg(feature = "std")]
pub mod scenes;
#[cfg(feature = "term")]
pub mod term;

#[cfg(feature = "term")]
pub use term::Term;
