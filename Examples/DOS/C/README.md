# DOS, C

Two C programs that draw with OWLOSUI through the resident:

* [HELLO.C](HELLO.C) - a window, words, a button.
* [DEMO.C](DEMO.C) - the demo: menu bar, status line, calculator,
  calendar, ASCII table, puzzle, File Open with the file panel, editor
  windows. The same program as
  `..\Asm\DEMO.ASM` and `..\Pascal\DEMO.PAS`, part for part.

Both are built with [lib/dos/c/OWLOSUI.C](../../../lib/dos/c/OWLOSUI.C)
and [OWLOSUI.H](../../../lib/dos/c/OWLOSUI.H), the whole binding: every
call builds a request, calls INT 60h with `int86x`, and reads the reply.

```c
win = owl_window(0, -1, -1, 42, 9, OWL_DIALOG, "Hello", 0);
owl_static(win, 2, 1, 36, 1, "Hello, world!");
owl_buttons(win, 1);
owl_button(CM_OK, OWL_DEFAULT, "~O~K");
owl_end();
do owl_wait(&pressed, &command); while (pressed != CM_OK);
```

## Run

Under DOS, in this folder (the `.EXE` files are in the repository):

```
RUN HELLO
RUN DEMO
```

From Windows, in DOSBox-X - set `DOSBOXX` to your `dosbox-x.exe`:

```
Examples\DOS\C\RUNWIN.BAT DEMO
```

## Build

With [Open Watcom](https://open-watcom.github.io) 1.9, the 16-bit DOS
compiler, small model:

```
wcl -ms -bt=dos DEMO.C ..\..\..\lib\dos\c\OWLOSUI.C
```

`MAKE.BAT` does that for both, under DOS. From Windows:

```
Examples\DOS\C\BUILD.CMD
```

which runs `MAKE.BAT` in plain DOSBox 0.74 with Watcom mounted as `W:`.
Set `WATCOMDIR` to the folder holding Watcom's `BINW` and `H`, and
`DOSBOX` to `DOSBox.exe`. Not DOSBox-X: Watcom's compiler runs on the
DOS/4GW extender, and that hangs in the DOSBox-X here without a word.

## Test

```
Examples\DOS\C\TEST.CMD
```

Builds both, runs them under the resident in DOSBox-X, types the keys of
[../TESTKEYS.BAT](../TESTKEYS.BAT) at them and checks the screens they
showed ([../CHECK.CMD](../CHECK.CMD)).
