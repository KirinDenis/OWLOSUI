# The wire

One protocol, several transports. On Windows it runs over a pipe to
`owlosui-serve`; in a browser the same server is a WebAssembly module and
a request is copied into its memory; on DOS it is INT 60h, answered by
OWLOSRES (`lib/dos`), with the request at `DS:SI` and room for the reply
at `ES:DI`. The bytes are the same in all three, and nothing in here
depends on which.

The client never renders on its own and the server never draws: the client
sends what happened, asks for the frame, and puts the cells on whatever it
has — a `System.Console`, a canvas, or video memory at `B800:0000`.

This is version 0. Numbers are not yet frozen; the *shape* is.

## Framing

Everything is little-endian. A request is

    op:u8  len:u16  payload[len]

and every request gets exactly one reply:

    status:u8  len:u16  payload[len]

`status` is `0` for OK, `1` for an error, in which case the payload is a
`str` saying what went wrong. An error never ends the session: the server is
still there, and the client may carry on.

Types used below:

| name   | bytes                                     |
|--------|-------------------------------------------|
| `str`  | `len:u16` then that many bytes of UTF-8   |
| `rect` | `x:i16 y:i16 w:i16 h:i16`                 |
| `id`   | `u16` — a view handle; `0` is the desktop |

Text crosses as UTF-8 and is turned into glyph indices on the server side,
because the core stores glyph indices and only the backend knows what a
glyph looks like. Which glyphs is the session's **font**. It starts as a
code page - 437 unless `INIT` or `CODEPAGE` says otherwise, 866 for
Cyrillic (same box-drawing at the same places) - and **grows**: a
character it has never seen gets the next free index, up to 65535. So a
Windows or browser client shows Russian and Slovak names on one screen,
while a DOS build (whose glyph is a byte) shows the 256 it has and `?` for
the rest. `GET_GLYPHS` gives the table, and every `FRAME` says how long
the font is now, so a client fetches the table again only when a frame
refers past what it holds. What comes back out (`GET_TEXT`, `TAKE_FILES`,
`MARKED_NAMES`) is decoded through the same font, so text goes round
unchanged.

Command numbers (`cmd`) are the application's own `u16`s, as in Turbo
Vision. **`0` means "no command"** and must not be used for a button.

## Session

| op   | name   | payload            | reply |
|------|--------|--------------------|-------|
| 0x00 | QUIT   | —                  | OK; the server then exits |
| 0x01 | INIT   | `w:i16 h:i16`      | OK    |
| 0x02 | RESIZE | `w:i16 h:i16`      | OK    |
| 0x03 | CODEPAGE | `n:u16`          | OK — 437 or 866 |

`INIT` must come first. It creates the desktop; everything else answers
with an error until it has. It may carry a trailing `codepage:u16`;
absent or `0` means 437.

`RESIZE` is what a console window being dragged bigger or smaller becomes.
Windows keep their top-left corner and follow the desktop's far edges - an
editor that filled the screen still fills it - down to their minimum size
and up to their maximum; a fixed-size window that was centred is centred
again. Any size is legal, including `1x1`, and the frame that comes back is
that size.

## Building

