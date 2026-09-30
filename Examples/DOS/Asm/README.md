# DOS, assembler

Two FASM programs that draw with OWLOSUI through the resident:

* [HELLO.ASM](HELLO.ASM) - a window, words, a button.
* [DEMO.ASM](DEMO.ASM) - the demo: menu bar, status line, calculator,
  calendar, ASCII table, puzzle, File Open with the file panel, editor
  windows. The same program as
  `..\C\DEMO.C` and `..\Pascal\DEMO.PAS`, part for part.

Both include [lib/dos/asm/OWLOSUI.INC](../../../lib/dos/asm/OWLOSUI.INC),
which is the whole binding: a request is built field by field with small
macros and sent through INT 60h.

```asm
        owl_op   OP_STATIC      ; a new request
        owl_w    [window]       ; parent
        owl_rect 2,1,36,1       ; x y w h
        owl_s    'Hello, world!'
        owl_send                ; AX = the new view's id
```

## Run

Under DOS, in this folder (the `.COM` files are in the repository):

```
RUN HELLO
RUN DEMO
```

From Windows, in DOSBox-X - set `DOSBOXX` to your `dosbox-x.exe`:

```
Examples\DOS\Asm\RUNWIN.BAT DEMO
```

## Build

```
Examples\DOS\Asm\BUILD.CMD
```

with the FASM for Windows (set `FASM`, or put `FASM.EXE` on the PATH).
Under DOS the DOS FASM does the same: `FASM DEMO.ASM DEMO.COM`.

## Test

```
Examples\DOS\Asm\TEST.CMD
```

Builds both, runs them under the resident in DOSBox-X, types the keys of
[../TESTKEYS.BAT](../TESTKEYS.BAT) at them and checks the screens they
showed ([../CHECK.CMD](../CHECK.CMD)).

## Worth reading

* `calc_key` and `calc_apply` - a pocket calculator's arithmetic on the
  386's 32-bit registers, with `jo` catching an overflow.
* The calculator's WINDOW_STATUS: seventeen keys with no labels, which the
  window carries while it is in front - that is how typing `12+30=` works
  without an input line.
* `blit_cells` - a canvas is filled with one BLIT request: a character and
  a colour a cell. The ASCII table sends each glyph as the Unicode
  character the glyph table (GET_GLYPHS) says it is.
