# lib - the toolkit

Everything a program uses. Programs that use it are in
[Examples/](../Examples/README.md).

```
core/         the core. No dependencies, ever: cells, views, events, the editor,
              the menu, the dialogs. Builds for DOS as it is.
console/      a screen: the terminal (VT output, raw input), and the code pages
window/       a screen: a native Windows window, CreateWindow and GDI
serve/        the core behind a pipe, speaking PROTOCOL.md; any language is a client
csharp/       the C# client of that pipe, with the server carried inside it
js/           the JavaScript client, and the server compiled to WebAssembly for it
dos/          DOS: OWLOSRES, the toolkit behind INT 60h, its bindings for
              assembler, C and Pascal, and the machine layer a Rust program links
PROTOCOL.md   the wire: every request, its bytes, its reply
```

## Two ways to use the core

**Link it**, from Rust. A program holds a `Ui` from `core`, hands it events
and draws its `Buffer` on a screen - `console` for a terminal, `window` for a
Windows window, or a screen of its own (the browser page and DOS each have
theirs, in their examples).

**Talk to it**, from anything else. `serve` is the core in a process of its
own - or, compiled to WebAssembly, in a module of its own, or on DOS behind
a software interrupt ([dos/](dos/README.md)) - answering the requests of
[PROTOCOL.md](PROTOCOL.md). The C# and JavaScript clients are thin: they
send requests, and put the frames that come back on a console or a canvas.
Or they ask the server to open its own window (`OPEN_WINDOW`) and draw
nothing at all - which is what every DOS program does: the resident owns
the screen.

## The editor

A text view is an editor, and a program says in one call what it offers:

```
Rust        ui.set_editor(text, offer::ALL, 0)
C#          owl.Editor(text, Owlosui.Offer.All)
JavaScript  owl.editor(text, Offer.All)
C           owl_editor(text, OWL_OFFER_ALL, 0)
Pascal      OwlEditor(Text, OfferAll, 0)
assembler   mov bx,text / mov ax,OFFER_ALL / call owl_editor
```

Each bit is a whole feature: Edit (Undo, Cut, Copy, Paste, Select all),
Find, Replace, Word wrap, Read only, Hex view, Classic keys. While the
editor's window is in front, the core puts them on the menu bar - in the
program's Edit menu if it has one, or in one of their own after File -
binds their keys (Ctrl+F, Ctrl+H, Ctrl+L; Ctrl+Q F and Ctrl+Q A with
Borland's keys), builds the Find and Replace dialogs, and says on the right
of the status line where the caret is. Nothing comes back to the program;
it writes no handler for any of it. With no menu bar, Find and Replace go on
the status line instead.

Word wrap folds long lines at the window's edge, after the last space that
fits. Only the picture folds: the lines, the saved file and the caret's
line and column are what they were, and Up and Down go by rows on the
screen. The person can change the settings from the menu; `GET_EDITOR`
(`editor_state`, `GetEditor`, `editorState`) reads them back.

An editor that offers nothing - the default - leaves both bars alone, and
the primitives underneath (`FIND`, `REPLACE`, `SET_READONLY`) are still
there for a program that builds its own.

## Building

From the repository root, with [Rust](https://rustup.rs):

```
cargo build                 # core, console, window, serve and the Rust examples
cargo test                  # the core's pictures and the wire's cases
lib\js\build.cmd            # the core for JavaScript: lib/js/owlosui-wire.wasm
lib\dos\build.cmd           # the core for DOS: OWLOSRES (nightly Rust and FASM)
```

The C# client builds with `dotnet build lib\csharp\Owlosui.csproj`, and runs
`cargo build -p owlosui-serve --release` itself to carry the server.

## The window

[window/src/lib.rs](window/src/lib.rs) is the screen a desktop program
gets. The dozen Win32 functions it needs are declared by hand in
[win32.rs](window/src/win32.rs) - no crates - because a window, a font and
a rectangle are all a text screen needs. Every cell is a filled rectangle
with a character on it. The box-drawing characters are drawn as bars from
the cell's edges, so the lines meet whatever the font, and the shades as
the dot patterns a DOS card drew. Key and mouse messages become the core's
events, a timer holds a pressed button down for the ninety milliseconds it
is shown, another blinks the caret, and the size in cells follows the size
in pixels.

A Rust program implements `Program` - its `Ui`, and what to do after each
key - and calls `run`. `serve` does the same around its own `Ui` when a
client sends `OPEN_WINDOW`, which is how the C# example
`Examples/Desktop/CSharp/06-Window` gets a window without a line of drawing
code.
