# Desktop

Programs for a desktop: a console, or a window of their own.

**Quickest:** double-click `CS_Demo.cmd` (the C# demo in a console),
`CS_Window.cmd` (the same in a window), `CS_Commander.cmd` (the file
manager), `Rust_Terminal.cmd` or `Rust_Window.cmd` in the repository's
root. They need the [.NET 8 SDK](https://dotnet.microsoft.com/download)
and [Rust](https://rustup.rs), and say so if either is missing.

![The C# demo in a native Windows window](../screens/window.png)

```
CSharp/
    01-HelloWorld       a window, words, a button: the shape of every program
    02-Notes            an editor that saves, finds, replaces, asks before leaving
    03-Commander        two panels of files: marks, copy, move, a tree of the drive
    04-Basez-Sokoban    a game on a canvas of cells the program draws itself
    05-OwlosDemo        everything the kit has, one tool per folder
    06-Window           05-OwlosDemo in a native window instead of the console
    Tests               every step driven by the wire, by a console agent, and in a window
Rust/
    01-Terminal         the core linked in, drawn on the terminal
    02-Window           the shared application in a native window
```

## C#

Needs the [.NET 8 SDK](https://dotnet.microsoft.com/download) and
[Rust](https://rustup.rs). The C# build runs cargo itself: the core is
compiled and carried inside the client library.

```
cd Examples\Desktop\CSharp\01-HelloWorld
dotnet run
```

The same in any step's folder. `OWLOSUI.sln` at the repository root holds
all of them, the client and the tests, for Visual Studio.

**How a C# program gets its screen.** The program talks to `owlosui-serve`,
the core behind a pipe ([lib/PROTOCOL.md](../../lib/PROTOCOL.md)), through
the client [lib/csharp/Owlosui.cs](../../lib/csharp/Owlosui.cs). Steps 1 to 5
put the cells on the console they were started in; the mouse works there
too. Step 6 adds one line, `owl.OpenWindow("...")`, and the server shows the
desktop in a window of its own and reads the keys and the mouse there -
the program draws nothing either way. Build a program as `WinExe`, as 06
does, and no console opens at all.

**One file to give away.** In any step's folder:

```
dotnet publish -c Release -r win-x64
```

leaves one `.exe` in `bin\Release\net8.0\win-x64\publish\`: the program, the
client and the core. It needs .NET 8 on the machine and nothing else.

**Your own program.** Reference `lib/csharp/Owlosui.csproj`, or copy
`Owlosui.cs`, `Win32.cs` and `ConsoleAgent.cs` into it - that is the whole
client, no packages. Build the windows, then `owl.Run(onCommand)`:
[01-HelloWorld](CSharp/01-HelloWorld/Program.cs) is that and nothing else.

**The tests.**

```
cd Examples\Desktop\CSharp\Tests
dotnet run
```

Each case builds the scene a step builds, by calling the step's own code,
then presses its buttons, drags its corners and types into it over the
wire, and reads the frame. The last cases leave the wire: a console agent
starts a step in a real console and puts mouse and key records into its
input buffer, and a window case opens the desktop in a window - without
taking the focus - and posts a key to it. Nobody's keyboard is used. A
picture of that window is left in the temp folder as `owlosui-window.bmp`.

## Rust

Needs [Rust](https://rustup.rs). From the repository root:

```
cargo run -p owlosui-demo        # 01-Terminal
cargo run -p owlosui-win         # 02-Window
```

**01-Terminal** is the reference application: every control, help, files,
a modal question. Drag windows by the title, resize from the corner, `F1`
help, `F3` open a file, `F4` a dialog with every control, `F5` zoom, `F6`
next window, `Alt-F3` close, `Alt-X` quit. One frame as plain text, with no
terminal at all:

```
cargo run -p owlosui-demo -- --dump [help|menu|files|controls|modal]
```

**02-Window** is forty lines: the application is
[shared/app.rs](../shared/app.rs), the same one the browser and DOS builds
run, and the window is [lib/window](../../lib/window/src/lib.rs). The
example only says what its core is and what to do after each key - that is
the whole of what a Rust program does to get a window.