| op   | name        | payload | reply |
|------|-------------|---------|-------|
| 0x10 | WINDOW      | `parent:id rect flags:u8 title:str [close_cmd:u16]` | `id` |
| 0x11 | TEXT        | `parent:id rect dock:u8 flags:u8 text:str` | `id` |
| 0x12 | STATIC      | `parent:id rect text:str` | `id` |
| 0x13 | INPUT       | `parent:id rect max:u16 label:str text:str` | `id` |
| 0x14 | BUTTONS     | `parent:id n:u8` then `n ×` (`cmd:u16 flags:u8 label:str`) | `id` |
| 0x15 | MESSAGE_BOX | `title:str text:str n:u8` then `n ×` (`cmd:u16 flags:u8 label:str`) | `id` |
| 0x16 | STATUS      | `n:u8` then `n ×` (`cmd:u16 key:key text:str`) | `id` |
| 0x17 | LABEL       | `parent:id rect target:id text:str` | `id` |
| 0x18 | PROGRESS    | `parent:id rect max:u32 flags:u8` | `id` |
| 0x19 | LIST        | `parent:id rect flags:u8 n:u16` then `n × str` | `id` |
| 0x1A | FILES       | `parent:id rect flags:u8 mask:str path:str entries` | `id` | The panel fills its window; `rect.y` is the number of rows left free above it.
| 0x1B | CANVAS      | `parent:id rect` | `id` |
| 0x1C | MENU_BAR    | `items` | `id` | — each item is `flags:u8 cmd:u16 text:str shortcut:str hint:str` then its sub-items; the hint shows on the status line while the item is under the cursor
| 0x1D | CLUSTER     | `parent:id rect kind:u8 n:u8` then `n × str` | `id` |
| 0x57 | FIND        | `id flags:u8 pattern:str` | `found:u8` — the next match after the caret is selected; flags bit 0 case-sensitive, bit 1 whole words; no wrapping |
| 0x58 | REPLACE     | `id flags:u8 pattern:str replacement:str` | `replaced:u8 found:u8` — the selected match replaced, and the next one found |
| 0x59 | REPLACE_ALL | `id flags:u8 pattern:str replacement:str` | `count:u16` |
| 0x53 | TREE        | `parent:id rect nodes` | `id` — a tree filling its window; `nodes` is `n:u8` then `n × (flags:u8 text:str nodes)`, flags bit 0 open, bit 1 lazy (children exist but are asked for when it is opened) |
| 0x54 | TREE_CHILDREN | `id depth:u8 depth×index:u16 nodes` | OK — the children of the node at that path, usually in answer to TREE_EXPAND; the node opens |
| 0x55 | TREE_EXPAND | `id` | `depth:u8` then `depth × (index:u16 text:str)` — the lazy node somebody opened, once, or depth `0` |
| 0x56 | TREE_PATH   | `id` | `n:u8` then `n × str` — the texts from the root down to the current row |
| 0x1F | HEX         | `parent:id rect bytes…` | `id` — a hex dump of the bytes (the rest of the payload), filling its window |
| 0x1E | BUTTON_ROW  | `parent:id rect flags:u8 n:u8` then `n ×` (`cmd:u16 flags:u8 label:str`) | `id` — a row placed by hand, from the left; `rect.w` of `0` means as wide as its buttons; row `flags` bit 0 takes it out of the Tab ring |

A `rect` with `x` or `y` of `-1` means *centre it* - now, and again after
every `RESIZE`, until it is dragged somewhere. The defaults are already
right: a window asked for with `(-1, -1, 40, 10)` lands in the middle of the
work area, and that is where most windows want to be.

`close_cmd`, if present and not zero, is what the close box `[■]` sends
instead of closing the window. Turbo Vision's sent `cmClose` and let the
program refuse; this is the same bargain, and it is how an editor gets to
ask "save changes?" first.

`WINDOW.flags`:

| bit | meaning |
|-----|---------|
| 0   | modal — nothing behind it answers until it is closed |
| 1   | not resizable |
| 2   | not zoomable |
| 3   | not closable |
| 4–5 | palette: `0` blue (documents), `1` cyan (help), `2` grey (dialogs) |
| 6   | no shadow — for windows that tile the screen rather than float on it |

`TEXT.dock`: `0` fill the parent, `1` stay where the `rect` put it.
`TEXT.flags`: bit 0 read-only, bit 1 drawn as a box of its own.

Windows on the desktop are numbered 1 to 9 as they open, the number
shown in the frame, and Alt+that number brings one to the front; Alt+0
lists them, Shift+F6 is the previous window and Ctrl+F5 moves or resizes
the active one from the keyboard (arrows move, Shift+arrows resize, Enter
keeps, Escape puts back). The core binds those four itself. Commands from
`0xFF00` up belong to the toolkit's own dialogs and never reach `TAKE`.

