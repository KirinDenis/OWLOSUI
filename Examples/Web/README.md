# Web

Programs in a browser page. Start here:

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
in the browser's settings erases it. The first time it is empty, so the
demo puts a `WELCOME.TXT` there to open. A file made with File > New is
saved here too.

**The server's folder.** [files/](files/) on the computer running
`RUN.CMD`. The page asks `httpd` for it over plain HTTP: `GET
/files/NAME` reads a file, `PUT /files/NAME` saves one, and `GET
/files/?list` lists a folder. Change the file in the demo, press F2, and
the file on the disk changes.

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
each, and all three answer the same three calls: `list(dir)`, `read(path)`
and `write(path, text)`. The file panel only draws. The program reads a
folder from a place, hands the names to the panel, and hears back which
one was chosen. So a fourth place, such as Dropbox, is one more file of
the same shape.

Files from DOS and Windows end their lines with CR LF. The demo keeps
that: it takes the CRs off to edit and puts them back on Save.

## Rust

[01-Linked](Rust/01-Linked/src/lib.rs) is the other way into a page: the
core and [the shared application](../shared/app.rs) compiled into one
module together, with no wire in between - a handful of `extern "C"`
functions and a frame read out of the module's memory, no wasm-bindgen.
[web/owlosui.js](Rust/01-Linked/web/owlosui.js) draws it. `RUN.CMD linked`
builds it and opens it; `Rust\01-Linked\build.cmd` only builds it.
