//! The machine, for a program the DOS loader has put in extended memory.
//!
//! Everything an OWLOSUI program on DOS needs from the PC and nothing it
//! needs from the toolkit: memory below a megabyte, the video card, the
//! BIOS keyboard buffer, the mouse driver, the clock, DOS itself, a heap
//! and a panic that says where it happened. Two programs stand on it:
//!
//!  * the resident, `src/owlosres.rs` - the toolkit behind a software
//!    interrupt, for programs written in assembler, Pascal or C;
//!  * `Examples/DOS/Rust`, a Rust program with the core linked in.
//!
//! The loader (`LOADER.INC`) asks DPMI for a block of extended memory,
//! reads the program into it, and makes a code and a data selector whose
//! base is that block. Every address below a megabyte - the video card at
//! B8000, the BIOS data area at 400 - is reached through the same data
//! selector, as the linear address minus the block's base, wrapping: the
//! selector's limit is 4 GB and DPMI hosts leave the first megabyte
//! where it is.
//!
//! `no_std`: `core`, `alloc` on the allocator below, and this.

#![no_std]

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::alloc::{GlobalAlloc, Layout};
use core::arch::asm;
use owlosui_core::{Buffer, Button, Event, Key, KeyCode, Mods, Mouse, MouseKind};

// ------------------------------------------------------------- the machine

/// The linear address the block was loaded at: set once by [`init`].
static mut BASE: u32 = 0;

/// The first thing a program does: say where the loader put it.
pub fn init(base: u32) {
    unsafe {
        BASE = base;
    }
}

/// A pointer to a linear address, through the data selector.
pub fn lin(addr: u32) -> *mut u8 {
    // The selector's base is BASE and its limit is 4 GB, so an offset
    // wraps round to any linear address, the first megabyte included.
    unsafe { addr.wrapping_sub(BASE) as *mut u8 }
}

/// A real-mode `segment:offset` as a linear address.
pub fn real(seg: u16, off: u16) -> u32 {
    seg as u32 * 16 + off as u32
}

pub fn peek8(addr: u32) -> u8 {
    unsafe { core::ptr::read_volatile(lin(addr)) }
}

pub fn poke8(addr: u32, v: u8) {
    unsafe { core::ptr::write_volatile(lin(addr), v) }
}

pub fn peek16(addr: u32) -> u16 {
    unsafe { core::ptr::read_volatile(lin(addr) as *const u16) }
}

pub fn poke16(addr: u32, v: u16) {
    unsafe { core::ptr::write_volatile(lin(addr) as *mut u16, v) }
}

pub fn peek32(addr: u32) -> u32 {
    unsafe { core::ptr::read_volatile(lin(addr) as *const u32) }
}

/// Bytes out of the first megabyte, or into it.
pub fn read_real(addr: u32, into: &mut [u8]) {
    unsafe { core::ptr::copy_nonoverlapping(lin(addr), into.as_mut_ptr(), into.len()) }
}

pub fn write_real(addr: u32, from: &[u8]) {
    unsafe { core::ptr::copy_nonoverlapping(from.as_ptr(), lin(addr), from.len()) }
}

pub fn outb(port: u16, v: u8) {
    unsafe { asm!("out dx, al", in("dx") port, in("al") v, options(nomem, nostack)) }
}

pub const VIDEO: u32 = 0xB8000;
const BIOS_KB_FLAGS: u32 = 0x417;
const BIOS_KB_HEAD: u32 = 0x41A;
const BIOS_KB_TAIL: u32 = 0x41C;
const BIOS_KB_START: u32 = 0x480;
const BIOS_KB_END: u32 = 0x482;
const BIOS_TICKS: u32 = 0x46C;

