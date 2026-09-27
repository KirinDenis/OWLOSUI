//! Rust step 4 of 4 - DOS: a flat binary under DPMI, drawing into B800.
//! Before: 03-Browser. The app itself is shared/app.rs, the same in 02, 03 and 04.
//!
//! The core on DOS.
//!
//! This is the DOS backend, and it is the shortest of them, because the
//! machine is the one the core was shaped for: a frame is a `Vec<Cell>`
//! of character-and-attribute pairs, and video memory at B800 is exactly
//! that, so drawing is a copy. Keys come out of the BIOS keyboard buffer
//! at 0040:001A. The clock is the BIOS tick at 0040:006C. There is no
//! operating system to ask for anything else, and nothing else is needed.
//!
//! The program is a flat 32-bit binary in extended memory. `OWLOS.COM`
//! (OWLOS.ASM, beside this) asks DPMI for the memory, loads the file into
//! it, makes a code and a data selector whose base is the block, and
//! jumps to `_start` with two numbers on the stack: where the block is,
//! and the flags. Every address below a megabyte - the video card, the
//! BIOS data area - is reached through that data selector too, as the
//! linear address minus the block's base, wrapping; the selector's limit
//! is 4 GB and the first megabyte is where DPMI hosts leave it.
//!
//! `no_std`, `no_main`: `core`, `alloc` on a bump allocator, and this.
//! The application is the same `app.rs` the browser build runs.

#![no_std]
#![no_main]

extern crate alloc;

#[path = "../../shared/app.rs"]
mod app;

use core::alloc::{GlobalAlloc, Layout};
use core::arch::asm;
use owlosui_core::{Buffer, Button, Event, Key, KeyCode, Mods, Mouse, MouseKind};

// ------------------------------------------------------------- the machine

/// The linear address the block was loaded at: set once by `_start`.
static mut BASE: u32 = 0;

/// A pointer to a linear address, through the data selector.
fn lin(addr: u32) -> *mut u8 {
    // The selector's base is BASE and its limit is 4 GB, so an offset
    // wraps round to any linear address, the first megabyte included.
    unsafe { addr.wrapping_sub(BASE) as *mut u8 }
}

fn peek8(addr: u32) -> u8 {
    unsafe { core::ptr::read_volatile(lin(addr)) }
}

fn peek16(addr: u32) -> u16 {
    unsafe { core::ptr::read_volatile(lin(addr) as *const u16) }
}

fn poke16(addr: u32, v: u16) {
    unsafe { core::ptr::write_volatile(lin(addr) as *mut u16, v) }
}

fn peek32(addr: u32) -> u32 {
    unsafe { core::ptr::read_volatile(lin(addr) as *const u32) }
}

fn outb(port: u16, v: u8) {
    unsafe { asm!("out dx, al", in("dx") port, in("al") v, options(nomem, nostack)) }
}

const VIDEO: u32 = 0xB8000;
const BIOS_KB_FLAGS: u32 = 0x417;
const BIOS_KB_HEAD: u32 = 0x41A;
const BIOS_KB_TAIL: u32 = 0x41C;
const BIOS_KB_START: u32 = 0x480;
const BIOS_KB_END: u32 = 0x482;
const BIOS_TICKS: u32 = 0x46C;

/// Leave through DOS. DPMI hosts pass INT 21h through, and AH=4Ch is
/// how a client ends.
fn dos_exit(code: u8) -> ! {
    unsafe {
        asm!("int 0x21", in("ax") 0x4C00u16 | code as u16, options(noreturn));
    }
}

// --------------------------------------------------------------- allocation

