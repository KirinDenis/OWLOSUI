//! OWLOSRES - the toolkit behind a software interrupt, for DOS programs in
//! any language.
//!
//!     OWLOSRES DEMO.EXE [arguments]
//!
//! It loads the core, hooks INT 60h, and runs the program named after it.
//! For as long as that program runs, INT 60h answers the requests of
//! `lib/PROTOCOL.md` - the same bytes the C# client sends down a pipe and
//! the JavaScript one into a WebAssembly module - and when the program
//! ends, the vector is put back, the screen cleared, and OWLOSRES leaves
//! with the program's own exit code. Like CWSDPMI without `-p`, it stays
//! for exactly one program.
//!
//! The call, from real mode:
//!
//!     DS:SI   the request: op:u8 len:u16 payload
//!     ES:DI   room for the reply: status:u8 len:u16 body
//!     CX      how big that room is
//!     INT 60h
//!     AL      the status: 0 OK, 1 an error (the body says what),
//!             2 the room was too small (only the header was written;
//!             its len says how much body there is)
//!     CX      the reply's whole length, header included
//!
//! Every other register comes back as it went in. The eight bytes before
//! the handler's address are `OWLOSUI\0`, so a program can see that it was
//! started by OWLOSRES before it calls.
//!
//! The screen is the resident's, as a window is the server's on Windows
//! (OPEN_WINDOW): it draws the desktop into B800, reads the BIOS keyboard
//! and the mouse, and the program draws nothing. WAIT (0x5C) is how a
//! program waits for the person: it answers after one key or click with
//! what that pressed or chose. So a DOS program's main loop is
//!
//!     repeat  WAIT;  act on the command  until done
//!
//! How it works: the program is a DPMI client (`LOADER.INC` put it in
//! extended memory). It asks DPMI for a real-mode callback - an address
//! in real mode that, when called, switches to protected mode and runs a
//! procedure here - points INT 60h at a four-byte stub that jumps there,
//! and runs the program through DOS's EXEC, itself called through DPMI.
//! While the program runs, this one is inside that EXEC call; each INT
//! 60h comes up through the callback onto a stack of its own.

#![no_std]
#![no_main]

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::vec;
use alloc::vec::Vec;
use core::arch::{asm, global_asm};
use owlosui_core::{Buffer, Event};
use owlosui_dos::*;
use owlosui_serve::{op, Server};

/// The interrupt the resident answers. 60h to 66h are the vectors the PC's
/// documentation leaves to user programs.
const VECTOR: u8 = 0x60;
const SIGNATURE: &[u8; 8] = b"OWLOSUI\0";

// ---------------------------------------------------------------- the host

struct Host {
    server: Server,
    /// Input the program has not asked for yet.
    queue: VecDeque<Event>,
    mouse: Option<MouseState>,
    buf: Buffer,
    /// INIT has been answered: the screen is ours until the program ends.
    screen: bool,
}

static mut HOST: Option<Host> = None;

impl Host {
    fn call(&mut self, op: u8, payload: &[u8]) -> (u8, Vec<u8>) {
        match op {
            op::INIT => {
                // On DOS the desktop is the screen: whatever size was
                // asked for, it is 80 by 25. The code page stays.
                let mut p = payload.to_vec();
                if p.len() >= 4 {
                    p[..4].copy_from_slice(&[80, 0, 25, 0]);
                }
                let (status, body, _) = self.server.call(op, &p);
                if status == 0 && !self.screen {
                    self.screen = true;
                    self.mouse = Some(MouseState::detect());
                }
                (status, body)
            }
            op::WAIT => match self.server.ui_mut() {
                Some(_) => (0, self.wait()),
                None => error("INIT first"),
            },
            op::OPEN_WINDOW => error("on DOS the screen is already the program's window"),
            _ => {
                let (status, body, _) = self.server.call(op, payload);
                (status, body)
            }
        }
    }

