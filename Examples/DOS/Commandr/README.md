# DOS, Pascal: the commander

A two-panel file manager in the classic DOS shape, for DOS,
written in Pascal: [COMMANDR.PAS](COMMANDR.PAS). The folder and the
program are called COMMANDR because a DOS name has at most eight letters.

**Quickest:** double-click `DOS_Commander.cmd` in the repository's root:
it starts DOSBox-X with the repository on the left and its `Examples` on
the right.

| The two sides, three files marked | The editor, coloured as Pascal |
|---|---|
| ![The file manager asking before it copies three marked files](../../screens/dos_commander.png) | ![The Pascal demo's DEMO.PAS in the file manager's editor, in colour](../../screens/dos_editor.png) |

The same program for Windows is
[Desktop/CSharp/03-Commander](../../Desktop/CSharp/03-Commander/Program.cs).
Here every folder, file and drive is DOS's own - `FindFirst`, `BlockRead`,
`MkDir`, `DiskSize` - and the toolkit, behind INT 60h, only shows what the
program hands it.

## Run it

Under DOS, in this folder (`COMMANDR.EXE` is in the repository):

```
RUN                 both sides on this folder
RUN C:\ C:\GAMES    the left side on C:\, the right side on C:\GAMES
```

From Windows, in DOSBox-X - set `DOSBOXX` to your `dosbox-x.exe`:

```
Examples\DOS\Commandr\RUNWIN.BAT
```

In there the repository is drive C:, so `RUNWIN C:\ C:\LIB` shows two
of its folders.

## The keys

| key | does |
|---|---|
| Tab | the other side |
| Enter | into a folder; `..` back out; on a `.COM`, `.EXE` or `.BAT`, it runs, in its own folder (see below); on any other file, what DOS knows about it. In the folder the cursor starts on its first name - on `..` only when there is nothing else |
| Insert | mark a file, and move down |
| F1 | help - with buttons for what needs Alt |
| F3 | view the file under the cursor |
| F4 | edit it; F2 saves, Escape closes without saving |
| F5, F6 | copy, move to the other side: the marked files, or the one under the cursor |
| F7 | make a folder |
| F8 | delete: the marked files, or the one under the cursor |
| F10 | quit; in the viewer or the editor, close it |
| Alt+F1, Alt+F2 | a drive for the left or the right side |
| Alt+F10 | the drive as a tree of folders; the other side follows the cursor |
| Ctrl+R | both folders read again - for files another program put there, such as the web page this commander runs in, in the live demo's DOS PC |

Copy, move and delete ask first, and **No** is the answer Enter gives:
doing something to files is chosen on purpose, with the arrow key and
Enter, or Alt+Y. The marks are used up once their files have been dealt
with, as they were in the classic file managers.

The viewer and the editor bring an Edit menu to nobody - there is no menu
bar here - but their keys work: Ctrl+F finds, Ctrl+L finds again, and the
text is coloured as the language its name says, `.PAS`, `.BAT`, `.ASM`.
The mouse does the rest: dragging with the left button selects, and a
right click opens Undo, Cut, Copy, Paste and Select all where the pointer
is. In DOSBox-X, and in a DOS box under Windows, the clipboard is the
computer's: text copied here pastes into Notepad and back. On a DOS with
nothing above it the toolkit keeps a clipboard of its own.

## What to know

* **How a program runs.** A toolkit program - the demos, HELLO - runs from
  inside the commander: the panels are put aside whole while it has the
  screen, and come back after it. Any other program gets the whole of
  DOS's memory, as from the prompt: the commander steps out of memory,
  `RUN.BAT` runs the program and starts the commander again in the same
  folders. A game like OWL FLY III needs that - it lays its buffers out
  at fixed distances above itself, and loaded higher it would write past
  640K into the video card. Started without `RUN.BAT` (no `/STEP`), the
  commander runs every program from inside.
* **The editor takes a whole file or nothing.** A file too big for one
  request to the resident (about 30 KB of text) is refused, and F3 shows
  its beginning, with the title saying how much. Half a file in an editor
  would save the other half away.
* **Tabs come back as spaces**, and lines end CR LF, the way DOS keeps a
  text file.
* **A file moves by renaming** when it stays on the same drive: nothing is
  copied. A folder, or a file to another drive, is copied and then deleted.
* **Read-only files** can be copied, and are deleted and replaced like any
  other - this is a file manager, and it asked first.

## Build

With Borland Pascal 7's command-line compiler, for real mode:

```
BPC -CD -U..\..\..\lib\dos\pascal COMMANDR.PAS
```

`MAKE.BAT` does that under DOS. From Windows, `BUILD.CMD` runs it in plain
DOSBox 0.74 with Borland Pascal mounted as P:; set `BPDIR` to the folder
holding `BIN\BPC.EXE`. Borland Pascal is commercial and is not in this
repository.

The program uses the Pascal unit
[lib/dos/pascal/OWLOSUI.PAS](../../../lib/dos/pascal/OWLOSUI.PAS). What a
file manager needed that the demo did not - marks, several names at once,
an input line, a list, a progress bar, a tree, an editor's text back as
DOS bytes - went into the unit, where the next program finds it.

## Test

```
Examples\DOS\Commandr\TEST.CMD
```

Builds it, makes a folder `PLAY` with a few files in it, and runs the
commander on `PLAY\A` and `PLAY\B` in DOSBox-X, which types the keys that
`RUN.BAT`'s `:test` lists: view, copy, make a folder, move a folder,
delete, edit and save, properties, and - through the Help box's buttons,
because AUTOTYPE cannot hold Alt - the tree and the drive list. Then it
checks twice: what was on the screen, and what is on the disk.
