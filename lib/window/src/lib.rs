//! The core's screen in a native Windows window: `CreateWindow`, a
//! message loop and GDI. The cells drawn by hand.
//!
//! This is a screen, like the terminal in `lib/console` and the canvas in
//! `lib/js/owlosui.js`: it draws what the core decided and hands the core
//! the keys and the mouse. Two kinds of program sit behind it:
//!
//!  * a Rust program that links the core, as
//!    `Examples/Desktop/Rust/02-Window` does - it implements [`Program`]
//!    around its `Ui` and calls [`run`];
//!  * any program on the other side of the wire. `owlosui-serve` answers
//!    the WINDOW request by moving its desktop into one of these windows,
//!    which is how a C# program gets a window without a line of drawing
//!    code of its own (`lib/serve/src/window.rs`).
//!
//! Nothing here is a framework. The dozen Win32 functions used are
//! declared by hand in `win32.rs`, because a window, a font and a
//! rectangle are all a text screen needs, and a toolkit that pulled in a
//! GUI library to draw eighty-by-twenty-five cells would have lost the
//! argument it exists to make.
//!
//! On any other system this crate is empty.

#![cfg(windows)]

mod win32;

use core::ffi::c_void;
use core::ptr::{null, null_mut};
use owlosui_core::{Buffer, Button, Event, Glyph, Key, KeyCode, Mods, Mouse, MouseKind, Ui};
use win32::*;

/// What stands behind the window: a core, and whoever acts on what it says.
pub trait Program {
    /// The core whose screen the window shows.
    fn ui(&mut self) -> &mut Ui;

    /// A key, a click or a new size from the window. Straight into the core
    /// unless the program has a reason to hold it back.
    fn input(&mut self, ev: Event) {
        self.ui().handle(ev);
    }

    /// After every input: act on whatever was pressed or chosen. False
    /// closes the window and ends [`run`].
    fn after_input(&mut self) -> bool;

    /// What a glyph index looks like. The core's own convention by default:
    /// below 256 code page 437, above it the character's own code point.
    /// A program with a font of its own - the server's code page 866, say -
    /// answers from that font instead.
    fn glyph(&self, g: Glyph) -> char {
        cp437(g)
    }

    /// Another thread called [`Waker::wake`]. False closes the window.
    fn woken(&mut self) -> bool {
        true
    }

    /// The core's clipboard as text for Windows' clipboard: its lines, each
    /// glyph as [`glyph`](Program::glyph) shows it.
    fn clip_text(&mut self) -> String {
        let lines = self.ui().clipboard.clone();
        let text: Vec<String> = lines.iter().map(|l| l.iter().map(|&g| self.glyph(g)).collect()).collect();
        text.join("\r\n")
    }

    /// Windows' clipboard as the core's lines: by default the core's own
    /// convention again, code page 437 below 256 and the character's code
    /// point above it.
    fn clip_lines(&mut self, text: &str) -> Vec<Vec<Glyph>> {
        text.replace("\r\n", "\n").split('\n').map(|l| l.chars().map(from_cp437).collect()).collect()
    }
}

/// A character as a glyph index of the core's default font: the inverse of
/// [`cp437`]. A character the code page has not got is its own code point,
/// as the growing font has it, or `?` past what a glyph can hold.
pub fn from_cp437(c: char) -> Glyph {
    if (' '..='~').contains(&c) {
        return c as u32 as Glyph;
    }
    if let Some(i) = CP437.iter().skip(1).position(|&u| u as u32 == c as u32) {
        return (i + 1) as Glyph;
    }
    if (c as u32) >= 256 && (c as u32) <= Glyph::MAX as u32 {
        c as u32 as Glyph
    } else {
        b'?' as Glyph
    }
}

/// How the window looks when it opens.
pub struct Options<'a> {
    /// The caption.
    pub title: &'a str,
    /// A fixed-pitch face. The block, shade and box characters never come
    /// from it - they are drawn - so any monospaced font will do.
    pub font: &'a str,
    /// The cell height in pixels; the width follows from the font.
    pub font_height: i32,
    /// Whether the window takes the focus when it opens. A test or an
    /// agent opens it without, so a person typing elsewhere keeps typing
    /// there; messages posted to the window reach it either way.
    pub activate: bool,
}

impl Default for Options<'_> {
    fn default() -> Self {
        Options { title: "OWLOSUI", font: "Consolas", font_height: 18, activate: true }
    }
}

