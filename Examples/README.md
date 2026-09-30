# Examples

Everything here *uses* the toolkit; the toolkit itself is [lib/](../lib/README.md).
The folders say where a program runs, then what it is written in:

```
Desktop/            a program on a Windows (or any) desktop
    CSharp/         six steps, 01-HelloWorld to 06-Window, and their tests
    Rust/           01-Terminal, 02-Window
Web/                a page in a browser
    RUN.CMD         builds, serves, opens the demo - start here
    JavaScript/     five steps, 01-HelloWorld to 05-Demo, and their tests
    Rust/           01-Linked: a Rust program compiled into the page
DOS/                a program on DOS, real or DOSBox-X
    Asm/            HELLO and DEMO in assembler   \
    C/              HELLO and DEMO in C            > through the resident, INT 60h
    Pascal/         HELLO and DEMO in Pascal      /
    Rust/           the Rust application as a flat binary under DPMI
shared/app.rs       the one Rust application that Desktop/Rust/02-Window,
                    Web/Rust and DOS/Rust each put on their own screen
screens/            the pictures in these READMEs
```

Each folder has a README with what is in it and the commands to build and
run it. Each program says in its first two lines which step it is and where
the next one is.

| | Desktop | Web | DOS |
|---|---|---|---|
| **start with** | `dotnet run` in [Desktop/CSharp/01-HelloWorld](Desktop/CSharp/01-HelloWorld/Program.cs) | `Web\RUN.CMD` | `DOS\Asm\RUNWIN.BAT DEMO` |
| **the full demo** | [Desktop/CSharp/05-OwlosDemo](Desktop/CSharp/05-OwlosDemo/Program.cs) in a console, [06-Window](Desktop/CSharp/06-Window/Program.cs) in a window | [Web/JavaScript/05-Demo](Web/JavaScript/05-Demo/app.js) | DEMO in [assembler](DOS/Asm/DEMO.ASM), [C](DOS/C/DEMO.C) and [Pascal](DOS/Pascal/DEMO.PAS); [the Rust app](DOS/Rust/src/main.rs) |
| **how it reaches the core** | C#: a pipe to `owlosui-serve`, carried inside the client. Rust: linked in | JavaScript: the same server compiled to WebAssembly. Rust: linked into the module | assembler, C, Pascal: INT 60h to OWLOSRES, the resident. Rust: linked into a flat binary |
| **read next** | [Desktop/README.md](Desktop/README.md) | [Web/README.md](Web/README.md) | [DOS/README.md](DOS/README.md) |

| A native Windows window | A browser | DOS |
|---|---|---|
| ![window](screens/window.png) | ![browser](screens/browser.png) | ![dos](screens/dos.png) |

The C# and JavaScript ladders are the same five programs - a greeting, a
notes editor, a tool, a game, the demo - so either one can be read against
the other. (The desktop's step 3 is a two-panel file manager, the web's a
calculator: a page cannot read a disk.)
