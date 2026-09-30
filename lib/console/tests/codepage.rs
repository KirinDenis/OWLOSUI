//! The font: the one place a glyph index becomes a character, tested from
//! both sides. A Cyrillic name that goes into the core through 866 must
//! come back out as the same name; a fixed font says `?` and says so
//! beforehand; a growing one takes what it has never seen and hands out
//! the next index.

use owlosui_console::codepage::Font;

const PRIVET: &str = "\u{41f}\u{440}\u{438}\u{432}\u{435}\u{442}";

#[test]
fn eight_six_six_carries_cyrillic_both_ways() {
    let mut f = Font::fixed(866).unwrap();
    let s = "\u{41f}\u{440}\u{438}\u{432}\u{435}\u{442}, \u{41c}\u{438}\u{440}! \u{401}\u{451}";
    let g = f.encode(s);
    assert_eq!(&g[..6], &[0x8F, 0xE0, 0xA8, 0xA2, 0xA5, 0xE2], "П р и в е т");
    assert_eq!(f.decode(&g), s, "round trip");
    assert!(f.fits(s));
    assert_eq!(f.len(), 256, "a fixed font stays a code page");
    // Through the core's string form and back.
    let core = f.to_core(s);
    assert_eq!(core.chars().count(), s.chars().count(), "one cell per character");
    assert_eq!(f.from_core(&core), s);
}

#[test]
fn a_fixed_437_has_no_cyrillic_and_says_so() {
    let mut f = Font::fixed(437).unwrap();
    assert!(!f.fits(PRIVET));
    assert_eq!(f.encode(PRIVET), vec![b'?' as u16; 6]);
    assert_eq!(f.encode_known(PRIVET), vec![b'?' as u16; 6]);
    assert!(f.fits("plain text\nwith lines"));
    assert!(f.fits("caf\u{e9} \u{2591}\u{2592}\u{2593}"), "437 has é and the shades");
}

#[test]
fn a_growing_font_takes_what_it_has_never_seen() {
    let mut f = Font::growing(437).unwrap();
    assert!(f.fits(PRIVET), "a growing font fits anything");
    let g = f.encode(PRIVET);
    // Six new glyphs, past the code page, in order of arrival; the same
    // letter twice gets the same index.
    assert_eq!(g, vec![256, 257, 258, 259, 260, 261]);
    assert_eq!(f.len(), 262);
    assert_eq!(f.decode(&g), PRIVET);
    let again = f.encode("\u{442}\u{41f}");
    assert_eq!(again, vec![261, 256], "known letters keep their index");
    assert_eq!(f.len(), 262);
    // Slovak on the same screen, since there is always room for more.
    let sk = f.encode("\u{13e}\u{161}\u{10d}");
    assert_eq!(sk, vec![262, 263, 264]);
    assert_eq!(f.decode(&sk), "\u{13e}\u{161}\u{10d}");
    // What the code page had keeps its place.
    assert_eq!(f.encode("caf\u{e9}"), vec![b'c' as u16, b'a' as u16, b'f' as u16, 0x82]);
}

#[test]
fn the_frames_are_where_they_always_were() {
    // Every glyph the toolkit draws a window with lives at the same index
    // on both pages: that is why a Russian program's frames looked like an
    // American one's, and why the palette's glyph constants need no page.
    let a = Font::fixed(437).unwrap();
    let b = Font::fixed(866).unwrap();
    for i in 0xB0..=0xDFu16 {
        assert_eq!(a.to_char(i), b.to_char(i), "glyph {i:02X}");
    }
    for i in 0..0x80u16 {
        assert_eq!(a.to_char(i), b.to_char(i), "glyph {i:02X}");
    }
    for i in [0xFB, 0xFE, 0xF9, 0xFA] {
        assert_eq!(a.to_char(i), b.to_char(i), "glyph {i:02X}");
    }
}

#[test]
fn an_unknown_page_is_refused() {
    assert!(Font::fixed(1251).is_none());
    assert!(Font::growing(0).is_none());
}

/// A canvas asks for pictures: on a fixed font ☺ and ♥ are glyphs 1 and 3,
/// where text would have had to turn them into `?`.
#[test]
fn a_canvas_cell_reaches_the_pictures_below_0x20() {
    let mut f = owlosui_console::codepage::Font::fixed(437).unwrap();
    assert_eq!(f.cell_glyph('\u{263A}'), 1);
    assert_eq!(f.cell_glyph('\u{2665}'), 3);
    assert_eq!(f.cell_glyph('A'), b'A' as owlosui_core::Glyph);
    assert_eq!(f.encode("\u{263A}"), vec![b'?' as owlosui_core::Glyph]);
}