/// A handle another thread may keep, to wake the window: the window's
/// thread then calls [`Program::woken`]. This is how requests arriving on
/// a pipe reach a core that lives on the window's thread.
#[derive(Clone, Copy)]
pub struct Waker(isize);

impl Waker {
    pub fn wake(&self) {
        unsafe {
            PostMessageW(self.0 as HWND, WM_APP, 0, 0);
        }
    }
}

/// A glyph index as the core means it by default: below 256 code page
/// 437, from 256 up the character's own code point.
pub fn cp437(g: Glyph) -> char {
    let g = g as u32;
    let code = if g < 256 { CP437[g as usize] as u32 } else { g };
    char::from_u32(code).unwrap_or('?')
}

/// The sixteen colours of the IBM CGA, as every emulator shows them.
const PALETTE: [(u8, u8, u8); 16] = [
    (0x00, 0x00, 0x00), (0x00, 0x00, 0xAA), (0x00, 0xAA, 0x00), (0x00, 0xAA, 0xAA),
    (0xAA, 0x00, 0x00), (0xAA, 0x00, 0xAA), (0xAA, 0x55, 0x00), (0xAA, 0xAA, 0xAA),
    (0x55, 0x55, 0x55), (0x55, 0x55, 0xFF), (0x55, 0xFF, 0x55), (0x55, 0xFF, 0xFF),
    (0xFF, 0x55, 0x55), (0xFF, 0x55, 0xFF), (0xFF, 0xFF, 0x55), (0xFF, 0xFF, 0xFF),
];

/// Code page 437, 0..255, as UTF-16: what a glyph index below 256 looks
/// like. Above 256 a glyph is the character's own code point, which is
/// how the core's growing font works.
const CP437: [u16; 256] = [
    0x0020, 0x263A, 0x263B, 0x2665, 0x2666, 0x2663, 0x2660, 0x2022, 0x25D8, 0x25CB, 0x25D9, 0x2642, 0x2640, 0x266A, 0x266B, 0x263C,
    0x25BA, 0x25C4, 0x2195, 0x203C, 0x00B6, 0x00A7, 0x25AC, 0x21A8, 0x2191, 0x2193, 0x2192, 0x2190, 0x221F, 0x2194, 0x25B2, 0x25BC,
    0x0020, 0x0021, 0x0022, 0x0023, 0x0024, 0x0025, 0x0026, 0x0027, 0x0028, 0x0029, 0x002A, 0x002B, 0x002C, 0x002D, 0x002E, 0x002F,
    0x0030, 0x0031, 0x0032, 0x0033, 0x0034, 0x0035, 0x0036, 0x0037, 0x0038, 0x0039, 0x003A, 0x003B, 0x003C, 0x003D, 0x003E, 0x003F,
    0x0040, 0x0041, 0x0042, 0x0043, 0x0044, 0x0045, 0x0046, 0x0047, 0x0048, 0x0049, 0x004A, 0x004B, 0x004C, 0x004D, 0x004E, 0x004F,
    0x0050, 0x0051, 0x0052, 0x0053, 0x0054, 0x0055, 0x0056, 0x0057, 0x0058, 0x0059, 0x005A, 0x005B, 0x005C, 0x005D, 0x005E, 0x005F,
    0x0060, 0x0061, 0x0062, 0x0063, 0x0064, 0x0065, 0x0066, 0x0067, 0x0068, 0x0069, 0x006A, 0x006B, 0x006C, 0x006D, 0x006E, 0x006F,
    0x0070, 0x0071, 0x0072, 0x0073, 0x0074, 0x0075, 0x0076, 0x0077, 0x0078, 0x0079, 0x007A, 0x007B, 0x007C, 0x007D, 0x007E, 0x2302,
    0x00C7, 0x00FC, 0x00E9, 0x00E2, 0x00E4, 0x00E0, 0x00E5, 0x00E7, 0x00EA, 0x00EB, 0x00E8, 0x00EF, 0x00EE, 0x00EC, 0x00C4, 0x00C5,
    0x00C9, 0x00E6, 0x00C6, 0x00F4, 0x00F6, 0x00F2, 0x00FB, 0x00F9, 0x00FF, 0x00D6, 0x00DC, 0x00A2, 0x00A3, 0x00A5, 0x20A7, 0x0192,
    0x00E1, 0x00ED, 0x00F3, 0x00FA, 0x00F1, 0x00D1, 0x00AA, 0x00BA, 0x00BF, 0x2310, 0x00AC, 0x00BD, 0x00BC, 0x00A1, 0x00AB, 0x00BB,
    0x2591, 0x2592, 0x2593, 0x2502, 0x2524, 0x2561, 0x2562, 0x2556, 0x2555, 0x2563, 0x2551, 0x2557, 0x255D, 0x255C, 0x255B, 0x2510,
    0x2514, 0x2534, 0x252C, 0x251C, 0x2500, 0x253C, 0x255E, 0x255F, 0x255A, 0x2554, 0x2569, 0x2566, 0x2560, 0x2550, 0x256C, 0x2567,
    0x2568, 0x2564, 0x2565, 0x2559, 0x2558, 0x2552, 0x2553, 0x256B, 0x256A, 0x2518, 0x250C, 0x2588, 0x2584, 0x258C, 0x2590, 0x2580,
    0x03B1, 0x00DF, 0x0393, 0x03C0, 0x03A3, 0x03C3, 0x00B5, 0x03C4, 0x03A6, 0x0398, 0x03A9, 0x03B4, 0x221E, 0x03C6, 0x03B5, 0x2229,
    0x2261, 0x00B1, 0x2265, 0x2264, 0x2320, 0x2321, 0x00F7, 0x2248, 0x00B0, 0x2219, 0x00B7, 0x221A, 0x207F, 0x00B2, 0x25A0, 0x00A0,
];