    fn draw(&mut self) -> Vec<u8> {
        let ui = self.server.ui_mut().expect("drawn after INIT");
        ui.draw(&mut self.buf);
        let caret = ui.cursor().map(|p| (p.x, p.y));
        let bytes = frame_bytes(&self.buf, self.mouse.as_ref().and_then(|m| m.pointer()));
        show(&bytes);
        cursor(caret);
        bytes
    }

    /// One event into the core - waited for, if none is queued - and what
    /// it pressed or chose: `pressed:u16 command:u16 w:i16 h:i16`.
    fn wait(&mut self) -> Vec<u8> {
        if self.queue.is_empty() {
            let frame = self.draw();
            let mouse = self.mouse.as_mut().expect("the mouse is asked for at INIT");
            wait_input(mouse, &mut self.queue, &frame);
        }
        let ev = self.queue.pop_front().expect("wait_input returns with something");
        let ui = self.server.ui_mut().unwrap();
        ui.handle(ev);
        // A pressed button is seen down before it happens.
        while self.server.ui_mut().unwrap().pick_pending() {
            self.draw();
            hold();
            self.server.ui_mut().unwrap().complete_pick();
        }
        self.draw();
        let ui = self.server.ui_mut().unwrap();
        let pressed = ui.take_pressed().unwrap_or(0);
        let command = ui.take_command().unwrap_or(0);
        let mut body = Vec::with_capacity(8);
        body.extend_from_slice(&pressed.to_le_bytes());
        body.extend_from_slice(&command.to_le_bytes());
        body.extend_from_slice(&80i16.to_le_bytes());
        body.extend_from_slice(&25i16.to_le_bytes());
        body
    }
}

fn error(what: &str) -> (u8, Vec<u8>) {
    let mut body = (what.len() as u16).to_le_bytes().to_vec();
    body.extend_from_slice(what.as_bytes());
    (1, body)
}

// ----------------------------------------------------------- the interrupt

/// What the callback was given: the program's registers at its INT 60h.
static mut CB_REGS: RealRegs = RealRegs {
    edi: 0, esi: 0, ebp: 0, _res: 0, ebx: 0, edx: 0, ecx: 0, eax: 0,
    flags: 0, es: 0, ds: 0, fs: 0, gs: 0, ip: 0, cs: 0, sp: 0, ss: 0,
};
/// The host's stack while a request is answered: ESP, then SS.
static mut CB_SAVE: [u32; 2] = [0; 2];
const CB_STACK_SIZE: usize = 256 * 1024;
static mut CB_STACK: [u8; CB_STACK_SIZE] = [0; CB_STACK_SIZE];

// The callback. DPMI calls it with interrupts off, DS:ESI on the real-mode
// stack (where INT 60h left the flags and the return address), ES:EDI on
// CB_REGS, and SS:ESP on a small stack of the host's. It does what the
// program's IRET would have done - the return address and the flags off
// the real-mode stack into the registers the host will return with - then
// moves to a stack of its own, big enough for Rust, answers the request
// with the interrupts on, and goes back with IRETD, ES:EDI still on the
// registers.
global_asm!(
    ".global owl_callback",
    "owl_callback:",
    "cld",
    "mov ax, word ptr [esi]",
    "mov word ptr es:[edi + 0x2A], ax",
    "mov ax, word ptr [esi + 2]",
    "mov word ptr es:[edi + 0x2C], ax",
    "mov ax, word ptr [esi + 4]",
    "mov word ptr es:[edi + 0x20], ax",
    "add word ptr es:[edi + 0x2E], 6",
    "mov ax, es",
    "mov ds, ax",
    "mov dword ptr [{save}], esp",
    "mov word ptr [{save} + 4], ss",
    "mov ss, ax",
    "lea esp, [{stack} + {size}]",
    "push edi",
    "sti",
    "call {handler}",
    "cli",
    "pop edi",
    "lss esp, fword ptr [{save}]",
    "push ds",
    "pop es",
    "iretd",
    save = sym CB_SAVE,
    stack = sym CB_STACK,
    size = const CB_STACK_SIZE,
    handler = sym owl_int,
);