/// Two megabytes of heap, handed out in size classes: a request is
/// rounded up to a power of two, and a block given back goes on the free
/// list of its class to be handed out again for the next request of that
/// size. Nothing is ever split or joined, so this is a hundred lines
/// short of a real allocator - but a text-mode program asks for the same
/// few sizes over and over (a line, a frame, a menu's items), and for
/// that a free list per size is exactly right and never fragments.
///
/// It replaced a bump allocator. That one gave nothing back, and a frame
/// copied for every mouse move emptied the heap in a minute of dragging
/// a window: PANIC, `alloc.rs:672`, out of memory. Taking back only the
/// last allocation was the next try, and dragging beat that too - the
/// core's temporaries are not made and dropped in one order.
///
/// The block the loader gives us is four megabytes, zeroed; the heap
/// lives in `.bss`, which the flat binary does not carry and the
/// loader's zeroing makes.
struct Classes;

const HEAP_SIZE: usize = 2 * 1024 * 1024;
static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];
static mut HEAP_AT: usize = 0;
/// One free list per power of two from 16 bytes to a megabyte; a free
/// block's first word is the next one.
static mut FREE: [usize; 17] = [0; 17];

fn class_of(layout: &Layout) -> Option<(usize, usize)> {
    let need = layout.size().max(layout.align()).max(16);
    let mut size = 16usize;
    let mut class = 0usize;
    while size < need {
        size <<= 1;
        class += 1;
        if class >= 17 {
            return None;
        }
    }
    Some((class, size))
}

unsafe impl GlobalAlloc for Classes {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let Some((class, size)) = class_of(&layout) else { return core::ptr::null_mut() };
        let base = core::ptr::addr_of_mut!(HEAP).cast::<u8>();
        let head = FREE[class];
        if head != 0 {
            // Something of this size came back earlier: hand it out again.
            let block = base.add(head);
            FREE[class] = *(block as *const usize);
            return block;
        }
        // Blocks are as aligned as they are big, up to a page.
        let align = size.min(4096);
        let at = (HEAP_AT + align - 1) & !(align - 1);
        let end = at + size;
        if end > HEAP_SIZE || at == 0 {
            // Offset 0 is kept out of use: it is the "no next block" mark.
            if at == 0 && end + 16 <= HEAP_SIZE {
                HEAP_AT = 16;
                return self.alloc(layout);
            }
            return core::ptr::null_mut();
        }
        HEAP_AT = end;
        base.add(at)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let Some((class, _)) = class_of(&layout) else { return };
        let base = core::ptr::addr_of_mut!(HEAP).cast::<u8>();
        let at = ptr as usize - base as usize;
        *(ptr as *mut usize) = FREE[class];
        FREE[class] = at;
    }
}

#[global_allocator]
static ALLOC: Classes = Classes;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    // Say so in the corner, in red, and leave: there is nothing to
    // unwind into. The file and line go to TRACE.TXT for whoever cannot
    // see the screen.
    trace(b'P');
    if let Some(loc) = info.location() {
        for b in loc.file().bytes().rev().take(24).collect::<alloc::vec::Vec<u8>>().into_iter().rev() {
            trace(b);
        }
        trace(b':');
        let mut n = loc.line();
        let mut digits = [0u8; 10];
        let mut k = 0;
        loop {
            digits[k] = b'0' + (n % 10) as u8;
            k += 1;
            n /= 10;
            if n == 0 {
                break;
            }
        }
        while k > 0 {
            k -= 1;
            trace(digits[k]);
        }
    }
    // And on the screen's top row, in red, for whoever is at the machine:
    // PANIC, the file and the line.
    let mut col: u32 = 0;
    let mut put = |b: u8| {
        if col < 80 {
            unsafe {
                *lin(VIDEO + col * 2) = b;
                *lin(VIDEO + col * 2 + 1) = 0x4F;
            }
            col += 1;
        }
    };
    for b in b"PANIC ".iter() {
        put(*b);
    }
    if let Some(loc) = info.location() {
        for b in loc.file().bytes().rev().take(40).collect::<alloc::vec::Vec<u8>>().into_iter().rev() {
            put(b);
        }
        put(b':');
        let mut n = loc.line();
        let mut digits = [0u8; 10];
        let mut k = 0;
        loop {
            digits[k] = b'0' + (n % 10) as u8;
            k += 1;
            n /= 10;
            if n == 0 {
                break;
            }
        }
        while k > 0 {
            k -= 1;
            put(digits[k]);
        }
    }
    dos_exit(3)
}