/// The box-drawing characters as arms: left, right, up, down, each 0
/// (none), 1 (single) or 2 (double). Drawn as rectangles, not looked
/// up in the font: a font's `═` is narrower than the cell and the lines
/// do not meet, and a card never had that problem because its lines
/// went edge to edge.
fn arms(code: u32) -> Option<[u8; 4]> {
    Some(match code {
        0x2500 => [1, 1, 0, 0],
        0x2502 => [0, 0, 1, 1],
        0x250C => [0, 1, 0, 1],
        0x2510 => [1, 0, 0, 1],
        0x2514 => [0, 1, 1, 0],
        0x2518 => [1, 0, 1, 0],
        0x251C => [0, 1, 1, 1],
        0x2524 => [1, 0, 1, 1],
        0x252C => [1, 1, 0, 1],
        0x2534 => [1, 1, 1, 0],
        0x253C => [1, 1, 1, 1],
        0x2550 => [2, 2, 0, 0],
        0x2551 => [0, 0, 2, 2],
        0x2552 => [0, 2, 0, 1],
        0x2553 => [0, 1, 0, 2],
        0x2554 => [0, 2, 0, 2],
        0x2555 => [2, 0, 0, 1],
        0x2556 => [1, 0, 0, 2],
        0x2557 => [2, 0, 0, 2],
        0x2558 => [0, 2, 1, 0],
        0x2559 => [0, 1, 2, 0],
        0x255A => [0, 2, 2, 0],
        0x255B => [2, 0, 1, 0],
        0x255C => [1, 0, 2, 0],
        0x255D => [2, 0, 2, 0],
        0x255E => [0, 2, 1, 1],
        0x255F => [0, 1, 2, 2],
        0x2560 => [0, 2, 2, 2],
        0x2561 => [2, 0, 1, 1],
        0x2562 => [1, 0, 2, 2],
        0x2563 => [2, 0, 2, 2],
        0x2564 => [2, 2, 0, 1],
        0x2565 => [1, 1, 0, 2],
        0x2566 => [2, 2, 0, 2],
        0x2567 => [2, 2, 1, 0],
        0x2568 => [1, 1, 2, 0],
        0x2569 => [2, 2, 2, 0],
        0x256A => [2, 2, 1, 1],
        0x256B => [1, 1, 2, 2],
        0x256C => [2, 2, 2, 2],
        _ => return None,
    })
}