`BUTTONS` are docked bottom-right, as buttons are. A button's `flags` bit 0
marks it as the default — the one Enter presses; bits 1–2 are its style,
`0` normal (green), `1` accent (cyan, for an operator or a mode), `2`
danger (red, for what throws something away); bit 3 disables it; bit 4
marks it as the one Escape presses, wherever its row is. Its label
carries the hotkey between tildes: `~O~pen`. Escape presses the last button
in the docked row, so put Cancel last. `BUTTON_ROW` is the same row placed
anywhere - several of them make a keypad - and `SET_BUTTON` turns one
button on or off. A mouse click presses a button without moving the focus,
as in the original: a keypad clicked leaves the caret in the display.

`MESSAGE_BOX` is a grey modal window sized to its words, built from the
parts above. It is here rather than in every client because every client
needs it and none of them should spell it differently.

`STATUS` is the bottom row of the screen, one per desktop; a second
replaces the first. Each item is a command, a `key` (`kind:u8 value:u16
mods:u8` as in `KEY`, or `kind = 0xFF` for none) and a label with the
key's name between tildes: `~F1~ Help`. The status line both shows the
keys and binds them: press F1 and `TAKE` reports the command, as it does
for a click on the words. Not while a modal window is up.

`LABEL` is words with a hotkey beside a control: `~N~ame:` next to an
input. Alt+N, or a click on the words, puts the focus on `target`. A
`target` of `0` is words with nothing to point at.

`PROGRESS` is a bar of `█` and `░`, `value` of `max`; `flags` bit 0 adds
the percentage at the right. `SET_PROGRESS` moves it.

`LIST` is a scrolling list of strings; `flags` bit 0 makes it
multiple-choice, where Insert marks the item under the cursor and moves
down, the way Norton Commander marked files. `GET_MARKED` and
`GET_CURRENT` read it back.

`CANVAS` is a rectangle of cells the program draws itself - Turbo
Vision's "a view with its own `draw`", over a wire. `BLIT` puts a block
of cells into it: `id x:i16 y:i16 w:i16 h:i16` then `w×h ×` (`ch:u16
attr:u8`), `ch` a Unicode code point that the font turns into a glyph. The
cells are shown as they are: clipped, moved with the window, covered by
what floats above, never wrapped. A game board, a chart, a piece of ANSI
art. A cell of `ch = 0, attr = 0xFF` is *clear*: not drawn, so the window
shows through - and a new canvas is all clear.

`MENU_BAR` is the bar across the top, one per desktop; a second replaces
the first. `items` is `n:u8` then `n ×` (`flags:u8 cmd:u16 text:str
shortcut:str` then that item's own `items`, the same shape, empty for a
plain command) - a menu as deep as it goes. `flags`: bit 0 a separator
line, bit 1 disabled, bit 2 ticked. A chosen item is reported by `TAKE`
as its `command`; F10 opens the bar and Alt with a letter opens the menu
whose letter it is, as in the original. `MENU_CHECK` sets or clears the
tick on the item that sends a command.

`CLUSTER` is check boxes (`kind = 0`, any number on) or radio buttons
(`kind = 1`, exactly one on, the first to begin with); `GET_CLUSTER`
reads them back.

`FILES` is the file panel: a path-and-mask line, the names in columns, a
pane of details, filling its window. The server reads no directories: the
client sends `entries` - `n:u16` then `n ×` (`name:str size:u32 year:u16
month:u8 day:u8 hour:u8 minute:u8 attrs:u8`), with `attrs` the DOS bits
(`0x10` directory, `0x01` read-only, `0x02` hidden, `0x04` system, `0x20`
archive) - and sends them again with `SET_FILES` when the person has gone
somewhere else, which `TAKE_FILES` reports. `flags` bit 0 allows marks;
bit 1 leaves the `Path:` word off the path line, for a panel that is
nothing but paths; bit 2 leaves the path line out altogether, for a
commander's panel, where the names start at the top and the foot says
where you are.

## Changing