/// Leave through DOS. DPMI hosts pass INT 21h through, and AH=4Ch is
/// how a client ends.
pub fn dos_exit(code: u8) -> ! {
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
/// The block the loader gives us is zeroed; the heap lives in `.bss`,
/// which the flat binary does not carry and the loader's zeroing makes.
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

fn decimal(mut n: u32, mut put: impl FnMut(u8)) {
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

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    // Say so in the corner, in red, and leave: there is nothing to
    // unwind into. The file and line go to TRACE.TXT for whoever cannot
    // see the screen.
    trace(b'P');
    if let Some(loc) = info.location() {
        let file = loc.file().as_bytes();
        for &b in &file[file.len().saturating_sub(24)..] {
            trace(b);
        }
        trace(b':');
        decimal(loc.line(), trace);
    }
    // And on the screen's top row, in red, for whoever is at the machine:
    // PANIC, the file and the line.
    let mut col: u32 = 0;
    let mut put = |b: u8| {
        if col < 80 {
            poke8(VIDEO + col * 2, b);
            poke8(VIDEO + col * 2 + 1, 0x4F);
            col += 1;
        }
    };
    for b in b"PANIC ".iter() {
        put(*b);
    }
    if let Some(loc) = info.location() {
        let file = loc.file().as_bytes();
        for &b in &file[file.len().saturating_sub(40)..] {
            put(b);
        }
        put(b':');
        decimal(loc.line(), put);
    }
    dos_exit(3)
}

// ----------------------------------------------------------------- keyboard

/// The next key out of the BIOS buffer, if there is one: the ASCII code
/// and the scan code, as INT 16h would hand them over. Reading the ring
/// directly instead of calling INT 16h saves a trip to real mode per
/// key; the BIOS keeps filling it, because its interrupt handler still
/// runs down there.
pub fn bios_key() -> Option<(u8, u8)> {
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
pub fn decode(ascii: u8, scan: u8) -> Option<Key> {
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
pub struct MouseState {
    pub present: bool,
    pub x: i16,
    pub y: i16,
    buttons: u16,
    /// For the double click: when and where the last press was.
    last_down: u32,
    last_at: (i16, i16, u16),
}

const MOUSE_POINTER: u8 = 0x4F; // white on red

impl MouseState {
    pub fn detect() -> MouseState {
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

    /// Where the pointer is, to paint it; `None` without a mouse.
    pub fn pointer(&self) -> Option<(i16, i16)> {
        if self.present {
            Some((self.x, self.y))
        } else {
            None
        }
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
    pub fn poll(&mut self, out: &mut VecDeque<Event>) {
        if !self.present {
            return;
        }
        let (x, y, buttons) = self.read();
        if x != self.x || y != self.y {
            let kind = if self.buttons != 0 { MouseKind::Drag } else { MouseKind::Move };
            out.push_back(Event::Mouse(Mouse { x, y, kind }));
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
            out.push_back(Event::Mouse(Mouse { x, y, kind }));
        }
        self.x = x;
        self.y = y;
        self.buttons = buttons;
    }
}

/// Wait for input: a key out of the BIOS buffer, or the mouse having
/// moved or pressed, into `out`. The mouse is asked every so many turns
/// of the loop - each question is a trip to real mode. F12 is the DOS
/// screen's own key: `frame` into SHOT.BIN and SHOT.TXT, for a test that
/// types at a program and wants to see what it saw.
pub fn wait_input(mouse: &mut MouseState, out: &mut VecDeque<Event>, frame: &[u8]) {
    let mut turns: u32 = 0;
    while out.is_empty() {
        if let Some((ascii, scan)) = bios_key() {
            if let Some(k) = decode(ascii, scan) {
                if k.code == KeyCode::F(12) && !k.mods.alt && !k.mods.ctrl {
                    shot(frame);
                    continue;
                }
                out.push_back(Event::Key(k));
            }
            continue;
        }
        turns = turns.wrapping_add(1);
        if turns % 256 == 0 {
            mouse.poll(out);
        }
        core::hint::spin_loop();
    }
}

// ------------------------------------------------------------------ screen

/// The frame as the card wants it, the pointer painted on: a cell is
/// two bytes and so is the card's, so this is the frame's own bytes with
/// one attribute changed.
pub fn frame_bytes(buf: &Buffer, pointer: Option<(i16, i16)>) -> Vec<u8> {
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

pub fn show(bytes: &[u8]) {
    write_real(VIDEO, &bytes[..bytes.len().min(80 * 25 * 2)]);
}

/// The screen blank, the cursor home: what DOS should come back to.
pub fn clear_screen() {
    unsafe {
        core::ptr::write_bytes(lin(VIDEO), 0, 80 * 25 * 2);
    }
    for i in 0..80 * 25u32 {
        poke8(VIDEO + i * 2 + 1, 0x07);
    }
    cursor(Some((0, 0)));
}

/// The hardware cursor, through the CRT controller: where the caret is,
/// or parked off the screen when there is none.
pub fn cursor(at: Option<(i16, i16)>) {
    let pos: u16 = match at {
        Some((x, y)) => (y as u16) * 80 + x as u16,
        None => 80 * 25 + 1,
    };
    outb(0x3D4, 0x0E);
    outb(0x3D5, (pos >> 8) as u8);
    outb(0x3D4, 0x0F);
    outb(0x3D5, (pos & 0xFF) as u8);
}

pub fn ticks() -> u32 {
    peek32(BIOS_TICKS)
}

/// A pressed button is shown down for two ticks, a ninth of a second,
/// before it happens. A DPMI client runs in ring 3, where HLT is not
/// allowed: spin, which under an emulator costs nothing that matters.
pub fn hold() {
    let until = ticks().wrapping_add(2);
    let mut spins: u32 = 0;
    while ticks().wrapping_sub(until) as i32 <= 0 {
        core::hint::spin_loop();
        // Should the tick not move - a host that keeps the timer to
        // itself - the moment ends by count instead.
        spins += 1;
        if spins > 4_000_000 {
            break;
        }
    }
}

// ------------------------------------------------------ DOS, from up here

/// The real-mode registers DPMI's 0300h reads and fills, and a real-mode
/// callback is handed.
#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct RealRegs {
    pub edi: u32,
    pub esi: u32,
    pub ebp: u32,
    pub _res: u32,
    pub ebx: u32,
    pub edx: u32,
    pub ecx: u32,
    pub eax: u32,
    pub flags: u16,
    pub es: u16,
    pub ds: u16,
    pub fs: u16,
    pub gs: u16,
    pub ip: u16,
    pub cs: u16,
    pub sp: u16,
    pub ss: u16,
}

/// An interrupt in real mode through DPMI's 0300h, the registers going
/// in and coming back through the block: how DOS and the mouse driver
/// are reached from up here.
pub fn real_int(n: u8, r: &mut RealRegs) {
    unsafe {
        asm!(
            "push es",
            "mov es, {sel:x}",
            "int 0x31",
            "pop es",
            sel = in(reg) ds_selector(),
            inout("ax") 0x0300u16 => _,
            in("bx") n as u16,
            in("cx") 0u16,
            in("edi") r as *mut RealRegs as u32,
        );
    }
}

pub fn ds_selector() -> u16 {
    let s: u16;
    unsafe { asm!("mov {0:x}, ds", out(reg) s, options(nomem, nostack)) }
    s
}

pub fn cs_selector() -> u16 {
    let s: u16;
    unsafe { asm!("mov {0:x}, cs", out(reg) s, options(nomem, nostack)) }
    s
}

/// A DOS memory block of `paragraphs`, through DPMI 0100h: its real-mode
/// segment, or `None`.
pub fn dos_alloc(paragraphs: u16) -> Option<u16> {
    let seg: u16;
    let failed: u16;
    unsafe {
        asm!(
            "int 0x31",
            "setc {f:l}",
            "movzx {f:e}, {f:l}",
            inout("ax") 0x0100u16 => seg,
            in("bx") paragraphs,
            f = lateout(reg_abcd) failed,
            lateout("dx") _,
            options(nostack),
        );
    }
    if failed != 0 {
        None
    } else {
        Some(seg)
    }
}

/// The one DOS block this program writes files through: 8 KB below a
/// megabyte, asked for once and kept - asked for again, DPMI hosts have
/// been seen to refuse. The trace uses its first 32 bytes, a picture of
/// the screen the next 6.5 KB, and the last 1.5 KB are free for the
/// program (the resident keeps its EXEC block there).
static mut SCRATCH: u16 = 0;
pub const SCRATCH_FREE: u16 = 0x1A00;

pub fn scratch() -> Option<u16> {
    unsafe {
        if SCRATCH == 0 {
            SCRATCH = dos_alloc(0x200)?;
        }
        Some(SCRATCH)
    }
}

/// A file written through DOS from a block of the scratch area.
fn dos_write_file(seg: u16, name_at: u16, data_at: u16, len: u16) {
    let mut r = RealRegs { eax: 0x3C00, ecx: 0, edx: name_at as u32, ds: seg, ..Default::default() };
    real_int(0x21, &mut r);
    if r.flags & 1 != 0 {
        return;
    }
    let handle = r.eax & 0xFFFF;
    let mut w = RealRegs { eax: 0x4000, ebx: handle, ecx: len as u32, edx: data_at as u32, ds: seg, ..Default::default() };
    real_int(0x21, &mut w);
    let mut c = RealRegs { eax: 0x3E00, ebx: handle, ..Default::default() };
    real_int(0x21, &mut c);
}

/// A character into TRACE.TXT, for whoever cannot see the screen: the
/// stages of a start, so a crash says where it was.
static mut TRACE_HANDLE: u32 = 0;

pub fn trace(c: u8) {
    let Some(seg) = scratch() else { return };
    let block = real(seg, 0);
    unsafe {
        if TRACE_HANDLE == 0 {
            write_real(block, b"TRACE.TXT\0");
            let mut r = RealRegs { eax: 0x3C00, ecx: 0, edx: 0, ds: seg, ..Default::default() };
            real_int(0x21, &mut r);
            if r.flags & 1 != 0 {
                return;
            }
            TRACE_HANDLE = r.eax & 0xFFFF;
        }
        poke8(block + 16, c);
        let mut w = RealRegs { eax: 0x4000, ebx: TRACE_HANDLE, ecx: 1, edx: 16, ds: seg, ..Default::default() };
        real_int(0x21, &mut w);
    }
}

/// Frames written so far by this run: the first one starts SHOTS.TXT
/// afresh, the later ones are added to its end.
static mut SHOTS: u32 = 0;

/// The frame into SHOT.BIN, exactly as the card holds it, into SHOT.TXT
/// as twenty-five lines of text, and onto the end of SHOTS.TXT, which
/// keeps every frame of the run in order - the way a test looks at a
/// program: the file is the screen, and `findstr` can read the text ones.
pub fn shot(bytes: &[u8]) {
    let Some(seg) = scratch() else { return };
    const BIN: u16 = 0x100;
    const TXT: u16 = BIN + 4000;
    const NAMES: u16 = TXT + 82 * 25;
    let n = bytes.len().min(4000);
    write_real(real(seg, BIN), &bytes[..n]);
    let mut text = [0u8; 82 * 25];
    for y in 0..25 {
        for x in 0..80 {
            let at = (y * 80 + x) * 2;
            let c = if at < n { bytes[at] } else { b' ' };
            text[y * 82 + x] = if c < 32 { b' ' } else { c };
        }
        text[y * 82 + 80] = b'\r';
        text[y * 82 + 81] = b'\n';
    }
    write_real(real(seg, TXT), &text);
    write_real(real(seg, NAMES), b"SHOT.BIN\0SHOT.TXT\0SHOTS.TXT\0");
    dos_write_file(seg, NAMES, BIN, 4000);
    dos_write_file(seg, NAMES + 9, TXT, 82 * 25);
    // SHOTS.TXT: created by the first frame, opened and added to after.
    let first = unsafe {
        SHOTS += 1;
        SHOTS == 1
    };
    if first {
        dos_write_file(seg, NAMES + 18, TXT, 82 * 25);
        return;
    }
    let mut o = RealRegs { eax: 0x3D01, edx: (NAMES + 18) as u32, ds: seg, ..Default::default() };
    real_int(0x21, &mut o);
    if o.flags & 1 != 0 {
        return;
    }
    let handle = o.eax & 0xFFFF;
    let mut end = RealRegs { eax: 0x4202, ebx: handle, ..Default::default() };
    real_int(0x21, &mut end);
    let mut w = RealRegs { eax: 0x4000, ebx: handle, ecx: 82 * 25, edx: TXT as u32, ds: seg, ..Default::default() };
    real_int(0x21, &mut w);
    let mut c = RealRegs { eax: 0x3E00, ebx: handle, ..Default::default() };
    real_int(0x21, &mut c);
}