/// A box character into its cell: every arm a bar from the cell's edge
/// towards the centre, and how far past the centre it reaches is what
/// makes the joins. A double arm is two bars, `d` apart. Each bar of a
/// double arm looks at the arm across it on its own side: a double one
/// there and the bar stops `d` short of the centre, at that arm's inner
/// line; a single one and it stops at the centre; none, and it runs `d`
/// past the centre so that it meets its opposite, or closes a corner
/// with the outer line of a double arm on the far side. A single arm
/// runs to the centre, or `d` past it when a double crosses it, so the
/// double's two lines are bridged.
unsafe fn draw_box(dc: HDC, r: RECT, a: [u8; 4], ink: HBRUSH) {
    let (w, h) = (r.right - r.left, r.bottom - r.top);
    let t = (h / 14).max(1);
    let d = (h / 8).max(2);
    let (cx, cy) = (r.left + w / 2, r.top + h / 2);
    let [l, rt, u, dn] = a;
    let bar = |x0: i32, y0: i32, x1: i32, y1: i32| {
        let rr = RECT { left: x0.min(x1), top: y0.min(y1), right: x0.max(x1), bottom: y0.max(y1) };
        FillRect(dc, &rr, ink);
    };
    // How far past the centre a double arm's bar reaches: `same` is the
    // arm across on this bar's side, `other` the one on the far side.
    let reach = |same: u8, other: u8| -> i32 {
        match (same, other) {
            (2, _) => -d,
            (1, _) => 0,
            (_, 1) => 0,
            _ => d,
        }
    };
    let single_reach = |across_a: u8, across_b: u8| -> i32 { if across_a == 2 || across_b == 2 { d } else { 0 } };
    // Horizontal arms: `end` is the x the bar reaches on the far side of
    // the centre from its edge.
    for (arm, dir) in [(l, -1), (rt, 1)] {
        if arm == 0 {
            continue;
        }
        let edge = if dir < 0 { r.left } else { r.right };
        if arm == 1 {
            let end = cx - dir * single_reach(u, dn);
            bar(edge, cy, end + if dir < 0 { t } else { 0 }, cy + t);
        } else {
            for s in [-1i32, 1] {
                let y = cy + s * d;
                let (same, other) = if s < 0 { (u, dn) } else { (dn, u) };
                let end = cx - dir * reach(same, other);
                bar(edge, y, end + if dir < 0 { t } else { 0 }, y + t);
            }
        }
    }
    // Vertical arms, the same way.
    for (arm, dir) in [(u, -1), (dn, 1)] {
        if arm == 0 {
            continue;
        }
        let edge = if dir < 0 { r.top } else { r.bottom };
        if arm == 1 {
            let end = cy - dir * single_reach(l, rt);
            bar(cx, edge, cx + t, end + if dir < 0 { t } else { 0 });
        } else {
            for s in [-1i32, 1] {
                let x = cx + s * d;
                let (same, other) = if s < 0 { (l, rt) } else { (rt, l) };
                let end = cy - dir * reach(same, other);
                bar(x, edge, x + t, end + if dir < 0 { t } else { 0 });
            }
        }
    }
}

/// The three shades as 8x8 patterns, one bit a pixel and a padding byte
/// a row, as a monochrome bitmap wants them: the dots a DOS card drew. A
/// monochrome pattern brush paints its 0 bits in the text colour and its
/// 1 bits in the background colour, so the rows are written inverted.
const SHADES: [[u8; 16]; 3] = [
    // ░ one dot in four
    [!0x22, 0, !0x88, 0, !0x22, 0, !0x88, 0, !0x22, 0, !0x88, 0, !0x22, 0, !0x88, 0],
    // ▒ every other
    [!0xAA, 0, !0x55, 0, !0xAA, 0, !0x55, 0, !0xAA, 0, !0x55, 0, !0xAA, 0, !0x55, 0],
    // ▓ three in four
    [!0xDD, 0, !0x77, 0, !0xDD, 0, !0x77, 0, !0xDD, 0, !0x77, 0, !0xDD, 0, !0x77, 0],
];

const TIMER_BLINK: usize = 1;
const TIMER_PICK: usize = 2;

struct State {
    program: Box<dyn Program>,
    buf: Buffer,
    font: HFONT,
    /// The shade brushes, made once: ░ ▒ ▓.
    shades: [HBRUSH; 3],
    cell_w: i32,
    cell_h: i32,
    /// Buttons held: motion with one held is a drag.
    buttons: u32,
    blink: bool,
    /// The core's copy count when Windows' clipboard last had its text.
    copied: u32,
}

static mut STATE: Option<State> = None;

/// The window's state, once it has one. Messages sent while the window is
/// being made arrive before it does, and go to the system's default.
#[allow(static_mut_refs)]
fn state_opt() -> Option<&'static mut State> {
    // One window, one UI thread: every message arrives on it in turn.
    unsafe { STATE.as_mut() }
}

fn state() -> &'static mut State {
    state_opt().expect("the window has its state before this message")
}

fn colour(ix: u8) -> COLORREF {
    let (r, g, b) = PALETTE[(ix & 15) as usize];
    rgb(r, g, b)
}