| op   | name     | payload   | reply |
|------|----------|-----------|-------|
| 0x20 | CLOSE    | `id`      | OK    |
| 0x21 | GET_TEXT | `id`      | `str` — a `TEXT` view's lines joined with `\n`, or an `INPUT`'s text |
| 0x22 | SET_PROGRESS | `id value:u32` | OK |
| 0x23 | GET_MARKED   | `id`      | `n:u16` then `n × u16` — the marked items of a `LIST` |
| 0x24 | GET_CURRENT  | `id`      | `u16` — the item under the cursor of a `LIST` |
| 0x25 | SET_FILES    | `id path:str mask:str entries` | OK — a new listing for a `FILES` panel |
| 0x26 | TAKE_FILES   | `id`      | `kind:u8 text:str` — `1` a name Enter was pressed on, `2` a path typed and entered, `0` nothing; read once |
| 0x27 | ACTIVATE     | `id`      | OK — bring a window to the front |
| 0x28 | MARKED_NAMES | `id`      | `n:u16` then `n × str` — a `FILES` panel's marked names, or the one under the cursor if none are |
| 0x29 | SET_FILES_ERROR | `id text:str` | OK — show a message in the panel's pane, e.g. a folder that could not be read |
| 0x5A | ADD_FILES | `id` then entries as in FILES | OK — more names for a panel: a listing bigger than one request (64K) is sent as FILES or SET_FILES with the first part and ADD_FILES with the rest |
| 0x2A | ACTIVE       | —         | `id` — the active window, or `0` |
| 0x2B | SET_TEXT     | `id text:str` | OK — new words for a `STATIC` (keeps its place and width), an `INPUT` (keeps its label) or a `TEXT` (starts over) |
| 0x2C | BLIT         | `id x:i16 y:i16 w:i16 h:i16` then `w×h ×` (`ch:u16 attr:u8`) | OK — cells into a `CANVAS` |
| 0x2D | MENU_CHECK   | `cmd:u16 on:u8` | OK — tick or untick the menu item that sends `cmd` |
| 0x2E | GET_CLUSTER  | `id`      | `n:u8` then `n × u8` (on or off) then `current:u8` |
| 0x2F | GET_CLICK    | `id`      | `has:u8 x:i16 y:i16` — where the mouse last went down on a `CANVAS`, in its cells; read once |

## Events in

| op   | name  | payload | reply |
|------|-------|---------|-------|
| 0x30 | KEY   | `kind:u8 value:u16 mods:u8` | OK |
| 0x31 | MOUSE | `kind:u8 button:u8 x:i16 y:i16` | OK — kind `0` down, `1` up, `2` drag, `3` move, `4` wheel up, `5` wheel down, `6` the second press of a double click (sent in place of its down; the client decides what "double" is) |
| 0x32 | TICK  | —       | OK |

`KEY.kind`: `0` a character (`value` is its Unicode scalar), `1` a function
key (`value` is its number), `2` a named key:

    0 Enter  1 Esc  2 Tab  3 BackTab  4 Backspace  5 Delete  6 Insert
    7 Home   8 End  9 PageUp  10 PageDown  11 Up  12 Down  13 Left  14 Right

`KEY.mods`: bit 0 shift, bit 1 ctrl, bit 2 alt.

`MOUSE.kind`: `0` down, `1` up, `2` drag, `3` move, `4` wheel up, `5` wheel
down. `MOUSE.button`: `0` left, `1` right, `2` middle.

`TICK` is sent when a `FRAME` asked for it (see `hold` below): the moment
has passed, deliver what was being shown. The core has no clock; the wait
is the client's, and about 90 ms is long enough to be seen.

## Results out

