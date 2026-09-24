# The wire

One protocol, several transports. On Windows it runs over a pipe to
`owlosui-serve`; in a browser it will be a WebSocket; on DOS it will be a
software interrupt with the same numbers in `AH` and the same bytes at
`DS:SI`. Nothing in here depends on which.

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
byte looks like. Which glyphs is the session's **code page**: 437 unless
`INIT` or `CODEPAGE` says otherwise, 866 for Cyrillic (same box-drawing
at the same places). A character the page has no glyph for becomes `?` -
on the way in, so a client that wants to know first asks `GET_GLYPHS`
and checks. What comes back out (`GET_TEXT`, `TAKE_FILES`,
`MARKED_NAMES`) is decoded through the same page, so text that fitted
goes round unchanged.

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
| 0x1A | FILES       | `parent:id rect flags:u8 mask:str path:str entries` | `id` |

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

`TEXT.dock`: `0` fill the parent, `1` stay where the `rect` put it.
`TEXT.flags`: bit 0 read-only, bit 1 drawn as a box of its own.

`BUTTONS` are docked bottom-right, as buttons are. A button's `flags` bit 0
marks it as the default — the one Enter presses. Its label carries the
hotkey between tildes: `~O~pen`. Escape presses the last button in the
row, so put Cancel last.

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
| 0x2A | ACTIVE       | —         | `id` — the active window, or `0` |

## Events in

| op   | name  | payload | reply |
|------|-------|---------|-------|
| 0x30 | KEY   | `kind:u8 value:u16 mods:u8` | OK |
| 0x31 | MOUSE | `kind:u8 button:u8 x:i16 y:i16` | OK |
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
| 0x40 | FRAME | —       | `w:i16 h:i16 cx:i16 cy:i16 hold:u8` then `w×h ×` (`glyph:u8 attr:u8`) |
| 0x41 | TAKE  | —       | `pressed:u16 command:u16` |
| 0x42 | GET_GLYPHS | —  | `256 × u16` — the Unicode code point of each glyph index on the session's code page |

`FRAME` is the whole screen, every time. At 80×25 that is 4000 bytes, and
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

## Not yet

Menu bars, hex view, trees and clusters have no ops yet. They exist in the core; the wire will grow to them one at a time, as
an example needs them.