/// The core's size, in cells.
fn size_of(ui: &mut Ui) -> (i16, i16) {
    let r = ui.rect(ui.root());
    (r.w, r.h)
}

/// Open the window, run it until the program says stop or the window is
/// destroyed, and return. `make` builds the program once the window
/// exists, and is given the [`Waker`] for it. The window opens at the
/// size of the program's core, in whole cells.
pub fn run<P: Program + 'static>(options: &Options, make: impl FnOnce(Waker) -> P) {
    unsafe {
        // The font first: the window is sized to whole cells of it.
        let face = wide(options.font);
        let font = CreateFontW(
            -options.font_height, 0, 0, 0, FW_NORMAL, 0, 0, 0, DEFAULT_CHARSET, OUT_TT_PRECIS, CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY, FIXED_PITCH, face.as_ptr(),
        );
        let dc = GetDC(null_mut());
        let old = SelectObject(dc, font);
        let mut size = SIZE { cx: 10, cy: 20 };
        let m = wide("M");
        GetTextExtentPoint32W(dc, m.as_ptr(), 1, &mut size);
        SelectObject(dc, old);
        ReleaseDC(null_mut(), dc);
        let (cell_w, cell_h) = (size.cx.max(6), size.cy.max(10));

        let mut shades = [null_mut(); 3];
        for (i, rows) in SHADES.iter().enumerate() {
            // A monochrome bitmap's rows are words, the pixels in the
            // high byte, leftmost first.
            let bmp = CreateBitmap(8, 8, 1, 1, rows.as_ptr() as *const c_void);
            shades[i] = CreatePatternBrush(bmp);
            DeleteObject(bmp);
        }

        let class = wide("OWLOSUI");
        let wc = WNDCLASSW {
            style: CS_DBLCLKS | CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: null_mut(),
            hIcon: null_mut(),
            hCursor: LoadCursorW(null_mut(), IDC_ARROW as *const u16),
            hbrBackground: null_mut(),
            lpszMenuName: null(),
            lpszClassName: class.as_ptr(),
        };
        RegisterClassW(&wc);

        let title = wide(options.title);
        let hwnd = CreateWindowExW(
            0,
            class.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            640,
            480,
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
        );

        let mut program: Box<dyn Program> = Box::new(make(Waker(hwnd as isize)));
        let (cols, rows) = size_of(program.ui());
        // Windows' clipboard is the core's from here on (share_clipboard).
        program.ui().host_clip.on = true;
        let copied = program.ui().host_clip.copied;
        STATE = Some(State {
            program,
            buf: Buffer::new(cols, rows),
            font,
            shades,
            cell_w,
            cell_h,
            buttons: 0,
            blink: true,
            copied,
        });

        // The client area is the core's cells exactly; the system says how
        // much frame goes around it.
        let mut r = RECT { left: 0, top: 0, right: cols as i32 * cell_w, bottom: rows as i32 * cell_h };
        AdjustWindowRectEx(&mut r, WS_OVERLAPPEDWINDOW, 0, 0);
        SetWindowPos(hwnd, null_mut(), 0, 0, r.right - r.left, r.bottom - r.top, SWP_NOMOVE | SWP_NOZORDER);
        ShowWindow(hwnd, if options.activate { SW_SHOW } else { SW_SHOWNOACTIVATE });
        UpdateWindow(hwnd);
        SetTimer(hwnd, TIMER_BLINK, 500, null());

        let mut msg = MSG { hwnd: null_mut(), message: 0, wParam: 0, lParam: 0, time: 0, pt: POINT { x: 0, y: 0 } };
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        // The program goes with the window: whatever it holds is dropped here.
        STATE = None;
        for b in shades {
            DeleteObject(b);
        }
        DeleteObject(font);
    }
}

/// After any input: the program acts on it, a pick is held for a moment,
/// the window is redrawn, and it closes if the program said so.
fn settle(hwnd: HWND) {
    let s = state();
    share_clipboard(hwnd);
    unsafe {
        if !s.program.after_input() {
            DestroyWindow(hwnd);
            return;
        }
        if s.program.ui().pick_pending() {
            SetTimer(hwnd, TIMER_PICK, 90, null());
        }
        InvalidateRect(hwnd, null(), 0);
    }
}

fn input(hwnd: HWND, ev: Event) {
    state().program.input(ev);
    settle(hwnd);
}

