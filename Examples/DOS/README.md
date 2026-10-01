# DOS

Programs on DOS: a real machine, DOSBox-X, or anything in between.

```
Asm/            HELLO and DEMO in assembler (FASM)       \
C/              HELLO and DEMO in C (Open Watcom)          > through the resident
Pascal/         HELLO and DEMO in Pascal (Borland Pascal 7) /
Rust/           HELLO and DEMO in Rust, and the shared application: the core linked in
Commandr/       a two-panel file manager in Pascal, Norton Commander's shape
TESTKEYS.BAT    the keys the tests type - the same for every language
CHECK.CMD       runs a folder's HELLO and DEMO and checks what they showed
```

## Assembler, C, Pascal: through the resident

A DOS program in any language reaches the toolkit through OWLOSRES,
[the resident](../../lib/dos/README.md): it loads the core, hooks INT 60h
and runs the program. The program sends the requests of
[PROTOCOL.md](../../lib/PROTOCOL.md) - the same ones the C# and
JavaScript examples send - and draws nothing: the resident owns the
screen, the keyboard and the mouse. Each language has a small binding in
[lib/dos](../../lib/dos/README.md).

The three folders hold the same two programs:

* **HELLO** - a window, words, a button. The shape of every program:
  build the windows, then wait until the answer is OK.
* **DEMO** - a menu bar, a status line, and four tools: a calculator
  (button rows, and the keys it carries while it is in front), a calendar
  (a canvas drawn again for every month), an ASCII table (a click names
  the glyph), the fifteen puzzle (click, or the arrows). File, Open (F3)
  is the file panel: the program reads the folder through DOS's FindFirst
  and FindNext and hands it over, Enter walks into a folder or opens a
  file in a viewer. File, New opens an editor; the Window menu zooms,
  cycles, cascades and tiles.

They are the same program part for part, so any one can be read against
the others - and they are tested with the same keys and the same checks.

In every folder:

```
RUN DEMO           under DOS: CWSDPMI, then OWLOSRES with the program;
                   from Windows: DOSBox-X starts and does the same in there
RUNWIN DEMO        from Windows, the same as RUN DEMO there
BUILD              from Windows: the programs from their sources
TEST               from Windows: build, run both, type at them, check
```

The built programs are in the repository, so `RUN` works on a DOS machine
with nothing else installed.

| | compiler | builds under | free |
|---|---|---|---|
| [Asm](Asm/README.md) | FASM | Windows (FASM for Windows) or DOS | yes |
| [C](C/README.md) | Open Watcom 1.9, 16-bit | DOSBox 0.74 | yes |
| [Pascal](Pascal/README.md) | Borland Pascal 7 | DOSBox 0.74 | no - not in this repository |

Watcom's compiler runs on the DOS/4GW extender, which hangs without a
word in the DOSBox-X here, and Borland's is built the same way for
consistency: the compilers run in plain DOSBox, the programs in DOSBox-X,
whose 32-bit DPMI the resident needs. `DOSBOX` and `DOSBOXX` say where
the two are.

## The commander

[Commandr/](Commandr/README.md) is a program rather than a demo of parts:
a file manager in Pascal, two folders side by side - view, edit, copy,
move, make a folder, delete, a drive list and a tree of the drive. Every
file it touches, it touches through DOS. `RUN` in its folder starts it,
`TEST.CMD` tries all of it on a folder of its own.

## Rust: the core linked in

[Rust/](Rust/README.md) has HELLO and DEMO too, the same two programs the
other way in: they link the core and make each part from its own types,
where the other three ask the resident for it. No interrupt: the program
is the toolkit. They stand on the same machine layer as the resident
(`lib/dos`) and get the same test. Beside them is
[the shared application](../shared/app.rs), the one the browser and the
Windows window run, built for DOS.

## Testing all of it

```
Examples\DOS\Asm\TEST.CMD
Examples\DOS\C\TEST.CMD
Examples\DOS\Pascal\TEST.CMD
Examples\DOS\Rust\test.cmd
Examples\DOS\Commandr\TEST.CMD
```

DOSBox-X types at the programs with AUTOTYPE, which presses keys one at a
time and cannot hold Alt, so the test reaches the menus with F10 and the
arrows. F12 photographs the screen into `SHOTS.TXT`, and `CHECK.CMD`
looks for what should be there: the menu bar, 12+30 shown as 42, the
calendar's days, the table, a tile slid, a file opened and a folder
walked, the About box. The keys are typed on a US layout whatever the
host's is: DOSBox-X follows the Windows keyboard, and on a Slovak one the
digit row types letters. The DOSBox-X here faults while closing after some
runs that ended well, so the emulator's exit code is not looked at; the
program's is.
