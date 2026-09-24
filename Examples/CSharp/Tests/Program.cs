// The examples, driven by the wire.
//
// Each case builds the same scene an example builds - by calling the
// example's own code - then does to it what a person would do with a mouse
// and a keyboard, and looks at the frame. No console is involved: the
// server is started headless at 80x25 and the cells come back as bytes.
//
// These exist because two things were found by a person and not by a test:
// the Notes window could not be resized, and no button could be pressed
// with the mouse. Both had one cause - the C# client never sent a mouse
// event - and both would have been one line here.
//
// Run from this folder, with the server built:
//
//     cargo build -p owlosui-serve
//     dotnet run

using HW = HelloWorld.App;
using NotesApp = Notes.App;

internal static class Tests
{
    private static int failed;

    private static int Main(string[] args)
    {
        // Notes writes a file named after its window title; keep the title
        // short by working in the temp folder rather than naming a long path.
        Directory.SetCurrentDirectory(Path.GetTempPath());
        const string notesFile = "OWLOSUI-TEST.TXT";

        // The second half of the suite runs in a process of its own, because
        // an agent has to give up its console to attach to the program's.
        if (args.Contains("--agent")) return ConsoleCases.Run(notesFile);

        // ---------------------------------------------------------- HelloWorld

        Case("HelloWorld: the words are on the screen", () =>
        {
            using var owl = Owl();
            HW.Build(owl);
            var f = owl.GetFrame();
            Check(f.Find("Hello, world!") != null, "the greeting is missing", f);
            Check(f.Find("shown by C#.") != null, "the second line did not wrap where expected", f);
            Check(f.Find(" OK ") != null, "no OK button", f);
        });

        Case("HelloWorld: OK can be pressed with the mouse", () =>
        {
            using var owl = Owl();
            HW.Build(owl);
            var (x, y) = Locate(owl.GetFrame(), " OK ");
            owl.Click(x + 1, y);
            var (pressed, _) = owl.Take();
            Check(pressed == HW.CmOk, $"the click pressed {pressed}, not OK");
            Check(!HW.OnCommand(pressed), "OK should end the program");
        });

        Case("HelloWorld: Enter and Escape both press OK", () =>
        {
            using var owl = Owl();
            HW.Build(owl);
            owl.GetFrame();
            owl.Press(ConsoleKey.Enter);
            Check(owl.Take().pressed == HW.CmOk, "Enter did not press the default button");
            owl.Press(ConsoleKey.Escape);
            Check(owl.Take().pressed == HW.CmOk, "Escape did not press the last button");
        });

        Case("HelloWorld: the window can be dragged by its title", () =>
        {
            using var owl = Owl();
            HW.Build(owl);
            var f = owl.GetFrame();
            var (x, y) = Locate(f, " Hello ");
            owl.Drag(x + 2, y, x + 7, y + 2);
            var after = owl.GetFrame();
            var moved = Locate(after, " Hello ");
            Check(moved == (x + 5, y + 2), $"the title went from ({x},{y}) to {moved}, expected ({x + 5},{y + 2})", after);
        });

        // --------------------------------------------------------------- Notes

        Case("Notes: the window can be resized from its corner", () =>
        {
            File.Delete(notesFile);
            using var owl = Owl();
            var app = new NotesApp(owl, notesFile);
            var f = owl.GetFrame();

            // 76x23 centred on 80x25 puts the frame at (2,1)-(77,23); the grip
            // is the bottom-right corner cell.
            Check(IsCorner(f, 77, 23), "the window is not where the layout says it is", f);
            owl.Drag(77, 23, 67, 18);
            var g = owl.GetFrame();
            Check(IsCorner(g, 67, 18), "the corner did not follow the mouse", g);
            Check(!IsCorner(g, 77, 23), "the old corner is still there", g);
        });

        Case("Notes: Exit with unsaved text asks, and a click behind the box does nothing", () =>
        {
            File.Delete(notesFile);
            using var owl = Owl();
            var app = new NotesApp(owl, notesFile);
            owl.GetFrame();
            owl.Type("hello");
            Check(app.Modified, "typing did not change the text");
            Check(owl.GetText(app.Editor) == "hello", $"the editor holds '{owl.GetText(app.Editor)}'");

            var f = owl.GetFrame();
            var save = Locate(f, "Save");
            var (ex, ey) = Locate(f, "Exit");
            owl.Click(ex, ey);
            var (pressed, _) = owl.Take();
            Check(pressed == NotesApp.CmExit, $"the click pressed {pressed}, not Exit");
            Check(app.OnCommand(pressed), "Exit with unsaved text should not end the program");

            // The box wraps the sentence; the tail is on a line of its own.
            var g = owl.GetFrame();
            Check(g.Find("without saving?") != null, "no question was asked", g);

            // The Save button is behind the modal box now. A click on it must
            // do nothing at all - not save, not even move the focus.
            owl.Click(save.x, save.y);
            Check(owl.Take().pressed == 0, "a button behind the modal box was pressed");
            Check(!File.Exists(notesFile), "the file was written by a click that should have done nothing");

            // Stay, by the mouse. The box goes and the text is still there.
            var (sx, sy) = Locate(g, "Stay");
            owl.Click(sx, sy);
            var (stay, _) = owl.Take();
            Check(stay == NotesApp.CmStay, $"the click pressed {stay}, not Stay");
            app.OnCommand(stay);
            var h = owl.GetFrame();
            Check(h.Find("without saving?") == null, "the box is still up after Stay", h);
            Check(owl.GetText(app.Editor) == "hello", "the text was lost");
        });

        Case("Notes: Leave by the mouse ends the program without writing", () =>
        {
            File.Delete(notesFile);
            using var owl = Owl();
            var app = new NotesApp(owl, notesFile);
            owl.GetFrame();
            owl.Type("gone");
            owl.Press(ConsoleKey.X, alt: true, ch: 'x');
            var (exit, _) = owl.Take();
            Check(exit == NotesApp.CmExit, $"Alt+X pressed {exit}, not Exit");
            app.OnCommand(exit);
            // "Leave" is also in the question. The button is on Stay's row.
            var f = owl.GetFrame();
            var (_, ly) = Locate(f, "Stay");
            var lx = f.Row(ly).IndexOf("Leave", StringComparison.Ordinal);
            Check(lx >= 0, "no Leave button beside Stay", f);
            owl.Click(lx, ly);
            var (leave, _) = owl.Take();
            Check(leave == NotesApp.CmLeave, $"the click pressed {leave}, not Leave");
            Check(!app.OnCommand(leave), "Leave should end the program");
            Check(!File.Exists(notesFile), "Leave wrote the file");
        });

        Case("Notes: Save writes the file, and then Exit does not ask", () =>
        {
            File.Delete(notesFile);
            using var owl = Owl();
            var app = new NotesApp(owl, notesFile);
            owl.GetFrame();
            owl.Type("kept");
            owl.Press(ConsoleKey.S, alt: true, ch: 's');
            var (save, _) = owl.Take();
            Check(save == NotesApp.CmSave, $"Alt+S pressed {save}, not Save");
            app.OnCommand(save);
            Check(File.ReadAllText(notesFile) == "kept", $"the file holds '{File.ReadAllText(notesFile)}'");
            Check(!app.Modified, "still marked modified after saving");
            Check(!app.OnCommand(NotesApp.CmExit), "Exit asked a question with nothing unsaved");
            Check(owl.GetFrame().Find("without saving?") == null, "a box appeared anyway");
        });

        Case("Notes: the text survives a reopen", () =>
        {
            File.WriteAllText(notesFile, "one\ntwo");
            using var owl = Owl();
            var app = new NotesApp(owl, notesFile);
            var f = owl.GetFrame();
            Check(f.Find("one") != null && f.Find("two") != null, "the file's lines are not on the screen", f);
            Check(!app.Modified, "freshly opened and already modified");
        });

        // ------------------------------------- Notes: every other thing a hand can do
        //
        // The list Turbo Vision's manual gives for a window: move, resize,
        // zoom, close, next; and for an editor: type, Enter, scroll. Each of
        // these is one thing a person will do in the first minute, and each
        // is checked by doing it, not by reading the code that should do it.

        Case("Notes: the close box asks the question instead of closing", () =>
        {
            File.Delete(notesFile);
            using var owl = Owl();
            var app = new NotesApp(owl, notesFile);
            owl.GetFrame();
            owl.Type("x");
            // [■] sits at x+2..x+4 on the title row; the window is at (2,1).
            owl.Click(5, 1);
            var (_, command) = owl.Take();
            Check(command == NotesApp.CmExit, $"the close box sent {command}, not Exit");
            Check(app.OnCommand(command), "closing with unsaved text should ask, not leave");
            var f = owl.GetFrame();
            Check(f.Find("saving?") != null, "no question after the close box", f);
            Check(f.Find(" " + notesFile + " ") != null, "the window was closed anyway", f);
        });

        Case("Notes: the zoom box fills the desktop and puts it back", () =>
        {
            File.Delete(notesFile);
            using var owl = Owl();
            _ = new NotesApp(owl, notesFile);
            var f = owl.GetFrame();
            // [↑] sits at right-5..right-3 on the title row; right edge is 78.
            owl.Click(74, 1);
            var g = owl.GetFrame();
            Check(IsCorner(g, 79, 24), "zoom did not fill the desktop", g);
            // Zoomed, the title row is row 0 and the box has moved with it.
            owl.Click(76, 0);
            var h = owl.GetFrame();
            Check(IsCorner(h, 77, 23), "unzoom did not put the window back", h);
        });

        Case("Notes: Enter in the editor is a new line, not the Save button", () =>
        {
            File.Delete(notesFile);
            using var owl = Owl();
            var app = new NotesApp(owl, notesFile);
            owl.GetFrame();
            owl.Type("a");
            owl.Press(ConsoleKey.Enter);
            owl.Type("b");
            var (pressed, _) = owl.Take();
            Check(pressed == 0, $"Enter pressed button {pressed} instead of breaking the line");
            Check(owl.GetText(app.Editor) == "a\nb", $"the editor holds '{owl.GetText(app.Editor).Replace("\n", "\\n")}'");
            Check(!File.Exists(notesFile), "Enter saved the file");
        });

        Case("Notes: Escape in the editor is Exit, which asks", () =>
        {
            File.Delete(notesFile);
            using var owl = Owl();
            var app = new NotesApp(owl, notesFile);
            owl.GetFrame();
            owl.Type("x");
            owl.Press(ConsoleKey.Escape);
            var (pressed, _) = owl.Take();
            Check(pressed == NotesApp.CmExit, $"Escape pressed {pressed}, not Exit");
            app.OnCommand(pressed);
            Check(owl.GetFrame().Find("saving?") != null, "no question after Escape");
        });

        Case("Notes: the wheel scrolls a long text", () =>
        {
            File.WriteAllText(notesFile, string.Join("\n", Enumerable.Range(1, 60).Select(i => $"L{i:00}")));
            using var owl = Owl();
            _ = new NotesApp(owl, notesFile);
            var f = owl.GetFrame();
            Check(f.Find("L01") != null && f.Find("L40") == null, "the text is not laid out as expected", f);
            owl.SendMouse(Owlosui.MouseKind.WheelDown, 20, 10);
            owl.SendMouse(Owlosui.MouseKind.WheelDown, 20, 10);
            var g = owl.GetFrame();
            Check(g.Find("L01") == null, "the wheel did not scroll", g);
            owl.SendMouse(Owlosui.MouseKind.WheelUp, 20, 10);
            owl.SendMouse(Owlosui.MouseKind.WheelUp, 20, 10);
            Check(owl.GetFrame().Find("L01") != null, "the wheel did not scroll back");
        });

        Case("Notes: a click in the text moves the caret there", () =>
        {
            File.WriteAllText(notesFile, "first line\nsecond line");
            using var owl = Owl();
            _ = new NotesApp(owl, notesFile);
            var f = owl.GetFrame();
            var (x, y) = Locate(f, "second");
            owl.Click(x + 3, y);
            var g = owl.GetFrame();
            Check((g.CursorX, g.CursorY) == (x + 3, y), $"the caret is at ({g.CursorX},{g.CursorY}), not ({x + 3},{y})", g);
        });

        Case("Notes: the question box can be dragged aside and still answers", () =>
        {
            File.Delete(notesFile);
            using var owl = Owl();
            var app = new NotesApp(owl, notesFile);
            owl.GetFrame();
            owl.Type("x");
            app.OnCommand(NotesApp.CmExit);
            var f = owl.GetFrame();
            var (x, y) = Locate(f, " Confirm ");
            owl.Drag(x + 2, y, x + 2 - 15, y + 4);
            var g = owl.GetFrame();
            var moved = Locate(g, " Confirm ");
            Check(moved == (x - 15, y + 4), $"the box went to {moved}", g);
            var (sx, sy) = Locate(g, "Stay");
            owl.Click(sx, sy);
            Check(owl.Take().pressed == NotesApp.CmStay, "Stay does not answer after the move");
        });

        Case("Notes: the desktop resized under it, every way, and it is still there", () =>
        {
            File.Delete(notesFile);
            using var owl = Owl();
            var app = new NotesApp(owl, notesFile);
            owl.GetFrame();
            owl.Type("kept");
            foreach (var (w, h) in new[] { (120, 40), (60, 15), (20, 5), (1, 1), (80, 25) })
            {
                owl.Resize(w, h);
                var f = owl.GetFrame();
                Check(f.W == Math.Max(w, 20) && f.H == Math.Max(h, 5), $"frame is {f.W}x{f.H} after {w}x{h}");
            }
            var g = owl.GetFrame();
            Check(g.Find(notesFile) != null, "the window is gone after the resizes", g);
            Check(g.Find("Exit") != null, "the buttons are gone after the resizes", g);
            Check(owl.GetText(app.Editor) == "kept", "the text was lost");
        });

        Case("Notes: the desktop grows and the editor grows with it", () =>
        {
            File.Delete(notesFile);
            using var owl = Owl();
            _ = new NotesApp(owl, notesFile);
            owl.GetFrame();
            owl.Resize(100, 30);
            var f = owl.GetFrame();
            Check(IsCorner(f, 97, 28), "the window did not follow the far edges", f);
            owl.Resize(80, 25);
            Check(IsCorner(owl.GetFrame(), 77, 23), "the window did not come back to size");
        });

        Case("Notes: a resize with the question up keeps the question centred", () =>
        {
            File.Delete(notesFile);
            using var owl = Owl();
            var app = new NotesApp(owl, notesFile);
            owl.GetFrame();
            owl.Type("x");
            app.OnCommand(NotesApp.CmExit);
            owl.Resize(120, 40);
            var f = owl.GetFrame();
            var (bx, _) = Locate(f, " Confirm ");
            Check(Math.Abs(bx + 4 - 60) <= 26, $"the box is not centred: title at {bx}", f);
            owl.Resize(80, 25);
            var (sx, sy) = Locate(owl.GetFrame(), "Stay");
            owl.Click(sx, sy);
            Check(owl.Take().pressed == NotesApp.CmStay, "Stay does not answer after the resizes");
        });

        Case("HelloWorld: the close box closes it", () =>
        {
            using var owl = Owl();
            HW.Build(owl);
            var f = owl.GetFrame();
            var (x, y) = Locate(f, " Hello ");
            // The frame's left edge is where the row's first non-desktop cell is.
            var left = f.Row(y).IndexOf('╔');
            owl.Click(left + 3, y);
            Check(owl.GetFrame().Find("Hello, world!") == null, "the close box did nothing");
        });

        // ------------------------------------------------------------- desktop

        Case("The desktop follows the console size", () =>
        {
            using var owl = Owl();
            HW.Build(owl);
            owl.Resize(100, 30);
            var f = owl.GetFrame();
            Check(f.W == 100 && f.H == 30, $"the frame is {f.W}x{f.H}");
            Check(f.Find("Hello, world!") != null, "the window was lost in the resize", f);
        });

        File.Delete(notesFile);

        // ------------------------------------------------- the real console

        Console.WriteLine();
        failed += RunAgent();

        Console.WriteLine(failed == 0 ? "\nall cases passed" : $"\n{failed} case(s) FAILED");
        return failed == 0 ? 0 : 1;
    }