/// Windows' clipboard and the core's, kept as one: what the core copied
/// goes out, and a Paste waiting for the computer's clipboard has it now.
/// A clipboard that cannot be had leaves the core's as it is.
fn share_clipboard(hwnd: HWND) {
    let s = state();
    let ui = s.program.ui();
    if !ui.host_clip.on {
        return;
    }
    let copied = ui.host_clip.copied;
    let wanted = ui.host_clip.paste_wanted.is_some();
    if copied != s.copied {
        s.copied = copied;
        let text = s.program.clip_text();
        set_clipboard(hwnd, &text);
    }
    if wanted {
        let lines = get_clipboard(hwnd).filter(|t| !t.is_empty()).map(|t| s.program.clip_lines(&t));
        s.program.ui().host_paste(lines, false);
    }
}

/// Another program may hold the clipboard for a moment; try a few times.
fn open_clipboard(hwnd: HWND) -> bool {
    for _ in 0..5 {
        if unsafe { OpenClipboard(hwnd) } != 0 {
            return true;
        }
        unsafe { Sleep(10) };
    }
    false
}

fn set_clipboard(hwnd: HWND, text: &str) {
    let w = wide(text);
    unsafe {
        let mem = GlobalAlloc(GMEM_MOVEABLE, w.len() * 2);
        if mem.is_null() {
            return;
        }
        let p = GlobalLock(mem) as *mut u16;
        if p.is_null() {
            GlobalFree(mem);
            return;
        }
        core::ptr::copy_nonoverlapping(w.as_ptr(), p, w.len());
        GlobalUnlock(mem);
        if !open_clipboard(hwnd) {
            GlobalFree(mem);
            return;
        }
        EmptyClipboard();
        // Given, the memory is the system's; refused, it is still ours.
        if SetClipboardData(CF_UNICODETEXT, mem).is_null() {
            GlobalFree(mem);
        }
        CloseClipboard();
    }
}

fn get_clipboard(hwnd: HWND) -> Option<String> {
    unsafe {
        if !open_clipboard(hwnd) {
            return None;
        }
        let mut text = None;
        let mem = GetClipboardData(CF_UNICODETEXT);
        if !mem.is_null() {
            let p = GlobalLock(mem) as *const u16;
            if !p.is_null() {
                let mut n = 0;
                while *p.add(n) != 0 {
                    n += 1;
                }
                text = Some(String::from_utf16_lossy(core::slice::from_raw_parts(p, n)));
                GlobalUnlock(mem);
            }
        }
        CloseClipboard();
        text
    }
}

fn mods() -> Mods {
    unsafe {
        Mods {
            shift: GetKeyState(VK_SHIFT) < 0,
            ctrl: GetKeyState(VK_CONTROL) < 0,
            alt: GetKeyState(VK_MENU) < 0,
        }
    }
}

/// A virtual key into the core's key: the named ones, F1..F12, and
/// letters and digits while Alt or Ctrl is held (those never come as
/// WM_CHAR). Plain typing arrives as WM_CHAR and is not this.
fn key_of(vk: usize, m: Mods) -> Option<Key> {
    let code = match vk {
        VK_RETURN => KeyCode::Enter,
        VK_ESCAPE => KeyCode::Esc,
        VK_TAB => {
            if m.shift {
                KeyCode::BackTab
            } else {
                KeyCode::Tab
            }
        }
        VK_BACK => KeyCode::Backspace,
        VK_DELETE => KeyCode::Delete,
        VK_INSERT => KeyCode::Insert,
        VK_HOME => KeyCode::Home,
        VK_END => KeyCode::End,
        VK_PRIOR => KeyCode::PageUp,
        VK_NEXT => KeyCode::PageDown,
        VK_UP => KeyCode::Up,
        VK_DOWN => KeyCode::Down,
        VK_LEFT => KeyCode::Left,
        VK_RIGHT => KeyCode::Right,
        VK_F1..=VK_F12 => KeyCode::F((vk - VK_F1 + 1) as u8),
        0x30..=0x39 | 0x41..=0x5A if m.alt || m.ctrl => KeyCode::Char((vk as u8).to_ascii_lowercase() as char),
        _ => return None,
    };
    let m = if matches!(code, KeyCode::BackTab) { Mods { shift: false, ..m } } else { m };
    Some(Key { code, mods: m })
}

fn cell_at(lparam: LPARAM) -> (i16, i16) {
    let s = state();
    let x = (lparam & 0xFFFF) as i16 as i32;
    let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
    ((x / s.cell_w) as i16, (y / s.cell_h) as i16)
}