// ----------------------------------------------------------------- keyboard

/// The next key out of the BIOS buffer, if there is one: the ASCII code
/// and the scan code, as INT 16h would hand them over. Reading the ring
/// directly instead of calling INT 16h saves a trip to real mode per
/// key; the BIOS keeps filling it, because its interrupt handler still
/// runs down there.
fn bios_key() -> Option<(u8, u8)> {
    let head = peek16(BIOS_KB_HEAD);
    let tail = peek16(BIOS_KB_TAIL);
    if head == tail {
        return None;
    }
    let entry = peek16(0x400 + head as u32);
    let mut next = head + 2;
    if next >= peek16(BIOS_KB_END) {
        next = peek16(BIOS_KB_START);
    }
    poke16(BIOS_KB_HEAD, next);
    Some(((entry & 0xFF) as u8, (entry >> 8) as u8))
}

/// A scan code and an ASCII code into the core's key. The tables are the
/// IBM PC's: the arrows and the function keys have no ASCII and are told
/// apart by scan code; Alt with a letter has no ASCII either, and its
/// scan code is the letter's key.
fn decode(ascii: u8, scan: u8) -> Option<Key> {
    let flags = peek8(BIOS_KB_FLAGS);
    let mut mods = Mods {
        shift: flags & 3 != 0,
        ctrl: flags & 4 != 0,
        alt: flags & 8 != 0,
    };
    const ROW1: &[u8] = b"qwertyuiop";
    const ROW2: &[u8] = b"asdfghjkl";
    const ROW3: &[u8] = b"zxcvbnm";
    let code = if ascii == 0 || ascii == 0xE0 {
        match scan {
            0x48 => KeyCode::Up,
            0x50 => KeyCode::Down,
            0x4B => KeyCode::Left,
            0x4D => KeyCode::Right,
            0x47 => KeyCode::Home,
            0x4F => KeyCode::End,
            0x49 => KeyCode::PageUp,
            0x51 => KeyCode::PageDown,
            0x52 => KeyCode::Insert,
            0x53 => KeyCode::Delete,
            0x0F => KeyCode::BackTab,
            0x3B..=0x44 => KeyCode::F(scan - 0x3B + 1),
            0x85 | 0x86 => KeyCode::F(scan - 0x85 + 11),
            // Shift, Ctrl and Alt with a function key have scan codes of
            // their own; the flags byte says which is held anyway.
            0x54..=0x5D => {
                mods.shift = true;
                KeyCode::F(scan - 0x54 + 1)
            }
            0x5E..=0x67 => {
                mods.ctrl = true;
                KeyCode::F(scan - 0x5E + 1)
            }
            0x68..=0x71 => {
                mods.alt = true;
                KeyCode::F(scan - 0x68 + 1)
            }
            0x10..=0x19 => {
                mods.alt = true;
                KeyCode::Char(ROW1[(scan - 0x10) as usize] as char)
            }
            0x1E..=0x26 => {
                mods.alt = true;
                KeyCode::Char(ROW2[(scan - 0x1E) as usize] as char)
            }
            0x2C..=0x32 => {
                mods.alt = true;
                KeyCode::Char(ROW3[(scan - 0x2C) as usize] as char)
            }
            0x78..=0x81 => {
                mods.alt = true;
                KeyCode::Char(b"1234567890"[(scan - 0x78) as usize] as char)
            }
            _ => return None,
        }
    } else {
        match ascii {
            13 => KeyCode::Enter,
            27 => KeyCode::Esc,
            9 => KeyCode::Tab,
            8 => KeyCode::Backspace,
            1..=26 if mods.ctrl => KeyCode::Char((b'a' + ascii - 1) as char),
            _ => {
                // Plain typing. The shift flag is not a modifier on a
                // letter that already arrived in its case.
                mods.shift = false;
                KeyCode::Char(ascii as char)
            }
        }
    };
    Some(Key { code, mods })
}

