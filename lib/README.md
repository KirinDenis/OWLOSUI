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
the WordStar keys), builds the Find and Replace dialogs, and says on the right
of the status line where the caret is. Nothing comes back to the program;
it writes no handler for any of it. With no menu bar, Find and Replace go on
the status line instead.

Word wrap folds long lines at the window's edge, after the last space that
fits. Only the picture folds: the lines, the saved file and the caret's
line and column are what they were, and Up and Down go by rows on the
screen. The person can change the settings from the menu; `GET_EDITOR`
(`editor_state`, `GetEditor`, `editorState`) reads them back.

**Line numbers and the position.** Two more settings beside Word wrap:
line numbers in a dark grey column on the left, for code - an assembler
says by number which line is wrong - and the caret's ` 12:5 ` on the
window's bottom edge, where the classic IDEs had it. Both are state bits
(`state::NUMBERS`, `POSITION`; `numbers`, `position` in JavaScript and
C#; `OWL_EDIT_NUMBERS`, `EditNumbers`, `EDIT_NUMBERS` and the same for
the position), and both are on the Edit menu, ticked. The web demo and
the DOS commander show the position always, and the numbers when the
file is code: when its name named a language.

**Syntax colours.** `set_syntax(text, "DEMO.PAS")` (`SYNTAX` on the wire;
`owl.syntax`, `Syntax`, `owl_syntax`, `OwlSyntax`) colours a text as the
language its name, extension or file name says; Edit > Syntax lets the
person choose. The core knows C, C#, JavaScript, JSON, Rust, Pascal,
Assembler, Batch and INI. Each is a few lines of
[core/src/syntax.ini](core/src/syntax.ini) - comments, strings, keywords,
no regular expressions, so it runs on DOS too - and that file says what
every line of the format means. A program adds or replaces a language with
`add_syntax` (`SYNTAX_DEFINE`) in the same format. The colours are palette
roles (Syntax keyword, type, comment, string, number, directive), so the
Colors dialog changes them. Typing on a line recolours from that line down,
and only as far as the window shows.

An editor that offers nothing - the default - leaves both bars alone, and
the primitives underneath (`FIND`, `REPLACE`, `SET_READONLY`) are still
there for a program that builds its own.

## The mouse and the clipboard

What the core does by itself, in every client, with nothing to write:
dragging with the left button selects in a text or a console, and scrolls
when the pointer goes past the edge; a right click over a text opens Undo,
Cut, Copy, Paste and Select all at the pointer (what cannot be done greyed
out), over a console Copy and Select all; the wheel scrolls whatever is
under the pointer - a text, a list or a tree in a dialog, a console - and
moves a file panel's cursor; a list's or a tree's own bar answers its
arrows, its track and its marker. The wheel and the bar move the view and
leave the cursor on its item, until a key or a click moves it.

The clipboard is the core's, one for every view. A client that has a
clipboard of its own shares it with `CLIPBOARD` (0x69-0x6B): what the core
copies goes out, and a Paste waits for the computer's clipboard to come in.
The page's `run()` does it with the browser's clipboard, and the C#
client's `Run` on a console with Windows' (`ShareClipboard`); a program
with its own loop calls `clipboard` / `ClipboardState` after each input.
The native window (`window/`) shares Windows' clipboard by itself, so a
C# program in a window and a Rust program in one have it with nothing to
call. So does DOS, where it runs under something that has a clipboard:
OWLOSRES and the Rust machine layer ask WinOldAp's INT 2Fh (AX 17xxh),
which a DOS box under Windows answers, and DOSBox-X with
`dos clipboard api=true` - every `RUNWIN.BAT` here sets it. The text goes
in the card's code page. A plain DOS, and the page's DOS, keep the core's.

## Windows

The top edge carries minimize `[↓]`, zoom `[↑]` and close `[■]` together at
the right, the close box in the corner. Minimize puts a window into a bar in
the bottom right corner, the bars stacking upwards; a click on a bar, or
activating the window, brings it back. Every window has a minimize box but a
modal one and one made without it:

```
Rust        window.minimizable = false
C#          owl.Window(..., minimize: false)
JavaScript  owl.window(..., { minimize: false })
C           flags | OWL_NO_MINIMIZE
Pascal      Flags or OwlNoMinimize
assembler   WF_NO_MINIMIZE
```

`MINIMIZE` (`minimize`, `Minimize`, `owl_minimize`, `OwlMinimize`) does it
from a program. A window's colours are one of four families: blue for
documents, cyan for help, grey for dialogs, and black for a terminal - a
console's window, its frame the colour of what is in it (`Style.Terminal`,
`OWL_BLACK`, `OwlBlack`, `WF_BLACK`). `SET_TAG` puts a word after the title
as `[modal]` is, `SET_INDICATOR` words at the right of the status line,
`PLACE` says where a window's inside is and whether anything covers it.

## The console

A view for text that keeps arriving - a log: `CONSOLE` makes one filling
its window, `CONSOLE_WRITE` adds text at its end, and the ANSI sequences in
the text colour it (SGR, the 256 and 24-bit colours folded onto the
sixteen, carriage return, tab, erase-line, clear). It keeps a scrollback,
folds long lines, follows the newest one until scrolled back, and gives its
record back plain through `GET_TEXT`. In the clients: `console` /
`consoleWrite`, `ConsoleView` / `ConsoleWrite`, `owl_console` /
`owl_console_write`, `OwlConsole` / `OwlConsoleWrite`. The web demo's
Tools > Console is one.

## Big texts

A request and a reply carry 65535 bytes each. A text bigger than that goes
in parts - `TEXT` or `SET_TEXT` with the first, `TEXT_APPEND` with the rest
- and comes back in parts with `GET_TEXT_PART`; the JavaScript and C#
clients do both by themselves, so `text`, `setText` and `getText` take a
file of any size. A reply that would be longer than a reply can be is an
error, never a length that wrapped round.

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
