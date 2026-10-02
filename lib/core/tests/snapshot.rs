//! Rendering tests.
//!
//! The output medium is a grid of characters, so a test can simply be a
//! picture of the expected screen. No terminal is involved, no backend, no
//! platform: this runs in CI on anything, and the day there is a DOS build it
//! must produce these same bytes or it is wrong.

use owlosui_core::{Buffer, Kind, Rect, TextView, Ui, Window};

/// Render the buffer as lines of ASCII, with box drawing and shading mapped to
/// stand-ins so the expected picture can be written in a source file.
fn picture(buf: &Buffer) -> String {
    let mut s = String::new();
    for y in 0..buf.height() {
        for x in 0..buf.width() {
            let c = buf.get(x, y).unwrap().ch;
            s.push(match c {
                0xB0 => '.',            // ░ desktop
                0xB1 => ':',            // ▒ scrollbar track
                0xDB => '#',            // █ scrollbar thumb
                0xC4 | 0xCD => '-',     // ─ ═
                0xB3 | 0xBA => '|',     // │ ║
                0xDA | 0xC9 => '+',     // ┌ ╔
                0xBF | 0xBB => '+',     // ┐ ╗
                0xC0 | 0xC8 => '+',     // └ ╚
                0xD9 | 0xBC => '+',     // ┘ ╝
                0x18 | 0x1E => '^',     // ↑ ▲
                0x19 | 0x1F => 'v',     // ↓ ▼
                0x10 | 0x11 => '<',     // ► ◄
                0xFE => '*',            // ■ close box
                b if b < 128 && ((b as u8).is_ascii_graphic() || b == 32) => (b as u8) as char,
                _ => '?',
            });
        }
        s.push('\n');
    }
    s
}

/// A shadow keeps the glyph and changes only the colour, so it is invisible in
/// `picture`. This one maps attributes instead: `s` wherever the shadow
/// attribute landed.
fn shadow_map(buf: &Buffer, shadow_attr: u8) -> String {
    let mut s = String::new();
    for y in 0..buf.height() {
        for x in 0..buf.width() {
            let c = buf.get(x, y).unwrap();
            s.push(if c.attr == shadow_attr { 's' } else { '.' });
        }
        s.push('\n');
    }
    s
}

#[test]
fn window_casts_a_shadow() {
    let mut ui = Ui::new(14, 7);
    let mut buf = Buffer::new(14, 7);
    let shadow = ui.palette.shadow;

    let mut w = Window::new("");
    w.closable = false;
    w.zoomable = false;
    ui.insert(ui.root(), Rect::new(2, 1, 7, 4), Kind::Window(w));
    ui.draw(&mut buf);

    // Two columns to the right starting one row down, one row below starting
    // two columns in — the offset that looks square on a non-square cell.
    assert_eq!(
        shadow_map(&buf, shadow),
        concat!(
            "..............\n",
            "..............\n",
            ".........ss...\n",
            ".........ss...\n",
            ".........ss...\n",
            "....sssssss...\n",
            "..............\n",
        )
    );
}

#[test]
fn desktop_and_one_window() {
    let mut ui = Ui::new(24, 7);
    let mut buf = Buffer::new(24, 7);

    let mut w = Window::new("Hi");
    w.zoomable = false; // keeps the top edge readable at this width
    let id = ui.insert(ui.root(), Rect::new(2, 1, 16, 5), Kind::Window(w));
    ui.insert(
        id,
        Rect::new(0, 0, 14, 3),
        Kind::Text(TextView::from_ascii("one\ntwo\nthree")),
    );

    ui.draw(&mut buf);

    assert_eq!(
        picture(&buf),
        concat!(
            "........................\n",
            "..+--- Hi -1-[*]-+......\n",
            "..|one           |......\n",
            "..|two           |......\n",
            "..|three         |......\n",
            "..+--------------+......\n",
            "........................\n",
        )
    );
}

/// The whole occlusion story: draw back to front, clip on the way down.
/// A window in front must cut a hole in the one behind it, and neither view
/// knows the other exists.
#[test]
fn front_window_clips_the_one_behind() {
    let mut ui = Ui::new(20, 6);
    let mut buf = Buffer::new(20, 6);

    for (i, x) in [1i16, 7].into_iter().enumerate() {
        let mut w = Window::new("");
        w.zoomable = false;
        w.closable = false;
        let id = ui.insert(ui.root(), Rect::new(x, 1, 10, 4), Kind::Window(w));
        let body = if i == 0 { "aaaaaaaa" } else { "bbbbbbbb" };
        ui.insert(id, Rect::new(0, 0, 8, 2), Kind::Text(TextView::from_ascii(body)));
    }

    ui.draw(&mut buf);

    let p = picture(&buf);
    // The second window sits on top, so the first one's text stops at its edge.
    assert!(p.contains(".+-----+--------+..."), "got:\n{p}");
    assert!(p.contains(".|aaaaa|bbbbbbbb|..."), "got:\n{p}");
}