// ------------------------------------------------------------------- mouse

/// The mouse, through INT 33h: the driver keeps the position and the
/// buttons, we ask for them between keys. The driver's own cursor is
/// never shown; the pointer is drawn into the frame as the red square
/// DOS programs drew, so the eye finds it wherever the palette is.
struct MouseState {
    present: bool,
    x: i16,
    y: i16,
    buttons: u16,
    /// For the double click: when and where the last press was.
    last_down: u32,
    last_at: (i16, i16, u16),
}

const MOUSE_POINTER: u8 = 0x4F; // white on red

impl MouseState {
    fn detect() -> MouseState {
        let mut r = RealRegs { eax: 0, ..Default::default() };
        real_int(0x33, &mut r);
        let present = r.eax & 0xFFFF == 0xFFFF;
        let mut m = MouseState { present, x: 40, y: 12, buttons: 0, last_down: 0, last_at: (-1, -1, 0) };
        if present {
            // The driver's picture stays off: ours is in the frame.
            let mut hide = RealRegs { eax: 2, ..Default::default() };
            real_int(0x33, &mut hide);
            let (x, y, b) = m.read();
            m.x = x;
            m.y = y;
            m.buttons = b;
        }
        m
    }

    /// Position in cells and the buttons held. In text mode the driver
    /// counts eight pixels a cell.
    fn read(&self) -> (i16, i16, u16) {
        let mut r = RealRegs { eax: 3, ..Default::default() };
        real_int(0x33, &mut r);
        (((r.ecx & 0xFFFF) / 8) as i16, ((r.edx & 0xFFFF) / 8) as i16, (r.ebx & 0x7) as u16)
    }

    /// What changed since last time, as the core's events: presses and
    /// releases by button, a drag or a move by motion. A second press on
    /// the same cell within half a second - nine ticks - is a double.
    fn poll(&mut self, out: &mut [Option<Event>; 4]) -> usize {
        if !self.present {
            return 0;
        }
        let (x, y, buttons) = self.read();
        let mut n = 0;
        let moved = x != self.x || y != self.y;
        if moved {
            out[n] = Some(Event::Mouse(Mouse { x, y, kind: if self.buttons != 0 { MouseKind::Drag } else { MouseKind::Move } }));
            n += 1;
        }
        let changed = buttons ^ self.buttons;
        for b in 0..3u16 {
            let bit = 1 << b;
            if changed & bit == 0 {
                continue;
            }
            let button = match b {
                1 => Button::Right,
                2 => Button::Middle,
                _ => Button::Left,
            };
            let kind = if buttons & bit != 0 {
                let now = ticks();
                let twice = now.wrapping_sub(self.last_down) < 9 && self.last_at == (x, y, b);
                self.last_down = if twice { 0 } else { now };
                self.last_at = (x, y, b);
                if twice {
                    MouseKind::Double(button)
                } else {
                    MouseKind::Down(button)
                }
            } else {
                MouseKind::Up(button)
            };
            if n < out.len() {
                out[n] = Some(Event::Mouse(Mouse { x, y, kind }));
                n += 1;
            }
        }
        self.x = x;
        self.y = y;
        self.buttons = buttons;
        n
    }
}

// ------------------------------------------------------------------ screen

/// The frame as the card wants it, the pointer painted on: a cell is
/// two bytes and so is the card's, so this is the frame's own bytes with
/// one attribute changed.
fn frame_bytes(buf: &Buffer, pointer: Option<(i16, i16)>) -> alloc::vec::Vec<u8> {
    let cells = buf.cells();
    let n = (cells.len() * 2).min(80 * 25 * 2);
    let mut v = alloc::vec![0u8; n];
    unsafe {
        core::ptr::copy_nonoverlapping(cells.as_ptr() as *const u8, v.as_mut_ptr(), n);
    }
    if let Some((x, y)) = pointer {
        if (0..80).contains(&x) && (0..25).contains(&y) {
            let at = (y as usize * 80 + x as usize) * 2 + 1;
            if at < n {
                v[at] = MOUSE_POINTER;
            }
        }
    }
    v
}

