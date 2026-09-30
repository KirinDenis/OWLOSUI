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
[httpd](httpd/src/main.rs), a hundred and fifty lines of Rust that serve files to
this machine only. Any other static server pointed at the repository root
does the same job.

```
RUN.CMD             build, serve, open
httpd/              the server RUN.CMD starts
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

## Rust

[01-Linked](Rust/01-Linked/src/lib.rs) is the other way into a page: the
core and [the shared application](../shared/app.rs) compiled into one
module together, with no wire in between - a handful of `extern "C"`
functions and a frame read out of the module's memory, no wasm-bindgen.
[web/owlosui.js](Rust/01-Linked/web/owlosui.js) draws it. `RUN.CMD linked`
builds it and opens it; `Rust\01-Linked\build.cmd` only builds it.