extern "C" {
    fn owl_callback();
}

/// One request: read it out of the program's memory, answer it, write the
/// reply back, and say in AL and CX how it went.
#[no_mangle]
extern "C" fn owl_int(r: &mut RealRegs) {
    let request = real(r.ds, r.esi as u16);
    let reply = real(r.es, r.edi as u16);
    let room = (r.ecx & 0xFFFF) as usize;
    let mut head = [0u8; 3];
    read_real(request, &mut head);
    let len = u16::from_le_bytes([head[1], head[2]]) as usize;
    let mut payload = vec![0u8; len];
    read_real(request + 3, &mut payload);

    #[allow(static_mut_refs)]
    let host = unsafe { HOST.as_mut().expect("the host is made before the program runs") };
    let (mut status, body) = host.call(head[0], &payload);
    let total = 3 + body.len();
    if total > room {
        status = 2;
    }
    if room >= 3 {
        let len = (body.len() as u16).to_le_bytes();
        write_real(reply, &[status, len[0], len[1]]);
    }
    if status != 2 {
        write_real(reply + 3, &body);
    }
    r.eax = (r.eax & 0xFFFF_FF00) | status as u32;
    r.ecx = (r.ecx & 0xFFFF_0000) | (total as u32 & 0xFFFF);
}

// ------------------------------------------------------------------- DPMI

/// A real-mode callback for `owl_callback`, its registers in CB_REGS: the
/// real-mode address, as `(segment, offset)`.
fn alloc_callback() -> Option<(u16, u16)> {
    let (seg, off): (u16, u16);
    let failed: u32;
    unsafe {
        // DS:ESI the procedure (the code selector can be read through),
        // ES:EDI its registers. ESI is LLVM's, so it is saved by hand.
        asm!(
            "push esi",
            "push ds",
            "push es",
            "push ds",
            "pop es",
            "mov ax, cs",
            "mov ds, ax",
            "mov esi, offset {cb}",
            "mov ax, 0x0303",
            "int 0x31",
            "setc al",
            "movzx eax, al",
            "pop es",
            "pop ds",
            "pop esi",
            cb = sym owl_callback,
            in("edi") core::ptr::addr_of_mut!(CB_REGS) as u32,
            out("eax") failed,
            out("cx") seg,
            out("dx") off,
        );
    }
    if failed != 0 {
        None
    } else {
        Some((seg, off))
    }
}

fn free_callback(seg: u16, off: u16) {
    unsafe {
        asm!("int 0x31", inout("ax") 0x0304u16 => _, in("cx") seg, in("dx") off);
    }
}

fn get_vector(n: u8) -> (u16, u16) {
    let (seg, off): (u16, u16);
    unsafe {
        asm!("int 0x31", inout("ax") 0x0200u16 => _, in("bl") n, out("cx") seg, out("dx") off);
    }
    (seg, off)
}

fn set_vector(n: u8, seg: u16, off: u16) {
    unsafe {
        asm!("int 0x31", inout("ax") 0x0201u16 => _, in("bl") n, in("cx") seg, in("dx") off);
    }
}

/// A `$`-ended line on the screen, through DOS.
fn say(seg: u16, text: &[u8]) {
    let at = SCRATCH_FREE + 0x300;
    write_real(real(seg, at), text);
    let mut r = RealRegs { eax: 0x0900, edx: at as u32, ds: seg, ..Default::default() };
    real_int(0x21, &mut r);
}

// ------------------------------------------------------------------ entry

