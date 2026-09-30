# DOS, Rust

The core and [the shared application](../../shared/app.rs) - the one the
browser and the Windows window run - on DOS.

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

That makes `OWLOS.BIN` (the program) and `OWLOS.COM` (its loader).

## Run

In DOSBox-X, from Windows - set `DOSBOXX` to your `dosbox-x.exe`:

```
Examples\DOS\Rust\RUNWIN.BAT
```

On a real machine, or inside any DOS with 32-bit DPMI: copy the folder and
type `RUN`. Plain DOSBox has no 32-bit DPMI and cannot run it.

In DOSBox-X's TrueType output (`output=ttf`) the double frames come out
single: that mode draws the screen with a Windows font, and the default one
draws `╔═╗` with single lines. `font = consola` in the `[ttf]` section, or
`output=surface`, and they are as they are in video memory.

## Test

```
Examples\DOS\Rust\test.cmd
```

Builds, runs one frame under DOSBox-X and checks that the frame the program
wrote through DOS (`SHOT.BIN`) is the screen the emulator shows (`SCREEN.BIN`,
dumped from B800 by `SCRDUMP.COM`): the file is the screen. Then DOSBox-X
types at the program - F4 for the dialog, Escape, F12 (a frame into
`SHOT.BIN`), F10 and the menu's Exit - and it must come back with code 0.
Last, `RUN STRESS` has the program drag, zoom and open dialogs by itself for
a while, and the heap has to hold.

A panic writes its file and line on the screen's top row and into
`TRACE.TXT`; `LOADER.LOG` is the loader's stage marks.

## What is in it

```
src/main.rs            the screen: text memory, BIOS keys, the INT 33h mouse, the heap
OWLOS.ASM              the loader (FASM): DPMI, four megabytes, two selectors, a jump
i386-owlos-none.json   the target: 32-bit x86, no OS, soft float
link.ld                a flat binary that starts at its first byte
SCRDUMP.ASM            the screen as it is, into SCREEN.BIN, for the test
RUN.BAT, RUNWIN.BAT    run it in DOS, or DOSBox-X from Windows
build.cmd, test.cmd    build, and the test above
CWSDPMI.EXE            the DPMI host, free, beside the program that needs it
```

The program is built with `build-std` for `i386-owlos-none`: no `std`,
`core` and `alloc` on a small allocator of size classes. The loader asks
DPMI for memory, reads the file into it, makes a code and a data selector
whose base is that block, and jumps to its first byte. Drawing is one copy:
a cell is the two bytes the video card wants. The mouse is drawn into the
frame as the red square DOS programs drew, and a double click is told apart
by the BIOS tick. [OWLOS.ASM](OWLOS.ASM) reads like a lesson.
