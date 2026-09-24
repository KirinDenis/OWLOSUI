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
using CommanderApp = Commander.App;

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
            owl.Press(ConsoleKey.Enter); owl.Tick();
            Check(owl.Take().pressed == HW.CmOk, "Enter did not press the default button");
            owl.Press(ConsoleKey.Escape); owl.Tick();
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
            owl.Press(ConsoleKey.X, alt: true, ch: 'x'); owl.Tick();
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
            owl.Press(ConsoleKey.S, alt: true, ch: 's'); owl.Tick();
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
            owl.Press(ConsoleKey.Enter); owl.Tick();
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
            owl.Press(ConsoleKey.Escape); owl.Tick();
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

        // ------------------------------------------------------------ code pages

        Case("Notes: a Cyrillic file under code page 866 is shown, edited and saved intact", () =>
        {
            File.WriteAllText(notesFile, "Привет, мир!");
            using var owl = Owl(codePage: 866);
            var app = new NotesApp(owl, notesFile);
            Check(!app.ReadOnly, "866 has these letters; the file should open for editing");
            var f = owl.GetFrame();
            Check(f.Find("Привет") != null, "the letters are not on the screen", f);
            Check(f.Glyphs[0x80] == 'А', "the glyph table is not 866's");
            owl.Press(ConsoleKey.End);
            owl.Type(" Ёж");
            owl.Press(ConsoleKey.S, alt: true, ch: 's'); owl.Tick();
            app.OnCommand(owl.Take().pressed);
            Check(File.ReadAllText(notesFile) == "Привет, мир! Ёж",
                  $"the file holds '{File.ReadAllText(notesFile)}'");
        });

        Case("Notes: a Cyrillic file under code page 437 opens read-only and is never written", () =>
        {
            const string original = "Привет";
            File.WriteAllText(notesFile, original);
            using var owl = Owl();
            var app = new NotesApp(owl, notesFile);
            Check(app.ReadOnly, "437 cannot hold these letters; the file should be read-only");
            var f = owl.GetFrame();
            Check(f.Find("read-only") != null, "no word about it", f);
            owl.Press(ConsoleKey.Enter); owl.Tick();
            app.OnCommand(owl.Take().pressed); // OK on the message
            owl.Type("x");
            owl.Press(ConsoleKey.S, alt: true, ch: 's'); owl.Tick();
            app.OnCommand(owl.Take().pressed);
            Check(File.ReadAllText(notesFile) == original, "the file was written with ? in it");
            Check(owl.GetFrame().Find("read-only") != null, "Save did not say why it refused");
        });

        Case("Commander: a Cyrillic file name is shown and comes back intact under 866", () =>
        {
            var (l, r) = TempTree();
            const string name = "Отчёт.txt";
            File.WriteAllText(Path.Combine(l, name), "x");
            using var owl = Owl(codePage: 866);
            var app = new CommanderApp(owl, l, r);
            var f = owl.GetFrame();
            Check(f.Find(name) != null, "the Cyrillic name is not on the screen", f);
            Check(f.Find("?????") == null, "question marks where the name should be", f);
            // Copy it across by name: the name the panel gives back must be the real one.
            var (x, y) = Locate(f, name);
            owl.Click(x, y);
            app.OnCommand(CommanderApp.CmCopy);
            owl.Press(ConsoleKey.Y, alt: true, ch: 'y'); owl.Tick();
            app.OnCommand(owl.Take().pressed);
            Check(File.Exists(Path.Combine(r, name)), "the file was not copied under its own name");
        });

        // ----------------------------------------- the first five of the pieces

        Case("Controls: the status line shows its keys and binds them", () =>
        {
            using var owl = Owl();
            owl.StatusLine(new Owlosui.StatusItem("~F1~ Help", 5, ConsoleKey.F1),
                           new Owlosui.StatusItem("~Alt-X~ Exit", 6, ConsoleKey.X, Alt: true),
                           ("About", 7));
            var f = owl.GetFrame();
            Check(f.Row(24).StartsWith(" F1 Help  Alt-X Exit  About"), $"'{f.Row(24)}'", f);
            owl.Press(ConsoleKey.F1);
            Check(owl.Take().command == 5, "F1 did not become Help");
            owl.Press(ConsoleKey.X, alt: true, ch: 'x');
            Check(owl.Take().command == 6, "Alt+X did not become Exit");
            var (x, y) = Locate(f, "About");
            owl.Click(x + 1, y);
            Check(owl.Take().command == 7, "a click on the words did nothing");
        });

        Case("Controls: a label focuses its field, a list takes marks, a bar fills", () =>
        {
            using var owl = Owl();
            var w = owl.Window("Pick", 60, 20, style: Owlosui.Style.Dialog);
            var name = owl.Input(w, 8, 1, 30, "");
            var list = owl.List(w, 2, 4, 20, 5, new[] { "one", "two", "three" }, multi: true);
            owl.Label(w, 2, 1, "~N~ame:", name);
            var bar = owl.Progress(w, 2, 12, 25);
            owl.GetFrame();

            owl.Press(ConsoleKey.Tab);
            owl.Press(ConsoleKey.Insert);
            owl.Press(ConsoleKey.Insert);
            Check(owl.Marked(list).SequenceEqual(new[] { 0, 1 }), $"marked {string.Join(",", owl.Marked(list))}");
            Check(owl.Current(list) == 2, "Insert did not step down");

            owl.Press(ConsoleKey.N, alt: true, ch: 'n');
            var f = owl.GetFrame();
            var wr = f.Find("Name:")!.Value;
            Check(f.CursorY == wr.y, "Alt+N did not put the caret in the field", f);

            owl.SetProgress(bar, 75);
            var g = owl.GetFrame();
            var (bx, by) = (wr.x, wr.y + 11);
            var line = g.Row(by).Substring(bx, 25);
            Check(line.EndsWith(" 75%"), $"'{line}'", g);
            Check(line.StartsWith(new string('█', 15)), $"'{line}'", g);
        });

        // ----------------------------------------------------------- Commander

        Case("Commander: two panels, Tab between them, Enter into a folder and back", () =>
        {
            var (l, r) = TempTree();
            using var owl = Owl();
            var app = new CommanderApp(owl, l, r);
            var f = owl.GetFrame();
            Check(f.Find("a.txt") != null && f.Find("sub") != null, "the left panel is not showing its folder", f);
            Check(f.Row(24).Contains("F5 Copy"), "no status line", f);
            Check(app.Active == app.Left, "the left panel should start active");

            owl.Press(ConsoleKey.Tab);
            var (_, cmd) = owl.Take();
            Check(cmd == CommanderApp.CmSwitch, "Tab did not become Switch");
            app.OnCommand(cmd);
            Check(app.Active == app.Right, "Tab did not switch panels");
            app.OnCommand(CommanderApp.CmSwitch);
            Check(app.Active == app.Left, "Tab did not switch back");

            // `..` is first, `sub` next: Down, Enter, and we are inside.
            owl.Press(ConsoleKey.DownArrow);
            owl.Press(ConsoleKey.Enter);
            app.Poll();
            Check(app.Left.Dir.EndsWith("sub"), $"Enter on the folder did not go in: {app.Left.Dir}");
            var g = owl.GetFrame();
            Check(g.Find("inner.txt") != null, "the folder's contents are not shown", g);
            owl.Press(ConsoleKey.Enter); // `..`
            app.Poll();
            Check(app.Left.Dir == l, $"`..` did not come back out: {app.Left.Dir}");
        });

        Case("Commander: no path line above the names, and the foot says where you are", () =>
        {
            var (l, r) = TempTree();
            using var owl = Owl();
            _ = new CommanderApp(owl, l, r);
            var f = owl.GetFrame();
            // Row 1 is the first row inside the left window: a name, not `*.*`.
            Check(!f.Row(1).Contains("*.*"), "a commander's panel should not show the mask line", f);
            Check(f.Row(1).Contains(".."), "the names should start on the first row", f);
            var foot = string.Join("\n", Enumerable.Range(0, f.H).Select(y => f.Row(y).Substring(0, 40)));
            Check(foot.Contains(Path.GetFileName(l)), "the foot does not say where the panel is", f);
        });

        Case("Files: a panel with a path line goes where the path says, and filters by its mask", () =>
        {
            // The Open-dialog shape of the panel: a path line on top. Typing a
            // folder and a mask there is the way to move; a folder that is not
            // there leaves the listing alone and says so.
            var (l, r) = TempTree();
            using var owl = Owl();
            var w = owl.Window("Open", 60, 20, style: Owlosui.Style.Dialog);
            var files = owl.Files(w, l, Owlosui.ReadDirectory(l));
            var pr = owl.GetFrame();
            var (px, py) = Locate(pr, "Path:");
            owl.Click(px + 8, py);
            owl.Press(ConsoleKey.End);
            for (var i = 0; i < 200; i++) owl.Press(ConsoleKey.Backspace);
            owl.Type(Path.Combine(r, "*.log"));
            owl.Press(ConsoleKey.Enter);
            var (kind, text) = owl.TakeFiles(files);
            Check(kind == Owlosui.FilesEvent.Path && text.EndsWith("*.log"), $"the panel reported {kind} '{text}'");
            owl.SetFiles(files, r, Owlosui.ReadDirectory(r), "*.log");
            var g = owl.GetFrame();
            Check(g.Find("notes.log") != null, "the .log file is not shown", g);
            Check(g.Find("other.txt") == null, "the mask did not filter", g);

            owl.SetFilesError(files, "Folder not found: " + Path.Combine(r, "nowhere"));
            var h = owl.GetFrame();
            Check(h.Find("Folder not found") != null, "no message for a missing folder", h);
            Check(h.Find("notes.log") != null, "the old listing was thrown away", h);
        });

        Case("Commander: Insert marks, F5 copies across, F8 deletes after asking", () =>
        {
            var (l, r) = TempTree();
            using var owl = Owl();
            var app = new CommanderApp(owl, l, r);
            owl.GetFrame();
            // Past `..` and `sub` to the files; mark both.
            owl.Press(ConsoleKey.DownArrow);
            owl.Press(ConsoleKey.DownArrow);
            owl.Press(ConsoleKey.Insert);
            owl.Press(ConsoleKey.Insert);
            owl.Press(ConsoleKey.F5);
            var (_, copy) = owl.Take();
            Check(copy == CommanderApp.CmCopy, "F5 did not become Copy");
            app.OnCommand(copy);
            var f = owl.GetFrame();
            Check(f.Find("Copy 2 file(s)") != null, "no question before copying", f);

            // Enter is No: nothing happens.
            owl.Press(ConsoleKey.Enter); owl.Tick();
            var (no, _) = owl.Take();
            Check(no == CommanderApp.CmNo, $"Enter pressed {no}, not No");
            app.OnCommand(no);
            Check(!File.Exists(Path.Combine(r, "a.txt")), "No copied anyway");

            // Ask again, Alt+Y: copied, and the right panel shows them.
            app.OnCommand(CommanderApp.CmCopy);
            owl.Press(ConsoleKey.Y, alt: true, ch: 'y'); owl.Tick();
            var (yes, _) = owl.Take();
            Check(yes == CommanderApp.CmYes, $"Alt+Y pressed {yes}, not Yes");
            app.OnCommand(yes);
            Check(File.Exists(Path.Combine(r, "a.txt")) && File.Exists(Path.Combine(r, "b.txt")), "the files were not copied");
            var g = owl.GetFrame();
            Check(g.Find("Copy") == null || g.Find("Confirm") == null, "the progress or question window is still up", g);

            // Over on the right, delete one of them.
            app.OnCommand(CommanderApp.CmSwitch);
            owl.Press(ConsoleKey.DownArrow); // past `..` to a.txt
            owl.Press(ConsoleKey.F8);
            app.OnCommand(owl.Take().command);
            Check(owl.GetFrame().Find("Delete 1 file(s)") != null, "no question before deleting");
            owl.Press(ConsoleKey.Y, alt: true, ch: 'y'); owl.Tick();
            app.OnCommand(owl.Take().pressed);
            Check(!File.Exists(Path.Combine(r, "a.txt")), "the file was not deleted");
            Check(File.Exists(Path.Combine(r, "b.txt")), "the wrong file was deleted");
            // The right half of the screen is the right panel.
            var last = owl.GetFrame();
            var rightHalf = string.Join("\n", Enumerable.Range(0, last.H).Select(y => last.Row(y).Substring(40)));
            Check(!rightHalf.Contains("a.txt"), "the right panel still lists the deleted file", last);
            Check(rightHalf.Contains("b.txt"), "the right panel lost the file that stayed", last);
        });

        Case("Commander: F3 views a file, F4 edits it, F7 makes a folder, F6 moves", () =>
        {
            var (l, r) = TempTree();
            using var owl = Owl();
            var app = new CommanderApp(owl, l, r);
            owl.GetFrame();
            owl.Press(ConsoleKey.DownArrow);
            owl.Press(ConsoleKey.DownArrow); // a.txt

            // F3: a cyan viewer with the file's words; Escape closes it.
            owl.Press(ConsoleKey.F3);
            app.OnCommand(owl.Take().command);
            var f = owl.GetFrame();
            Check(f.Find("aaa") != null, "the viewer does not show the file", f);
            owl.Press(ConsoleKey.Escape); owl.Tick();
            app.OnCommand(owl.Take().pressed);
            Check(owl.GetFrame().Find("aaa") == null, "the viewer did not close");

            // F4: an editor; type, Alt+S, and the file has changed.
            owl.Press(ConsoleKey.F4);
            app.OnCommand(owl.Take().command);
            owl.Press(ConsoleKey.End);
            owl.Type("+");
            owl.Press(ConsoleKey.S, alt: true, ch: 's'); owl.Tick();
            app.OnCommand(owl.Take().pressed);
            Check(File.ReadAllText(Path.Combine(l, "a.txt")) == "aaa+", "the edit was not saved");

            // F7: a name, Enter, and the folder exists on the active side.
            owl.Press(ConsoleKey.F7);
            app.OnCommand(owl.Take().command);
            owl.Type("made");
            owl.Press(ConsoleKey.Enter); owl.Tick();
            app.OnCommand(owl.Take().pressed);
            Check(Directory.Exists(Path.Combine(l, "made")), "F7 did not make the folder");
            Check(owl.GetFrame().Find("made") != null, "the new folder is not listed");

            // F6 on b.txt: gone from the left, present on the right.
            owl.Press(ConsoleKey.Home);
            owl.Press(ConsoleKey.DownArrow);
            owl.Press(ConsoleKey.DownArrow);
            owl.Press(ConsoleKey.DownArrow); // .., made, sub, then a.txt... find b.txt by marks instead
            var g = owl.GetFrame();
            var (bx, by) = Locate(g, "b.txt");
            owl.Click(bx, by);
            owl.Press(ConsoleKey.F6);
            app.OnCommand(owl.Take().command);
            Check(owl.GetFrame().Find("Move 1 file(s)") != null, "no question before moving");
            owl.Press(ConsoleKey.Y, alt: true, ch: 'y'); owl.Tick();
            app.OnCommand(owl.Take().pressed);
            Check(!File.Exists(Path.Combine(l, "b.txt")) && File.Exists(Path.Combine(r, "b.txt")), "the file was not moved");
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

    // A key that presses a button puts it down first and fires on the tick;
    // `Run` waits 90 ms between the two so the press is seen. A test has no
    // eyes, so it ticks at once - that is the `owl.Tick()` after every Press.
    private static Owlosui Owl(int codePage = 437) => new(width: 80, height: 25, codePage: codePage);

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

    /// <summary>
    /// Two fresh folders for the Commander: the left with two files and a
    /// sub-folder holding one more, the right with a .log and a .txt.
    /// </summary>
    private static (string left, string right) TempTree()
    {
        var root = Path.Combine(Path.GetTempPath(), "owlosui-cmd-" + Guid.NewGuid().ToString("N")[..8]);
        var l = Path.Combine(root, "L");
        var r = Path.Combine(root, "R");
        Directory.CreateDirectory(Path.Combine(l, "sub"));
        Directory.CreateDirectory(r);
        File.WriteAllText(Path.Combine(l, "a.txt"), "aaa");
        File.WriteAllText(Path.Combine(l, "b.txt"), "bbb");
        File.WriteAllText(Path.Combine(l, "sub", "inner.txt"), "in");
        File.WriteAllText(Path.Combine(r, "notes.log"), "log");
        File.WriteAllText(Path.Combine(r, "other.txt"), "txt");
        return (l, r);
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
