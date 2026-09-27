# OWLOSUI

A Turbo Vision-shaped text mode UI toolkit with one portable core.

One core draws the same windows on a terminal, in a native Windows window,
in a browser and on DOS. Nothing above the platform layer knows which of
those it is running on, and a program in any language can use it without
linking to it.

| A native Windows window | A browser | DOS |
|---|---|---|
| ![window](Examples/screens/window.png) | ![browser](Examples/screens/browser.png) | ![dos](Examples/screens/dos.png) |

## Where to start

* **Try it** — `cargo run -p owlosui-win` opens the window above; `cargo run
  -p owlosui-demo` is the same kit on the terminal.
* **Use it from C#** — [Examples/CSharp/01-HelloWorld](Examples/CSharp/01-HelloWorld/Program.cs),
  then `dotnet run` in that folder. The client is one file,
  [lib/csharp/Owlosui.cs](lib/csharp/Owlosui.cs), and `dotnet publish -c
  Release -r win-x64` gives one `.exe` with the core inside it.
* **Walk the ladder** — [Examples/README.md](Examples/README.md): five C#
  steps from a button to a file manager, four Rust steps from the terminal to
  DOS, each program saying in its first two lines where the next one is.
* **Write a backend** — the contract is below, and the four existing ones are
  short: [the terminal](lib/console/src/term.rs),
  [the Windows window](Examples/Rust/02-Window/src/main.rs),
  [the browser page](Examples/Rust/03-Browser/web/owlosui.js),
  [DOS](Examples/Rust/04-DOS/src/main.rs).
* **Speak the wire** — [lib/PROTOCOL.md](lib/PROTOCOL.md): every request, its
  bytes and its reply. The C# client is one implementation of it.

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
lib/core        no dependencies, ever. Cell grid, view tree, event dispatch.
lib/console     terminal backend: VT output, raw input, CP437 translation;
                and owlosui-match, which checks scenes against real Turbo Vision.
lib/serve       the core behind a pipe — speaks lib/PROTOCOL.md on stdin/stdout.
lib/csharp      Owlosui.cs: the C# client of that pipe. One file, no packages.
lib/PROTOCOL.md the wire. Same numbers for a pipe, a WebSocket, an interrupt.

Examples/CSharp the C# ladder: 01-HelloWorld, 02-Notes, 03-Commander,
                04-Basez-Sokoban, 05-OwlosDemo, and Tests, which drives them
                all by the wire and through a real console.
Examples/Rust   the Rust ladder: 01-Demo (terminal), 02-Window (a Windows
                window, CreateWindow and GDI), 03-Browser (WebAssembly and a
                canvas), 04-DOS (a flat binary under DPMI); shared/app.rs is
                the one application steps 2 to 4 draw.
Examples/C      (empty) — DOS clients, through the resident, when it exists.
```

`lib/core` must stay buildable for a machine where the whole program lives in
64K. That is why it has no dependencies, why views are an `enum` rather than
generics or trait objects, why handles are numbers, and why there are no
callbacks: a callback cannot cross an interrupt, and neither can a closure
cross a pipe.

## Running it

The reference application, on the terminal:

```
cargo run -p owlosui-demo
```

Drag windows by the title bar, resize from the bottom-right corner, click the
`[■]` to close and `[↑]` to zoom. `F1` help, `F3` open a file, `F4` the
dialog with every control in it, `F5` zoom, `F6` next window, `Alt-F3` close,
`Alt-X` quit — which asks first if something has been changed.

One frame as plain text, with no terminal involved:

```
cargo run -p owlosui-demo -- --dump [help|menu|files|controls|modal]
```

Because the output medium *is* a grid of characters, a rendered screen is its
own screenshot. That makes the tests readable pictures (`lib/core/tests/`),
and it is what lets a model check its own layout work later without vision.

## From C#

```
cd Examples/CSharp/01-HelloWorld
dotnet run
```

The build of the client runs `cargo build -p owlosui-serve --release`
itself and carries the result inside `Owlosui.dll`, so the .NET SDK and
Rust installed is all it takes. `OWLOSUI.sln` at the root holds the client
library, the five examples and their tests, for Visual Studio or `dotnet
build OWLOSUI.sln`. While an example runs, its server binary is in use and
cannot be rebuilt: close the example first.

To give a program away:

```
dotnet publish -c Release -r win-x64
```

leaves one `.exe` in `bin\Release\net8.0\win-x64\publish\` - the program,
the client and the core. The client looks for the core beside the program,
then in the repository's `target\` (whichever of debug and release was built
last), and only then writes the one it carries to `%TEMP%\owlosui\` and runs
that; so a developer always gets the core just built, and a person with
only the `.exe` gets the one inside it.

The whole of HelloWorld:

```csharp
using var owl = new Owlosui();

