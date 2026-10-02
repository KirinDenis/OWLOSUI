//! The wire, exercised from the outside.
//!
//! This test is a client. It starts the real `owlosui-serve` binary, speaks
//! bytes to it through a pipe and reads bytes back, exactly as `Owlosui.cs`
//! does — so if this passes and the C# client does not, the difference is in
//! the C#, and if both fail the difference is here.

use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};

struct Client {
    child: Child,
}

impl Client {
    fn start() -> Client {
        let child = Command::new(env!("CARGO_BIN_EXE_owlosui-serve"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("start owlosui-serve");
        Client { child }
    }

    /// One request, one reply: `(status, payload)`.
    fn call(&mut self, op: u8, payload: &[u8]) -> (u8, Vec<u8>) {
        let stdin = self.child.stdin.as_mut().unwrap();
        stdin.write_all(&[op]).unwrap();
        stdin.write_all(&(payload.len() as u16).to_le_bytes()).unwrap();
        stdin.write_all(payload).unwrap();
        stdin.flush().unwrap();

        let stdout = self.child.stdout.as_mut().unwrap();
        let mut head = [0u8; 3];
        stdout.read_exact(&mut head).unwrap();
        let len = u16::from_le_bytes([head[1], head[2]]) as usize;
        let mut body = vec![0u8; len];
        stdout.read_exact(&mut body).unwrap();
        (head[0], body)
    }

    fn ok(&mut self, op: u8, payload: &[u8]) -> Vec<u8> {
        let (status, body) = self.call(op, payload);
        assert_eq!(
            status,
            0,
            "op {op:#04x} failed: {}",
            String::from_utf8_lossy(&body[2..])
        );
        body
    }

    fn err(&mut self, op: u8, payload: &[u8]) -> String {
        let (status, body) = self.call(op, payload);
        assert_eq!(status, 1, "op {op:#04x} should have failed");
        String::from_utf8_lossy(&body[2..]).into_owned()
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.call(0x00, &[]);
        let _ = self.child.wait();
    }
}

// Little helpers so a test reads as the message it sends.
fn u16(v: u16) -> [u8; 2] {
    v.to_le_bytes()
}
fn i16(v: i16) -> [u8; 2] {
    (v as u16).to_le_bytes()
}
fn s(text: &str) -> Vec<u8> {
    let mut v = u16(text.len() as u16).to_vec();
    v.extend_from_slice(text.as_bytes());
    v
}
fn rect(x: i16, y: i16, w: i16, h: i16) -> Vec<u8> {
    [i16(x), i16(y), i16(w), i16(h)].concat()
}
fn id_of(body: &[u8]) -> u16 {
    u16::from_le_bytes([body[0], body[1]])
}

struct Frame {
    w: i16,
    h: i16,
    cursor: (i16, i16),
    /// "Show this, wait, then TICK": something chosen is being shown.
    hold: bool,
    /// How long the session's font is now.
    font_len: u16,
    /// Three bytes a cell: `glyph:u16 attr:u8`.
    cells: Vec<u8>,
}

fn frame(c: &mut Client) -> Frame {
    let b = c.ok(0x40, &[]);
    let w = i16::from_le_bytes([b[0], b[1]]);
    let h = i16::from_le_bytes([b[2], b[3]]);
    let cx = i16::from_le_bytes([b[4], b[5]]);
    let cy = i16::from_le_bytes([b[6], b[7]]);
    Frame {
        w,
        h,
        cursor: (cx, cy),
        hold: b[8] != 0,
        font_len: u16::from_le_bytes([b[9], b[10]]),
        cells: b[11..].to_vec(),
    }
}

impl Frame {
    /// The glyph index of a cell.
    fn glyph(&self, x: i16, y: i16) -> u16 {
        let i = ((y * self.w + x) * 3) as usize;
        u16::from_le_bytes([self.cells[i], self.cells[i + 1]])
    }

    /// A row as characters: a glyph index below 256 as the Latin-1 char of
    /// that number (so `find` works on ASCII and frame bytes alike), and
    /// anything past the code page as U+FFFD.
    fn row(&self, y: i16) -> String {
        (0..self.w)
            .map(|x| {
                let g = self.glyph(x, y);
                if g < 256 { g as u8 as char } else { '\u{FFFD}' }
            })
            .collect()
    }
}

#[test]
fn a_window_with_words_and_a_button() {
    let mut c = Client::start();
    c.ok(0x01, &[i16(40), i16(12)].concat());

    let win = c.ok(
        0x10,
        &[
            u16(0).to_vec(),
            rect(-1, -1, 30, 7),
            vec![0x20], // grey
            s("Hello"),
        ]
        .concat(),
    );
    let win = id_of(&win);
    assert!(win > 0);

    c.ok(
        0x12,
        &[u16(win).to_vec(), rect(2, 1, 24, 1), s("Hello, world!")].concat(),
    );
    c.ok(
        0x14,
        &[u16(win).to_vec(), vec![1], u16(7).to_vec(), vec![1], s("~O~K")].concat(),
    );

    let f = frame(&mut c);
    assert_eq!((f.w, f.h), (40, 12));
    assert_eq!(f.cells.len(), 40 * 12 * 3);

    // Centred: (40 - 30) / 2 = 5, and the words are inside.
    let title_row = (0..12).find(|&y| f.row(y).contains("Hello")).expect("title");
    // Double or single frame corner - the byte comes through as Latin-1 here,
    // and it is a character position we want, not a byte offset.
    let corner = f
        .row(title_row)
        .chars()
        .position(|c| c == '\u{C9}' || c == '\u{DA}');
    assert_eq!(corner, Some(5));
    assert!(f.row(title_row + 2).contains("Hello, world!"));
    assert!(f.row(title_row + 4).contains(" OK "));

    // Enter presses the default button. The frame after it says "hold":
    // the button is drawn down and nothing has happened yet; TICK ends the
    // moment, and TAKE reports the press - once.
    c.ok(0x30, &[2, 0, 0, 0]);
    let f = frame(&mut c);
    assert!(f.hold, "the frame after a key press should ask to be held");
    let t = c.ok(0x41, &[]);
    assert_eq!(u16::from_le_bytes([t[0], t[1]]), 0, "fired before it was seen");
    c.ok(0x32, &[]);
    let f = frame(&mut c);
    assert!(!f.hold);
    let t = c.ok(0x41, &[]);
    assert_eq!(u16::from_le_bytes([t[0], t[1]]), 7);
    let t = c.ok(0x41, &[]);
    assert_eq!(u16::from_le_bytes([t[0], t[1]]), 0);
}

#[test]
fn typing_goes_in_and_text_comes_back_out() {
    let mut c = Client::start();
    c.ok(0x01, &[i16(40), i16(12)].concat());
    let win = id_of(&c.ok(
        0x10,
        &[u16(0).to_vec(), rect(0, 0, 40, 12), vec![0], s("Notes")].concat(),
    ));
    let text = id_of(&c.ok(
        0x11,
        &[u16(win).to_vec(), rect(0, 0, 0, 0), vec![0, 0], s("abc")].concat(),
    ));

    // Draw once so the editor has a size, then End and two characters.
    frame(&mut c);
    c.ok(0x30, &[2, 8, 0, 0]);
    c.ok(0x30, &[0, b'd' as u8, 0, 0]);
    c.ok(0x30, &[0, b'e' as u8, 0, 0]);

    let b = c.ok(0x21, &u16(text));
    assert_eq!(String::from_utf8_lossy(&b[2..]), "abcde");

    // The caret is where the client should show it.
    let f = frame(&mut c);
    assert_eq!(f.cursor, (1 + 5, 1));
}

#[test]
fn a_message_box_is_modal_and_answers_on_escape() {
    let mut c = Client::start();
    c.ok(0x01, &[i16(60), i16(20)].concat());
    c.ok(
        0x15,
        &[
            s("Confirm"),
            s("Leave without saving?"),
            vec![2],
            u16(1).to_vec(),
            vec![0],
            s("~L~eave"),
            u16(2).to_vec(),
            vec![1],
            s("~S~tay"),
        ]
        .concat(),
    );
    let f = frame(&mut c);
    assert!((0..20).any(|y| f.row(y).contains("Leave without saving?")));

    c.ok(0x30, &[2, 1, 0, 0]); // Esc
    c.ok(0x32, &[]); // the moment it is shown pressed
    let t = c.ok(0x41, &[]);
    assert_eq!(u16::from_le_bytes([t[0], t[1]]), 2, "Escape presses the last button");
}

/// Where some words are on the screen, as a character position.
fn find(f: &Frame, text: &str) -> Option<(i16, i16)> {
    (0..f.h).find_map(|y| {
        let row: Vec<char> = f.row(y).chars().collect();
        let pat: Vec<char> = text.chars().collect();
        row.windows(pat.len())
            .position(|w| w == pat.as_slice())
            .map(|x| (x as i16, y))
    })
}

fn corner(f: &Frame, x: i16, y: i16) -> bool {
    matches!(f.glyph(x, y), 0xBC | 0xD9)
}

fn mouse(c: &mut Client, kind: u8, x: i16, y: i16) {
    c.ok(0x31, &[vec![kind, 0], i16(x).to_vec(), i16(y).to_vec()].concat());
}

// The two things a person found that no test had: a button that the mouse
// could not press, and a corner that the mouse could not drag. Both were
// the C# client's fault and not the wire's - which is exactly what these
// establish, so the next time it happens the search starts on the right side.

#[test]
fn a_button_can_be_pressed_with_the_mouse() {
    let mut c = Client::start();
    c.ok(0x01, &[i16(40), i16(12)].concat());
    let win = id_of(&c.ok(
        0x10,
        &[u16(0).to_vec(), rect(-1, -1, 30, 7), vec![0x20], s("Hello")].concat(),
    ));
    c.ok(
        0x14,
        &[u16(win).to_vec(), vec![1], u16(7).to_vec(), vec![1], s("~O~K")].concat(),
    );
    let f = frame(&mut c);
    let (x, y) = find(&f, " OK ").expect("the button is on the screen");

    // Down and up on the label. Down alone must not fire: sliding off before
    // release is how a person changes their mind.
    mouse(&mut c, 0, x + 1, y);
    let t = c.ok(0x41, &[]);
    assert_eq!(u16::from_le_bytes([t[0], t[1]]), 0, "fired on the press");
    mouse(&mut c, 1, x + 1, y);
    let t = c.ok(0x41, &[]);
    assert_eq!(u16::from_le_bytes([t[0], t[1]]), 7, "did not fire on the release");
}

#[test]
fn a_window_can_be_resized_from_its_corner() {
    let mut c = Client::start();
    c.ok(0x01, &[i16(60), i16(20)].concat());
    c.ok(
        0x10,
        &[u16(0).to_vec(), rect(5, 3, 30, 10), vec![0], s("Notes")].concat(),
    );
    let f = frame(&mut c);
    // (5,3) with 30x10 puts the bottom-right corner at (34,12).
    assert!(corner(&f, 34, 12), "corner not where the layout says\n{}", picture(&f));

    mouse(&mut c, 0, 34, 12);
    mouse(&mut c, 2, 44, 16);
    mouse(&mut c, 1, 44, 16);

    let g = frame(&mut c);
    assert!(corner(&g, 44, 16), "the corner did not follow the mouse\n{}", picture(&g));
    assert!(!corner(&g, 34, 12), "the old corner is still there\n{}", picture(&g));
}

#[test]
fn a_window_can_be_moved_by_its_title() {
    let mut c = Client::start();
    c.ok(0x01, &[i16(60), i16(20)].concat());
    c.ok(
        0x10,
        &[u16(0).to_vec(), rect(5, 3, 30, 10), vec![0], s("Notes")].concat(),
    );
    let f = frame(&mut c);
    let (x, y) = find(&f, " Notes ").expect("the title is on the screen");
    mouse(&mut c, 0, x + 2, y);
    mouse(&mut c, 2, x + 7, y + 4);
    mouse(&mut c, 1, x + 7, y + 4);
    let g = frame(&mut c);
    assert_eq!(find(&g, " Notes "), Some((x + 5, y + 4)), "\n{}", picture(&g));
}

#[test]
fn a_click_behind_a_message_box_does_nothing() {
    let mut c = Client::start();
    c.ok(0x01, &[i16(80), i16(25)].concat());
    let win = id_of(&c.ok(
        0x10,
        &[u16(0).to_vec(), rect(2, 1, 76, 23), vec![0], s("NOTES.TXT")].concat(),
    ));
    c.ok(
        0x14,
        &[
            u16(win).to_vec(),
            vec![2],
            u16(1).to_vec(),
            vec![1],
            s("~S~ave"),
            u16(2).to_vec(),
            vec![0],
            s("E~x~it"),
        ]
        .concat(),
    );
    let f = frame(&mut c);
    let save = find(&f, "Save").expect("Save is on the screen");

    c.ok(
        0x15,
        &[
            s("Confirm"),
            s("Leave without saving?"),
            vec![2],
            u16(3).to_vec(),
            vec![0],
            s("~L~eave"),
            u16(4).to_vec(),
            vec![1],
            s("~S~tay"),
        ]
        .concat(),
    );
    let g = frame(&mut c);

    // Save is behind the box now.
    mouse(&mut c, 0, save.0, save.1);
    mouse(&mut c, 1, save.0, save.1);
    let t = c.ok(0x41, &[]);
    assert_eq!(u16::from_le_bytes([t[0], t[1]]), 0, "a button behind the modal box fired");

    // Stay, by the mouse, and the box is gone.
    let (sx, sy) = find(&g, "Stay").expect("Stay is on the screen");
    mouse(&mut c, 0, sx, sy);
    mouse(&mut c, 1, sx, sy);
    let t = c.ok(0x41, &[]);
    assert_eq!(u16::from_le_bytes([t[0], t[1]]), 4);
}

fn picture(f: &Frame) -> String {
    (0..f.h)
        .map(|y| {
            f.row(y)
                .chars()
                .map(|ch| match ch as u32 {
                    0x20..=0x7E => ch,
                    0xB0..=0xB2 => '.',
                    _ => '#',
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The Notes scene at 80x25: a document window filling the desktop but
/// for a margin, an editor in it, two buttons.
fn notes(c: &mut Client) -> u16 {
    c.ok(0x01, &[i16(80), i16(25)].concat());
    let win = id_of(&c.ok(
        0x10,
        &[u16(0).to_vec(), rect(-1, -1, 76, 23), vec![0], s("NOTES.TXT")].concat(),
    ));
    c.ok(
        0x11,
        &[u16(win).to_vec(), rect(0, 0, 0, 0), vec![0, 0], s("one\ntwo\nthree")].concat(),
    );
    c.ok(
        0x14,
        &[
            u16(win).to_vec(),
            vec![2],
            u16(1).to_vec(),
            vec![1],
            s("~S~ave"),
            u16(2).to_vec(),
            vec![0],
            s("E~x~it"),
        ]
        .concat(),
    );
    win
}

// A person resized the console window and everything fell over. The
// desktop can be asked for any size at any moment - smaller than the
// window in it, smaller than the window's own minimum, one cell, enormous -
// and with a modal box up. Each of these must come back as a frame of the
// size asked for, with the server still standing.

#[test]
fn any_resize_is_survivable() {
    let mut c = Client::start();
    notes(&mut c);
    frame(&mut c);

    for &(w, h) in &[(40i16, 12i16), (10, 3), (1, 1), (3, 2), (200, 60), (80, 25), (79, 24)] {
        c.ok(0x02, &[i16(w), i16(h)].concat());
        let f = frame(&mut c);
        assert_eq!((f.w, f.h), (w, h), "frame after resize to {w}x{h}");
        assert_eq!(f.cells.len(), (w as usize) * (h as usize) * 3);
        // And it still takes input afterwards.
        c.ok(0x30, &[2, 12, 0, 0]); // Down
        c.ok(0x31, &[0, 0, 0, 0, 0, 0]); // click at 0,0
        c.ok(0x31, &[1, 0, 0, 0, 0, 0]);
    }
}

#[test]
fn a_resize_with_a_modal_box_up_is_survivable() {
    let mut c = Client::start();
    notes(&mut c);
    c.ok(
        0x15,
        &[
            s("Confirm"),
            s("Leave without saving?"),
            vec![1],
            u16(4).to_vec(),
            vec![1],
            s("~S~tay"),
        ]
        .concat(),
    );
    frame(&mut c);
    for &(w, h) in &[(30i16, 8i16), (12, 4), (1, 1), (120, 40), (80, 25)] {
        c.ok(0x02, &[i16(w), i16(h)].concat());
        let f = frame(&mut c);
        assert_eq!((f.w, f.h), (w, h));
    }
    // Back at a sane size the box is still there and still answers.
    let f = frame(&mut c);
    assert!(find(&f, "Stay").is_some(), "the box was lost in the resizes\n{}", picture(&f));
    c.ok(0x30, &[2, 0, 0, 0]);
    c.ok(0x32, &[]);
    let t = c.ok(0x41, &[]);
    assert_eq!(u16::from_le_bytes([t[0], t[1]]), 4);
}

#[test]
fn a_window_dragged_off_the_edge_stays_reachable() {
    let mut c = Client::start();
    notes(&mut c);
    let f = frame(&mut c);
    let (x, y) = find(&f, " NOTES.TXT ").unwrap();
    // Far past every edge in turn.
    for &(dx, dy) in &[(-200i16, 0i16), (200, 0), (0, -200), (0, 200)] {
        mouse(&mut c, 0, x + 2, y);
        mouse(&mut c, 2, x + 2 + dx, y + dy);
        mouse(&mut c, 1, x + 2 + dx, y + dy);
        let g = frame(&mut c);
        // The classic desktops let a window go all but one column off the side, and
        // so does this. What matters is that the column left is part of the
        // title row, so the mouse can bring it back - which is the proof.
        let ty = (0..g.h)
            .find(|&r| {
                g.row(r)
                    .chars()
                    .any(|ch| matches!(ch as u32, 0xC9 | 0xBB | 0xCD | 0xDA | 0xBF | 0xC4))
            })
            .unwrap_or_else(|| panic!("no title row left after dragging by ({dx},{dy})\n{}", picture(&g)));
        let tx = g
            .row(ty)
            .chars()
            .position(|ch| matches!(ch as u32, 0xC9 | 0xBB | 0xCD | 0xDA | 0xBF | 0xC4))
            .unwrap() as i16;
        // Drag that cell to the opposite side of the screen, which puts the
        // window back on it whichever way it went.
        let to = (
            if dx < 0 { 78 } else if dx > 0 { 1 } else { tx },
            if dy != 0 { 5 } else { ty },
        );
        mouse(&mut c, 0, tx, ty);
        mouse(&mut c, 2, to.0, to.1);
        mouse(&mut c, 1, to.0, to.1);
        let h = frame(&mut c);
        assert!(
            find(&h, " NOTES.TXT ").is_some(),
            "could not bring the window back after dragging by ({dx},{dy})\n{}",
            picture(&h)
        );
    }
}

// ---- the first five of the missing pieces, over the wire -------------

#[test]
fn the_status_line_shows_and_binds_its_keys() {
    let mut c = Client::start();
    c.ok(0x01, &[i16(60), i16(12)].concat());
    // F1 -> 5, Alt-X -> 6 (a character with the alt bit), and a click-only Exit.
    c.ok(
        0x16,
        &[
            vec![3],
            u16(5).to_vec(), vec![1], u16(1).to_vec(), vec![0], s("~F1~ Help"),
            u16(6).to_vec(), vec![0], u16(b'x' as u16).to_vec(), vec![4], s("~Alt-X~ Exit"),
            u16(7).to_vec(), vec![0xFF], u16(0).to_vec(), vec![0], s("About"),
        ]
        .concat(),
    );
    let f = frame(&mut c);
    assert!(f.row(11).starts_with(" F1 Help  Alt-X Exit  About"), "{:?}", f.row(11));

    c.ok(0x30, &[1, 1, 0, 0]); // F1
    let t = c.ok(0x41, &[]);
    assert_eq!(u16::from_le_bytes([t[2], t[3]]), 5, "F1 did not become the command");
    c.ok(0x30, &[0, b'x', 0, 4]); // Alt+X
    let t = c.ok(0x41, &[]);
    assert_eq!(u16::from_le_bytes([t[2], t[3]]), 6);

    let (x, y) = find(&f, "About").unwrap();
    mouse(&mut c, 0, x + 1, y);
    mouse(&mut c, 1, x + 1, y);
    let t = c.ok(0x41, &[]);
    assert_eq!(u16::from_le_bytes([t[2], t[3]]), 7, "a click on the words did nothing");
}

#[test]
fn a_label_focuses_its_field_and_a_list_takes_marks() {
    let mut c = Client::start();
    c.ok(0x01, &[i16(60), i16(20)].concat());
    let win = id_of(&c.ok(
        0x10,
        &[u16(0).to_vec(), rect(0, 0, 60, 20), vec![0x20], s("Pick")].concat(),
    ));
    let name = id_of(&c.ok(
        0x13,
        &[u16(win).to_vec(), rect(8, 1, 30, 1), u16(0).to_vec(), s(""), s("")].concat(),
    ));
    let list = id_of(&c.ok(
        0x19,
        &[u16(win).to_vec(), rect(2, 4, 20, 5), vec![1], u16(3).to_vec(), s("one"), s("two"), s("three")].concat(),
    ));
    c.ok(
        0x17,
        &[u16(win).to_vec(), rect(2, 1, 6, 1), u16(name).to_vec(), s("~N~ame:")].concat(),
    );
    let f = frame(&mut c);
    assert!(find(&f, "Name:").is_some());

    // Tab to the list, mark two, read them back.
    c.ok(0x30, &[2, 2, 0, 0]); // Tab
    frame(&mut c);
    c.ok(0x30, &[2, 6, 0, 0]); // Insert
    c.ok(0x30, &[2, 6, 0, 0]);
    let m = c.ok(0x23, &u16(list));
    assert_eq!(&m[..], &[2, 0, 0, 0, 1, 0], "marked {:?}", m);
    let cur = c.ok(0x24, &u16(list));
    assert_eq!(u16::from_le_bytes([cur[0], cur[1]]), 2);

    // Alt+N goes back to the field: the caret is in it.
    c.ok(0x30, &[0, b'n', 0, 4]);
    let g = frame(&mut c);
    assert_eq!(g.cursor, (1 + 8, 1 + 1), "Alt+N did not put the caret in the field\n{}", picture(&g));
}

#[test]
fn a_progress_bar_moves() {
    let mut c = Client::start();
    c.ok(0x01, &[i16(40), i16(6)].concat());
    let win = id_of(&c.ok(
        0x10,
        &[u16(0).to_vec(), rect(0, 0, 40, 6), vec![0x20], s("Copy")].concat(),
    ));
    let bar = id_of(&c.ok(
        0x18,
        &[u16(win).to_vec(), rect(1, 1, 25, 1), 100u32.to_le_bytes().to_vec(), vec![1]].concat(),
    ));
    c.ok(0x22, &[u16(bar).to_vec(), 50u32.to_le_bytes().to_vec()].concat());
    let f = frame(&mut c);
    let line: String = f.row(2).chars().skip(2).take(25).collect();
    assert!(line.starts_with(&"\u{DB}".repeat(10)), "{line:?}");
    assert!(line.ends_with(" 50%"), "{line:?}");
    assert!(c.err(0x22, &[u16(win).to_vec(), 1u32.to_le_bytes().to_vec()].concat()).contains("not a progress bar"));
}

// ---- code pages ---------------------------------------------------------

#[test]
fn code_page_866_carries_cyrillic_both_ways() {
    let mut c = Client::start();
    // INIT with a trailing code page.
    c.ok(0x01, &[i16(60), i16(12), u16(866)].concat());
    let win = id_of(&c.ok(
        0x10,
        &[u16(0).to_vec(), rect(0, 0, 60, 12), vec![0], s("\u{41e}\u{442}\u{447}\u{451}\u{442}")].concat(),
    ));
    let text = id_of(&c.ok(
        0x11,
        &[u16(win).to_vec(), rect(0, 0, 0, 0), vec![0, 0], s("\u{41f}\u{440}\u{438}\u{432}\u{435}\u{442}")].concat(),
    ));
    let f = frame(&mut c);
    // The bytes on the screen are 866's: П р и в е т.
    let row: Vec<u16> = (1..7).map(|x| f.glyph(x, 1)).collect();
    assert_eq!(row, vec![0x8F, 0xE0, 0xA8, 0xA2, 0xA5, 0xE2], "{}", picture(&f));
    assert_eq!(f.font_len, 256, "every letter was on the page; nothing grew");

    // Typed in, read back out: the same letters.
    c.ok(0x30, &[2, 8, 0, 0]); // End
    c.ok(0x30, &[0, 0x36, 0x04, 0]); // 'ж' U+0436, as a character
    let b = c.ok(0x21, &u16(text));
    assert_eq!(String::from_utf8_lossy(&b[2..]), "\u{41f}\u{440}\u{438}\u{432}\u{435}\u{442}\u{436}");

    // And the table a client draws with says so.
    let g = c.ok(0x42, &[]);
    assert_eq!(g[0], 1, "the font grows");
    assert_eq!(u16::from_le_bytes([g[1], g[2]]), 256);
    let at = |i: usize| u16::from_le_bytes([g[3 + i * 2], g[4 + i * 2]]);
    assert_eq!(at(0x80), 0x0410, "0x80 is А");
    assert_eq!(at(0xC4), 0x2500, "0xC4 is still ─");
}

#[test]
fn the_font_grows_past_its_code_page_and_every_frame_says_how_long_it_is() {
    let mut c = Client::start();
    c.ok(0x01, &[i16(60), i16(12)].concat());
    let f0 = frame(&mut c);
    assert_eq!(f0.font_len, 256, "437 to begin with");
    let win = id_of(&c.ok(
        0x10,
        &[u16(0).to_vec(), rect(0, 0, 60, 12), vec![0], s("Notes")].concat(),
    ));
    // Russian and Slovak in one text, on a 437 session: neither is on the
    // page, both get glyphs.
    let mixed = "\u{41f}\u{440}\u{438} \u{13e}\u{161}\u{10d} ok";
    let text = id_of(&c.ok(
        0x11,
        &[u16(win).to_vec(), rect(0, 0, 0, 0), vec![0, 0], s(mixed)].concat(),
    ));
    let f = frame(&mut c);
    assert_eq!(f.font_len, 256 + 6, "six new glyphs");
    assert_eq!(f.glyph(1, 1), 256, "the first new character got the first new index");
    assert_eq!(f.glyph(5, 1), 259);
    assert!(f.row(1).contains(" ok"), "{}", picture(&f));
    let b = c.ok(0x21, &u16(text));
    assert_eq!(String::from_utf8_lossy(&b[2..]), mixed, "the text came back changed");

    // The table has them, in order of arrival.
    let g = c.ok(0x42, &[]);
    assert_eq!(u16::from_le_bytes([g[1], g[2]]), 262);
    let at = |i: usize| u16::from_le_bytes([g[3 + i * 2], g[4 + i * 2]]);
    assert_eq!(at(256), 0x41f);
    assert_eq!(at(261), 0x10d);

    // Switching the page is a request like any other, and a page that is
    // not there is refused.
    c.ok(0x03, &u16(866));
    assert!(c.err(0x03, &u16(1251)).contains("no code page"));
}

#[test]
fn a_cyrillic_file_name_survives_the_panel() {
    let mut c = Client::start();
    c.ok(0x01, &[i16(60), i16(20), u16(866)].concat());
    let win = id_of(&c.ok(
        0x10,
        &[u16(0).to_vec(), rect(0, 0, 60, 20), vec![0], s("Open")].concat(),
    ));
    let name = "\u{41e}\u{442}\u{447}\u{451}\u{442}.txt";
    let entry = [
        s(name),
        7u32.to_le_bytes().to_vec(),
        u16(2026).to_vec(),
        vec![9, 24, 12, 0, 0x20],
    ]
    .concat();
    let files = id_of(&c.ok(
        0x1A,
        &[u16(win).to_vec(), rect(0, 0, 0, 0), vec![1], s("*.*"), s("C:\\*.*"), u16(1).to_vec(), entry].concat(),
    ));
    frame(&mut c);
    let m = c.ok(0x28, &u16(files));
    assert_eq!(u16::from_le_bytes([m[0], m[1]]), 1);
    assert_eq!(String::from_utf8_lossy(&m[4..]), name, "the name came back changed");
}

#[test]
fn a_window_may_be_asked_to_cast_no_shadow() {
    let mut c = Client::start();
    c.ok(0x01, &[i16(60), i16(20)].concat());
    // Two windows at the same place, one after the other: with a shadow
    // the cells just past the right edge are darkened; without, they are
    // the desktop as it was.
    let desktop_attr = frame(&mut c).cells[2];
    let shadowed = id_of(&c.ok(
        0x10,
        &[u16(0).to_vec(), rect(2, 2, 20, 6), vec![0], s("A")].concat(),
    ));
    let f = frame(&mut c);
    let attr = |f: &Frame, x: i16, y: i16| f.cells[((y * f.w + x) * 3 + 2) as usize];
    assert_ne!(attr(&f, 22, 3), desktop_attr, "no shadow to the right of an ordinary window");
    c.ok(0x20, &u16(shadowed));

    c.ok(
        0x10,
        &[u16(0).to_vec(), rect(2, 2, 20, 6), vec![0x40], s("B")].concat(),
    );
    let g = frame(&mut c);
    assert_eq!(attr(&g, 22, 3), desktop_attr, "flag 0x40 should leave the desktop beside it alone");
    assert_eq!(attr(&g, 3, 8), desktop_attr, "and below it");
}

#[test]
fn a_canvas_shows_the_cells_it_was_given_and_a_static_takes_new_words() {
    let mut c = Client::start();
    c.ok(0x01, &[i16(40), i16(12)].concat());
    let win = id_of(&c.ok(
        0x10,
        &[u16(0).to_vec(), rect(0, 0, 40, 12), vec![0], s("Board")].concat(),
    ));
    let canvas = id_of(&c.ok(0x1B, &[u16(win).to_vec(), rect(1, 1, 10, 3)].concat()));
    let label = id_of(&c.ok(
        0x12,
        &[u16(win).to_vec(), rect(1, 5, 20, 1), s("moves 0")].concat(),
    ));

    // A 3x2 block at (2,1): wall, box, keeper / floor, floor, floor.
    let mut cells = Vec::new();
    for (ch, attr) in [('\u{2593}', 0x07u8), ('\u{2592}', 0x0E), ('@', 0x0F), ('\u{2591}', 0x08), ('\u{2591}', 0x08), ('.', 0x0E)] {
        cells.extend_from_slice(&u16(ch as u16));
        cells.push(attr);
    }
    c.ok(0x2C, &[u16(canvas).to_vec(), i16(2).to_vec(), i16(1).to_vec(), i16(3).to_vec(), i16(2).to_vec(), cells].concat());
    let f = frame(&mut c);
    // The canvas sits at the window's (1,1) -> screen (2,2); the block at
    // (2,1) inside it -> screen (4,3).
    assert_eq!(f.glyph(4, 3), 0xB2, "the wall glyph\n{}", picture(&f));
    assert_eq!(f.glyph(5, 3), 0xB1);
    assert_eq!(f.glyph(6, 3), b'@' as u16);
    let attr = |f: &Frame, x: i16, y: i16| f.cells[((y * f.w + x) * 3 + 2) as usize];
    assert_eq!(attr(&f, 5, 3), 0x0E, "the box is yellow");
    assert_eq!(attr(&f, 4, 4), 0x08, "the floor is dark grey");
    // Outside the block the canvas is its blank self, not the window.
    assert_eq!(f.glyph(2, 2), b' ' as u16);

    // New words in the same static, same place.
    c.ok(0x2B, &[u16(label).to_vec(), s("moves 1")].concat());
    let g = frame(&mut c);
    assert!(find(&g, "moves 1").is_some(), "{}", picture(&g));
    assert!(find(&g, "moves 0").is_none());

    // A block half off the canvas is trimmed, not refused.
    let mut two = Vec::new();
    for _ in 0..2 {
        two.extend_from_slice(&u16(b'#' as u16));
        two.push(0x07);
    }
    c.ok(0x2C, &[u16(canvas).to_vec(), i16(9).to_vec(), i16(0).to_vec(), i16(2).to_vec(), i16(1).to_vec(), two].concat());
    let h = frame(&mut c);
    assert_eq!(h.glyph(2 + 9, 2), b'#' as u16);
    assert!(c.err(0x2C, &[u16(label).to_vec(), i16(0).to_vec(), i16(0).to_vec(), i16(0).to_vec(), i16(0).to_vec()].concat()).contains("not a canvas"));
}

/// `flags cmd text shortcut sub...` for one plain item.
fn item(flags: u8, cmd: u16, text: &str, shortcut: &str, sub: Vec<u8>) -> Vec<u8> {
    [vec![flags], u16(cmd).to_vec(), s(text), s(shortcut), s(""), sub].concat()
}

/// The same, with a hint for the status line.
fn hinted(cmd: u16, text: &str, hint: &str) -> Vec<u8> {
    [vec![0], u16(cmd).to_vec(), s(text), s(""), s(hint), vec![0]].concat()
}

#[test]
fn a_menu_bar_over_the_wire_opens_chooses_and_ticks() {
    let mut c = Client::start();
    c.ok(0x01, &[i16(60), i16(20)].concat());
    // File: Open, ---, Exit.  Options: Clock (ticked), More > Deep(9).
    let file = [vec![3], item(0, 1, "~O~pen", "F3", vec![0]), item(1, 0, "", "", vec![0]), item(0, 2, "E~x~it", "Alt+X", vec![0])].concat();
    let more = [vec![1], item(0, 9, "~D~eep", "", vec![0])].concat();
    let options = [vec![2], item(4, 5, "~C~lock", "", vec![0]), item(0, 0, "~M~ore", "", more)].concat();
    let bar = [vec![2], item(0, 0, "~F~ile", "", file), item(0, 0, "~O~ptions", "", options)].concat();
    c.ok(0x1C, &bar);
    let f = frame(&mut c);
    assert!(f.row(0).contains("File") && f.row(0).contains("Options"), "{}", picture(&f));

    // F10 opens File; Down, Down (over the line) lands on Exit; Enter.
    c.ok(0x30, &[1, 10, 0, 0]);
    let g = frame(&mut c);
    assert!(find(&g, "Open").is_some() && find(&g, "Exit").is_some(), "{}", picture(&g));
    c.ok(0x30, &[2, 12, 0, 0]);
    c.ok(0x30, &[2, 0, 0, 0]);
    let t = c.ok(0x41, &[]);
    assert_eq!(u16::from_le_bytes([t[2], t[3]]), 2, "Exit was not chosen");

    // Alt+O opens Options; the tick is there; Right on More opens Deep.
    c.ok(0x30, &[0, b'o' as u16 as u8, 0, 4]);
    let g = frame(&mut c);
    let (cx, cy) = find(&g, "Clock").unwrap();
    assert_eq!(g.glyph(cx - 1, cy), 0xFB, "no tick before Clock\n{}", picture(&g));
    c.ok(0x30, &[2, 12, 0, 0]);
    c.ok(0x30, &[2, 14, 0, 0]);
    let h = frame(&mut c);
    assert!(find(&h, "Deep").is_some(), "the submenu did not open\n{}", picture(&h));
    c.ok(0x30, &[2, 0, 0, 0]);
    let t = c.ok(0x41, &[]);
    assert_eq!(u16::from_le_bytes([t[2], t[3]]), 9);

    // Untick the clock and look again.
    c.ok(0x2D, &[u16(5).to_vec(), vec![0]].concat());
    c.ok(0x30, &[0, b'o' as u16 as u8, 0, 4]);
    let g = frame(&mut c);
    let (cx, cy) = find(&g, "Clock").unwrap();
    assert_ne!(g.glyph(cx - 1, cy), 0xFB, "the tick did not come off");
    assert!(c.err(0x2D, &[u16(77).to_vec(), vec![1]].concat()).contains("no menu item"));
}

#[test]
fn clusters_canvas_clicks_next_and_zoom_over_the_wire() {
    let mut c = Client::start();
    c.ok(0x01, &[i16(60), i16(20)].concat());
    let a = id_of(&c.ok(0x10, &[u16(0).to_vec(), rect(0, 0, 30, 10), vec![0], s("A")].concat()));
    let b = id_of(&c.ok(0x10, &[u16(0).to_vec(), rect(20, 5, 30, 10), vec![0x20], s("B")].concat()));

    // Check boxes in B: Space toggles the first, Down + Space the second.
    let checks = id_of(&c.ok(
        0x1D,
        &[u16(b).to_vec(), rect(1, 1, 20, 2), vec![0, 2], s("~R~everse"), s("~S~how")].concat(),
    ));
    frame(&mut c);
    c.ok(0x30, &[0, b' ' as u16 as u8, 0, 0]);
    c.ok(0x30, &[2, 12, 0, 0]);
    c.ok(0x30, &[0, b' ' as u16 as u8, 0, 0]);
    let st = c.ok(0x2E, &u16(checks));
    assert_eq!(&st[..], &[2, 1, 1, 1], "on, on, cursor on the second: {st:?}");

    // Radio buttons: exactly one, the first to begin with.
    let radio = id_of(&c.ok(
        0x1D,
        &[u16(b).to_vec(), rect(1, 4, 20, 2), vec![1, 2], s("~L~ow"), s("~H~igh")].concat(),
    ));
    let st = c.ok(0x2E, &u16(radio));
    assert_eq!(&st[..2], &[2, 1]);

    // A canvas in A remembers a click, once, in its own cells.
    let cv = id_of(&c.ok(0x1B, &[u16(a).to_vec(), rect(2, 2, 10, 4)].concat()));
    frame(&mut c);
    mouse(&mut c, 0, 1 + 2 + 3, 1 + 2 + 1);
    mouse(&mut c, 1, 1 + 2 + 3, 1 + 2 + 1);
    let k = c.ok(0x2F, &u16(cv));
    assert_eq!(&k[..], &[1, 3, 0, 1, 0], "{k:?}");
    let k = c.ok(0x2F, &u16(cv));
    assert_eq!(k[0], 0, "a click is reported once");

    // The click also brought A to the front; CYCLE sends it back.
    let act = c.ok(0x2A, &[]);
    assert_eq!(u16::from_le_bytes([act[0], act[1]]), a);
    c.ok(0x43, &[]);
    let act = c.ok(0x2A, &[]);
    assert_eq!(u16::from_le_bytes([act[0], act[1]]), b);

    // ZOOM fills the work area and back.
    c.ok(0x44, &u16(b));
    let f = frame(&mut c);
    assert!(corner(&f, 59, 19), "not zoomed\n{}", picture(&f));
    c.ok(0x44, &u16(b));
    let f = frame(&mut c);
    assert!(corner(&f, 49, 14), "not back\n{}", picture(&f));
}

fn attr_at(f: &Frame, x: i16, y: i16) -> u8 {
    f.cells[((y * f.w + x) * 3 + 2) as usize]
}

/// A row placed by hand: `parent rect n` then `cmd flags label` each.
fn button_row(c: &mut Client, win: u16, y: i16, keys: &[(u16, u8, &str)]) -> u16 {
    // Row flags 1: not a Tab stop.
    let mut p = [u16(win).to_vec(), rect(1, y, 0, 2), vec![1, keys.len() as u8]].concat();
    for (cmd, flags, label) in keys {
        p.extend(u16(*cmd));
        p.push(*flags);
        p.extend(s(label));
    }
    id_of(&c.ok(0x1E, &p))
}

fn take(c: &mut Client) -> u16 {
    let t = c.ok(0x41, &[]);
    u16::from_le_bytes([t[0], t[1]])
}

#[test]
fn a_keypad_of_placed_rows_over_the_wire() {
    let mut c = Client::start();
    c.ok(0x01, &[i16(60), i16(20)].concat());
    let win = id_of(&c.ok(0x10, &[u16(0).to_vec(), rect(-1, -1, 50, 12), vec![0x20], s("Calc")].concat()));
    // A display first, so it has the focus.
    let display = id_of(&c.ok(0x13, &[u16(win).to_vec(), rect(1, 0, 30, 1), u16(0).to_vec(), s(""), s("")].concat()));
    // Flags: bit0 default, bits1-2 style (1 accent, 2 danger), bit3 disabled.
    // C: danger and cancel (bit 4). A: disabled.
    let row1 = button_row(&mut c, win, 2, &[(22, 4 | 16, " C "), (7, 0, " 7 "), (10, 8, " A ")]);
    let _row2 = button_row(&mut c, win, 4, &[(20, 2, " + "), (21, 1 | 2, " = ")]);

    let f = frame(&mut c);
    let (cx, cy) = find(&f, "  C  ").expect("C on screen");
    let (sx, sy) = find(&f, "  7  ").expect("7 on screen");
    let (ax, ay) = find(&f, "  A  ").expect("A on screen");
    let (px, py) = find(&f, "  +  ").expect("+ on screen");
    assert_eq!(sy, cy, "one row");
    assert_eq!(py, cy + 2, "the next row two below");
    assert_eq!(attr_at(&f, cx + 2, cy), 0x4F, "danger: white on red");
    assert_eq!(attr_at(&f, sx + 2, sy), 0x20, "normal: black on green");
    assert_eq!(attr_at(&f, px + 2, py), 0x30, "accent: black on cyan");
    assert_eq!(attr_at(&f, ax + 2, ay), 0x78, "disabled: dark grey on grey");
    // The last key of a row keeps its right-hand shadow.
    assert_eq!(f.glyph(ax + 6, ay), 0xDD, "the shadow beside the last key");
    // Only = wears the default marks: a placed row implies no default.
    assert!(find(&f, "\u{10}  =  \u{11}").is_some(), "= is the default");
    assert!(find(&f, "\u{10}  C  \u{11}").is_none(), "C is not");

    // A click on 7 presses it and the display keeps the caret: the next key
    // typed lands in it.
    mouse(&mut c, 0, sx + 2, sy);
    mouse(&mut c, 1, sx + 2, sy);
    assert_eq!(take(&mut c), 7);
    c.ok(0x30, &[0, b'8', 0, 0]);
    let t = c.ok(0x21, &u16(display));
    assert_eq!(&t[2..], b"8", "the 8 went into the display, not into the keypad");
    // Enter presses the default in the placed row; Escape the cancel one.
    c.ok(0x30, &[2, 0, 0, 0]);
    c.ok(0x32, &[]);
    assert_eq!(take(&mut c), 21, "Enter found = in a placed row");
    c.ok(0x30, &[2, 1, 0, 0]);
    c.ok(0x32, &[]);
    assert_eq!(take(&mut c), 22, "Escape found C");
    // Tab does not stop at the keypad: the display is the only stop, so
    // Tab leaves the focus where it was and the next key still types.
    c.ok(0x30, &[2, 2, 0, 0]);
    c.ok(0x30, &[0, b'9', 0, 0]);
    let t = c.ok(0x21, &u16(display));
    assert_eq!(&t[2..], b"89", "after Tab the 9 still went into the display");
    // FOCUS puts the caret on a control by its id, and refuses a stranger.
    c.ok(0x46, &u16(display));
    let (status, _) = c.call(0x46, &u16(9999));
    assert_ne!(status, 0);
    // A disabled key does not press; enabled by SET_BUTTON, it does.
    mouse(&mut c, 0, ax + 2, ay);
    mouse(&mut c, 1, ax + 2, ay);
    assert_eq!(take(&mut c), 0, "disabled");
    c.ok(0x45, &[u16(row1).to_vec(), vec![2, 1]].concat());
    mouse(&mut c, 0, ax + 2, ay);
    mouse(&mut c, 1, ax + 2, ay);
    assert_eq!(take(&mut c), 10, "enabled");
    let (status, _) = c.call(0x45, &[u16(row1).to_vec(), vec![9, 1]].concat());
    assert_ne!(status, 0, "no button 9 is an error, not a crash");
}

#[test]
fn cascade_and_tile_over_the_wire() {
    let mut c = Client::start();
    c.ok(0x01, &[i16(80), i16(25)].concat());
    for t in ["A", "B"] {
        c.ok(0x10, &[u16(0).to_vec(), rect(10, 5, 30, 8), vec![0], s(t)].concat());
    }
    c.ok(0x48, &[]);
    let f = frame(&mut c);
    // Two windows tiled: one column, two rows; the corners say where.
    assert!(corner(&f, 79, 11), "A's corner: rows 0..11 of 25");
    assert!(corner(&f, 79, 24), "B's corner");
    c.ok(0x47, &[]);
    let f = frame(&mut c);
    assert!(find(&f, " A ").is_some() && find(&f, " B ").is_some(), "both titles show after a cascade");
    assert!(corner(&f, 79, 24), "A fills the work area");
    assert!(corner(&f, 79, 24) && f.glyph(1, 1) != b' ' as u16, "B starts one cell in");
    let (_, by) = find(&f, " B ").unwrap();
    let (_, ay) = find(&f, " A ").unwrap();
    assert_eq!((ay, by), (0, 1), "A's title on row 0, B's a row lower");
}

#[test]
fn a_viewer_becomes_an_editor_and_a_hex_dump_shows_bytes() {
    let mut c = Client::start();
    c.ok(0x01, &[i16(80), i16(25)].concat());
    let win = id_of(&c.ok(0x10, &[u16(0).to_vec(), rect(0, 0, 60, 10), vec![0], s("A.TXT")].concat()));
    // Read-only text: flags bit 0.
    let text = id_of(&c.ok(0x11, &[u16(win).to_vec(), rect(0, 0, 0, 0), vec![0, 1], s("hello")].concat()));
    let f = frame(&mut c);
    assert!(find(&f, " A.TXT [view] ").is_some(), "a viewer says so");
    c.ok(0x30, &[0, b'x', 0, 0]);
    let t = c.ok(0x21, &u16(text));
    assert_eq!(&t[2..], b"hello", "typing into a viewer does nothing");
    // SET_READONLY off: an editor now.
    c.ok(0x49, &[u16(text).to_vec(), vec![0]].concat());
    let f = frame(&mut c);
    assert!(find(&f, " A.TXT ").is_some() && find(&f, "[view]").is_none(), "the tag is gone");
    c.ok(0x30, &[0, b'x', 0, 0]);
    let t = c.ok(0x21, &u16(text));
    assert_eq!(&t[2..], b"xhello", "typing lands in the editor");
    c.ok(0x49, &[u16(text).to_vec(), vec![1]].concat());
    assert!(find(&frame(&mut c), "[view]").is_some());
    let (status, _) = c.call(0x49, &[u16(win).to_vec(), vec![1]].concat());
    assert_ne!(status, 0, "a window is not a text");

    // A hex window of a few bytes.
    let hw = id_of(&c.ok(0x10, &[u16(0).to_vec(), rect(0, 12, 70, 8), vec![0], s("A.BIN")].concat()));
    c.ok(0x1F, &[u16(hw).to_vec(), rect(0, 0, 0, 0), b"Hello".to_vec()].concat());
    let f = frame(&mut c);
    assert!(find(&f, " A.BIN [hex] ").is_some(), "a hex window says so");
    assert!(find(&f, "00000000").is_some(), "the offset column");
    assert!(find(&f, "48 65 6C 6C 6F").is_some(), "the bytes");
}

#[test]
fn an_editor_offers_its_edit_menu_and_reports_how_it_is_set() {
    let mut c = Client::start();
    c.ok(0x01, &[i16(80), i16(25)].concat());
    // A bar with File only, and a window with an editor in it.
    let file = item(0, 0, "~F~ile", "", [vec![1], item(0, 1, "~N~ew", "", vec![0])].concat());
    c.ok(0x1C, &[vec![1], file].concat());
    let win = id_of(&c.ok(0x10, &[u16(0).to_vec(), rect(0, 1, 30, 10), vec![0], s("A.TXT")].concat()));
    let text = id_of(&c.ok(0x11, &[u16(win).to_vec(), rect(0, 0, 0, 0), vec![0, 0], s("one two three four five six seven")].concat()));
    assert!(find(&frame(&mut c), "Edit").is_none(), "nothing offered yet");
    // EDITOR: everything offered, word wrap on.
    c.ok(0x5D, &[u16(text).to_vec(), vec![0x7F, 1]].concat());
    let f = frame(&mut c);
    assert_eq!(find(&f, "Edit").map(|p| p.1), Some(0), "an Edit menu on the bar");
    // Twenty-eight columns inside: "seven" goes down to a row of its own.
    assert_eq!(find(&f, "one two three four five six ").map(|p| p.1), Some(2), "{}", picture(&f));
    assert_eq!(find(&f, "seven").map(|p| p.1), Some(3), "folded");
    // GET_EDITOR: offers, state, line, column.
    let st = c.ok(0x5E, &u16(text));
    assert_eq!(&st[..2], &[0x7F, 1]);
    // The person turns on Read only from the menu: Alt+E, O.
    c.ok(0x30, &[0, b'e', 0, 4]);
    c.ok(0x30, &[0, b'o', 0, 0]);
    // A menu pick shows for a moment before it acts: TICK delivers it.
    c.ok(0x32, &[]);
    c.ok(0x41, &[]);
    let st = c.ok(0x5E, &u16(text));
    assert_eq!(st[1], 1 | 2, "wrap and read only: {st:?}");
    let (status, _) = c.call(0x5D, &[u16(win).to_vec(), vec![0, 0]].concat());
    assert_ne!(status, 0, "a window is not a text");
}

#[test]
fn a_text_is_coloured_as_a_language_and_keeps_it_through_set_text() {
    let mut c = Client::start();
    c.ok(0x01, &[i16(80), i16(25)].concat());
    let win = id_of(&c.ok(0x10, &[u16(0).to_vec(), rect(0, 0, 40, 10), vec![0], s("A.PAS")].concat()));
    let text = id_of(&c.ok(0x11, &[u16(win).to_vec(), rect(0, 0, 0, 0), vec![0, 0], s("begin end.")].concat()));
    // SYNTAX by file name: found, and the language's name.
    let r = c.ok(0x5F, &[u16(text).to_vec(), s("A.PAS")].concat());
    assert_eq!(r[0], 1);
    assert_eq!(&r[3..], b"Pascal");
    let f = frame(&mut c);
    assert_eq!(attr_at(&f, 1, 1), 0x1F, "begin is a keyword: white on blue");
    assert_eq!(attr_at(&f, 7, 1), 0x1F, "end too");
    // SET_TEXT: new words, the same language and the same offers.
    c.ok(0x5D, &[u16(text).to_vec(), vec![0x7F, 1]].concat());
    c.ok(0x2B, &[u16(text).to_vec(), s("{ note }")].concat());
    let f = frame(&mut c);
    assert_eq!(attr_at(&f, 1, 1), 0x17, "a comment: light grey");
    let st = c.ok(0x5E, &u16(text));
    assert_eq!(&st[..2], &[0x7F, 1 | 16], "offers, wrap and the colours survived");
    // Nobody answers to it: not found, and plain again.
    let r = c.ok(0x5F, &[u16(text).to_vec(), s("A.XYZ")].concat());
    assert_eq!(r[0], 0);
    assert_eq!(attr_at(&frame(&mut c), 1, 1), 0x1E, "the editor's yellow");
    // A language of the program's own.
    let r = c.ok(0x60, &s("[Talk]\nfiles = TLK\nkeywords = note\n"));
    assert_eq!(r[0], 1);
    c.ok(0x5F, &[u16(text).to_vec(), s("Talk")].concat());
    assert_eq!(attr_at(&frame(&mut c), 3, 1), 0x1F, "note is a Talk keyword");
}

#[test]
fn a_window_carries_its_keys_and_menu_items_over_the_wire() {
    let mut c = Client::start();
    c.ok(0x01, &[i16(80), i16(25)].concat());
    // A bar with File and Options, and a status line with F3.
    c.ok(0x1C, &[vec![2], item(0, 0, "~F~ile", "", [vec![1], item(0, 1, "~O~pen", "F3", vec![0])].concat()),
                 item(0, 0, "~O~ptions", "", [vec![1], item(0, 2, "~M~ouse...", "", vec![0])].concat())].concat());
    c.ok(0x16, &[vec![1], u16(1).to_vec(), vec![1], u16(3).to_vec(), vec![0], s("~F3~ Open")].concat());
    let win = id_of(&c.ok(0x10, &[u16(0).to_vec(), rect(2, 2, 40, 10), vec![0], s("A.TXT")].concat()));
    // WINDOW_STATUS: F4 Edit, command 10. WINDOW_MENU: into Options.
    c.ok(0x4A, &[u16(win).to_vec(), vec![1], u16(10).to_vec(), vec![1], u16(4).to_vec(), vec![0], s("~F4~ Edit")].concat());
    c.ok(0x4B, &[u16(win).to_vec(), vec![1], item(0, 0, "~O~ptions", "", [vec![1], item(0, 10, "~E~dit / view", "F4", vec![0])].concat())].concat());
    let f = frame(&mut c);
    assert!(f.row(24).contains("F3 Open") && f.row(24).contains("F4 Edit"), "{:?}", f.row(24));
    c.ok(0x30, &[1, 4, 0, 0]);
    let t = c.ok(0x41, &[]);
    assert_eq!(u16::from_le_bytes([t[2], t[3]]), 10, "F4 is the window's command while it is active");
    // Alt+O: the Options panel holds Mouse... and Edit / view.
    c.ok(0x30, &[0, b'o', 0, 4]);
    let f = frame(&mut c);
    assert!(find(&f, "Mouse...").is_some() && find(&f, "Edit / view").is_some(), "the window's item is not in Options");
    c.ok(0x30, &[2, 1, 0, 0]);
    // Another window in front: F4 gone from the line and unbound.
    c.ok(0x10, &[u16(0).to_vec(), rect(10, 5, 40, 10), vec![0], s("B")].concat());
    let f = frame(&mut c);
    assert!(!f.row(24).contains("F4"), "{:?}", f.row(24));
    c.ok(0x30, &[1, 4, 0, 0]);
    let t = c.ok(0x41, &[]);
    assert_eq!(u16::from_le_bytes([t[2], t[3]]), 0, "F4 still bound with B in front");
    let (status, _) = c.call(0x4A, &[u16(9999).to_vec(), vec![0]].concat());
    assert_ne!(status, 0);
}

#[test]
fn a_double_click_over_the_wire_zooms_a_title_bar() {
    let mut c = Client::start();
    c.ok(0x01, &[i16(80), i16(25)].concat());
    c.ok(0x10, &[u16(0).to_vec(), rect(10, 5, 40, 10), vec![0], s("Doc")].concat());
    // Down, up, double (kind 6), up on the title row.
    mouse(&mut c, 0, 20, 5);
    mouse(&mut c, 1, 20, 5);
    mouse(&mut c, 6, 20, 5);
    mouse(&mut c, 1, 20, 5);
    let f = frame(&mut c);
    assert!(corner(&f, 79, 24), "the window did not zoom");
    mouse(&mut c, 0, 20, 0);
    mouse(&mut c, 1, 20, 0);
    mouse(&mut c, 6, 20, 0);
    mouse(&mut c, 1, 20, 0);
    let f = frame(&mut c);
    assert!(corner(&f, 49, 14), "the window did not zoom back");
    let (status, _) = c.call(0x31, &[vec![9, 0], i16(0).to_vec(), i16(0).to_vec()].concat());
    assert_ne!(status, 0, "no mouse kind 9");
}

#[test]
fn the_desktop_keys_work_over_the_wire() {
    let mut c = Client::start();
    c.ok(0x01, &[i16(80), i16(25)].concat());
    for (i, t) in ["Alpha", "Beta"].iter().enumerate() {
        c.ok(0x10, &[u16(0).to_vec(), rect(40 * i as i16, 3, 30, 8), vec![0], s(t)].concat());
    }
    let f = frame(&mut c);
    assert!(f.row(3).contains('1') && f.row(3).contains('2'), "numbers in the frames: {:?}", f.row(3));
    // Alt+1: Alpha comes to the front.
    c.ok(0x30, &[0, b'1', 0, 4]);
    let a = c.ok(0x2A, &[]);
    let alpha = u16::from_le_bytes([a[0], a[1]]);
    assert_eq!(alpha, 1, "Alpha is the first window made");
    // Shift+F6: Beta again.
    c.ok(0x30, &[1, 6, 0, 1]);
    let a = c.ok(0x2A, &[]);
    assert_eq!(u16::from_le_bytes([a[0], a[1]]), 2);
    // Alt+0: the list; Down, Enter picks Alpha; TAKE reports nothing.
    c.ok(0x30, &[0, b'0', 0, 4]);
    let f = frame(&mut c);
    assert!(find(&f, "Windows [modal]").is_some() && find(&f, "1  Alpha").is_some(), "{}", picture(&f));
    c.ok(0x30, &[2, 12, 0, 0]);
    c.ok(0x30, &[2, 0, 0, 0]);
    c.ok(0x32, &[]);
    let t = c.ok(0x41, &[]);
    assert_eq!(&t[..4], &[0, 0, 0, 0], "the list's buttons are the toolkit's");
    let a = c.ok(0x2A, &[]);
    assert_eq!(u16::from_le_bytes([a[0], a[1]]), 1, "Alpha again");
    // Ctrl+F5, Right, Enter: Alpha moved one cell.
    c.ok(0x30, &[1, 5, 0, 2]);
    c.ok(0x30, &[2, 14, 0, 0]);
    c.ok(0x30, &[2, 0, 0, 0]);
    let f = frame(&mut c);
    let (x, _) = find(&f, " Alpha ").unwrap();
    assert_eq!(x, 1 + (30 - 7) / 2, "the title moved with the window");
    // The three verbs as ops too.
    c.ok(0x4D, &[]);
    c.ok(0x4E, &[]);
    c.ok(0x30, &[2, 1, 0, 0]);
    c.ok(0x4C, &[]);
    assert!(find(&frame(&mut c), "Windows [modal]").is_some());
}

#[test]
fn hints_and_history_over_the_wire() {
    let mut c = Client::start();
    c.ok(0x01, &[i16(80), i16(25)].concat());
    c.ok(0x1C, &[vec![1], item(0, 0, "~F~ile", "", [vec![1], hinted(1, "~O~pen", "Open a file")].concat())].concat());
    c.ok(0x16, &[vec![1], u16(9).to_vec(), vec![0xFF], u16(0).to_vec(), vec![0], s("~F1~ Help")].concat());
    c.ok(0x30, &[1, 10, 0, 0]); // F10
    let f = frame(&mut c);
    assert!(f.row(24).contains("Open a file") && !f.row(24).contains("F1 Help"), "{:?}", f.row(24));
    c.ok(0x30, &[2, 1, 0, 0]); // Esc
    assert!(frame(&mut c).row(24).contains("F1 Help"));

    // An input with a history set from outside; Down, Enter fills it.
    let win = id_of(&c.ok(0x10, &[u16(0).to_vec(), rect(5, 5, 50, 8), vec![0], s("Go")].concat()));
    let input = id_of(&c.ok(0x13, &[u16(win).to_vec(), rect(1, 1, 40, 1), u16(0).to_vec(), s("Path:"), s("")].concat()));
    c.ok(0x4F, &[u16(input).to_vec(), vec![2], s("D:\\B"), s("C:\\A")].concat());
    let f = frame(&mut c);
    assert_eq!(f.glyph(5 + 1 + 40, 7), 0x1F, "the ▼ at the field's end");
    c.ok(0x30, &[2, 12, 0, 0]); // Down
    assert!(find(&frame(&mut c), "D:\\B").is_some(), "the history panel");
    c.ok(0x30, &[2, 12, 0, 0]);
    c.ok(0x30, &[2, 0, 0, 0]); // Enter on C:\A
    c.ok(0x32, &[]);
    let t = c.ok(0x41, &[]);
    assert_eq!(&t[..4], &[0, 0, 0, 0], "a history pick is not a command");
    let t = c.ok(0x21, &u16(input));
    assert_eq!(&t[2..], b"C:\\A");
    // Enter in the field adds to the history; GET_HISTORY reads it back.
    c.ok(0x30, &[2, 0, 0, 0]);
    let h = c.ok(0x50, &u16(input));
    assert_eq!(h[0], 2, "still two: C:\\A moved to the top, not doubled");
    assert_eq!(&h[3..7], b"C:\\A");
}

#[test]
fn the_palette_is_listed_and_set_over_the_wire() {
    let mut c = Client::start();
    c.ok(0x01, &[i16(40), i16(10)].concat());
    let p = c.ok(0x51, &[]);
    let n = p[0] as usize;
    assert!(n > 60);
    // Walk the entries to find "Desktop: desktop".
    let mut at = 1;
    let mut desktop = None;
    for ix in 0..n {
        let gl = u16::from_le_bytes([p[at], p[at + 1]]) as usize;
        let group = String::from_utf8_lossy(&p[at + 2..at + 2 + gl]).to_string();
        at += 2 + gl;
        let nl = u16::from_le_bytes([p[at], p[at + 1]]) as usize;
        let name = String::from_utf8_lossy(&p[at + 2..at + 2 + nl]).to_string();
        at += 2 + nl;
        let _attr = p[at];
        at += 1;
        if group == "Desktop" && name == "desktop" {
            desktop = Some(ix as u8);
        }
    }
    let desktop = desktop.expect("the desktop colour is listed");
    let f = frame(&mut c);
    let before = f.cells[(5 * 40 + 5) * 3 + 2];
    c.ok(0x52, &[desktop, 0x4F]);
    let f = frame(&mut c);
    assert_eq!(f.cells[(5 * 40 + 5) * 3 + 2], 0x4F, "the desktop wears the new colour");
    assert_ne!(before, 0x4F);
    let (status, _) = c.call(0x52, &[250, 0]);
    assert_ne!(status, 0, "no entry 250");
}

#[test]
fn a_lazy_tree_over_the_wire() {
    let mut c = Client::start();
    c.ok(0x01, &[i16(60), i16(20)].concat());
    let win = id_of(&c.ok(0x10, &[u16(0).to_vec(), rect(0, 0, 40, 15), vec![0], s("Tree")].concat()));
    // Nodes: n then (flags text children). C: open, with Users and Windows lazy.
    let nodes = [vec![1], vec![1], s("C:"), vec![2], vec![2], s("Users"), vec![0], vec![2], s("Windows"), vec![0]].concat();
    let tree = id_of(&c.ok(0x53, &[u16(win).to_vec(), rect(0, 0, 0, 0), nodes].concat()));
    let f = frame(&mut c);
    assert!(find(&f, "+ Users").is_some(), "{}", picture(&f));
    c.ok(0x30, &[2, 12, 0, 0]); // Down
    c.ok(0x30, &[2, 14, 0, 0]); // Right
    let e = c.ok(0x55, &u16(tree));
    assert_eq!(e[0], 2, "a path of two");
    assert_eq!(&e[1..3], &[0, 0]);
    assert_eq!(&e[5..7], b"C:");
    let e2 = c.ok(0x55, &u16(tree));
    assert_eq!(e2[0], 0, "asked once");
    // The answer: one child.
    c.ok(0x54, &[u16(tree).to_vec(), vec![2], u16(0).to_vec(), u16(0).to_vec(), vec![1], vec![2], s("Egor"), vec![0]].concat());
    let f = frame(&mut c);
    assert!(find(&f, "- Users").is_some() && find(&f, "+ Egor").is_some(), "{}", picture(&f));
    let p = c.ok(0x56, &u16(tree));
    assert_eq!(p[0], 2);
    assert_eq!(&p[3..5], b"C:");
    let (status, _) = c.call(0x54, &[u16(tree).to_vec(), vec![1], u16(7).to_vec(), vec![0]].concat());
    assert_ne!(status, 0, "no node 7");
}

#[test]
fn find_and_replace_over_the_wire() {
    let mut c = Client::start();
    let win = notes(&mut c);
    let text = id_of(&[2, 0]);
    let _ = win;
    // "one two three": find "t", case-insensitive: "two".
    let f = c.ok(0x57, &[u16(text).to_vec(), vec![0], s("tw")].concat());
    assert_eq!(f[0], 1);
    let (r, f2) = {
        let r = c.ok(0x58, &[u16(text).to_vec(), vec![0], s("tw"), s("TW")].concat());
        (r[0], r[1])
    };
    assert_eq!((r, f2), (1, 0), "replaced, and no next");
    let t = c.ok(0x21, &u16(text));
    assert_eq!(&t[2..], b"one\nTWo\nthree");
    let n = c.ok(0x59, &[u16(text).to_vec(), vec![0], s("e"), s("E")].concat());
    assert_eq!(u16::from_le_bytes([n[0], n[1]]), 3);
    let t = c.ok(0x21, &u16(text));
    assert_eq!(&t[2..], b"onE\nTWo\nthrEE");
    let (status, _) = c.call(0x57, &[u16(win).to_vec(), vec![0], s("x")].concat());
    assert_ne!(status, 0, "a window is not a text");
}

#[test]
fn a_listing_too_big_for_one_request_arrives_in_pieces() {
    let mut c = Client::start();
    c.ok(0x01, &[i16(80), i16(25)].concat());
    let win = id_of(&c.ok(0x10, &[u16(0).to_vec(), rect(0, 0, 80, 24), vec![0], s("Files")].concat()));
    let entry = |name: &str| [s(name), 3u32.to_le_bytes().to_vec(), u16(2026).to_vec(), vec![9, 27, 12, 0, 0]].concat();
    let piece = |from: usize, n: usize| {
        let mut p = u16(n as u16).to_vec();
        for i in from..from + n {
            p.extend(entry(&format!("F{i:05}.TXT")));
        }
        p
    };
    // FILES with the first 2000, ADD_FILES with the next 2000.
    let files = id_of(&c.ok(0x1A, &[u16(win).to_vec(), rect(0, 0, 0, 0), vec![0], s("*.*"), s("C:\\*.*"), piece(0, 2000)].concat()));
    c.ok(0x5A, &[u16(files).to_vec(), piece(2000, 2000)].concat());
    // End: the last name is the 4000th.
    c.ok(0x30, &[2, 8, 0, 0]);
    let f = frame(&mut c);
    assert!(find(&f, "F03999.TXT").is_some(), "{}", picture(&f));
    let (status, _) = c.call(0x5A, &[u16(win).to_vec(), u16(0).to_vec()].concat());
    assert_ne!(status, 0, "a window is not a file panel");
}

#[test]
fn mistakes_are_replies_not_crashes() {
    let mut c = Client::start();
    assert!(c.err(0x40, &[]).contains("INIT"), "frame before init");
    c.ok(0x01, &[i16(40), i16(12)].concat());

    assert!(c.err(0x99, &[]).contains("no op"));
    assert!(c.err(0x10, &[0, 0]).contains("rect.x"), "a payload cut short names the field");
    assert!(c.err(0x20, &u16(999)).contains("no view 999"));
    let e = c.err(
        0x14,
        &[u16(0).to_vec(), vec![1], u16(0).to_vec(), vec![0], s("Bad")].concat(),
    );
    assert!(e.contains("command 0"));

    // And it is still there afterwards.
    let f = frame(&mut c);
    assert_eq!((f.w, f.h), (40, 12));
}

/// OPEN_WINDOW and WAIT belong to a window, and a server without one says
/// so rather than hanging a client that asks. (The window itself is opened
/// by the C# tests, which can post it a key; this suite stays headless.)
#[test]
fn a_window_is_asked_for_in_order() {
    let mut c = Client::start();
    let e = c.err(0x5B, &s("too early"));
    assert!(e.contains("INIT"), "OPEN_WINDOW before INIT said: {e}");
    c.ok(0x01, &[i16(40), i16(12), u16(437)].concat());
    let e = c.err(0x5C, &[]);
    assert!(e.contains("OPEN_WINDOW"), "WAIT with no window said: {e}");
}

#[test]
fn another_folder_starts_on_its_first_name_and_drops_the_marks() {
    let mut c = Client::start();
    c.ok(0x01, &[i16(80), i16(25)].concat());
    let win = id_of(&c.ok(0x10, &[u16(0).to_vec(), rect(0, 0, 80, 24), vec![0], s("Files")].concat()));
    // name, size, date, attributes: 0x10 a folder.
    let entry = |name: &str, attrs: u8| [s(name), 3u32.to_le_bytes().to_vec(), u16(2026).to_vec(), vec![9, 27, 12, 0, attrs]].concat();
    let list = |names: &[(&str, u8)]| {
        let mut p = u16(names.len() as u16).to_vec();
        for (n, a) in names {
            p.extend(entry(n, *a));
        }
        p
    };
    let current = |c: &mut Client, files: u16| {
        let r = c.ok(0x28, &u16(files));
        let n = u16::from_le_bytes([r[2], r[3]]) as usize;
        String::from_utf8(r[4..4 + n].to_vec()).unwrap()
    };
    // A panel with marks, on C:\A: it starts on ..
    let files = id_of(&c.ok(0x1A, &[u16(win).to_vec(), rect(0, 0, 0, 0), vec![1], s("*.*"), s(r"C:\A\*.*"),
        list(&[("..", 0x10), ("ONE.TXT", 0)])].concat()));
    assert_eq!(current(&mut c, files), "..");
    // Down, Insert marks ONE.TXT.
    c.ok(0x30, &[2, 12, 0, 0]);
    c.ok(0x30, &[2, 6, 0, 0]);
    // Into C:\A\SUB: the cursor on its first name, the mark gone.
    c.ok(0x25, &[u16(files).to_vec(), s(r"C:\A\SUB\*.*"), s("*.*"), list(&[("..", 0x10), ("DEEP", 0x10), ("ONE.TXT", 0)])].concat());
    assert_eq!(current(&mut c, files), "DEEP", "the first name, not ..");
    // The same folder read again keeps the cursor where it is.
    c.ok(0x30, &[2, 12, 0, 0]);
    c.ok(0x25, &[u16(files).to_vec(), s(r"C:\A\SUB\*.*"), s("*.*"), list(&[("..", 0x10), ("DEEP", 0x10), ("ONE.TXT", 0)])].concat());
    assert_eq!(current(&mut c, files), "ONE.TXT");
    // An empty folder: only .., and the cursor on it.
    c.ok(0x25, &[u16(files).to_vec(), s(r"C:\A\SUB\DEEP\*.*"), s("*.*"), list(&[("..", 0x10)])].concat());
    assert_eq!(current(&mut c, files), "..");
}

#[test]
fn a_commanders_panel_says_its_folder_in_the_title_and_only_details_at_the_foot() {
    let mut c = Client::start();
    c.ok(0x01, &[i16(80), i16(25)].concat());
    let win = id_of(&c.ok(0x10, &[u16(0).to_vec(), rect(0, 0, 40, 24), vec![0], s("Left")].concat()));
    let entry = [s("ONE.TXT"), 1234u32.to_le_bytes().to_vec(), u16(2026).to_vec(), vec![9, 30, 21, 55, 0x20]].concat();
    let entries = [u16(1).to_vec(), entry].concat();
    // Flags: marks, no path line, details only.
    c.ok(0x1A, &[u16(win).to_vec(), rect(0, 0, 0, 0), vec![1 | 4 | 8], s("*.*"), s(r"C:\A\*.*"), entries].concat());
    // SET_TEXT on the window: its title.
    c.ok(0x2B, &[u16(win).to_vec(), s(r"C:\A")].concat());
    let f = frame(&mut c);
    assert!(f.row(0).contains(r" C:\A "), "the folder is the title: {}", picture(&f));
    assert!(!f.row(0).contains("Left"), "the old title is gone");
    // The foot is one row, above the frame: size, date, time - no name, no path.
    let foot = f.row(22);
    assert!(foot.contains("1234") && foot.contains("30-09-2026 21:55"), "{foot:?}");
    assert!(!foot.contains("ONE.TXT") && !picture(&f).contains(r"C:\A\*.*"), "{}", picture(&f));
}

#[test]
fn a_console_colours_what_is_written_and_gives_it_back_plain() {
    let mut c = Client::start();
    c.ok(0x01, &[i16(40), i16(12)].concat());
    let win = id_of(&c.ok(0x10, &[u16(0).to_vec(), rect(0, 0, 40, 10), vec![0x20], s("Console")].concat()));
    // CONSOLE: parent, rect (it fills the window anyway), scrollback.
    let con = id_of(&c.ok(0x67, &[u16(win).to_vec(), rect(0, 0, 0, 0), u16(0).to_vec()].concat()));
    // CONSOLE_WRITE, in two pieces cut inside a sequence.
    c.ok(0x68, &[u16(con).to_vec(), s("first \x1b[3")].concat());
    c.ok(0x68, &[u16(con).to_vec(), s("2mgreen\x1b[0m\nПривет ═\n")].concat());

    let f = frame(&mut c);
    let y = (0..12).find(|&y| f.row(y).contains("first green")).expect("the written line");
    let x = f.row(y).chars().position(|ch| ch == 'g').unwrap() as i16;
    let attr = f.cells[((y * f.w + x) * 3 + 2) as usize];
    assert_eq!(attr, 0x02, "ANSI 32 is green on black");

    // GET_TEXT: the record without its colours, the Cyrillic intact.
    let body = c.ok(0x21, &u16(con));
    let n = u16::from_le_bytes([body[0], body[1]]) as usize;
    let text = String::from_utf8(body[2..2 + n].to_vec()).unwrap();
    assert_eq!(text, "first green\nПривет ═\n");

    // Not a console: an error, not a crash.
    let e = c.err(0x68, &[u16(win).to_vec(), s("x")].concat());
    assert!(e.contains("not a console"), "{e}");
}

#[test]
fn the_host_shares_its_clipboard_copy_goes_out_and_paste_comes_in() {
    let mut c = Client::start();
    c.ok(0x01, &[i16(60), i16(20)].concat());
    let win = id_of(&c.ok(0x10, &[u16(0).to_vec(), rect(0, 0, 40, 10), vec![0x20], s("Doc")].concat()));
    // TEXT: parent, rect, dock 0 (fill), flags 0 (editable).
    let t = id_of(&c.ok(0x11, &[u16(win).to_vec(), rect(0, 0, 0, 0), vec![0, 0], s("hello world")].concat()));
    // CLIPBOARD, host on: nothing copied yet, no paste waiting.
    let r = c.ok(0x69, &[1]);
    let before = u32::from_le_bytes([r[0], r[1], r[2], r[3]]);
    assert_eq!(r[4], 0);
    // Ctrl+A, Ctrl+C: the count goes up and CLIPBOARD_GET has the text.
    let ctrl = |ch: u8| [vec![0], u16(ch as u16).to_vec(), vec![2]].concat();
    c.ok(0x30, &ctrl(b'a'));
    c.ok(0x30, &ctrl(b'c'));
    let r = c.ok(0x69, &[1]);
    assert_eq!(u32::from_le_bytes([r[0], r[1], r[2], r[3]]), before + 1);
    let g = c.ok(0x6A, &[]);
    let n = u16::from_le_bytes([g[0], g[1]]) as usize;
    assert_eq!(String::from_utf8(g[2..2 + n].to_vec()).unwrap(), "hello world");
    // The host's paste key: its clipboard, pasted now over the selection.
    let p = c.ok(0x6B, &[vec![1], s("Привет\r\nмир")].concat());
    assert_eq!(p[0], 1);
    let body = c.ok(0x21, &u16(t));
    let n = u16::from_le_bytes([body[0], body[1]]) as usize;
    assert_eq!(String::from_utf8(body[2..2 + n].to_vec()).unwrap(), "Привет\nмир");
    // Ctrl+V while the host shares its clipboard: the paste waits for it.
    c.ok(0x30, &ctrl(b'v'));
    assert_eq!(c.ok(0x69, &[1])[4], 1, "Ctrl+V should wait for the host's clipboard");
    c.ok(0x6B, &[vec![0], s("!")].concat());
    let body = c.ok(0x21, &u16(t));
    let n = u16::from_le_bytes([body[0], body[1]]) as usize;
    assert_eq!(String::from_utf8(body[2..2 + n].to_vec()).unwrap(), "Привет\nмир!");
}
