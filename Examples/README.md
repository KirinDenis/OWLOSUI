# Examples — a ladder

One core, two ways of using it, and every step one command. Start at the top
of either column; each program says in its first two lines which step it is
and where the next one is.

| | C# — the core behind a pipe | Rust — the core linked in |
|---|---|---|
| 1 | [01-HelloWorld](CSharp/01-HelloWorld/Program.cs) — a window, words, a button | [01-Demo](Rust/01-Demo/src/main.rs) — every control, on the terminal |
| 2 | [02-Notes](CSharp/02-Notes/Program.cs) — an editor that saves, and asks before leaving | [02-Window](Rust/02-Window/src/main.rs) — a native Windows window, cells drawn by hand |
| 3 | [03-Commander](CSharp/03-Commander/Program.cs) — two panels of files, a tree of the drive | [03-Browser](Rust/03-Browser/src/lib.rs) — the same app in WebAssembly |
| 4 | [04-Basez-Sokoban](CSharp/04-Basez-Sokoban/Program.cs) — a game on a canvas of cells | [04-DOS](Rust/04-DOS/src/main.rs) — the same app on DOS, drawing into B800 |
| 5 | [05-OwlosDemo](CSharp/05-OwlosDemo/Program.cs) — everything, one tool per folder | |

Steps 2, 3 and 4 of the Rust column are one program on three screens: the
application is [shared/app.rs](Rust/shared/app.rs), and each step is only the
screen it is drawn on.

![A native Windows window](screens/window.png)
![The browser](screens/browser.png)
![DOS](screens/dos.png)

## C#

Needs the .NET 8 SDK and Rust (for the core; the build runs cargo itself).

```
cd Examples\CSharp\01-HelloWorld
dotnet run
```

The same in any of the five folders. `OWLOSUI.sln` at the repository root
holds all of them and their tests.

**One file to give away.** In any example folder:

```
dotnet publish -c Release -r win-x64
```

and `bin\Release\net8.0\win-x64\publish\` holds a single `.exe`. The client
(`lib/csharp/Owlosui.cs`) is folded into it, and the core rides inside the
client: when the program finds no `owlosui-serve.exe` beside it, it writes
its own copy to `%TEMP%\owlosui\` and starts that. Nothing else to carry.

**Your own program.** Reference `lib/csharp/Owlosui.csproj`, or copy
`Owlosui.cs`, `Win32.cs` and `ConsoleAgent.cs` into your project — they are
the whole client, no packages. [01-HelloWorld](CSharp/01-HelloWorld/Program.cs)
is the shape of every program: build the windows, then `owl.Run(onCommand)`.

**The tests** drive every example by the wire and through a real console:

```
cd Examples\CSharp\Tests
dotnet run
```

## Rust

```
cargo run -p owlosui-demo        # step 1, the terminal
cargo run -p owlosui-win         # step 2, a Windows window
```

**Step 3, the browser.** Once: `rustup target add wasm32-unknown-unknown`.
Then:

```
Examples\Rust\03-Browser\build.cmd
python -m http.server 8765 --directory Examples\Rust\03-Browser\web
```

and open <http://localhost:8765>. The page is
[index.html](Rust/03-Browser/web/index.html) and
[owlosui.js](Rust/03-Browser/web/owlosui.js), two hundred lines that draw
cells on a canvas.

**Step 4, DOS.** Once: `rustup toolchain install nightly --component
rust-src`, and [DOSBox-X](https://dosbox-x.com) for running it on Windows.
Then:

```
Examples\Rust\04-DOS\build.cmd
Examples\Rust\04-DOS\RUNWIN.BAT
```

On a real machine or in any DOS: `RUN` in that folder. `test.cmd` there
builds it, runs it under DOSBox-X and checks that the frame the program drew
is the screen the emulator shows, that keys reach it, and that a minute of
dragging windows does not exhaust its memory. The loader is
[OWLOS.ASM](Rust/04-DOS/OWLOS.ASM), FASM, and reads like a lesson.

## C

[Empty on purpose](C/README.md): DOS programs will reach a resident copy of
the toolkit through a software interrupt, once there is one.