var w = owl.Window("Hello", 40, 9, style: Owlosui.Style.Dialog);
owl.Static(w, 2, 1, "Hello, world!");
owl.Buttons(w, ("~O~K", CmOk));

owl.Run(cmd => cmd != CmOk);
```

`Owlosui` starts `owlosui-serve`, sends it what happened, asks for the frame
and puts the cells on `System.Console`. The Rust core decides what every cell
looks like; the C# program never sees a keystroke of the editor and never
draws a line of a frame. `02-Notes` is an editor with undo, a clipboard and a
"leave without saving?" box, and it is sixty lines.

On Windows the mouse works too — the client reads it through
`ReadConsoleInput`, since `System.Console` does not know a mouse exists —
and the desktop follows the console window when it is resized.

The same protocol, over a different transport, is how DOS programs will reach
a resident copy of the toolkit and how a browser page will reach a
WebAssembly one. The C# client exists first because it is the fastest way to
find out whether the API is pleasant to use.

### The examples, tested

```
cd Examples/CSharp/Tests
dotnet run
```

Each case builds the scene an example builds — by calling the example's own
code — then presses its buttons, drags its corners and types into it by the
wire, headless, and looks at the frame. No test framework, no packages: a
program that prints a line per case and exits 1 if one failed.

The same cases exist on the Rust side, in `lib/serve/tests/protocol.rs`,
against the server alone. When something the mouse should do does not
happen, the two together say which side of the wire to look at.

The cases follow the list a Turbo Vision manual gives for a window — move,
resize, zoom, close, next — and for an editor — type, Enter, scroll, click —
plus the things a console does to a program: change size under it, down to
one cell and back. Each one is done, not read about. A person resizing the
console window found the desktop losing every window; that is a case now.

The last cases are not headless. `ConsoleAgent` (`lib/csharp`) starts
Notes in a hidden console of its own, attaches to it, puts mouse and key
records straight into its input buffer — the same `INPUT_RECORD`s a person's
mouse produces — and reads the screen back. That is the layer between the
wire and the console, the one that broke first, tested without anybody at the
keyboard. The agent is not specific to OWLOSUI; it will drive any program that
reads a Windows console.

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

## In a browser

```
rustup target add wasm32-unknown-unknown
Examples\Rust\03-Browser\build.cmd
python -m http.server 8765 --directory Examples\Rust\03-Browser\web
```

and open `http://localhost:8765`. The module is the core and a small
application ([Examples/Rust/shared/app.rs](Examples/Rust/shared/app.rs): an
editor window, a dialog with every control, help, the window list) compiled
for `wasm32-unknown-unknown` with no wasm-bindgen and no toolchain beyond
`cargo`: a handful of `extern "C"` functions and a frame read out of the
module's memory. The page ([owlosui.js](Examples/Rust/03-Browser/web/owlosui.js),
two hundred lines) draws three bytes a cell on a canvas - the glyph through
a code page 437 table, the attribute through the sixteen CGA colours, the
box lines and shades drawn as rectangles and dots so no font can break them
- and turns `keydown`, mouse and wheel events into the same numbers the wire
uses. It knows nothing about windows.

## In a native Windows window

```
cargo run -p owlosui-win
```

[Examples/Rust/02-Window](Examples/Rust/02-Window/src/main.rs): `CreateWindow`,
a message loop and GDI, nothing else - the dozen Win32 functions it needs
are declared by hand in [win32.rs](Examples/Rust/02-Window/src/win32.rs), no
crates. The core and the same application the browser and DOS run are
linked in, and `main.rs` is the screen: every cell is a filled rectangle
with a character on it; the box-drawing characters are drawn as bars from
the cell's edges, so the lines meet whatever the font, and the shades as
the dot patterns a DOS card drew. Key and mouse messages become the core's
events, a timer holds a pressed button the ninety milliseconds it is shown
down, another blinks the caret, and the window's size in cells follows its
size in pixels.

