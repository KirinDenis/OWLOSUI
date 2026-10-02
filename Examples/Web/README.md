# Web

Programs in a browser page.

**Quickest of all:** the [live demo](https://kirindenis.github.io/OWLOSUI/) -
this folder's demo, published by [site/build.mjs](site/build.mjs) on every
push. **On your own machine:** double-click `Web_Demo.cmd` in the repository's root. It
needs [Rust](https://rustup.rs) and, once, Rust's WebAssembly target, and
says how to add it if it is missing.

| Opening a file | The file in the editor |
|---|---|
| ![The browser demo's Open dialog](../screens/browser_open.png) | ![Pascal in colour in the browser editor](../screens/browser_code.png) |

From a console, the same thing:

```
Examples\Web\RUN.CMD
```

It builds the core for the browser, serves the repository on
`http://localhost:8765` and opens the demo. Ctrl+C in its window stops the
server. Other pages:

```
Examples\Web\RUN.CMD 01-HelloWorld     any JavaScript step, by its folder name
Examples\Web\RUN.CMD ladder            the list of the five steps
Examples\Web\RUN.CMD linked            the Rust program linked into WebAssembly
```

Needs [Rust](https://rustup.rs) and, once, its WebAssembly target:

```
rustup target add wasm32-unknown-unknown
```

A page loaded straight from the disk (`file://`) is not allowed to load a
WebAssembly module, so some web server is always needed; `RUN.CMD` uses
[httpd](httpd/src/main.rs), a small Rust server that answers this machine
only. Any other static server pointed at the repository root shows the
pages just as well; only the demo's File menu needs `httpd` for two of its
three places (see [Files](#files)).

```
RUN.CMD             build, serve, open
httpd/              the server RUN.CMD starts
files/              the folder the demo's File menu opens on the server
dos/                OWL FLY III, which the demo's DOS PC carries
site/build.mjs      the live demo as a static site, for GitHub Pages
JavaScript/
    01-HelloWorld   a window, words, a button
    02-Notes        an editor that keeps its text, finds, replaces, asks before leaving
    03-Calculator   a keypad of real buttons, bases and trigonometry, the arithmetic apart
    04-Sokoban      sixty warehouses drawn on a canvas of cells
    05-Demo         everything the kit has, with a menu bar and a status line
    test.mjs        every step driven without a browser
Rust/
    01-Linked       the shared Rust application compiled into the page
```

## JavaScript

Each step is an `index.html` that is only a canvas, and an `app.js` that is
the program. The client is [lib/js/owlosui.js](../../lib/js/owlosui.js): it
loads the core - `lib/serve`, the same server the C# programs talk to,
compiled to `lib/js/owlosui-wire.wasm` - and speaks the same requests to it
([lib/PROTOCOL.md](../../lib/PROTOCOL.md)) through the module's memory
instead of a pipe. It draws the frames on the canvas and turns keys and
the mouse into events. So the JavaScript steps mirror the C# ones call for
call: [05-Demo/app.js](JavaScript/05-Demo/app.js) and
[Desktop/CSharp/05-OwlosDemo](../Desktop/CSharp/05-OwlosDemo/Program.cs)
read side by side.

To build only the core for the browser: `lib\js\build.cmd`.

**Your own page.** Copy `owlosui.js` and `owlosui-wire.wasm`, then:

```js
import { run } from './owlosui.js';
run(canvas, owl => new App(owl));
```

where `App` builds its windows in the constructor and answers
`onCommand(cmd)` - return `false` and the program ends.
[01-HelloWorld/app.js](JavaScript/01-HelloWorld/app.js) is exactly that.

**The tests.** Node loads the same module and the same client, builds each
step's `App`, presses its keys and clicks its buttons, and reads the frames
as text - what the canvas would show:

```
node Examples\Web\JavaScript\test.mjs
```

It starts `httpd` for the file cases, so build that once first:
`cargo build --release -p owlosui-httpd`.

## Files

A web page cannot look at the files on your computer the way a DOS or
Windows program can - the browser keeps it out, for good reasons. So the
demo's **File > Open from** offers three places a page *is* allowed to
reach, and each one says in its Open dialog what it is:

**This browser's storage.** Every modern browser keeps a small private disk
for each website, called OPFS (the Origin Private File System). It lives
inside the browser's own data on this computer: nothing is uploaded, it
works without a network, and no other website can see it. You will not
find it in Explorer - only this page sees it, and clearing the site's data
in the browser's settings erases it. The first time, the demo puts a
`WELCOME.TXT` there, and a few of this project's own files to try things
on: the DOS demo in Pascal, C and assembler, a DOS program, the C# hello
world, a launcher and a screenshot. A file made with File > New is saved
here too.

**The server's folder.** [files/](files/) on the computer running
`RUN.CMD`. The page asks `httpd` for it over plain HTTP: `GET
/files/NAME` reads a file, `PUT /files/NAME` saves one, and `GET
/files/?list` lists a folder; `DELETE`, `MKCOL` (make a folder), `MOVE`
and `COPY` change it, the same verbs WebDAV uses. Change the file in the
demo, press F2, and the file on the disk changes.

**A WebDAV folder.** WebDAV is the protocol a NAS box, Nextcloud and many
servers use to share folders over the web. `httpd` shares the same
[files/](files/) folder at `/dav/`, and the dialog offers that address, so
there is one to try without setting anything up. Type another address to
open someone else's share. A share on another site has to allow this page
to call it (the browser calls this CORS), and a user name and password, if
it asks for them, go with every request.

Save (F2) puts a file back where it came from. `RUN.CMD` hands the folder
to the server; to share another one, start it yourself:

```
cargo run --release -p owlosui-httpd -- . /Examples/Web/JavaScript/05-Demo/ --files C:\some\folder
```

The three places are [lib/js/files/](../../lib/js/files/), one small file
each, and all three answer the same calls: `list(dir)`, `read(path)`,
`readBytes(path)`, `write(path, text or bytes)`, `remove(path)`,
`mkdir(path)` and `rename(from, to)`. A fourth,
[repository.js](../../lib/js/files/repository.js), is this project's own
`Examples` folder as the server shows it, and it is read-only. The file
panel only draws. The program reads a
folder from a place, hands the names to the panel, and hears back which
one was chosen. So a fourth place, such as Dropbox, is one more file of
the same shape.

Files from DOS and Windows end their lines with CR LF. The demo keeps
that: it takes the CRs off to edit and puts them back on Save.

### Tools > Commander

The same two-panel file manager as the desktop and DOS ones, over the
places above: [05-Demo/commander.js](JavaScript/05-Demo/commander.js). A
"drive" is a place: Alt+F1 and Alt+F2 choose what the left and the right
side show, as a drive letter did on DOS. The keys are Volkov
Commander's:

| Key | What it does |
|---|---|
| Enter | into a folder; on a file, what its extension says - a text opens in the editor, a picture or a PDF in the browser, a DOS program runs on the demo's DOS PC - from the floppy, or copied onto it with the files beside it; other bytes open as hex |
| F3 / F4 | view (read-only, or hex) / edit; F2 in the editor saves back where the file came from |
| F5 / F6 | copy / rename or move, to the line the dialog offers - the other side's folder, or a new name |
| F7 / F8 | make a folder / delete, asking first, No the default |
| Insert, Tab | mark a file, go to the other side |
| Ctrl+U, Ctrl+R | swap the sides, read both folders again |
| F2 | this computer: **upload** files into the side in front, or **download** the marked ones; dropping files on the page uploads them too |

Copying works between any two places, folders with everything in them.
The examples are read-only, so they are a good side to copy from.

## DOS, in the page

The demo's DOS menu switches on a DOS PC in a window: DOSBox, compiled to
WebAssembly ([lib/js/jsdos](../../lib/js/jsdos/NOTICE.md), js-dos 8.3.20),
put together in the page ([05-Demo/dos.js](JavaScript/05-Demo/dos.js)):

| On the PC | What it is |
|---|---|
| `C:\OWLOS` | OWLOSRES: the same Rust core as the page, built for DOS, resident behind INT 60h |
| `C:\DEMO` | the DOS examples - the commander, the demo in Pascal, C and assembler - with their sources |
| `C:\GAMES\OWLFLY3` | OWL FLY III, a DOS flight game played over the network ([dos/](dos/README.md)) |
| `A:`, `B:` | floppies that are folders of this browser's storage: `DOS A Drive` and `DOS B Drive` |

Each disk is built from this repository's own files when the PC is switched
on; nothing is a disk image. What it runs first is a line of its AUTOEXEC.

**The picture is laid over the window.** The core draws every cell; DOSBox's
picture is an element of the page put exactly over the window's inside - the
core says where (`owl.place`) - and taken away while a menu or another window
is over it. Click the picture, or press Enter on the window, and the keyboard
is DOS's; **Right Ctrl** gives it back, as in a virtual machine.

**Your files in DOS.** In this browser's storage there are two folders,
`DOS A Drive` and `DOS B Drive`, each with a README: they ARE the PC's
floppies A: and B:. What is in them goes onto the disks when the PC is
switched on; while it runs, what the page puts there - a copy, an upload, an
editor's save - goes straight into DOS, and the page presses Ctrl+R in DOS's
commander for you; what DOS saves there comes back into the folder within a
couple of seconds. They are kept after the page is closed. Enter on a
program in them runs it from its floppy; Enter on one anywhere else copies
it, with the files beside it, into `DOS A Drive\RUN` first
([lib/js/dosbox/gates.js](../../lib/js/dosbox/gates.js)).

**The drive lights.** At the right of the status line, while the DOS window
is in front: `[A: B: C:]`. A letter turns green while files go to or from
that drive - A: and B: between their folders and DOS, C: when DOS writes to
its hard disk. What DOS only reads, the page cannot see, so a game loading
from C: lights nothing.

They are floppies because DOSBox reads a floppy's folder afresh every time
DOS looks, and keeps a hard disk's in memory: a file the page put on a hard
disk would stay out of DOS's sight until the next switch-on.

**The network.** DOSBox's IPX card, in a browser, is a WebSocket to a relay
that hands each packet to every machine in the same room. The page owns that
WebSocket, so it counts it: **DOS > Network monitor** shows the bytes, a
graph of the last minute and the packets themselves, with their IPX
addresses ([lib/js/dosbox/nettap.js](../../lib/js/dosbox/nettap.js)).
**DOS > Machine monitor** shows what js-dos counts in the emulator: cycles a
second, how busy it is, its disks. DOS's own memory and programs it does not
show the page, so neither does the monitor.

**DOS > Settings** is DOSBox's own: video card, how its picture is shown -
sharp, every DOS pixel a whole square of screen pixels and the window sized
round it, or stretched to 4:3 and smoothed like a monitor - memory (16 MB or
more: the toolkit and DOS-extended games need it), CPU core, type and
speed, sound cards, EMS and UMB (XMS is always on: the toolkit lives in it), what to start, the network card and
the relay. It writes the `dosbox.conf` the PC boots with, and shows it.

**Awake.** A browser pauses a hidden page's emulator; the PC does not let it,
and says so with `[awake]` after its window's title, as a modal dialog says
`[modal]`. A game played in step over the network - Duke Nukem 3D - waits
for every player, so one hidden tab would stop it for all of them. Settings
> DOS turns it off, to spare a laptop's battery.

DOSBox runs on threads that share memory, which a browser allows a page only
when it is isolated from other sites. RUN.CMD's server and GitHub Pages
cannot say so in headers, so [coi-serviceworker.js](JavaScript/05-Demo/coi-serviceworker.js)
does, and the page reloads itself once on the first visit.

## Help > Console

Everything the page has done since it opened, in a terminal window: the
browser and screen it is running on, every window opened and closed, the DOS
PC switching on and off with its settings, what DOSBox itself prints, programs
run, files crossing to the floppies, the network, and every error nobody
caught, with its stack. [log.js](JavaScript/05-Demo/log.js) records it from
the first moment - before the console is opened, before anything can go wrong
unseen - and [console.js](JavaScript/05-Demo/console.js) shows it.

For a bug report: **F4** writes the state of everything now - windows,
memory, storage, the DOS PC and its emulator's counters, the network's
packets - then **Ctrl+C** copies the whole console as plain text, or **F2**
saves it as a file. F8 empties the window; the page goes on recording.

The window is the toolkit's own `CONSOLE` view, the same in every client: the
lines are written with ANSI colour sequences, and the core colours them as a
terminal would and keeps them out of the copied text
([lib/PROTOCOL.md](../../lib/PROTOCOL.md), `CONSOLE` and `CONSOLE_WRITE`).
`?mouselog` at the end of the demo's address shows the mouse's events in a
corner instead, for a click that goes missing in one browser only.

## The live demo

[site/build.mjs](site/build.mjs) makes the static site GitHub Pages serves:
the repository's own layout, cut to what a browser fetches, so every path in
the pages stays true. A static host answers no `?list`, so it also writes
`Examples/listing.json` for the commander's examples drive; the server's
folder and WebDAV are left out of the drives there, having nothing to answer
them. [.github/workflows/pages.yml](../../.github/workflows/pages.yml) builds
the core and the site on every push to main.

```
lib\js\build.cmd
node Examples\Web\site\build.mjs _site
```

## Rust

[01-Linked](Rust/01-Linked/src/lib.rs) is the other way into a page: the
core and [the shared application](../shared/app.rs) compiled into one
module together, with no wire in between - a handful of `extern "C"`
functions and a frame read out of the module's memory, no wasm-bindgen.
[web/owlosui.js](Rust/01-Linked/web/owlosui.js) draws it. `RUN.CMD linked`
builds it and opens it; `Rust\01-Linked\build.cmd` only builds it.