    /// <summary>Spawn this program again as the agent, relay its lines, return how many cases it failed.</summary>
    private static int RunAgent()
    {
        if (!OperatingSystem.IsWindows())
        {
            Console.WriteLine("skip  console cases: the agent drives a Windows console");
            return 0;
        }
        var self = Environment.ProcessPath ?? "dotnet";
        var args = "--agent";
        if (Path.GetFileName(self).Equals("dotnet.exe", StringComparison.OrdinalIgnoreCase))
            args = $"\"{Path.Combine(AppContext.BaseDirectory, "Tests.dll")}\" --agent";
        var psi = new System.Diagnostics.ProcessStartInfo(self, args)
        {
            UseShellExecute = false,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
        };
        using var p = System.Diagnostics.Process.Start(psi)!;
        p.ErrorDataReceived += (_, e) => { if (e.Data != null) Console.WriteLine("      " + e.Data); };
        p.BeginErrorReadLine();
        string? line;
        while ((line = p.StandardOutput.ReadLine()) != null) Console.WriteLine(line);
        p.WaitForExit();
        return p.ExitCode;
    }

    // ------------------------------------------------------------- harness

    private static Owlosui Owl() => new(width: 80, height: 25);

    private static void Case(string name, Action body)
    {
        try
        {
            body();
            Console.WriteLine("ok    " + name);
        }
        catch (Exception e)
        {
            failed++;
            Console.WriteLine("FAIL  " + name);
            Console.WriteLine("      " + e.Message.Replace("\n", "\n      "));
        }
    }

