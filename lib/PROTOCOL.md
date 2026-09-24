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

Text crosses as UTF-8 and is turned into code page 437 on the server side,
because the core stores glyph indices and only the backend knows what a
byte looks like. A character that has no CP437 glyph becomes `?`.

Command numbers (`cmd`) are the application's own `u16`s, as in Turbo
Vision. **`0` means "no command"** and must not be used for a button.

## Session

| op   | name   | payload            | reply |
|------|--------|--------------------|-------|
| 0x00 | QUIT   | —                  | OK; the server then exits |
| 0x01 | INIT   | `w:i16 h:i16`      | OK    |
| 0x02 | RESIZE | `w:i16 h:i16`      | OK    |

`INIT` must come first. It creates the desktop; everything else answers
with an error until it has.

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

## Changing

| op   | name     | payload   | reply |
|------|----------|-----------|-------|
| 0x20 | CLOSE    | `id`      | OK    |
| 0x21 | GET_TEXT | `id`      | `str` — a `TEXT` view's lines joined with `\n`, or an `INPUT`'s text |

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

Menu bars, the file dialog, hex view, lists, trees and clusters have no
ops. They exist in the core; the wire will grow to them one at a time, as
an example needs them.
