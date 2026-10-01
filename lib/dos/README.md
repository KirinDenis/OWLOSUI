# lib/dos - the toolkit on DOS

Two ways onto a DOS machine, both built on the same few hundred lines of
PC plumbing:

* **OWLOSRES, the resident.** The toolkit behind a software interrupt, for
  programs written in anything that can call one: assembler, Pascal, C.
  The examples are in [Examples/DOS](../../Examples/DOS/README.md).
* **The machine layer**, a Rust library for a program that links the core
  itself. [Examples/DOS/Rust](../../Examples/DOS/Rust/README.md) is one.

```
OWLOSRES.COM, OWLOSRES.BIN   the resident, built; start a program with it
CWSDPMI.EXE                  the DPMI host both need
LOADER.INC                   the loader: DPMI, four megabytes, two selectors, a jump
OWLOSRES.ASM                 the resident's loader: LOADER.INC and nothing else
src/owlosres.rs              the resident: INT 60h, EXEC, the screen
src/lib.rs                   the machine: memory, B800, the keyboard, INT 33h, a heap,
                             DOS's folders and files, and `run`, a program's main loop
asm/OWLOSUI.INC              the binding for FASM
c/OWLOSUI.H, c/OWLOSUI.C     the binding for C (Open Watcom, 16-bit)
pascal/OWLOSUI.PAS           the binding for Pascal (Borland Pascal 7)
build.cmd                    builds OWLOSRES.BIN and OWLOSRES.COM
```

## Running a program with it

```
CWSDPMI
OWLOSRES DEMO.EXE [its arguments]
```

OWLOSRES loads the core, hooks INT 60h, runs the program it was given,
and when that program ends puts the vector back, clears the screen and
leaves with the program's exit code. Like CWSDPMI without `-p`, it stays
for exactly one program. Each example folder's `RUN.BAT` does exactly
this.

It needs a 386 and 32-bit DPMI: a real machine with CWSDPMI, or DOSBox-X.
Plain DOSBox has no 32-bit DPMI. The program it runs may be any real-mode
DOS program: a `.COM` from FASM, an `.EXE` from a Pascal compiler or Watcom.

## The call

A request is exactly the bytes of [PROTOCOL.md](../PROTOCOL.md) - the ones
the C# client sends down a pipe and the JavaScript one into a WebAssembly
module - handed over in memory:

```
DS:SI   the request: op:u8 len:u16 payload
ES:DI   room for the reply: status:u8 len:u16 body
CX      the size of that room
INT 60h
AL      the status: 0 OK, 1 an error (the body is the message),
        2 the room was too small (only the header is written;
        its len says how much body there is)
CX      the reply's whole length, header included
```

Every other register comes back as it went in. The eight bytes before
the handler are `OWLOSUI` and a zero, so a program can check that it was
started by OWLOSRES before it calls; each binding's `owl_check` does, and
says how to start the program if not.

The screen belongs to the resident, as a window belongs to the server on
Windows after OPEN_WINDOW. It draws the desktop into video memory, reads
the BIOS keyboard and the mouse, and the program draws nothing. The
program waits for the person with WAIT (0x5C), which answers after one
key or click with what that pressed or chose. So every DOS program's main
loop is the same:

```
repeat
    WAIT            pressed, command
    act on them
until done
```

On DOS the desktop is always 80 by 25, whatever INIT asked for, and the
font is the card's: 437, or 866 when INIT says so, and a character with
no glyph becomes `?`.

F12 is the resident's own key: the screen into `SHOT.BIN` and `SHOT.TXT`,
and onto the end of `SHOTS.TXT`, in the current folder. That is how the
tests look at a program.

## Running another program from one

A program may run another - the commander does, on Enter - and the other
one may be a toolkit program with windows of its own. SUSPEND (0x63) puts
the first program's session aside, every window whole, and gives the screen
to DOS in text mode; then the program runs the other through DOS's EXEC,
which finds the resident still there; then RESUME (0x64) brings text mode
back - a game may have left graphics - and the first session, drawn. Asked
to, RESUME first leaves what a plain DOS program printed on the screen until
a key. In Pascal, `OwlRun(Path, Args, Pause)` is all of it; the program's
`$M` must leave DOS the memory the other program needs.

## How it works

The resident is a DPMI client: LOADER.INC put it in extended memory. It
asks DPMI for a real-mode callback - an address in real mode that, when
called, switches to protected mode and runs a procedure of its own -
and points INT 60h at a small stub that holds the signature and jumps
there. Then it runs the program through DOS's EXEC, itself called through
DPMI. While the program runs, the resident is inside that EXEC call, and
each INT 60h comes up through the callback, onto a stack of its own, into
the same `Server` that `owlosui-serve` runs on Windows. The callback
first does what the program's IRET would have done - the return address
and flags off the real-mode stack - so the program sees an ordinary
interrupt.

## Building it

```
lib\dos\build.cmd
```

Needs, once, Rust's nightly toolchain with its sources, because `core`
and `alloc` are rebuilt for a target of our own (`i386-owlos-none`), and
a FASM for Windows (set `FASM`, or put it on the PATH):

```
rustup toolchain install nightly --component rust-src
```

`OWLOSRES.BIN` and `OWLOSRES.COM` are kept in the repository, so a
program in assembler, Pascal or C does not need Rust at all.

## The bindings

Each one is the wire and nothing else: build a request field by field,
send it, read the reply. They are small enough to read in one sitting,
and they are named alike, so a program reads the same in all three:

| | assembler | C | Pascal |
|---|---|---|---|
| a window | `owl_op OP_WINDOW` ... `owl_send` | `owl_window(...)` | `OwlWindow(...)` |
| a request with parts | `owl_button` in the request | `owl_buttons(w, 2); owl_button(...); owl_end();` | `OwlButtons(W, 2); OwlButton(...); OwlEnd;` |
| wait | `call owl_wait` - AX, DX | `owl_wait(&pressed, &command)` | `OwlWait(Pressed, Command)` |
| a file panel | `call owl_files` - BX, DX, SI | `owl_files(win, 1, dir)` | `OwlFiles(Win, 1, Dir)` |
| what it reports | `call owl_take_files` - AL | `owl_take_files(panel, text, n)` | `OwlTakeFiles(Panel, Text)` |
| a file as text | `call owl_text_dos` | `owl_text_dos(win, bytes, n, 1)` | `OwlTextDos(Win, Buf, N, 1)` |

The server reads no disks, on any platform: a program reads the folder
and sends it. The bindings do that part through DOS - FindFirst and
FindNext into a listing, sent in pieces when it is bigger than one
request - so a DOS program only walks the paths. `owl_text_dos` sends a
file's bytes as text: each byte through the glyph table, the machine's
code page, into UTF-8, which is what the wire carries.
