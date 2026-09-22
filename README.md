# OWLOSUI

A Turbo Vision-shaped text mode UI toolkit with one portable core.

The same core is meant to drive a terminal, a browser canvas, a native window
and — eventually — DOS text memory. Nothing above the platform layer knows
which of those it is running on.

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
core/      no dependencies, ever. Cell grid, view tree, event dispatch.
console/   terminal backend: VT output, raw input, CP437 translation.
```

`core` must stay buildable for a machine where the whole program lives in 64K.
That is why it has no dependencies, why views are an `enum` rather than
generics or trait objects, and why `no_std` is the direction of travel.

## Running it

```
cargo run -p owlosui-console
```

Drag windows by the title bar, resize from the bottom-right corner, click the
`[■]` to close and `[↑]` to zoom. `F5` zoom, `F6` next window, `Alt-F3` close,
`Alt-X` quit.

One frame as plain text, with no terminal involved:

```
cargo run -p owlosui-console -- --dump
```

Because the output medium *is* a grid of characters, a rendered screen is its
own screenshot. That makes the tests readable pictures (`core/tests/`), and it
is what lets a model check its own layout work later without vision.

## Checking against the real thing

Several scenes exist twice: here, and in `TOOLS/REFGEN/REFGEN.PAS` in the
wire-city repository, where they are built with the actual Borland Turbo
Vision units and dumped straight out of video memory. Then:

```
cargo run -p owlosui-console -- --match one-window path/to/REF01.BIN --rows 1:23
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

Those live in `core/tests/scripts`, and a failure prints the script line, what
it got and what it wanted.

## Help

A small HTML viewer: links, headings, `<b>`, paragraphs, rules, lists,
`<pre>`. No tables, no images, no CSS — and no `<i>`, because a character cell
cannot be slanted and whatever DOS cannot do does not exist anywhere.

The view cannot open a file and does not try. Following a link puts the href
in `pending`; resolving it belongs to the application.

## Status

Working: desktop, overlapping framed windows (move, resize, zoom, close,
z-order), scrolling text views with working scrollbars, editing with undo,
selection and a clipboard, two keymaps, a help viewer, CP437 translation,
comparison against real Turbo Vision.

There is a measure pass now, with two rules: a window's content fills its
client area, and a help page is then wrapped to the width it ended up with.
Layout belongs to the toolkit — an application asked to remember it will
forget, and the symptom is content left at its old size with the window's
background showing round it.

Not yet: sizes that come from the content rather than from the window (a
dialog cannot yet work out how big it should be), menus, a status line as a
real view, saving to disk, the rest of the control set, any backend other than
the terminal.

## Licence

MIT OR Apache-2.0. Nothing here is derived from Borland's or Free Vision's
source; the design is reconstructed from published documentation and from the
behaviour of the original programs.
