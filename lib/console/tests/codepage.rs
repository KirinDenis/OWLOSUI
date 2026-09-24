//! Code pages: the one place a byte becomes a character, tested from both
//! sides. A Cyrillic name that goes into the core through 866 must come
//! back out as the same name, and the frames must be drawn with the same
//! bytes whichever page is loaded.

use owlosui_console::codepage::CodePage;

#[test]
fn eight_six_six_carries_cyrillic_both_ways() {
    let cp = CodePage::by_number(866).unwrap();
    let s = "\u{41f}\u{440}\u{438}\u{432}\u{435}\u{442}, \u{41c}\u{438}\u{440}! \u{401}\u{451}";
    let b = cp.encode(s);
    assert_eq!(&b[..6], &[0x8F, 0xE0, 0xA8, 0xA2, 0xA5, 0xE2], "П р и в е т");
    assert_eq!(cp.decode(&b), s, "round trip");
    assert!(cp.fits(s));
    // Through the core's byte-string form and back.
    let core = cp.to_core(s);
    assert!(core.chars().all(|c| (c as u32) < 256));
    assert_eq!(core.chars().count(), s.chars().count(), "one cell per character");
    assert_eq!(cp.from_core(&core), s);
}

#[test]
fn four_three_seven_has_no_cyrillic_and_says_so() {
    let cp = CodePage::by_number(437).unwrap();
    let s = "\u{41f}\u{440}\u{438}\u{432}\u{435}\u{442}";
    assert!(!cp.fits(s));
    assert_eq!(cp.encode(s), b"??????");
    assert!(cp.fits("plain text\nwith lines"));
    assert!(cp.fits("caf\u{e9} \u{2591}\u{2592}\u{2593}"), "437 has é and the shades");
}

#[test]
fn the_frames_are_where_they_always_were() {
    // Every glyph the toolkit draws a window with lives at the same index
    // on both pages: that is why a Russian program's frames looked like an
    // American one's, and why the palette's glyph constants need no page.
    let a = CodePage::by_number(437).unwrap();
    let b = CodePage::by_number(866).unwrap();
    for i in 0xB0..=0xDFu8 {
        assert_eq!(a.to_char(i), b.to_char(i), "glyph {i:02X}");
    }
    for i in 0..0x80u8 {
        assert_eq!(a.to_char(i), b.to_char(i), "glyph {i:02X}");
    }
    // And the few from the top row the toolkit uses.
    for i in [0xFB, 0xFE, 0xF9, 0xFA] {
        assert_eq!(a.to_char(i), b.to_char(i), "glyph {i:02X}");
    }
}

#[test]
fn an_unknown_page_is_refused() {
    assert!(CodePage::by_number(1251).is_none());
    assert!(CodePage::by_number(0).is_none());
}