fn show(bytes: &[u8]) {
    unsafe {
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), lin(VIDEO), bytes.len().min(80 * 25 * 2));
    }
}

/// The hardware cursor, through the CRT controller: where the caret is,
/// or parked off the screen when there is none.
fn cursor(at: Option<(i16, i16)>) {
    let pos: u16 = match at {
        Some((x, y)) => (y as u16) * 80 + x as u16,
        None => 80 * 25 + 1,
    };
    outb(0x3D4, 0x0E);
    outb(0x3D5, (pos >> 8) as u8);
    outb(0x3D4, 0x0F);
    outb(0x3D5, (pos & 0xFF) as u8);
}

fn ticks() -> u32 {
    peek32(BIOS_TICKS)
}

// ------------------------------------------------------- a picture, to file

/// `/SHOT`: draw once, write the frame to SHOT.BIN through DOS, and
/// leave. The way a test looks at this program: the file is the screen.
///
/// DOS wants its buffers below a megabyte, so a block is asked of DPMI
/// (function 0100h, which hands back a real-mode segment), the frame is
/// copied there through the data selector, and INT 21h is run in real
/// mode through DPMI's 0300h with a register block saying what to do.
#[repr(C)]
#[derive(Default)]
struct RealRegs {
    edi: u32,
    esi: u32,
    ebp: u32,
    _res: u32,
    ebx: u32,
    edx: u32,
    ecx: u32,
    eax: u32,
    flags: u16,
    es: u16,
    ds: u16,
    fs: u16,
    gs: u16,
    ip: u16,
    cs: u16,
    sp: u16,
    ss: u16,
}

fn real_int21(r: &mut RealRegs) {
    real_int(0x21, r)
}

/// An interrupt in real mode through DPMI's 0300h, the registers going
/// in and coming back through the block: how DOS and the mouse driver
/// are reached from up here.
fn real_int(n: u8, r: &mut RealRegs) {
    unsafe {
        asm!(
            "push es",
            "mov es, {sel:x}",
            "int 0x31",
            "pop es",
            sel = in(reg) ds_selector(),
            in("ax") 0x0300u16,
            in("bx") n as u16,
            in("cx") 0u16,
            in("edi") r as *mut RealRegs as u32,
        );
    }
}

fn ds_selector() -> u16 {
    let s: u16;
    unsafe { asm!("mov {0:x}, ds", out(reg) s, options(nomem, nostack)) }
    s
}

/// A character into TRACE.TXT, for whoever cannot see the screen: the
/// stages of a start, so a crash says where it was.
static mut TRACE_SEG: u16 = 0;
static mut TRACE_HANDLE: u32 = 0;

fn trace(c: u8) {
    unsafe {
        if TRACE_SEG == 0 {
            let seg: u16;
            let ok: u16;
            asm!(
                "int 0x31",
                "setc {ok:l}",
                "movzx {ok:e}, {ok:l}",
                in("ax") 0x0100u16,
                in("bx") 2u16,
                lateout("ax") seg,
                ok = lateout(reg_abcd) ok,
                lateout("dx") _,
                options(nostack),
            );
            if ok != 0 {
                return;
            }
            TRACE_SEG = seg;
            let block = seg as u32 * 16;
            let name = b"TRACE.TXT\0";
            core::ptr::copy_nonoverlapping(name.as_ptr(), lin(block), name.len());
            let mut r = RealRegs { eax: 0x3C00, ecx: 0, edx: 0, ds: seg, ..Default::default() };
            real_int21(&mut r);
            if r.flags & 1 != 0 {
                return;
            }
            TRACE_HANDLE = r.eax & 0xFFFF;
        }
        let block = TRACE_SEG as u32 * 16;
        *lin(block + 16) = c;
        let mut w = RealRegs { eax: 0x4000, ebx: TRACE_HANDLE, ecx: 1, edx: 16, ds: TRACE_SEG, ..Default::default() };
        real_int21(&mut w);
    }
}