| op   | name  | payload | reply |
|------|-------|---------|-------|
| 0x40 | FRAME | —       | `w:i16 h:i16 cx:i16 cy:i16 hold:u8 glyphs:u16` then `w×h ×` (`glyph:u16 attr:u8`) |
| 0x41 | TAKE  | —       | `pressed:u16 command:u16` |
| 0x42 | GET_GLYPHS | —  | `growing:u8 n:u16` then `n × u16` — the Unicode code point of each glyph index in the session's font |
| 0x43 | CYCLE | —       | OK — the front window goes to the back (Turbo Vision's F6) |
| 0x44 | ZOOM  | `id`    | OK — a window fills the work area, or goes back to its size (F5) |
| 0x45 | SET_BUTTON | `id index:u8 enabled:u8` | OK — enable or disable one button of a row |
| 0x46 | FOCUS | `id`    | OK — put the focus on that control, in its window |
| 0x47 | CASCADE | —     | OK — the windows along the diagonal, every title showing; a fixed-size window only moves |
| 0x51 | PALETTE | — | `n:u8` then `n × (group:str name:str attr:u8)` — every colour of the palette, by role, in a fixed order |
| 0x52 | SET_COLOR | `index:u8 attr:u8` | OK — one colour changed; the next frame wears it |
| 0x4F | SET_HISTORY | `id n:u8` then `n × str` | OK — what an input line has been given before, newest first; Down, or the `▼` at its end, lists them and a pick fills the field. Enter in the field adds to it |
| 0x50 | GET_HISTORY | `id` | `n:u8` then `n × str` — the history, newest first, to keep for next time |
| 0x4C | WINDOW_LIST | — | OK — the list of windows, a modal dialog; Enter brings the chosen one to the front (Alt+0) |
| 0x4D | CYCLE_BACK | — | OK — the window at the back comes to the front (Shift+F6) |
| 0x4E | SIZE_MOVE | — | OK — the active window is moved or resized from the keyboard until Enter or Escape (Ctrl+F5) |
| 0x4A | WINDOW_STATUS | `id` then the items of STATUS | OK — the keys the window carries: on the status line, and bound, only while it is the active window |
| 0x4B | WINDOW_MENU | `id` then the items of MENU_BAR | OK — the window's menus, merged into the bar while it is active: a submenu named like one on the bar goes into it after a line, any other goes on the end |
| 0x49 | SET_READONLY | `id on:u8` | OK — a text view becomes a viewer (`[view]` in its title) or an editor again |
| 0x48 | TILE  | —       | OK — the windows share the work area with no overlap; a fixed-size one stands in its cell at its own size |

`FRAME` is the whole screen, every time. At 80×25 that is 6000 bytes, and
a client that wants to redraw only what changed keeps the previous frame
and compares — the server does not know what the client has on its screen.
`cx, cy` is where the caret should be, or `-1, -1` for none; the client
shows the hardware cursor there, as Turbo Vision did.

`hold` is `1` when this frame shows something chosen that has not yet
happened: a button a key just pressed, drawn down; a menu item just
clicked, drawn lit. Show the frame, wait about 90 ms, send `TICK`, then
`TAKE`. A press that fires before it is seen is a press nobody believes —
Alt+S saved the file and nothing on the screen said so.

A cell's `attr` is the IBM byte: low nibble foreground, high nibble
background, sixteen colours in the order Black, Blue, Green, Cyan, Red,
Magenta, Brown, LightGray, DarkGray, LightBlue, LightGreen, LightCyan,
LightRed, LightMagenta, Yellow, White. On DOS this byte goes to the video
card unchanged; `System.ConsoleColor` happens to use the same order.

`TAKE` returns what the last event caused: a button that was pressed, a
menu command that was chosen, or `0` for neither. Ask after every event.
There are no callbacks — a callback cannot cross an interrupt.

## A window of its own

| op   | name        | payload | reply |
|------|-------------|---------|-------|
| 0x5B | OPEN_WINDOW | `title:str [flags:u8]` | OK — then the desktop is shown in a native window, which reads the keys and the mouse itself. `flags` bit 0: open without taking the focus (for a test or an agent). After INIT; Windows only |
| 0x5C | WAIT        | —       | `pressed:u16 command:u16 w:i16 h:i16` — after one key or click in the window has been through the core: what it caused, as TAKE, and the desktop's size now |

Only the stdio server answers these; a host with no window of its own - the
WebAssembly module, say - replies with an error. On DOS, OWLOSRES answers
WAIT from the start and refuses OPEN_WINDOW: the screen is always its own,
so a DOS program never asks for it. After OPEN_WINDOW every
other request works as before and the client draws nothing: `FRAME` still
answers, for a test that wants to read the screen. WAIT takes the place of
reading the console. Input that arrives while the client is busy is queued
and handed over one event per WAIT, as a console's input buffer would, and
a pressed button is shown down for its moment before the WAIT that reports
it returns. The window's close box arrives as Alt+X.