fn mouse(hwnd: HWND, kind: MouseKind, lparam: LPARAM) {
    let (x, y) = cell_at(lparam);
    input(hwnd, Event::Mouse(Mouse { x, y, kind }));
}

/// Screen coordinates (the wheel's) into a cell of this window.
fn screen_to_cell(hwnd: HWND, sx: i32, sy: i32) -> (i16, i16) {
    let s = state();
    let mut p = POINT { x: sx, y: sy };
    unsafe {
        ScreenToClient(hwnd, &mut p);
    }
    ((p.x / s.cell_w).max(0) as i16, (p.y / s.cell_h).max(0) as i16)
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let Some(s) = state_opt() else {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    };
    match msg {
        WM_PAINT => {
            paint(hwnd);
            0
        }
        WM_ERASEBKGND => 1,
        WM_SIZE => {
            let w = (lparam & 0xFFFF) as i32;
            let h = ((lparam >> 16) & 0xFFFF) as i32;
            if w == 0 || h == 0 {
                return 0; // minimised: the core keeps its size
            }
            let cols = (w / s.cell_w).max(20) as i16;
            let rows = (h / s.cell_h).max(8) as i16;
            if (cols, rows) != size_of(s.program.ui()) {
                input(hwnd, Event::Resize(cols, rows));
            }
            0
        }
        WM_KEYDOWN | WM_SYSKEYDOWN => {
            let m = mods();
            if let Some(k) = key_of(wparam, m) {
                input(hwnd, Event::Key(k));
                return 0;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_CHAR => {
            let m = mods();
            let c = wparam as u32;
            if c >= 32 && !m.ctrl && !m.alt {
                if let Some(ch) = char::from_u32(c) {
                    input(hwnd, Event::Key(Key { code: KeyCode::Char(ch), mods: Mods::default() }));
                }
            }
            0
        }
        WM_SYSCHAR => 0, // Alt+letter went through WM_SYSKEYDOWN; no beep
        WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN => {
            let b = match msg {
                WM_RBUTTONDOWN => Button::Right,
                WM_MBUTTONDOWN => Button::Middle,
                _ => Button::Left,
            };
            s.buttons |= 1;
            SetCapture(hwnd);
            mouse(hwnd, MouseKind::Down(b), lparam);
            0
        }
        WM_LBUTTONUP | WM_RBUTTONUP | WM_MBUTTONUP => {
            let b = match msg {
                WM_RBUTTONUP => Button::Right,
                WM_MBUTTONUP => Button::Middle,
                _ => Button::Left,
            };
            s.buttons = 0;
            ReleaseCapture();
            mouse(hwnd, MouseKind::Up(b), lparam);
            0
        }
        WM_LBUTTONDBLCLK | WM_RBUTTONDBLCLK => {
            let b = if msg == WM_RBUTTONDBLCLK { Button::Right } else { Button::Left };
            s.buttons |= 1;
            SetCapture(hwnd);
            mouse(hwnd, MouseKind::Double(b), lparam);
            0
        }
        WM_MOUSEMOVE => {
            if s.buttons != 0 {
                mouse(hwnd, MouseKind::Drag, lparam);
            }
            0
        }
        WM_MOUSEWHEEL => {
            // The wheel reports screen coordinates, and the core scrolls
            // what is under the pointer, so they are made the window's.
            let delta = ((wparam >> 16) & 0xFFFF) as u16 as i16;
            let kind = if delta > 0 { MouseKind::ScrollUp } else { MouseKind::ScrollDown };
            let x = (lparam & 0xFFFF) as i16 as i32;
            let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
            let (cx, cy) = screen_to_cell(hwnd, x, y);
            input(hwnd, Event::Mouse(Mouse { x: cx, y: cy, kind }));
            0
        }
        WM_TIMER => {
            if wparam == TIMER_BLINK {
                s.blink = !s.blink;
                InvalidateRect(hwnd, null(), 0);
            } else if wparam == TIMER_PICK {
                KillTimer(hwnd, TIMER_PICK);
                let ui = s.program.ui();
                if ui.pick_pending() {
                    ui.complete_pick();
                }
                settle(hwnd);
            }
            0
        }
        WM_APP => {
            if s.program.woken() {
                settle(hwnd);
            } else {
                DestroyWindow(hwnd);
            }
            0
        }
        WM_CLOSE => {
            // The close box of the system's frame is Exit: the program may
            // ask first, as it would for Alt+X.
            input(hwnd, Event::Key(Key { code: KeyCode::Char('x'), mods: Mods { alt: true, ..Mods::default() } }));
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Every cell, into a memory bitmap, then onto the window in one blit:
/// a text screen redrawn whole is a few thousand rectangles, and
/// drawing them straight on the window would flicker.
unsafe fn paint(hwnd: HWND) {
    let s = state();
    let (cols, rows) = size_of(s.program.ui());
    if (s.buf.width(), s.buf.height()) != (cols, rows) {
        s.buf = Buffer::new(cols, rows);
    }
    s.program.ui().draw(&mut s.buf);
    let cursor = s.program.ui().cursor();

    let mut ps: PAINTSTRUCT = core::mem::zeroed();
    let dc = BeginPaint(hwnd, &mut ps);
    let mut client = RECT::default();
    GetClientRect(hwnd, &mut client);
    let (w, h) = (client.right - client.left, client.bottom - client.top);
    let mem = CreateCompatibleDC(dc);
    let bmp = CreateCompatibleBitmap(dc, w.max(1), h.max(1));
    let old_bmp = SelectObject(mem, bmp);
    let old_font = SelectObject(mem, s.font);
    SetBkMode(mem, OPAQUE);

    // The window may be wider than the cells: black beyond them.
    let black = CreateSolidBrush(0);
    FillRect(mem, &client, black);
    DeleteObject(black);

    for y in 0..rows {
        for x in 0..cols {
            let Some(cell) = s.buf.get(x, y) else { continue };
            let r = RECT {
                left: x as i32 * s.cell_w,
                top: y as i32 * s.cell_h,
                right: (x as i32 + 1) * s.cell_w,
                bottom: (y as i32 + 1) * s.cell_h,
            };
            let code = s.program.glyph(cell.ch) as u32;
            SetBkColor(mem, colour(cell.attr >> 4));
            SetTextColor(mem, colour(cell.attr & 15));
            // The block and shade glyphs are drawn as blocks, not looked
            // up in the font: a font's ▀ leaves gaps a DOS card never had.
            let fg = CreateSolidBrush(colour(cell.attr & 15));
            let bg = CreateSolidBrush(colour(cell.attr >> 4));
            match code {
                0x2588 => {
                    FillRect(mem, &r, fg);
                }
                0x2580 | 0x2584 | 0x258C | 0x2590 => {
                    FillRect(mem, &r, bg);
                    let half = match code {
                        0x2580 => RECT { bottom: r.top + s.cell_h / 2, ..r },
                        0x2584 => RECT { top: r.top + s.cell_h / 2, ..r },
                        0x258C => RECT { right: r.left + s.cell_w / 2, ..r },
                        _ => RECT { left: r.left + s.cell_w / 2, ..r },
                    };
                    FillRect(mem, &half, fg);
                }
                0x2591 | 0x2592 | 0x2593 => {
                    // A shade is dots: the pattern brush paints them in the
                    // text and background colours set on the DC.
                    FillRect(mem, &r, s.shades[(code - 0x2591) as usize]);
                }
                0x2500..=0x256C if arms(code).is_some() => {
                    FillRect(mem, &r, bg);
                    draw_box(mem, r, arms(code).unwrap(), fg);
                }
                _ => {
                    // One UTF-16 unit a cell: what lies beyond it is a `?`.
                    let ch = [if code > 0xFFFF { b'?' as u16 } else { code as u16 }];
                    let n = if code == 0 || code == 32 { 0 } else { 1 };
                    ExtTextOutW(mem, r.left, r.top, ETO_OPAQUE, &r, ch.as_ptr(), n, null());
                }
            }
            DeleteObject(fg);
            DeleteObject(bg);
            // The caret: a bar at the bottom of its cell, blinking.
            if s.blink {
                if let Some(p) = cursor {
                    if p.x == x && p.y == y {
                        let bar = RECT { top: r.bottom - 3, bottom: r.bottom - 1, ..r };
                        let ink = CreateSolidBrush(colour(cell.attr & 15));
                        FillRect(mem, &bar, ink);
                        DeleteObject(ink);
                    }
                }
            }
        }
    }
    BitBlt(dc, 0, 0, w, h, mem, 0, 0, SRCCOPY);
    SelectObject(mem, old_font);
    SelectObject(mem, old_bmp);
    DeleteObject(bmp);
    DeleteDC(mem);
    EndPaint(hwnd, &ps);
}
