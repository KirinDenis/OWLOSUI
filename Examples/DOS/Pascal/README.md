# DOS, Pascal

Two Pascal programs that draw with OWLOSUI through the resident:

* [HELLO.PAS](HELLO.PAS) - a window, words, a button.
* [DEMO.PAS](DEMO.PAS) - the demo: menu bar, status line, calculator,
  calendar, ASCII table, puzzle, File Open with the file panel, editor
  windows. The same program as
  `..\Asm\DEMO.ASM` and `..\C\DEMO.C`, part for part.

Both use the unit [lib/dos/pascal/OWLOSUI.PAS](../../../lib/dos/pascal/OWLOSUI.PAS),
the whole binding: every call builds a request, calls INT 60h with
`Intr`, and reads the reply. A Turbo Vision programmer will know the
shape, with the drawing done by the resident instead of linked-in units.

```pascal
Win := OwlWindow(0, -1, -1, 42, 9, OwlDialog, 'Hello', 0);
OwlStatic(Win, 2, 1, 36, 1, 'Hello, world!');
OwlButtons(Win, 1);
OwlButton(CmOk, OwlDefault, '~O~K');
OwlEnd;
repeat OwlWait(Pressed, Command) until Pressed = CmOk;
```

## Run

Under DOS, in this folder (the `.EXE` files are in the repository):

```
RUN HELLO
RUN DEMO
```

From Windows, in DOSBox-X - set `DOSBOXX` to your `dosbox-x.exe`:

```
Examples\DOS\Pascal\RUNWIN.BAT DEMO
```

## Build

With Borland Pascal 7's command-line compiler, for real mode:

```
BPC -CD -U..\..\..\lib\dos\pascal DEMO.PAS
```

`MAKE.BAT` does that for both, under DOS. From Windows:

```
Examples\DOS\Pascal\BUILD.CMD
```

which runs `MAKE.BAT` in plain DOSBox 0.74 with Borland Pascal mounted as
`P:`. Set `BPDIR` to the folder holding `BIN\BPC.EXE`, and `DOSBOX` to
`DOSBox.exe`.

Borland Pascal is commercial, and it is not in this repository. The
programs are plain Turbo Pascal - `Dos` is the only unit used besides
OWLOSUI - so Turbo Pascal 7 and Free Pascal's `i8086-msdos` target, in
`{$mode tp}`, should build them too. Only Borland Pascal 7 has been tried.

## Test

```
Examples\DOS\Pascal\TEST.CMD
```

Builds both, runs them under the resident in DOSBox-X, types the keys of
[../TESTKEYS.BAT](../TESTKEYS.BAT) at them and checks the screens they
showed ([../CHECK.CMD](../CHECK.CMD)).