    private static void Check(bool ok, string what, Owlosui.Frame? frame = null)
    {
        if (ok) return;
        throw new Exception(frame is { } f ? what + "\n" + f : what);
    }

    private static (int x, int y) Locate(Owlosui.Frame f, string text) =>
        f.Find(text) ?? throw new Exception($"'{text}' is not on the screen\n{f}");

    /// <summary>A bottom-right frame corner: ╝ on a double frame, ┘ on a single one or a resize grip.</summary>
    internal static bool IsCorner(Owlosui.Frame f, int x, int y) =>
        x < f.W && y < f.H && f.Glyph(x, y) is 0xBC or 0xD9;

    internal static int Failed => failed;
    internal static void RunCase(string name, Action body) => Case(name, body);
    internal static void Require(bool ok, string what, Owlosui.Frame? frame = null) => Check(ok, what, frame);
}

/// <summary>
/// The examples in a real console, driven by an agent.
///
/// The headless cases prove the wire. These prove the layer between the
/// wire and the console - the one that broke first, when the client did not
/// read the mouse at all. Notes is started in a hidden console of its own;
/// the agent attaches, puts the same INPUT_RECORDs into its input buffer that
/// a person's mouse would, and reads the screen back.
/// </summary>
internal static class ConsoleCases
{
    private static readonly string NotesExe = Path.Combine(AppContext.BaseDirectory, "Notes.exe");

