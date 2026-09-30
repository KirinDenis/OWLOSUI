# DOS, Rust

Rust programs on DOS with the core linked in - no resident, no interrupt:
the program is the toolkit.

* [hello.rs](src/hello.rs) and [demo.rs](src/demo.rs) - HELLO and DEMO,
  the same programs as in `..\Asm`, `..\C` and `..\Pascal`, part for part:
  menu bar, status line, calculator, calendar, ASCII table, puzzle, File
  Open with the file panel, editor windows. Where those ask the resident
  for each part over INT 60h, these make it from the core's own types -
  [kit.rs](src/kit.rs) is the handful of helpers that do what the server
  does for a request - and hand the core to `owlosui_dos::run`, the loop
  every DOS program here has.
* [main.rs](src/main.rs) - [the shared application](../../shared/app.rs),
  the one the browser and the Windows window run, and the test modes that
  prove the screen is the frame.

![DOS](../../screens/dos.png)

## Build

Needs, once, Rust's nightly toolchain with its sources (the standard
library is rebuilt for a target of our own), and FASM for Windows:

```
rustup toolchain install nightly --component rust-src
```

Set `FASM` to your `fasm.exe`, or put it on the PATH. Then:

```
Examples\DOS\Rust\build.cmd
```

That makes `HELLO.BIN`, `DEMO.BIN` and `OWLOS.BIN` (the programs, flat
32-bit binaries) and a loader `.COM` for each.

## Run

In DOSBox-X, from Windows - set `DOSBOXX` to your `dosbox-x.exe`:

```
Examples\DOS\Rust\RUNWIN.BAT DEMO
Examples\DOS\Rust\RUNWIN.BAT HELLO
Examples\DOS\Rust\RUNWIN.BAT
```

The last is the shared application. On a real machine, or inside any DOS
with 32-bit DPMI: copy the folder and type `RUN DEMO`, `RUN HELLO` or `RUN`.
Plain DOSBox has no 32-bit DPMI and cannot run them.

In DOSBox-X's TrueType output (`output=ttf`) the double frames come out
single: that mode draws the screen with a Windows font, and the default one
draws `╔═╗` with single lines. `font = consola` in the `[ttf]` section, or
`output=surface`, and they are as they are in video memory.

## Test

```
Examples\DOS\Rust\test.cmd
```

Builds, runs one frame of the shared application under DOSBox-X and checks
that the frame it wrote through DOS (`SHOT.BIN`) is the screen the emulator
shows (`SCREEN.BIN`, dumped from B800 by `SCRDUMP.COM`): the file is the
screen. Then DOSBox-X types at it - F4 for the dialog, Escape, F12 (a frame
into `SHOT.BIN`), F10 and the menu's Exit - and it must come back with code
0. `RUN STRESS` has it drag, zoom and open dialogs by itself for a while,
and the heap has to hold. Last, HELLO and DEMO get the keys and the checks
every DOS folder shares ([../TESTKEYS.BAT](../TESTKEYS.BAT),
[../CHECK.CMD](../CHECK.CMD)).

A panic writes its file and line on the screen's top row and into
`TRACE.TXT`; `LOADER.LOG` is the loader's stage marks.

## What is in it

```
src/hello.rs, src/demo.rs   HELLO and DEMO
src/kit.rs             the parts they are made of, from the core's types
src/main.rs            the shared application, and the test modes
HELLO.ASM, DEMO.ASM,   their loaders: lib/dos/LOADER.INC - DPMI, four
OWLOS.ASM              megabytes, two selectors, a jump
i386-owlos-none.json   the target: 32-bit x86, no OS, soft float
link.ld                a flat binary that starts at its first byte
SCRDUMP.ASM            the screen as it is, into SCREEN.BIN, for the test
RUN.BAT, RUNWIN.BAT    run them in DOS, or DOSBox-X from Windows
build.cmd, test.cmd    build, and the test above
CWSDPMI.EXE            the DPMI host, free, beside the programs that need it
```

The machine under them - video memory, the BIOS keyboard, the INT 33h
mouse, the heap, and DOS's folders and files - is
[lib/dos](../../../lib/dos/README.md), the layer the resident stands on
too. The programs are built with `build-std` for `i386-owlos-none`: no
`std`, `core` and `alloc` on a small allocator of size classes. Drawing is
one copy: a cell is the two bytes the video card wants. On DOS a glyph is
that byte, so the ASCII table puts glyph 1 in a cell and gets the face, and
a file's bytes are its text as they are. The mouse is drawn into the frame
as the red square DOS programs drew, and a double click is told apart by
the BIOS tick.