/// The DOS block the frame goes through: 4000 bytes and a name, 251
/// paragraphs, asked for once and kept. Asked for again it was refused.
static mut SHOT_SEG: u16 = 0;

fn shot(bytes: &[u8]) {
    let seg = unsafe {
        if SHOT_SEG == 0 {
            let seg: u16;
            let ok: u16;
            asm!(
                "int 0x31",
                "setc {ok:l}",
                "movzx {ok:e}, {ok:l}",
                in("ax") 0x0100u16,
                in("bx") 251u16,
                lateout("ax") seg,
                ok = lateout(reg_abcd) ok,
                lateout("dx") _,
                options(nostack),
            );
            if ok != 0 {
                return;
            }
            SHOT_SEG = seg;
        }
        SHOT_SEG
    };
    let block = seg as u32 * 16;
    unsafe {
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), lin(block), bytes.len().min(4000));
        let name = b"SHOT.BIN\0";
        core::ptr::copy_nonoverlapping(name.as_ptr(), lin(block + 4000), name.len());
    }
    let mut r = RealRegs { eax: 0x3C00, ecx: 0, edx: 4000, ds: seg, ..Default::default() };
    real_int21(&mut r);
    if r.flags & 1 != 0 {
        return;
    }
    let handle = r.eax & 0xFFFF;
    let mut w = RealRegs { eax: 0x4000, ebx: handle, ecx: 4000, edx: 0, ds: seg, ..Default::default() };
    real_int21(&mut w);
    let mut c = RealRegs { eax: 0x3E00, ebx: handle, ..Default::default() };
    real_int21(&mut c);
}

// ------------------------------------------------------------------- entry