    public static int Run(string notesFile)
    {
        // An agent must have no console of its own before it can attach to
        // another program's. Our stdout is a pipe to the runner, so nothing
        // is lost.
        ConsoleAgent.ReleaseOwnConsole();
        var workDir = Path.GetTempPath();

        Tests.RunCase("Console: Notes asks the console for the mouse", () =>
        {
            File.Delete(notesFile);
            using var a = ConsoleAgent.Start(NotesExe, "", workDir);
            var f = a.WaitFor(s => s.Find("Exit") != null);
            Tests.Require(f.Find("Exit") != null, "Notes did not draw its window\n" + a.Trace(), f);
            Tests.Require(a.MouseEnabled, $"input mode is {a.InputMode:X4}: mouse off or quick-edit on\n" + a.Trace());
        });

        Tests.RunCase("Console: the window is resized by dragging its corner", () =>
        {
            File.Delete(notesFile);
            using var a = ConsoleAgent.Start(NotesExe, "", workDir);
            var f = a.WaitFor(s => s.Find("Exit") != null);
            // The window is (W-4)x(H-2), centred, so its corner is at (W-3, H-2)
            // for a console W x H - whatever size the hidden console came up as.
            int cx = f.W - 3, cy = f.H - 2;
            Tests.Require(Tests.IsCorner(f, cx, cy), $"no corner at ({cx},{cy})", f);
            a.Drag(cx, cy, cx - 10, cy - 5);
            var g = a.WaitFor(s => Tests.IsCorner(s, cx - 10, cy - 5));
            Tests.Require(Tests.IsCorner(g, cx - 10, cy - 5), "the corner did not follow the mouse\n" + a.Trace(), g);
        });

        Tests.RunCase("Console: typing, then a click on Exit, asks the question", () =>
        {
            File.Delete(notesFile);
            using var a = ConsoleAgent.Start(NotesExe, "", workDir);
            var f = a.WaitFor(s => s.Find("Exit") != null);
            a.Type("hello");
            f = a.WaitFor(s => s.Find("hello") != null);
            Tests.Require(f.Find("hello") != null, "typed text did not appear\n" + a.Trace(), f);
            var (x, y) = f.Find("Exit") ?? throw new Exception("Exit vanished");
            a.Click(x, y);
            var g = a.WaitFor(s => s.Find("saving?") != null);
            Tests.Require(g.Find("saving?") != null, "the click on Exit did nothing\n" + a.Trace(), g);

            // Stay, by the mouse: the box goes, the text stays.
            var (sx, sy) = g.Find("Stay") ?? throw new Exception("no Stay button");
            a.Click(sx, sy);
            var h = a.WaitFor(s => s.Find("saving?") == null);
            Tests.Require(h.Find("saving?") == null, "the click on Stay did nothing\n" + a.Trace(), h);
            Tests.Require(h.Find("hello") != null, "the text was lost", h);
        });

        Tests.RunCase("Console: the console window is resized and the program survives", () =>
        {
            File.Delete(notesFile);
            using var a = ConsoleAgent.Start(NotesExe, "", workDir);
            var f = a.WaitFor(s => s.Find("Exit") != null);
            a.Type("kept");
            a.WaitFor(s => s.Find("kept") != null);

            // Smaller, then back. Each time the buttons must come back into
            // view and the corner must be where a window that follows the
            // desktop's far edges puts it.
            foreach (var (w, h) in new[] { (60, 16), (100, 30), (f.W, f.H) })
            {
                a.Resize(w, h);
                // Wait for the corner, not for "Exit": the old picture, torn
                // into the new buffer width, still has the word in it for a
                // moment before the program redraws.
                var g = a.WaitFor(s => s.W == w && s.H == h && Tests.IsCorner(s, w - 3, h - 2));
                Tests.Require(!a.HasExited, $"the program died on resize to {w}x{h}\n" + a.Trace());
                Tests.Require(g.W == w && g.H == h, $"the screen is {g.W}x{g.H}, not {w}x{h}", g);
                Tests.Require(g.Find("Exit") != null, $"the buttons are off screen after {w}x{h}\n" + a.Trace(), g);
                Tests.Require(Tests.IsCorner(g, w - 3, h - 2), $"the window does not fill {w}x{h}\n" + a.Trace(), g);
                Tests.Require(g.Find("kept") != null, "the text was lost in the resize", g);
            }
        });

        Tests.RunCase("Console: Alt+X with nothing changed ends the program", () =>
        {
            File.Delete(notesFile);
            using var a = ConsoleAgent.Start(NotesExe, "", workDir);
            a.WaitFor(s => s.Find("Exit") != null);
            a.Key(ConsoleKey.X, alt: true);
            Tests.Require(a.WaitForExit(), "Alt+X did not end the program\n" + a.Trace());
        });

        File.Delete(notesFile);
        return Tests.Failed;
    }
}
