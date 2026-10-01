# Examples

Everything here *uses* the toolkit; the toolkit itself is [lib/](../lib/README.md).

**The quickest way to see any of it** is a launcher in the repository's
root - `CS_Demo.cmd`, `Web_Demo.cmd`, `DOS_Pascal_Demo.cmd` and the rest:
double-click one. [The root README](../README.md#try-it) lists them all
with what each one needs.

The folders say where a program runs, then what it is written in:

```
Desktop/            a program on Windows: in a console, or in a window of its own
    CSharp/         six steps, 01-HelloWorld to 06-Window, and their tests
    Rust/           01-Terminal, 02-Window
Web/                a page in a browser
    RUN.CMD         builds, serves, opens the demo
    JavaScript/     five steps, 01-HelloWorld to 05-Demo, and their tests
    Rust/           01-Linked: a Rust program compiled into the page
DOS/                a program on DOS, real or in DOSBox-X
    Asm/            HELLO and DEMO in assembler   \
    C/              HELLO and DEMO in C            > through the resident, INT 60h
    Pascal/         HELLO and DEMO in Pascal      /
    Rust/           the Rust application as a flat binary under DPMI
    Commandr/       a two-panel file manager in Pascal
shared/app.rs       the one Rust application that Desktop/Rust/02-Window,
                    Web/Rust and DOS/Rust each put on their own screen
screens/            the pictures in these READMEs
```

Each folder has a README with what is in it and the commands to build and
run it. Each program says in its first two lines which step it is and where
the next one is.

| | Desktop | Web | DOS |
|---|---|---|---|
| **double-click** | `CS_Demo.cmd`, `CS_Window.cmd`, `CS_Commander.cmd`, `Rust_Terminal.cmd`, `Rust_Window.cmd` | `Web_Demo.cmd` | `DOS_Pascal_Demo.cmd`, `DOS_C_Demo.cmd`, `DOS_Asm_Demo.cmd`, `DOS_Rust_Demo.cmd`, `DOS_Commander.cmd` |
| **the first program to read** | [Desktop/CSharp/01-HelloWorld](Desktop/CSharp/01-HelloWorld/Program.cs) | [Web/JavaScript/01-HelloWorld](Web/JavaScript/01-HelloWorld/app.js) | [DOS/Pascal/HELLO.PAS](DOS/Pascal/HELLO.PAS) |
| **the full demo** | [Desktop/CSharp/05-OwlosDemo](Desktop/CSharp/05-OwlosDemo/Program.cs) in a console, [06-Window](Desktop/CSharp/06-Window/Program.cs) in a window | [Web/JavaScript/05-Demo](Web/JavaScript/05-Demo/app.js) | DEMO in [assembler](DOS/Asm/DEMO.ASM), [C](DOS/C/DEMO.C) and [Pascal](DOS/Pascal/DEMO.PAS) through the resident, and in [Rust](DOS/Rust/src/demo.rs) with the core linked in |
| **a file manager** | [Desktop/CSharp/03-Commander](Desktop/CSharp/03-Commander/Program.cs) | - a page cannot read a disk; the demo's File menu opens files from three places instead | [DOS/Commandr](DOS/Commandr/README.md) |
| **how it reaches the core** | C#: a pipe to `owlosui-serve`, carried inside the client. Rust: linked in | JavaScript: the same server compiled to WebAssembly. Rust: linked into the module | assembler, C, Pascal: INT 60h to OWLOSRES, the resident. Rust: linked into a flat binary |
| **read next** | [Desktop/README.md](Desktop/README.md) | [Web/README.md](Web/README.md) | [DOS/README.md](DOS/README.md) |

| On DOS | On DOS: files | In a browser |
|---|---|---|
| ![The demo on DOS](screens/dos_demo.png) | ![The file manager on DOS](screens/dos_commander.png) | ![The browser demo opening a file](screens/browser_open.png) |
| **On Windows** | **On DOS: the editor** | **In a browser: the editor** |
| ![The C# demo in a window](screens/window.png) | ![Pascal in colour on DOS](screens/dos_editor.png) | ![Pascal in colour in a browser](screens/browser_code.png) |

The C# and JavaScript ladders are the same five programs - a greeting, a
notes editor, a tool, a game, the demo - so either one can be read against
the other. (The desktop's step 3 is a two-panel file manager, the web's a
calculator: a page cannot read a disk.)
