//! What goes onto Windows' clipboard comes back as the same glyphs.

#![cfg(windows)]

use owlosui_window::{cp437, from_cp437};

#[test]
fn the_clipboard_text_comes_back_as_the_same_glyphs() {
    // Glyph 0 is drawn as a space, and a space is 32: the one that is not its own.
    for g in 1..256u16 {
        assert_eq!(from_cp437(cp437(g)), g, "glyph {g} as {:?}", cp437(g));
    }
    assert_eq!(from_cp437(' '), 32);
    // Past the code page, the growing font: the character's own code point.
    assert_eq!(from_cp437('Ж'), 0x416);
    assert_eq!(from_cp437('😀'), b'?' as u16);
}