/// Where the loader jumps: `base` is the block's linear address, `flags`
/// bit 0 is /SHOT. Both are on the stack as a C call would put them,
/// under a return address nobody uses - this never returns; it leaves
/// through DOS.
#[no_mangle]
#[link_section = ".text.start"]
pub extern "C" fn _start(base: u32, flags: u32) -> ! {
    unsafe {
        BASE = base;
    }
    trace(b'A');
    let mut app = app::App::new(80, 25);
    trace(b'B');
    let mut buf = Buffer::new(80, 25);
    let mut mouse = MouseState::detect();
    let pointer = |m: &MouseState| if m.present { Some((m.x, m.y)) } else { None };

    if flags & 4 != 0 {
        // /STRESS: what a person does to a window with a mouse for a
        // while, done in a second - drag it about, zoom it by double
        // click, open and close the dialog - so the heap is proven to
        // hold up before anyone drags anything. Then the frame goes to
        // SHOT.BIN and the code says whether it lived.
        trace(b'S');
        let title = Mouse { x: 30, y: 2, kind: MouseKind::Down(Button::Left) };
        for round in 0..20 {
            app.ui.handle(Event::Mouse(title));
            for step in 0..60i16 {
                let x = 30 + if round % 2 == 0 { step } else { 60 - step };
                app.ui.handle(Event::Mouse(Mouse { x, y: 2 + step / 20, kind: MouseKind::Drag }));
                app.after_input();
                app.ui.draw(&mut buf);
                let bytes = frame_bytes(&buf, Some((x, 2)));
                show(&bytes);
            }
            app.ui.handle(Event::Mouse(Mouse { x: 60, y: 4, kind: MouseKind::Up(Button::Left) }));
            // A double click on the title bar where the window is now:
            // zoomed, and on the next round zoomed back.
            if let Some(w) = app.ui.active_window() {
                let r = app.ui.rect(w);
                let (tx, ty) = (r.x + 8, r.y);
                app.ui.handle(Event::Mouse(Mouse { x: tx, y: ty, kind: MouseKind::Down(Button::Left) }));
                app.ui.handle(Event::Mouse(Mouse { x: tx, y: ty, kind: MouseKind::Up(Button::Left) }));
                app.ui.handle(Event::Mouse(Mouse { x: tx, y: ty, kind: MouseKind::Double(Button::Left) }));
                app.ui.handle(Event::Mouse(Mouse { x: tx, y: ty, kind: MouseKind::Up(Button::Left) }));
            }
            app.after_input();
            app.ui.handle(Event::Key(Key { code: KeyCode::F(4), mods: Mods::default() }));
            app.after_input();
            app.ui.draw(&mut buf);
            app.ui.handle(Event::Key(Key { code: KeyCode::Esc, mods: Mods::default() }));
            while app.ui.pick_pending() {
                app.ui.complete_pick();
            }
            app.after_input();
            app.ui.draw(&mut buf);
            // The window back where the drag can find it.
            app.ui.cascade();
            app.ui.draw(&mut buf);
        }
        app.ui.draw(&mut buf);
        let bytes = frame_bytes(&buf, None);
        show(&bytes);
        shot(&bytes);
        trace(b'T');
        dos_exit(0);
    }

    if flags & 1 != 0 {
        app.ui.draw(&mut buf);
        trace(b'C');
        let bytes = frame_bytes(&buf, pointer(&mouse));
        show(&bytes);
        trace(b'D');
        shot(&bytes);
        trace(b'E');
        dos_exit(0);
    }

    loop {
        app.ui.draw(&mut buf);
        let bytes = frame_bytes(&buf, pointer(&mouse));
        show(&bytes);
        cursor(app.ui.cursor().map(|p| (p.x, p.y)));
        if app.done {
            break;
        }
        // Something shown chosen: hold it two ticks (a ninth of a
        // second), then deliver.
        if app.ui.pick_pending() {
            let until = ticks().wrapping_add(2);
            // A DPMI client runs in ring 3, where HLT is not allowed:
            // spin, which under an emulator costs nothing that matters.
            let mut spins: u32 = 0;
            while ticks().wrapping_sub(until) as i32 <= 0 {
                core::hint::spin_loop();
                // Should the tick not move - a host that keeps the timer
                // to itself - the moment ends by count instead.
                spins += 1;
                if spins > 4_000_000 {
                    break;
                }
            }
            app.ui.complete_pick();
            app.after_input();
            continue;
        }
        // Wait for something: a key out of the BIOS buffer, or the mouse
        // having moved or pressed. The mouse is asked every so many turns
        // of the loop - each question is a trip to real mode.
        let mut events: [Option<Event>; 4] = [None, None, None, None];
        let mut count = 0;
        let mut turns: u32 = 0;
        while count == 0 {
            if let Some((ascii, scan)) = bios_key() {
                if let Some(k) = decode(ascii, scan) {
                    // F12 is the DOS backend's own key: the frame into
                    // SHOT.BIN, for a test that types at the program and
                    // wants to see what it saw.
                    if k.code == KeyCode::F(12) && !k.mods.alt && !k.mods.ctrl {
                        shot(&bytes);
                        continue;
                    }
                    events[0] = Some(Event::Key(k));
                    count = 1;
                }
                continue;
            }
            turns = turns.wrapping_add(1);
            if turns % 256 == 0 {
                count = mouse.poll(&mut events);
            }
            core::hint::spin_loop();
        }
        for ev in events.iter_mut().take(count) {
            if let Some(ev) = ev.take() {
                app.ui.handle(ev);
                app.after_input();
            }
        }
    }
    // A clean screen for DOS to come back to.
    unsafe {
        core::ptr::write_bytes(lin(VIDEO), 0, 80 * 25 * 2);
    }
    cursor(Some((0, 0)));
    dos_exit(0)
}