/// Where the loader jumps: `base` is the block's linear address, `flags`
/// the loader's switches (unused here), `psp` this program's PSP, whose
/// command tail names the program to run.
#[no_mangle]
#[link_section = ".text.start"]
pub extern "C" fn _start(base: u32, _flags: u32, psp: u32) -> ! {
    init(base);
    let Some(seg) = scratch() else { dos_exit(4) };

    // The command tail: the program's name, then its own arguments.
    let tail_at = real(psp as u16, 0x80);
    let n = peek8(tail_at) as usize;
    let mut tail = vec![0u8; n];
    read_real(tail_at + 1, &mut tail);
    let text: &[u8] = &tail;
    let start = text.iter().position(|&c| c != b' ').unwrap_or(text.len());
    let end = text[start..].iter().position(|&c| c == b' ').map_or(text.len(), |e| start + e);
    let name = &text[start..end];
    let args = &text[end..];
    if name.is_empty() {
        say(seg, b"OWLOSRES runs a DOS program with the OWLOSUI toolkit behind INT 60h.\r\n\
Usage: OWLOSRES PROGRAM.EXE [arguments]\r\n$");
        dos_exit(1);
    }

    // The EXEC block, in the free end of the scratch area: the name, the
    // tail DOS will copy into the program's PSP, the parameter block, two
    // empty FCBs, and the stub INT 60h points at - the signature, then a
    // far jump to the callback.
    let b = SCRATCH_FREE;
    let (name_at, tail_at, params_at, fcb_at, stub_at) = (b, b + 0x80, b + 0x100, b + 0x120, b + 0x180);
    let mut z = vec![0u8; 0x200];
    z[..name.len().min(127)].copy_from_slice(&name[..name.len().min(127)]);
    write_real(real(seg, name_at), &z[..0x80]);
    let mut t = vec![0u8; 0x80];
    let an = args.len().min(126);
    t[0] = an as u8;
    t[1..1 + an].copy_from_slice(&args[..an]);
    t[1 + an] = b'\r';
    write_real(real(seg, tail_at), &t);
    let mut p = Vec::new();
    p.extend_from_slice(&0u16.to_le_bytes()); // the environment: ours
    for off in [tail_at, fcb_at, fcb_at] {
        p.extend_from_slice(&off.to_le_bytes());
        p.extend_from_slice(&seg.to_le_bytes());
    }
    write_real(real(seg, params_at), &p);
    write_real(real(seg, fcb_at), &[0u8; 0x40]);

    let Some((cb_seg, cb_off)) = alloc_callback() else {
        say(seg, b"OWLOSRES: the DPMI host gave no real-mode callback.\r\n$");
        dos_exit(4);
    };
    let mut stub = [0u8; 13];
    stub[..8].copy_from_slice(SIGNATURE);
    stub[8] = 0xEA; // JMP FAR ptr16:16
    stub[9..11].copy_from_slice(&cb_off.to_le_bytes());
    stub[11..13].copy_from_slice(&cb_seg.to_le_bytes());
    write_real(real(seg, stub_at), &stub);

    unsafe {
        HOST = Some(Host {
            server: Server::new(),
            queue: VecDeque::new(),
            mouse: None,
            buf: Buffer::new(80, 25),
            screen: false,
        });
    }
    let (old_seg, old_off) = get_vector(VECTOR);
    set_vector(VECTOR, seg, stub_at + 8);

    // The program. This call returns when it has ended.
    let mut r = RealRegs {
        eax: 0x4B00,
        edx: name_at as u32,
        ebx: params_at as u32,
        ds: seg,
        es: seg,
        ..Default::default()
    };
    real_int(0x21, &mut r);
    let ran = r.flags & 1 == 0;
    let dos_error = r.eax & 0xFFFF;

    set_vector(VECTOR, old_seg, old_off);
    free_callback(cb_seg, cb_off);
    #[allow(static_mut_refs)]
    let took_screen = unsafe { HOST.as_ref().is_some_and(|h| h.screen) };
    if took_screen {
        clear_screen();
    }
    if !ran {
        say(seg, if dos_error == 2 {
            b"OWLOSRES: no such program.\r\n$"
        } else {
            b"OWLOSRES: DOS would not run the program.\r\n$"
        });
        dos_exit(5);
    }
    let mut code = RealRegs { eax: 0x4D00, ..Default::default() };
    real_int(0x21, &mut code);
    dos_exit((code.eax & 0xFF) as u8)
}
