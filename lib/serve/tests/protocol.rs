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
        cells: b[8..].to_vec(),
    }
}

impl Frame {
    fn row(&self, y: i16) -> String {
        let mut out = String::new();
        for x in 0..self.w {
            let i = ((y * self.w + x) * 2) as usize;
            out.push(self.cells[i] as char);
        }
        out
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
    assert_eq!(f.cells.len(), 40 * 12 * 2);

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

    // Enter presses the default button and TAKE reports it - once.
    c.ok(0x30, &[2, 0, 0, 0]);
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
    let i = ((y * f.w + x) * 2) as usize;
    matches!(f.cells.get(i), Some(0xBC) | Some(0xD9))
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
        assert_eq!(f.cells.len(), (w as usize) * (h as usize) * 2);
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
        // Turbo Vision let a window go all but one column off the side, and
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
