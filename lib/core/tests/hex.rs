//! The hex view.
//!
//! A separate view from the editor, and the reason is the data rather than the
//! drawing: an editor's content is lines and this one's is a flat run of bytes
//! with no lines in it at all. A binary forced into a list of lines has one
//! line of however many megabytes, and every operation that thought in lines
//! is then wrong.

use owlosui_core::hex::looks_binary;
use owlosui_core::cell::{glyphs, Glyph};
use owlosui_core::{Buffer, HexView, Kind, Rect, Ui, Window};

#[test]
fn text_and_binaries_are_told_apart() {
    assert!(!looks_binary(b"hello\r\n\tworld\n"));
    assert!(!looks_binary(b""), "an empty file is not a binary");

    // A NUL settles it. Text does not contain one, and every format that does
    // is a format no editor should open by accident.
    assert!(looks_binary(b"MZ\x90\x00\x03\x00"));
    assert!(looks_binary(&[0u8; 32]));

    // Failing that, a run of control characters says the same more slowly.
    assert!(looks_binary(&(1u8..=31).cycle().take(200).collect::<Vec<_>>()));

    // And a file of ordinary text with one bell in it is still text.
    let mut mostly = b"a perfectly ordinary line of text".to_vec();
    mostly.push(7);
    assert!(!looks_binary(&mostly));
}

/// Sixteen bytes to a row, whatever the window does.
///
/// The whole point of sixteen is that the low nibble of the address is the
/// column number: read the offset at the left, count along, and you know where
/// a byte is without arithmetic. A row length that changes with the window
/// takes that away, and takes with it most of what a hex dump is for.
#[test]
fn the_row_is_always_sixteen() {
    let mut h = HexView::new((0u8..=255).collect());
    for w in [40, 60, 80, 160] {
        h.layout(Rect::new(0, 0, w, 10));
        assert_eq!(h.per_row(), 16, "at width {w}");
    }
    assert_eq!(h.total_rows(), 16);
}

#[test]
fn moving_about() {
    let mut h = HexView::new((0u8..=255).collect());
    h.layout(Rect::new(0, 0, 80, 4)); // sixteen a row, four rows

    h.step(1);
    assert_eq!(h.cursor, 1);
    h.step(16); // a row down
    assert_eq!(h.cursor, 17);

    // Moving past the bottom scrolls, and only as far as it must.
    assert_eq!(h.top, 0);
    h.step(16 * 4);
    assert_eq!(h.row_of(h.cursor), 5);
    assert_eq!(h.top, 2, "scrolled just enough to show the caret");

    // The ends are ends, not wrapping points.
    h.end();
    assert_eq!(h.cursor, 255);
    h.step(100);
    assert_eq!(h.cursor, 255);
    h.home();
    assert_eq!(h.cursor, 0);
    h.step(-100);
    assert_eq!(h.cursor, 0);
}

#[test]
fn a_row_as_drawn() {
    let mut ui = Ui::new(80, 6);
    let root = ui.root();
    let mut w = Window::new("bin");
    w.closable = false;
    w.zoomable = false;
    let wid = ui.insert(root, Rect::new(0, 0, 80, 6), Kind::Window(w));

    let mut bytes: Vec<u8> = b"MZ".to_vec();
    bytes.extend_from_slice(&[0x90, 0x00, 0x03, 0x00, 0x00, 0x00]);
    bytes.extend_from_slice(b"Hello!\x07\x00");
    ui.insert(wid, Rect::default(), Kind::Hex(HexView::new(bytes)));

    let mut buf = Buffer::new(80, 6);
    ui.draw(&mut buf);

    let mut row = String::new();
    for x in 1..=62 {
        let c = buf.get(x, 1).unwrap().ch;
        row.push(if c < 128 && ((c as u8).is_ascii_graphic() || c == 32) {
            (c as u8) as char
        } else {
            '?'
        });
    }

    // Unprintable bytes are dots in the text column. A control character sent
    // to the screen as itself moves the cursor or clears the line, and a
    // viewer a file can reformat is not a viewer.
    assert_eq!(
        row,
        "00000000  4D 5A 90 00 03 00 00 00  48 65 6C 6C 6F 21 07 00 |MZ"
    );
}