## On DOS

```
rustup toolchain install nightly --component rust-src
Examples\Rust\04-DOS\build.cmd
Examples\Rust\04-DOS\RUNWIN.BAT
```

The last line starts DOSBox-X (set `DOSBOXX` to yours); on a real machine
it is `RUN` in that folder. The program is `OWLOS.BIN`, a flat 32-bit
binary of the core and the same application the browser runs, built
with nightly's `build-std` for a target of our own (`i386-owlos-none`):
no `std`, `core` and `alloc` on a small allocator of size classes, three
pages of glue. [OWLOS.COM](Examples/Rust/04-DOS/OWLOS.ASM) (FASM) is the
loader: it asks DPMI for four megabytes, reads the file into them, makes
two selectors whose base is the block and jumps in. CWSDPMI is beside it as
the host. Drawing is one copy - a cell is the two bytes the card wants -
keys come straight out of the BIOS keyboard buffer, and the mouse is asked
through INT 33h between keys and drawn into the frame as the red square DOS
programs drew, double clicks told apart by the BIOS tick.

`test.cmd` builds, runs one frame under DOSBox-X and checks that the frame
the program wrote through DOS (`SHOT.BIN`) is the screen the emulator shows
(`SCREEN.BIN`, dumped from B800 by `SCRDUMP.COM`): the file is the screen.
Then it lets DOSBox-X type at the program - F4 for the dialog, Escape,
F12 (the DOS backend's own key: the frame into `SHOT.BIN`), F10 and the
menu's Exit - and checks it came back with code 0. Last, `/STRESS` has the
program drag, zoom and open dialogs by itself for a while, and the heap has
to hold. A panic writes its file and line on the screen's top row and into
`TRACE.TXT`; `LOADER.LOG` is the loader's stage marks.

In DOSBox-X's TrueType output mode (`output=ttf`) the double frames come out
single: that mode draws the text screen with a Windows font, and the default
one draws `╔═╗` with single lines. `font = consola` in the `[ttf]` section,
or `output=surface`, and they are as they are in video memory.

## Checking against the real thing

Several scenes exist twice: here, and in `TOOLS/REFGEN/REFGEN.PAS` in the
wire-city repository, where they are built with the actual Borland Turbo
Vision units and dumped straight out of video memory. Then:

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
  application. Borland shipped four keymaps for one editor this way.
* **No callbacks out of the core.** Results are collected — `take_pressed`,
  `take_command`, `pending` — never delivered. That is what lets the same
  core sit behind a pipe, an interrupt or a WebAssembly boundary unchanged.

## Editing

The editor holds no key codes. It knows commands with names — `WordRight`,
`DeleteLine` — and a table turns keys into them. That is how Borland shipped
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
z-order, modal, following the desktop when it resizes), scrolling text
views with working scrollbars, editing with undo, selection and a
clipboard, two keymaps, a help viewer, menu bar with panels, submenus and
ticked items, a status line that shows its keys and binds them, a file-open
dialog with a path/mask line, a hex viewer, buttons (docked rows and
placed ones, so a keypad is buttons too; normal, accent and danger
colours), labels with hotkeys,
input lines, check boxes and radio buttons, lists with Insert-marks, trees,
static text, a progress bar, a message box, a canvas of cells the program
draws itself (a game board, a chart); four backends - the terminal, a
native Windows window, a browser canvas and DOS; the pipe server and its C#
client, published as one .exe with the core inside; comparison against
real Turbo Vision.

Also there, as Turbo Vision had them: numbered windows and Alt+1..9,
the window list on Alt+0, Shift+F6, Ctrl+F5 to move or resize from the
keyboard, cascade and tile, double clicks, a hint per menu item on the
status line, input-line history, a colour dialog over the palette, a tree
filled as it opens (a drive as folders, with a drive chooser, in the
Commander), and find/replace in the editor.

Not yet at all: a grid, tabs, a drop-down list, a masked field, an ANSI
viewer; a JavaScript client of the wire; the resident.

## Licence

MIT OR Apache-2.0. Nothing here is derived from Borland's or Free Vision's
source; the design is reconstructed from published documentation and from the
behaviour of the original programs.
