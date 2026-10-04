# OWLOSUI

**[Live demo: Rust, WebAssembly and DOS in one page](https://kirindenis.github.io/OWLOSUI/)** -
the toolkit in your browser, and a DOS PC beside it, in DOSBox, running the
same core: the two share a floppy, and the PC plays a DOS game over the network.

A text mode UI toolkit in the classic DOS style, with one portable core.

Windows you can drag and resize, menus, dialogs, an editor with syntax
colours, a two-panel file manager - all drawn with characters, the way DOS
programs were. One core, written in Rust, draws all of it. Programs in C#,
JavaScript, Rust, Pascal, C and assembler use that core, and the same
windows appear on a Windows console, in a window of their own, in a
browser and on DOS.

Free and open source, under the [MIT licence](LICENSE).

## See it

| The live demo: what to see first | The live demo: a DOS PC in a window |
|---|---|
| ![The demo's first window: a list of what to see, a few words on each](Examples/screens/browser_welcome.png) | ![DOSBox in an OWLOSUI window, running the file manager on DOS](Examples/screens/browser_dos.png) |
| **One floppy, two worlds: the browser's files beside DOS** | **DOS on the network: every packet, as the page sees it** |
| ![The browser's files on the left, DOS on the right, a file copied between them](Examples/screens/browser_floppy.png) | ![The network monitor: bytes, a graph and the IPX packets of a DOS game](Examples/screens/browser_network.png) |
| **On DOS: the demo** | **On DOS: the file manager** |
| ![The demo on DOS: calculator, calendar, ASCII table, puzzle](Examples/screens/dos_demo.png) | ![A two-panel file manager on DOS, three files marked, asking before it copies](Examples/screens/dos_commander.png) |
| **On DOS: the editor, coloured as Pascal** | **On Windows: a window of its own** |
| ![The file manager's editor showing Pascal source in colour](Examples/screens/dos_editor.png) | ![The C# demo in a native Windows window, a dialog of controls](Examples/screens/window.png) |
| **In a browser: opening a file from the server** | **In a browser: the same file in the editor** |
| ![The browser demo's Open dialog, explaining where the files are](Examples/screens/browser_open.png) | ![Pascal source in the browser editor, in colour](Examples/screens/browser_code.png) |
| **In a browser: the editor's own Edit menu** | **On DOS: the shared Rust application** |
| ![The Edit menu: undo, find, replace, syntax, word wrap, hex view](Examples/screens/browser_menu.png) | ![The Rust application in DOSBox-X](Examples/screens/dos.png) |

## Try it

Or nothing to install: the [live demo](https://kirindenis.github.io/OWLOSUI/)
is the browser one, with its DOS PC. To run things on your own machine,
double-click one of these in the repository's folder - or type its name
in a console there. Each one checks that what it needs is installed, and
says where to get it if not. The first run of a C#, Rust or browser one
builds first, which takes a minute or two.

| Double-click | What you see | Needs |
|---|---|---|
| `CS_Demo.cmd` | the C# demo in the console: every tool and control | [.NET 8 SDK](https://dotnet.microsoft.com/download), [Rust](https://rustup.rs) |
| `CS_Window.cmd` | the same demo in a window of its own | .NET 8 SDK, Rust |
| `CS_Commander.cmd` | a two-panel file manager in C#, on real files | .NET 8 SDK, Rust |
| `Rust_Terminal.cmd` | the Rust demo in the console | Rust |
| `Rust_Window.cmd` | the Rust application in a window | Rust |
| `Web_Demo.cmd` | the demo in your browser, at http://localhost:8765 | Rust, and once `rustup target add wasm32-unknown-unknown` |
| `DOS_Pascal_Demo.cmd` | the demo on DOS, written in Pascal | [DOSBox-X](https://dosbox-x.com) |
| `DOS_C_Demo.cmd` | the same demo, written in C | DOSBox-X |
| `DOS_Asm_Demo.cmd` | the same demo, written in assembler | DOSBox-X |
| `DOS_Rust_Demo.cmd` | the same demo, written in Rust | DOSBox-X |
| `DOS_Commander.cmd` | the file manager on DOS, in Pascal | DOSBox-X |

The DOS programs are already built and in the repository, so the DOS ones
need nothing but the emulator. In DOSBox-X the repository is drive C:.
When a DOS program has ended, close DOSBox-X's window.

In the demos **F10** opens the menu bar and **Alt+X** leaves; in the file
managers the keys along the bottom say what they do, and on DOS and in the
Windows console **F10** quits - in the page, where F10 is the menu bar's,
the commander is a window like the others and **Alt+F3** closes it.
Everywhere **Tab** moves between the parts of a dialog, **F1** is help,
and the mouse works: the wheel scrolls, dragging selects text, a right
click opens Cut, Copy and Paste.

## What is where

```
CS_Demo.cmd ... DOS_Commander.cmd    the launchers above
Examples/                            every program that uses the toolkit
    Desktop/                         on Windows: C# (six steps), Rust
    Web/                             in a browser: JavaScript (five steps), Rust
    DOS/                             on DOS: assembler, C, Pascal, Rust,
                                     and the file manager, Commandr
    screens/                         the pictures on this page
lib/                                 the toolkit itself
```

Every folder has a README that says what is in it and how to build and
run it, and every program says in its first lines which step it is and
where the next one is. Read on below for how the toolkit is built.

## For programmers: where to start

The toolkit is in [lib/](lib/README.md). Everything that uses it is in
[Examples/](Examples/README.md), by where it runs and then by language:

* **Desktop** - [Examples/Desktop](Examples/Desktop/README.md). C#: `dotnet
  run` in [Examples/Desktop/CSharp/01-HelloWorld](Examples/Desktop/CSharp/01-HelloWorld/Program.cs),
  six steps up to the full demo in a window of its own. Rust: `cargo run -p
  owlosui-demo` on the terminal, `cargo run -p owlosui-win` in a window.
* **Web** - [Examples/Web](Examples/Web/README.md). `Examples\Web\RUN.CMD`
  builds the core for the browser, serves it and opens the demo; five
  JavaScript steps mirror the C# ones.
* **DOS** - [Examples/DOS](Examples/DOS/README.md). The same demo in
  assembler, C and Pascal, each drawing through OWLOSRES - the toolkit
  behind INT 60h - and the Rust application as a DOS program of its own.
  `Examples\DOS\Asm\RUNWIN.BAT DEMO` shows it in DOSBox-X.
* **Speak the wire** - [lib/PROTOCOL.md](lib/PROTOCOL.md): every request, its
  bytes and its reply. The C# and JavaScript clients are two implementations
  of it.
* **Write a screen** - the contract is below, and the ones that exist are
  short: [the terminal](lib/console/src/term.rs),
  [the Windows window](lib/window/src/lib.rs),
  [the canvas](lib/js/owlosui.js),
  [DOS](lib/dos/src/lib.rs).

## The contract

Three things cross the line between the core and a backend, and nothing else:

* events go **in** — `Event::Key`, `Event::Mouse`, `Event::Resize`, `Event::Tick`
* a grid of cells comes **out** — `Buffer`, two bytes per cell
* the backend decides how to show it

A cell is a glyph index plus an IBM PC attribute byte: low nibble foreground,
high nibble background, sixteen colours. On DOS a row of those goes to the
video hardware with no conversion at all. Everywhere else the backend
translates — and the backend is the only thing that needs to.

## Layout

```
lib/                  the toolkit
    core/             no dependencies, ever. Cell grid, view tree, event dispatch.
    console/          the terminal screen, the code pages; owlosui-match, which
                      checks scenes against reference DOS screens
    window/           the Windows window screen: CreateWindow and GDI
    serve/            the core behind a pipe, speaking PROTOCOL.md
    csharp/           the C# client of that pipe, the core carried inside it
    js/               the JavaScript client, and the core as WebAssembly for it;
                      apps/: a commander, editors with Open and Save as, a DOS
                      PC with its settings and monitors, a log and its console
    dos/              OWLOSRES, the core behind INT 60h; its bindings for
                      assembler, C and Pascal; the DOS machine layer
    PROTOCOL.md       the wire. Same numbers for a pipe, a module, an interrupt.

Examples/             everything that uses it
    Desktop/CSharp    six steps from HelloWorld to the demo in a window, and Tests
    Desktop/Rust      01-Terminal, 02-Window
    Web/RUN.CMD       build, serve, open the demo
    Web/JavaScript    five steps from HelloWorld to the demo, and test.mjs
    Web/Rust          01-Linked: the core and the application in one module
    DOS/Asm, DOS/C, DOS/Pascal
                      HELLO and the demo, the same in each, through the resident
    DOS/Rust          HELLO and the demo with the core linked in, and the shared
                      application: flat binaries under DPMI
    DOS/Commandr      a two-panel file manager in Pascal, through the resident
    shared/app.rs     the one Rust application the window, the web and DOS draw
    screens/          the pictures in the READMEs

*.cmd                 the launchers: CS_, Rust_, Web_ and DOS_ something
```

## A program

The whole of HelloWorld, in C#:

```csharp
using var owl = new Owlosui();

var w = owl.Window("Hello", 40, 9, style: Owlosui.Style.Dialog);
owl.Static(w, 2, 1, "Hello, world!");
owl.Buttons(w, ("~O~K", CmOk));

owl.Run(cmd => cmd != CmOk);
```

`Owlosui` starts `owlosui-serve`, sends it what happened, asks for the frame
and puts the cells on the console - or, after `owl.OpenWindow()`, lets the
server show them in a window of its own. The Rust core decides what every
cell looks like; the C# program never sees a keystroke of the editor and
never draws a line of a frame. The JavaScript version is the same calls on
a canvas.

Because the output medium *is* a grid of characters, a rendered screen is its
own screenshot. That makes the tests readable pictures (`lib/core/tests/`),
and it is what lets a program - or a model - check a layout without vision.

## Glyphs, code pages and Unicode

The core never sees Unicode. A cell is a glyph index; a title is a string
of glyph indices; an editor line is a `Vec<Glyph>` — the screen of a DOS
machine, where a number in video memory *is* the picture. One type alias
decides how big that number is:

```
cargo build -p owlosui-core --features dos    # Glyph = u8:  the byte in B800
cargo build -p owlosui-core                    # Glyph = u16: a font that grows
```

Nothing else in the core changes between the two, and the tests pass in
both. The frames, shades and arrows live below 256 in either, so they are
drawn by the same numbers on a DOS card and on a Windows console.

What a glyph *looks like* is the **font**, in one place:
`lib/console/src/codepage.rs`. It starts as a code page — 437 (the IBM
PC's) or 866 (Cyrillic, same frames at the same places) — and is either
*fixed* at those 256 (the DOS build, and the terminal backend, which draws
for one console) or *growing*: every character it has never seen gets the
next index, so a Windows or browser client shows Russian and Slovak on one
screen, as Far does, while the same program on DOS shows what its page
has and `?` for the rest. The server's font grows. Text crosses the wire as
UTF-8, becomes glyph indices there and text again on the way out;
`GET_GLYPHS` gives a client the table and every `FRAME` says how long it
is now, so the client fetches it again only when a frame reaches past what
it holds. `Owlosui.Fits(text)` still answers the DOS question — on a fixed
font, "will this show or turn to `?`" — and Notes opens a file it cannot
show read-only rather than save `?`s back into it.

The page to start from: `new Owlosui(codePage: 866)`, or
`OWLOSUI_CODEPAGE=866` in the environment, or the system's own OEM page
when it is one we have; the terminal demo reads the same variable.

## Checking against the real thing

Several scenes exist twice: here, and in `TOOLS/REFGEN/REFGEN.PAS` in the
wire-city repository, where the same scenes are built with the classic DOS
toolkit they follow and dumped straight out of video memory. Then:

```
cargo run -p owlosui-console --bin owlosui-match -- one-window path/to/REF01.BIN --rows 1:23
```

renders the scene here and reports which cells disagree — grouped by the
*kind* of difference, because forty cells wrong in the same way are one
mistake, not forty.

The reference dumps are not committed: they are the output of commercial
units. What is committed is what they taught us, as constants with the
measurement noted beside them.

This closed the argument the project had been having with screenshots. Three
of the four scenes now match byte for byte, and the differences it found were
ones nobody could have seen: the status-line keys were bright red instead of
red, an inactive window's *background* is a different colour from an active
one's, and the yellow everybody remembers is the editor's text and not the
window's fill.

## Rules this project keeps

* **Colour comes from the palette by role, never from the element.** An element
  says what it is; the palette says what colour that is. This is the first rule
  people will want to break and the one that keeps an application coherent.
* **Whatever DOS cannot do does not exist anywhere.** The constraint is the
  design. A backend may degrade (an external link opens a tab in a browser and
  prints a URL on DOS) but may never be richer.
* **Font and code page are swappable assets.** The core emits glyph *indices*;
  CP437 is a default, not an assumption. A Cyrillic CP866 or a home computer
  character set is another file, not another code path.
* **The defaults are already right.** Construct a thing and it looks correct —
  centred, framed, navigable. Coordinates are an override, not a starting
  point.
* **Keys are a table, not code.** The core exposes primitives (`toggle_zoom`,
  `cycle_windows`, `close`); which key invokes which belongs to the
  application. The classic DOS editors shipped four keymaps for one editor this way.
* **No callbacks out of the core.** Results are collected — `take_pressed`,
  `take_command`, `pending` — never delivered. That is what lets the same
  core sit behind a pipe, an interrupt or a WebAssembly boundary unchanged.

## Editing

The editor holds no key codes. It knows commands with names — `WordRight`,
`DeleteLine` — and a table turns keys into them. That is how the classic DOS editors shipped
four arrangements for one editor; here there are two, `Keymap::Classic` (the
WordStar control diamond) and `Keymap::Modern` (Ctrl+C/V/Z), with the CUA
clipboard keys in both because Ctrl+C is taken by the signal in a terminal.

Every change to the text is one operation — a span replaced by some lines — so
undo is written once and is already right for commands that do not exist yet.

Because the commands have names, a test can be a script:

```text
text    one\ntwo
keys    <Down><Home><BS>
expect  onetwo
cursor  0 3
```

Those live in `lib/core/tests/scripts`, and a failure prints the script line,
what it got and what it wanted.

## Help

A small HTML viewer: links, headings, `<b>`, paragraphs, rules, lists,
`<pre>`. No tables, no images, no CSS — and no `<i>`, because a character cell
cannot be slanted and whatever DOS cannot do does not exist anywhere.

The view cannot open a file and does not try. Following a link puts the href
in `pending`; resolving it belongs to the application.

## Status

Working: desktop, overlapping framed windows (move, resize, zoom, close,
minimize into a bar in the corner, z-order, modal, following the desktop
when it resizes - the minimize, zoom and close boxes together at the right
of the top edge), scrolling text views with working scrollbars, editing
with undo, selection by keys and by dragging the mouse, a right-click
menu (Undo, Cut, Copy, Paste, Select all) and a clipboard that is the
computer's own - in the browser, in the C# client on a console and in its
window, in a Rust program's window, and on DOS under Windows or in
DOSBox-X - files of any size
(sent and read in parts), two keymaps, a help viewer, menu bar with
panels, submenus and ticked items, a status line that shows its keys and
binds them, a file-open dialog with a path/mask line, a hex viewer, a
console that colours what is written to it by its ANSI sequences, keeps a
scrollback and copies plain, buttons (docked rows and placed ones, so a
keypad is buttons too; normal, accent and danger colours), labels with
hotkeys, input lines, check boxes and radio buttons, lists with
Insert-marks, trees, lists and trees that scroll with the wheel and their
own bar, static text, a progress bar, a message box, a canvas of cells the
program draws itself (a game board, a chart); four window colours
(documents, help, dialogs, a terminal's black); four backends - the
terminal, a native Windows window, a browser canvas and DOS; the pipe
server, its C# client (published as one .exe with the core inside, on a
console or in a window) and its JavaScript client (the server as
WebAssembly); the resident, the same server behind INT 60h on DOS, with
clients in assembler, C and Pascal; comparison against reference DOS
screens.

Also there, as the classic DOS desktops had them: numbered windows and Alt+1..9,
the window list on Alt+0, Shift+F6, Ctrl+F5 to move or resize from the
keyboard, cascade and tile, double clicks, a hint per menu item on the
status line, input-line history, a colour dialog over the palette, a tree
filled as it opens (a drive as folders, with a drive chooser, in the
Commander), and find/replace in the editor.

Not yet at all: a grid, tabs, a drop-down list, a masked field; cursor
movement up and down in the console (it colours a log, it does not lay out
a full-screen ANSI drawing); the computer's clipboard in the page's DOS,
whose emulator gives a program none; a resident that stays after
its program ends, so several DOS programs can share one.

## Licence

**MIT** - see [LICENSE](LICENSE). Use it, change it, sell what you build
with it; keep the copyright notice.

Nothing here is derived from any other toolkit's source; the design is
reconstructed from published documentation and from the behaviour of the
original programs.

A few files are not this project's and not under MIT, each in a folder of
its own and listed in [THIRD-PARTY.md](THIRD-PARTY.md): js-dos - DOSBox
compiled to WebAssembly, GPL-2.0 - in `lib/js/jsdos`, which the live demo's
DOS PC runs on; `CWSDPMI.EXE`, the DPMI host the DOS programs start with,
which is Charles W. Sandmann's and comes with its own terms; and a small
MIT service worker the demo needs on a static host.
