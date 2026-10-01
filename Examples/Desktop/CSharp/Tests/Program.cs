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
        // The list a classic DOS manual gives for a window: move, resize,
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

        Case("Notes: Russian and Slovak in one file, on a 437 session - the font grows", () =>
        {
            // Neither alphabet is on code page 437. A DOS screen would show
            // `?`; here the font takes each new letter as it comes, so the
            // file is shown whole, edited, and saved exactly as it was.
            const string original = "Привет, ľudía! čuž";
            File.WriteAllText(notesFile, original);
            using var owl = Owl();
            var app = new NotesApp(owl, notesFile);
            Check(owl.FontGrows, "the server's font should be the growing kind");
            Check(!app.ReadOnly, "a growing font holds anything; the file should open for editing");
            var f = owl.GetFrame();
            Check(f.Find(original) != null, "the mixed line is not on the screen whole", f);
            Check(owl.Glyphs.Length > 256, $"the font did not grow: {owl.Glyphs.Length} glyphs");
            owl.Press(ConsoleKey.End);
            owl.Type(" Жť");
            owl.Press(ConsoleKey.S, alt: true, ch: 's'); owl.Tick();
            app.OnCommand(owl.Take().pressed);
            Check(File.ReadAllText(notesFile) == original + " Жť", $"the file holds '{File.ReadAllText(notesFile)}'");
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
            // Inside, the cursor starts on the first name, not on `..`.
            var at = owl.MarkedNames(app.Left.Files);
            Check(at.Length == 1 && at[0] == "inner.txt", $"the cursor is not on the first name: {string.Join(",", at)}");
            owl.Press(ConsoleKey.Home);
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

        Case("Commander: a double click on a file shows its properties, on a folder enters it", () =>
        {
            var (l, r) = TempTree();
            using var owl = Owl();
            var app = new CommanderApp(owl, l, r);
            var f = owl.GetFrame();
            var (ax, ay) = Locate(f, "a.txt");
            owl.DoubleClick(ax + 1, ay);
            app.Poll();
            var g = owl.GetFrame();
            Check(g.Find(" Properties [modal] ") != null, "no properties dialog", g);
            Check(g.Find("a.txt") != null && g.Find("Size:") != null && g.Find("3 bytes") != null, "the size is not there", g);
            Check(g.Find("Modified:") != null && g.Find("Attributes:") != null && g.Find("Folder:") != null, "dates and attributes are not there", g);
            // Enter is OK: the dialog goes.
            owl.Press(ConsoleKey.Enter); owl.Tick();
            app.OnCommand(owl.Take().pressed);
            Check(owl.GetFrame().Find("Properties") == null, "OK did not close the dialog", owl.GetFrame());
            // A folder: entered.
            var (sx, sy) = Locate(owl.GetFrame(), "sub");
            owl.DoubleClick(sx + 1, sy);
            app.Poll();
            Check(owl.GetFrame().Find("inner.txt") != null, "the double click did not enter the folder", owl.GetFrame());
        });

        Case("Notes: Find finds and finds again, Replace all replaces", () =>
        {
            File.WriteAllText(notesFile, "the cat sat\non the mat\nthe end\n");
            using var owl = Owl();
            var app = new NotesApp(owl, notesFile);
            var f = owl.GetFrame();
            Check(f.Find(" Find ") != null && f.Find(" Replace ") != null, "no Find and Replace buttons", f);
            void Hotkey(char c) { owl.Press((ConsoleKey)char.ToUpper(c), alt: true, ch: c); owl.Tick(); app.OnCommand(owl.Take().pressed); }
            Hotkey('f');
            var g = owl.GetFrame();
            Check(g.Find(" Find [modal] ") != null && g.Find("Case sensitive") != null, "no Find dialog", g);
            owl.Type("the");
            owl.Press(ConsoleKey.Enter); owl.Tick(); app.OnCommand(owl.Take().pressed);
            Check(app.LastPattern == "the" && owl.GetFrame().Find("Find [modal]") == null, "OK did not take the pattern", owl.GetFrame());
            // Find again, twice, with the remembered pattern: the third "the";
            // a fourth time says not found.
            for (var k = 0; k < 3; k++)
            {
                Hotkey('f');
                owl.Press(ConsoleKey.Enter); owl.Tick(); app.OnCommand(owl.Take().pressed);
            }
            var h = owl.GetFrame();
            Check(h.Find("not found") != null, "the end of the text did not say so", h);
            owl.Press(ConsoleKey.Enter); owl.Tick(); app.OnCommand(owl.Take().pressed);

            // Replace all "the" with "a".
            Hotkey('r');
            var i = owl.GetFrame();
            Check(i.Find(" Replace [modal] ") != null && i.Find("Replace with:") != null, "no Replace dialog", i);
            // The Find field remembers "the"; Tab to the second field and type.
            owl.Press(ConsoleKey.Tab);
            owl.Type("a");
            owl.Press(ConsoleKey.A, alt: true, ch: 'a'); owl.Tick(); app.OnCommand(owl.Take().pressed);
            var j = owl.GetFrame();
            Check(j.Find("3 replaced") != null, "Replace all did not say how many", j);
            owl.Press(ConsoleKey.Enter); owl.Tick(); app.OnCommand(owl.Take().pressed);
            Check(owl.GetText(app.Editor).StartsWith("a cat sat\non a mat\na end"), $"replaced wrongly: {owl.GetText(app.Editor)}");
            File.Delete(notesFile);
        });

        Case("Notes: a double click on the title bar zooms the window and back", () =>
        {
            File.Delete(notesFile);
            using var owl = Owl();
            _ = new NotesApp(owl, notesFile);
            var f = owl.GetFrame();
            var (tx, ty) = Locate(f, " OWLOSUI-TEST.TXT ");
            owl.DoubleClick(tx + 2, ty);
            var g = owl.GetFrame();
            Check(IsCorner(g, 79, 24), "the double click did not zoom", g);
            var (ux, uy) = Locate(g, " OWLOSUI-TEST.TXT ");
            owl.DoubleClick(ux + 2, uy);
            var h = owl.GetFrame();
            Check(!IsCorner(h, 79, 24) && Locate(h, " OWLOSUI-TEST.TXT ") == (tx, ty), "the second double click did not put it back", h);
        });

        Case("Commander: Alt+F10 shows the drive as a tree the other panel follows; Alt+F1 picks a drive", () =>
        {
            var (l, r) = TempTree();
            using var owl = Owl();
            var app = new CommanderApp(owl, l, r);
            owl.GetFrame();
            // The left panel becomes a tree of its drive, filled as it opens.
            owl.Press(ConsoleKey.F10, alt: true);
            app.OnCommand(owl.Take().command);
            app.Poll();
            var f = owl.GetFrame();
            var rootName = Path.GetPathRoot(l)!.TrimEnd('\\');
            Check(app.Left.Tree != 0 && f.Find(rootName) != null, "no tree", f);
            Check(f.Find("a.txt") == null, "the files are still there", f);
            // The other panel follows the cursor: on the root now.
            Check(app.Right.Dir == Path.GetPathRoot(l), $"the right panel did not follow: {app.Right.Dir}");
            // Down: the first folder of the drive; the right panel goes there.
            owl.Press(ConsoleKey.DownArrow); app.Poll();
            var path = owl.TreePath(app.Left.Tree);
            Check(path.Length == 2, $"Down did not land on a folder: {string.Join("\\", path)}");
            Check(app.Right.Dir == Path.Combine(Path.GetPathRoot(l)!, path[1]), $"the right panel is at {app.Right.Dir}");
            // Right opens it: the program is asked for its subfolders and answers.
            owl.Press(ConsoleKey.RightArrow); app.Poll();
            Check(owl.TreeExpand(app.Left.Tree) == null, "the expansion was not answered");
            // Alt+F10 again: files, in the folder the cursor was on.
            owl.Press(ConsoleKey.F10, alt: true);
            app.OnCommand(owl.Take().command);
            Check(app.Left.Tree == 0 && app.Left.Files != 0 && app.Left.Dir == app.Right.Dir, $"back to files at {app.Left.Dir}");

            // Alt+F1: the drive dialog, modal, with this drive in it.
            owl.Press(ConsoleKey.F1, alt: true);
            app.OnCommand(owl.Take().command);
            var g = owl.GetFrame();
            Check(g.Find(" Drive [modal] ") != null && g.Find(rootName) != null, "no drive dialog", g);
            owl.Press(ConsoleKey.Escape); owl.Tick(); app.OnCommand(owl.Take().pressed);
            Check(owl.GetFrame().Find("Drive [modal]") == null, "Cancel did not close the drive dialog", owl.GetFrame());
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

        // ------------------------------------------------------------- Sokoban

        Case("Sokoban: the list of levels, Enter to play, an arrow to move", () =>
        {
            using var owl = Owl();
            var app = new BaseZ47.App(owl);
            var f = owl.GetFrame();
            Check(f.Find("Level  1") != null && f.Find("BASE-Z 47") != null, "no level list", f);

            // Enter presses Play, the default button.
            owl.Press(ConsoleKey.Enter); owl.Tick();
            var (play, _) = owl.Take();
            Check(play != 0, "Enter did not press Play");
            app.OnCommand(play);
            var g = owl.GetFrame();
            Check(g.Find("Level 1 moves 0") != null, "the first level did not open", g);
            // The board is a canvas in the original game's colours: periwinkle
            // walls, pale green floor, an orange square for a mark - and the
            // nothing outside the walls is black, not floor.
            int Count(byte attr) => Enumerable.Range(0, g.H).Sum(y => Enumerable.Range(0, g.W).Count(x => g.Attr(x, y) == attr));
            Check(Count(BaseZ47.Warehouse.WallAttr) > 0, "no walls in the wall colour", g);
            Check(Count(BaseZ47.Warehouse.FloorAttr) > 0, "no floor in the floor colour", g);
            Check(Count(BaseZ47.Warehouse.MarkAttr) > 0, "no marks", g);
            Check(Count(BaseZ47.Warehouse.KeeperAttr) == 2, "the keeper is not exactly one cell wide", g);
            // Outside the walls the window shows through: no black block,
            // and no grey bar from the caption either.
            Check(Count(Owlosui.Attr(ConsoleColor.Black, ConsoleColor.Black)) == 0, "the space outside the walls is painted black", g);
            var (cx, cy) = Locate(g, "moves 0");
            Check(g.Attr(cx, cy) == 0x1E, $"the caption should be the window's text colour, not {g.Attr(cx, cy):X2}", g);
            // The caption's row is blue from edge to edge - the only grey on
            // the screen is the status line at the bottom.
            Check(Enumerable.Range(1, g.W - 2).All(x => g.Attr(x, cy) != Owlosui.Attr(ConsoleColor.Black, ConsoleColor.Gray)), "a grey bar across a blue window", g);

            // The arrows are bound by the status line; one of them is a
            // move on any level, so try each until the count changes.
            var moved = false;
            foreach (var key in new[] { ConsoleKey.RightArrow, ConsoleKey.DownArrow, ConsoleKey.LeftArrow, ConsoleKey.UpArrow })
            {
                owl.Press(key);
                var (_, cmd) = owl.Take();
                Check(cmd != 0, $"{key} is not bound");
                app.OnCommand(cmd);
                if (owl.GetFrame().Find("moves 1") != null) { moved = true; break; }
            }
            Check(moved, "no arrow moved the keeper", owl.GetFrame());

            // Esc goes back to the list.
            owl.Press(ConsoleKey.Escape);
            var (_, esc) = owl.Take();
            app.OnCommand(esc);
            Check(owl.GetFrame().Find("Level  1") != null, "Escape did not return to the list", owl.GetFrame());
        });

        // ---------------------------------------------------------------- Demo

        Case("Demo: the menu bar opens, and Tools > Calculator opens a calculator that calculates", () =>
        {
            using var owl = Owl();
            var app = new OwlosDemo.App(owl);
            var f = owl.GetFrame();
            Check(f.Row(0).Contains("File") && f.Row(0).Contains("Tools") && f.Row(0).Contains("Help"), "no menu bar", f);
            Check(f.Row(24).Contains("F1 Help"), "no status line", f);

            // Alt+T opens Tools; Enter chooses the first item, Calculator.
            owl.Press(ConsoleKey.T, alt: true, ch: 't');
            Check(owl.GetFrame().Find("Calculator") != null, "Alt+T did not open Tools", owl.GetFrame());
            owl.Press(ConsoleKey.Enter);
            var (_, cmd) = owl.Take();
            Check(cmd == OwlosDemo.App.CmCalc, $"Enter chose {cmd}, not Calculator");
            app.OnCommand(cmd);
            var g = owl.GetFrame();
            Check(g.Find(" Calculator ") != null, "no calculator window", g);

            // Type an expression, Enter presses "=", the display shows the answer.
            owl.Type("12+34*2");
            owl.Press(ConsoleKey.Enter); owl.Tick();
            app.OnCommand(owl.Take().pressed);
            Check(app.Calc.Display == "80", $"12+34*2 with precedence is 80, not {app.Calc.Display}", owl.GetFrame());
        });

        Case("Calculator: the keypad is buttons - C red and top left, a click keeps the caret in the display", () =>
        {
            using var owl = Owl();
            var app = new OwlosDemo.App(owl);
            app.OnCommand(OwlosDemo.App.CmCalc);
            var f = owl.GetFrame();

            // C is the first key of the first row, and it is white on red.
            var (cx, cy) = Locate(f, "  C  ");
            var (sevenX, sevenY) = Locate(f, "  7  ");
            Check(cy < sevenY && cx <= sevenX, "C is not above the digits at the left", f);
            Check(f.Attr(cx + 2, cy) == Owlosui.Attr(ConsoleColor.White, ConsoleColor.DarkRed), $"C is {f.Attr(cx + 2, cy):X2}, not white on red", f);
            // A digit is green, an operator cyan: the eye tells them apart.
            var (plusX, plusY) = Locate(f, "  +  ");
            Check(f.Attr(sevenX + 2, sevenY) == Owlosui.Attr(ConsoleColor.Black, ConsoleColor.DarkGreen), "7 is not black on green", f);
            Check(f.Attr(plusX + 2, plusY) == Owlosui.Attr(ConsoleColor.Black, ConsoleColor.DarkCyan), "+ is not black on cyan", f);
            // Hex digits are disabled in decimal: grey.
            var (ax, ay) = Locate(f, "  A  ");
            Check(f.Attr(ax + 2, ay) == Owlosui.Attr(ConsoleColor.DarkGray, ConsoleColor.Gray), "A is not disabled in decimal", f);

            // The palette's colours are the IBM sixteen: its Red is what
            // ConsoleColor calls DarkRed, and so on down the list.
            // Click 7, +, then type 8, click =, and it is 15: the click did
            // not take the caret away from the display.
            ushort Press(int x, int y) { owl.Click(x, y); var (p, _) = owl.Take(); return p; }
            app.OnCommand(Press(sevenX + 2, sevenY));
            app.OnCommand(Press(plusX + 2, plusY));
            owl.Type("8");
            var (eqX, eqY) = Locate(f, "  =  ");
            app.OnCommand(Press(eqX + 2, eqY));
            Check(app.Calc.Display == "15", $"7 + 8 = gave '{app.Calc.Display}'", owl.GetFrame());
            // Typing after an answer starts over; C clears.
            owl.Type("2");
            Check(app.Calc.Display == "152", "typing appends to the answer", owl.GetFrame());
            app.OnCommand(Press(cx + 2, cy));
            Check(app.Calc.Display == "", "C did not clear", owl.GetFrame());
            // Enter presses = from the display: the default is in a placed row.
            owl.Type("2^10");
            owl.Press(ConsoleKey.Enter); owl.Tick();
            app.OnCommand(owl.Take().pressed);
            Check(app.Calc.Display == "1024", $"Enter did not press =: '{app.Calc.Display}'", owl.GetFrame());
            // Escape is C.
            owl.Press(ConsoleKey.Escape); owl.Tick(); app.OnCommand(owl.Take().pressed);
            Check(app.Calc.Display == "", $"Escape did not clear: '{app.Calc.Display}'", owl.GetFrame());
            // Tab does not wander into the keypad: one Tab lands on the
            // modes, the next is back on the display, and the digit typed
            // there lands in it.
            owl.Press(ConsoleKey.Tab); owl.Press(ConsoleKey.Tab); owl.Press(ConsoleKey.Tab);
            owl.Type("5");
            Check(app.Calc.Display == "5", $"after Tab, Tab, Tab the 5 went astray: '{app.Calc.Display}'", owl.GetFrame());

            // The operations make the right-hand column and the hex digits
            // stand beside the decimal ones.
            var h = owl.GetFrame();
            var (slashX, slashY) = Locate(h, "  /  ");
            var (bX, bY) = Locate(h, "  B  ");
            var (nineX, _) = Locate(h, "  9  ");
            Check(slashY == sevenY && slashX > bX && bX > nineX, "7 8 9 A B / is not the order of the row", h);
            var (starX, _) = Locate(h, "  *  ");
            var (minusX, _) = Locate(h, "  -  ");
            Check(starX == slashX && minusX == slashX && plusX == slashX, "the four operations are not one column", h);
            // A blank row between the title and the display.
            var (_, titleY) = Locate(h, " Calculator ");
            Check(h.Row(titleY + 1).Trim('\u2551', ' ', '\u2591').Length == 0, "the row under the title is not blank", h);
        });

        Case("Calculator: switching base converts the display and enables the digits of that base", () =>
        {
            using var owl = Owl();
            var app = new OwlosDemo.App(owl);
            app.OnCommand(OwlosDemo.App.CmCalc);
            owl.Type("255");
            // The Hex radio button.
            var f = owl.GetFrame();
            var (hx, hy) = Locate(f, "Hex");
            owl.Click(hx - 3, hy);
            app.Poll();
            Check(app.Calc.Display == "FF", $"255 in hex is FF, not '{app.Calc.Display}'", owl.GetFrame());
            // Choosing a mode hands the caret back: the next key typed is a digit of the number.
            owl.Type("0");
            Check(app.Calc.Display == "FF0", $"after choosing Hex, typing went astray: '{app.Calc.Display}'", owl.GetFrame());
            app.OnCommand(100); owl.Type("FF");
            var g = owl.GetFrame();
            var (ax, ay) = Locate(g, "  A  ");
            Check(g.Attr(ax + 2, ay) == Owlosui.Attr(ConsoleColor.Black, ConsoleColor.DarkGreen), "A is not enabled in hex", g);
            var (dx, dy) = Locate(g, "  .  ");
            Check(g.Attr(dx + 2, dy) == Owlosui.Attr(ConsoleColor.DarkGray, ConsoleColor.Gray), "the point is not disabled in hex", g);
            // Click A, then =: FFA.
            owl.Click(ax + 2, ay); app.OnCommand(owl.Take().pressed);
            Check(app.Calc.Display == "FFA", $"A did not type: '{app.Calc.Display}'", owl.GetFrame());
            // Back to binary: FFA is 111111111010.
            var (bx, by) = Locate(g, "Bin");
            owl.Click(bx - 3, by);
            app.Poll();
            Check(app.Calc.Display == "111111111010", $"FFA in binary: '{app.Calc.Display}'", owl.GetFrame());

            // Arrows in radio buttons choose, as the classic ones did: the
            // click left the focus on Bin, and Down makes it Oct. The
            // display follows the dot, not the cursor.
            // The caret went back to the display when Bin was chosen, so
            // Tab first: the one stop after the display is the modes.
            owl.Press(ConsoleKey.Tab);
            owl.Press(ConsoleKey.DownArrow); app.Poll();
            Check(app.Calc.Display == "7772", $"Down did not choose Oct: '{app.Calc.Display}'", owl.GetFrame());
            owl.Press(ConsoleKey.Tab);
            owl.Press(ConsoleKey.UpArrow); app.Poll();
            Check(app.Calc.Display == "111111111010", "Up did not choose Bin again", owl.GetFrame());

            // Back to decimal, and the Radians box: sin(90) is 1 in degrees
            // and 0.89 in radians.
            var (decX, decY) = Locate(g, "Dec");
            owl.Click(decX - 3, decY); app.Poll();
            app.OnCommand(100); // C
            // A click on a radio button focuses it, as it should; the hand
            // then goes back to the display before typing.
            var (_, titleY) = Locate(g, " Calculator ");
            var displayAt = (x: g.Row(titleY).IndexOf('╔') + 2, y: titleY + 2);
            owl.Click(displayAt.x, displayAt.y);
            owl.Type("sin(90)");
            owl.Press(ConsoleKey.Enter); owl.Tick(); app.OnCommand(owl.Take().pressed);
            Check(app.Calc.Display == "1", $"sin(90) in degrees: '{app.Calc.Display}'", owl.GetFrame());
            var (rx, ry) = Locate(g, "Radians");
            owl.Click(rx - 3, ry); app.Poll();
            app.OnCommand(100);
            owl.Click(displayAt.x, displayAt.y);
            owl.Type("sin(90)");
            owl.Press(ConsoleKey.Enter); owl.Tick(); app.OnCommand(owl.Take().pressed);
            Check(app.Calc.Display == "0.893996663600558", $"sin(90) in radians: '{app.Calc.Display}'", owl.GetFrame());
        });

        Case("Calculator engine: precedence, functions, bases, memory and errors, with no window", () =>
        {
            var c = new OwlosDemo.Calculator.CalcEngine();
            void Is(string expr, string want)
            {
                var got = c.Evaluate(expr);
                Check(got == want, $"{expr} = {got}, wanted {want}");
            }
            Is("1+2*3", "7");
            Is("(1+2)*3", "9");
            Is("10/4", "2.5");
            Is("2^3^2", "512");
            Is("-3^2", "-9");
            Is("2^-1", "0.5");
            Is("17 mod 5", "2");
            Is("17%5", "2");
            Is("5!", "120");
            Is("sqrt(2)", "1.4142135623731");
            Is("sqrt(16)+1", "5");
            Is("abs(-4)", "4");
            Is("pi", "3.14159265358979");
            Is("e", "2.71828182845905");
            Is("ln(e)", "1");
            Is("log(1000)", "3");
            Is("exp(0)", "1");
            Is("1e", "Error");
            Is("0.1+0.2", "0.3");
            Is("1/3", "0.333333333333333");
            Is("2^70", "1.18059162071741E+21");

            // Degrees by default; radians when told.
            Is("sin(30)", "0.5");
            Is("cos(60)", "0.5");
            Is("tan(45)", "1");
            Is("sin(180)", "0");
            Is("asin(1)", "90");
            c.Degrees = false;
            Is("sin(pi/2)", "1");
            Is("cos(pi)", "-1");
            Is("atan(1)*4", "3.14159265358979");
            c.Degrees = true;

            // Errors are words.
            Is("1/0", "Divide by zero");
            Is("5 mod 0", "Divide by zero");
            Is("7-", "Error");
            Is("(1+2", "Error");
            Is("1 2", "Error");
            Is("foo(1)", "Error");
            Is("sqrt(-1)", "Invalid input");
            Is("ln(0)", "Invalid input");
            Is("(-1)!", "Invalid input");
            Is("2.5!", "Invalid input");
            Is("200!", "Overflow");
            Is("10^400", "Overflow");
            Is("", "");
            Is("   ", "");

            // Bases: read and written in the base, integers only.
            c.Base = OwlosDemo.Calculator.NumberBase.Hex;
            Is("FF+1", "100");
            Is("ff", "FF");
            Is("ace", "ACE");
            Is("10/4", "4");
            Is("-A", "-A");
            Is("cos(0)", "1");
            Is("1.5", "Invalid input");
            Is("FFFFFFFFFFFFFFFFFF", "Overflow");
            c.Base = OwlosDemo.Calculator.NumberBase.Bin;
            Is("1010+1", "1011");
            Is("2", "Error");
            c.Base = OwlosDemo.Calculator.NumberBase.Oct;
            Is("17+1", "20");
            Is("8", "Error");
            c.Base = OwlosDemo.Calculator.NumberBase.Dec;
            Check(c.IsDigit('9') && !c.IsDigit('A'), "decimal digits");
            c.Base = OwlosDemo.Calculator.NumberBase.Hex;
            Check(c.IsDigit('a') && c.IsDigit('F') && !c.IsDigit('G'), "hex digits");
            c.Base = OwlosDemo.Calculator.NumberBase.Dec;

            // Memory: a number the expression can name.
            c.Memory = 42;
            Is("m+1", "43");
            Is("M*2", "84");
            Check(c.HasMemory, "memory set");
            c.Memory = 0;
            Check(!c.HasMemory, "memory cleared");
            Is("m", "0");
        });

        Case("Demo: calendar, ASCII table and puzzle are canvases the mouse can use", () =>
        {
            using var owl = Owl();
            var app = new OwlosDemo.App(owl);
            app.OnCommand(OwlosDemo.App.CmCalendar);
            var f = owl.GetFrame();
            Check(f.Find(DateTime.Today.ToString("MMMM yyyy", System.Globalization.CultureInfo.InvariantCulture)) != null, "the calendar is not on this month", f);
            Check(f.Find("Su  Mo  Tu  We  Th  Fr  Sa") != null, "no day header", f);
            app.OnCommand(OwlosDemo.Calendar.CalendarWindow.CmNext);
            var next = new DateTime(DateTime.Today.Year, DateTime.Today.Month, 1).AddMonths(1);
            Check(owl.GetFrame().Find(next.ToString("MMMM yyyy", System.Globalization.CultureInfo.InvariantCulture)) != null, "> did not turn the page");
            // A click on the 15th chooses it and names it in full.
            var (sux, suy) = Locate(owl.GetFrame(), "Su  Mo");
            var slot = (int)next.DayOfWeek + 14;
            owl.Click(sux + (slot % 7) * 4 + 1, suy + 1 + (slot / 7) * 2); app.Poll();
            var fifteenth = next.AddDays(14).ToString("dddd, d MMMM yyyy", System.Globalization.CultureInfo.InvariantCulture);
            Check(owl.GetFrame().Find(fifteenth) != null, $"the click did not choose the 15th ({fifteenth})", owl.GetFrame());
            Check(app.Cal.Selected == next.AddDays(14), "Selected is not the 15th");
            // Today brings the month back.
            app.OnCommand(OwlosDemo.Calendar.CalendarWindow.CmToday);
            Check(owl.GetFrame().Find(DateTime.Today.ToString("dddd, d MMMM yyyy", System.Globalization.CultureInfo.InvariantCulture)) != null, "Today did not choose today", owl.GetFrame());

            app.OnCommand(OwlosDemo.App.CmAscii);
            var g = owl.GetFrame();
            Check(g.Find("Click a glyph.") != null, "no ASCII table", g);
            // Rows are headed 00, 10, ... ; 'A' is 0x41: the row headed 40, column 1.
            var (rx, ry) = Locate(g, "40 ");
            owl.Click(rx + 3 + 1 * 3 + 1, ry); app.Poll();
            Check(owl.GetFrame().Find("dec 65 hex 41 oct 101 bin 01000001") != null, "the click did not name glyph 65", owl.GetFrame());
            Check(app.Table.Picked == 65, "Picked is not 65");

            app.OnCommand(OwlosDemo.App.CmPuzzle);
            var h = owl.GetFrame();
            Check(h.Find("Moves: 0") != null, "no puzzle", h);
            var board = app.Game.Board;
            Check(!board.Solved, "a scrambled puzzle should not be solved");
            // Click the tile above the hole: it slides down and the count says so.
            var hole = board.Hole;
            var above = hole - 4;
            if (above < 0) above = hole + 4;
            var (px, py) = (above % 4, above / 4);
            var (gx, gy) = Locate(h, " Puzzle ");
            // The board sits five cells in and one row down inside the frame.
            var left = h.Row(gy).IndexOf('\u2554') + 1 + 5;
            owl.Click(left + px * 6 + 3, gy + 1 + 1 + py * 3 + 1); app.Poll();
            Check(board.Moves == 1, $"the click did not slide a tile (moves {board.Moves})", owl.GetFrame());
            Check(owl.GetFrame().Find("Moves: 1") != null, "the count did not follow", owl.GetFrame());
        });

        Case("Desktop: Window > Cascade and Tile arrange the windows, and a fixed dialog keeps its size", () =>
        {
            using var owl = Owl();
            var app = new OwlosDemo.App(owl);
            // One fixed dialog at the back, then three document windows.
            app.OnCommand(OwlosDemo.App.CmCalc);
            var before = Locate(owl.GetFrame(), " Calculator ");
            var a = owl.Window("Alpha", 30, 8, 5, 3);
            var b = owl.Window("Beta", 30, 8, 8, 5);
            var g = owl.Window("Gamma", 30, 8, 11, 7);

            app.OnCommand(OwlosDemo.App.CmTile);
            var f = owl.GetFrame();
            // Four windows: two columns of two. The calculator, fixed, stands
            // in the top-left cell at its own size; the documents fill theirs.
            Check(IsCorner(f, 39, 23) && IsCorner(f, 79, 11) && IsCorner(f, 79, 23), "the documents are not in their cells", f);
            // Beta, in the cell to the right, overlaps the dialog's title's
            // last space, so look for the word alone.
            var calc = Locate(f, "Calculator");
            Check(calc.y == 1 && calc.x < before.x, "Tile did not put the dialog in its cell", f);

            app.OnCommand(OwlosDemo.App.CmCascade);
            var h = owl.GetFrame();
            var cy = Locate(h, "Calculator").y;
            var ay = Locate(h, " Alpha ").y;
            var by = Locate(h, " Beta ").y;
            var gy = Locate(h, " Gamma ").y;
            Check(cy == 1 && ay == 2 && by == 3 && gy == 4, $"cascade titles at {cy},{ay},{by},{gy}, not 1,2,3,4", h);
            Check(IsCorner(h, 79, 23), "the front document reaches the work area's corner", h);
            // The menu has the verbs: Alt+W opens Window, and the items are there.
            owl.Press(ConsoleKey.W, alt: true, ch: 'w');
            var m = owl.GetFrame();
            Check(m.Find("Cascade") != null && m.Find("Tile") != null, "Window menu lacks Cascade / Tile", m);
            owl.Press(ConsoleKey.Escape);
            _ = (a, b, g);
        });

        Case("Demo: files open as viewers that carry F4 and F7 with them; F4 edits, F7 hex, closing asks", () =>
        {
            using var owl = Owl();
            var app = new OwlosDemo.App(owl);
            var dir = Path.GetTempPath();
            var one = Path.Combine(dir, "owlosui-one-" + Guid.NewGuid().ToString("N")[..8] + ".txt");
            var two = Path.Combine(dir, "owlosui-two-" + Guid.NewGuid().ToString("N")[..8] + ".txt");
            File.WriteAllText(one, "Hello\nworld\n");
            File.WriteAllText(two, "Second\n");
            try
            {
                // No file open: the status line has no F4, and F4 does nothing.
                Check(!owl.GetFrame().Row(24).Contains("F4"), "F4 on the status line with no file open", owl.GetFrame());
                owl.Press(ConsoleKey.F4);
                Check(owl.Take().command == 0, "F4 was bound with no file open");

                Check(app.Open(one) && app.Open(two), "the files did not open");
                Check(app.Docs.Count == 2, "two files, two windows");
                var f = owl.GetFrame();
                var n2 = Path.GetFileName(two);
                Check(f.Find($" {n2} [view] ") != null, "the second file is not a viewer in front", f);
                Check(f.Row(24).Contains("F4 Edit") && f.Row(24).Contains("F7 Hex"), "the file window did not bring its keys", f);
                // The Options menu got Edit / view and Hex, after a line.
                owl.Press(ConsoleKey.O, alt: true, ch: 'o');
                var m = owl.GetFrame();
                Check(m.Find("Edit / view") != null && m.Find("Hex") != null && m.Find("Mouse...") != null, "Options lacks the file's items", m);
                owl.Press(ConsoleKey.Escape);

                // F4 acts on the front file only: the second becomes an editor, the first stays a viewer.
                owl.Press(ConsoleKey.F4); app.OnCommand(owl.Take().command);
                var second = app.Docs[1];
                var first = app.Docs[0];
                Check(!second.ReadOnly && first.ReadOnly, "F4 did not edit the front file alone");
                owl.Type("X");
                Check(second.Changed && !first.Changed, "typing did not land in the front file");
                Check(owl.GetFrame().Find($" {n2} ") != null && owl.GetFrame().Find($" {n2} [view] ") == null, "the tag did not go", owl.GetFrame());

                // F7: the bytes in a window of their own, [hex]; its own key says F7 Text.
                owl.Press(ConsoleKey.F7); app.OnCommand(owl.Take().command);
                var g = owl.GetFrame();
                Check(second.HexId != 0 && g.Find($" {n2} [hex] ") != null, "no hex window", g);
                Check(g.Find("53 65 63 6F 6E 64") != null, "the bytes of Second are not shown", g);
                Check(g.Row(24).Contains("F7 Text") && !g.Row(24).Contains("F4 Edit"), "the hex window's keys are not its own", g);
                owl.Press(ConsoleKey.F7); app.OnCommand(owl.Take().command);
                Check(owl.Active() == second.Id, "F7 in the hex window did not go back to the text");

                // Behind another tool the keys are gone.
                app.OnCommand(OwlosDemo.App.CmCalc);
                Check(!owl.GetFrame().Row(24).Contains("F4"), "the calculator in front still shows F4", owl.GetFrame());
                app.OnCommand(OwlosDemo.App.CmClose);

                // Closing the edited file asks; No drops the change and the hex window.
                owl.Activate(second.Id);
                app.OnCommand(OwlosDemo.App.CmClose);
                var h = owl.GetFrame();
                Check(h.Find("Save changes to") != null && h.Find("[modal]") != null, "closing an edited file did not ask", h);
                owl.Press(ConsoleKey.N, alt: true, ch: 'n'); owl.Tick(); app.OnCommand(owl.Take().pressed);
                Check(app.Docs.Count == 1 && second.Gone, "No did not close the file");
                Check(File.ReadAllText(two) == "Second\n", "No must not write");
                // The first file is in front now, with its keys back; Yes writes.
                Check(owl.GetFrame().Row(24).Contains("F4 Edit"), "the other file's keys did not come back", owl.GetFrame());
                app.OnCommand(OwlosDemo.Editor.FileWindow.CmEditView);
                owl.Type("Z");
                app.OnCommand(OwlosDemo.App.CmClose);
                app.OnCommand(OwlosDemo.Editor.FileWindow.CmSaveYes);
                Check(app.Docs.Count == 0 && File.ReadAllText(one).StartsWith("ZHello"), $"Yes did not write: {File.ReadAllText(one)}");
            }
            finally { File.Delete(one); File.Delete(two); }
        });

        Case("Desktop: numbered windows, Alt+digit, Alt+0 list, Shift+F6 and Ctrl+F5 in the demo", () =>
        {
            using var owl = Owl();
            var app = new OwlosDemo.App(owl);
            app.OnCommand(OwlosDemo.App.CmCalc);
            app.OnCommand(OwlosDemo.App.CmCalendar);
            var f = owl.GetFrame();
            var (_, cy) = Locate(f, " Calendar ");
            Check(f.Row(cy).Contains('2'), "the calendar is not numbered 2", f);
            Check(owl.Active() == app.Calendar, "the calendar is in front");
            // Alt+1: the calculator, whose frame says 1.
            owl.Press(ConsoleKey.D1, alt: true, ch: '1');
            Check(owl.Active() == app.Calculator, "Alt+1 did not bring the calculator to the front");
            var g1 = owl.GetFrame();
            var (_, ky) = Locate(g1, " Calculator ");
            Check(g1.Row(ky).Contains('1'), "the calculator is not numbered 1", g1);
            // Shift+F6: back to the calendar.
            owl.Press(ConsoleKey.F6, shift: true);
            Check(owl.Active() == app.Calendar, "Shift+F6 did not bring the previous window");
            // Alt+0: the list, Down, Enter -> the calculator.
            owl.Press(ConsoleKey.D0, alt: true, ch: '0');
            var g = owl.GetFrame();
            Check(g.Find(" Windows [modal] ") != null && g.Find("1  Calculator") != null && g.Find("2  Calendar") != null, "no window list", g);
            owl.Press(ConsoleKey.DownArrow);
            owl.Press(ConsoleKey.Enter); owl.Tick();
            var (pressed, cmd) = owl.Take();
            Check(pressed == 0 && cmd == 0, "the list's OK reached the program");
            Check(owl.Active() == app.Calculator && owl.GetFrame().Find("Windows [modal]") == null, "the list did not pick the calculator", owl.GetFrame());
            // Ctrl+F5, Right, Right, Enter: the calculator moved two cells.
            var before = Locate(owl.GetFrame(), " Calculator ");
            owl.Press(ConsoleKey.F5, ctrl: true);
            owl.Press(ConsoleKey.RightArrow); owl.Press(ConsoleKey.RightArrow);
            owl.Press(ConsoleKey.Enter);
            var after = Locate(owl.GetFrame(), " Calculator ");
            Check(after == (before.x + 2, before.y), $"Ctrl+F5 did not move it: {before} -> {after}", owl.GetFrame());
            // And the menu has the verbs.
            owl.Press(ConsoleKey.W, alt: true, ch: 'w');
            var m = owl.GetFrame();
            Check(m.Find("Size/Move") != null && m.Find("Previous") != null && m.Find("List...") != null, "Window menu lacks the verbs", m);
            owl.Press(ConsoleKey.Escape);
        });

        Case("Demo: a menu item's hint takes the status line while the cursor is on it", () =>
        {
            using var owl = Owl();
            _ = new OwlosDemo.App(owl);
            owl.Press(ConsoleKey.F, alt: true, ch: 'f');
            var f = owl.GetFrame();
            Check(f.Row(24).Contains("Open a file in a window of its own") && !f.Row(24).Contains("F1 Help"), "no hint for Open", f);
            // Down skips the line and lands on Exit.
            owl.Press(ConsoleKey.DownArrow);
            Check(owl.GetFrame().Row(24).Contains("Leave the program"), "no hint for Exit", owl.GetFrame());
            owl.Press(ConsoleKey.Escape);
            Check(owl.GetFrame().Row(24).Contains("F1 Help"), "the keys did not come back", owl.GetFrame());
        });

        Case("Calculator: the display remembers what was evaluated, and Down lists it", () =>
        {
            using var owl = Owl();
            var app = new OwlosDemo.App(owl);
            app.OnCommand(OwlosDemo.App.CmCalc);
            owl.Type("2+2"); owl.Press(ConsoleKey.Enter); owl.Tick(); app.OnCommand(owl.Take().pressed);
            app.OnCommand(100); // C
            owl.Type("7*6"); owl.Press(ConsoleKey.Enter); owl.Tick(); app.OnCommand(owl.Take().pressed);
            Check(app.Calc.Display == "42", $"7*6: '{app.Calc.Display}'");
            var h = owl.GetHistory(app.Calc.DisplayId);
            Check(h.Length == 2 && h[0] == "7*6" && h[1] == "2+2", $"history: {string.Join("|", h)}");
            // Down lists them; Down, Enter puts 2+2 back on the display.
            owl.Press(ConsoleKey.DownArrow);
            Check(owl.GetFrame().Find("7*6") != null && owl.GetFrame().Find("2+2") != null, "no history panel", owl.GetFrame());
            owl.Press(ConsoleKey.DownArrow);
            owl.Press(ConsoleKey.Enter); owl.Tick(); owl.Take();
            Check(app.Calc.Display == "2+2", $"the pick did not fill the display: '{app.Calc.Display}'", owl.GetFrame());
        });

        Case("Demo: the Colors dialog changes a role live, and Cancel puts it back", () =>
        {
            using var owl = Owl();
            var app = new OwlosDemo.App(owl);
            var before = owl.GetFrame().Attr(2, 2); // a desktop cell
            app.OnCommand(OwlosDemo.App.CmColors);
            var f = owl.GetFrame();
            Check(f.Find(" Colors [modal] ") != null && f.Find("Desktop: desktop") != null && f.Find("Foreground") != null, "no colour dialog", f);
            // The first entry is the desktop; click the second background
            // cell of the grid (colour 1, blue) and the desktop turns blue.
            var (bx, by) = Locate(f, "Background");
            app.Poll();
            owl.Click(bx + 3 + 1, by + 1); app.Poll();
            var g = owl.GetFrame();
            Check((g.Attr(2, 2) >> 4) == 1, $"the desktop did not turn blue: {g.Attr(2, 2):X2}", g);
            Check(g.Find("Sample text") != null, "no sample", g);
            // Cancel: back as it was.
            owl.Press(ConsoleKey.Escape); owl.Tick(); app.OnCommand(owl.Take().pressed);
            var h = owl.GetFrame();
            Check(h.Attr(2, 2) == before && h.Find("Colors") == null, "Cancel did not put the colour back", h);
            // Again, and OK keeps it.
            app.OnCommand(OwlosDemo.App.CmColors);
            var i = owl.GetFrame();
            var (cx, cy) = Locate(i, "Background");
            app.Poll();
            owl.Click(cx + 3 + 1, cy + 1); app.Poll();
            owl.Press(ConsoleKey.Enter); owl.Tick(); app.OnCommand(owl.Take().pressed);
            Check((owl.GetFrame().Attr(2, 2) >> 4) == 1 && app.Palette.Id == 0, "OK did not keep the colour", owl.GetFrame());
        });

        Case("Demo: the Open dialog's Tree and Drive buttons", () =>
        {
            using var owl = Owl();
            var app = new OwlosDemo.App(owl);
            app.OnCommand(OwlosDemo.App.CmOpen);
            var f = owl.GetFrame();
            Check(f.Find(" Tree ") != null && f.Find(" Drive ") != null, "the Open dialog lacks Tree and Drive", f);
            // Alt+T: the panel becomes the tree of its drive.
            owl.Press(ConsoleKey.T, alt: true, ch: 't'); owl.Tick(); app.OnCommand(owl.Take().pressed);
            app.Poll();
            var root = Path.GetPathRoot(Directory.GetCurrentDirectory())!.TrimEnd('\\');
            var g = owl.GetFrame();
            Check(g.Find(root) != null && g.Find("Path:") == null, "no tree in the Open dialog", g);
            // Alt+D: the drive dialog over it; Escape.
            owl.Press(ConsoleKey.D, alt: true, ch: 'd'); owl.Tick(); app.OnCommand(owl.Take().pressed);
            var h = owl.GetFrame();
            Check(h.Find(" Drive [modal] ") != null && h.Find(root) != null, "no drive dialog", h);
            owl.Press(ConsoleKey.Escape); owl.Tick(); app.OnCommand(owl.Take().pressed);
            Check(owl.GetFrame().Find("Drive [modal]") == null, "Escape did not close the drive dialog", owl.GetFrame());
            // Alt+T again: files, at the root the cursor stood on.
            owl.Press(ConsoleKey.T, alt: true, ch: 't'); owl.Tick(); app.OnCommand(owl.Take().pressed);
            var i = owl.GetFrame();
            Check(i.Find("Path:") != null && i.Find(root + "\\*.*") != null, "Tree again did not go back to files at the root", i);
        });

        Case("Puzzle board: only a neighbour of the hole slides, and a scramble is solvable", () =>
        {
            var b = new OwlosDemo.Puzzle.Board();
            Check(b.Solved && b.Hole == 15 && b.Moves == 0, "a new board is solved with the hole last");
            Check(!b.Slide(0), "a far tile must not slide");
            Check(b.Slide(14), "the tile beside the hole slides");
            Check(b.Tiles[15] == 15 && b.Tiles[14] == 0 && b.Moves == 1, "it moved into the hole");
            Check(!b.Solved, "and the board is no longer solved");
            Check(b.Slide(15), "and back");
            Check(b.Solved && b.Moves == 2, "solved again, two moves");
            Check(!b.Slide(-1) && !b.Slide(16) && !b.Slide(b.Hole), "off the board and the hole itself do nothing");
            b.Scramble(seed: 1);
            Check(!b.Solved && b.Moves == 0, "scrambled, with the count at zero");
            var sum = b.Tiles.Sum();
            Check(sum == 120 && b.Tiles.Distinct().Count() == 16, "every tile is still there once");
            // Solvable: the same seed scrambles the same way, and the
            // slides can be undone in reverse order - so replay them.
            var c = new OwlosDemo.Puzzle.Board();
            var rnd = new Random(1);
            var path = new List<int>();
            for (var n = 0; n < 200; n++)
            {
                var hole = c.Hole;
                var (r, col) = (hole / 4, hole % 4);
                var moves = new List<int>();
                if (r > 0) moves.Add(hole - 4);
                if (r < 3) moves.Add(hole + 4);
                if (col > 0) moves.Add(hole - 1);
                if (col < 3) moves.Add(hole + 1);
                var at = moves[rnd.Next(moves.Count)];
                path.Add(c.Hole);
                c.Slide(at);
            }
            Check(c.Tiles.SequenceEqual(b.Tiles), "the seed did not scramble the same way");
            for (var i = path.Count - 1; i >= 0; i--) b.Slide(path[i]);
            Check(b.Solved, "undoing the scramble did not solve it");
        });

        Case("Demo: the Mouse dialog reads its boxes back, and Window > Next / Zoom work", () =>
        {
            using var owl = Owl();
            var app = new OwlosDemo.App(owl);
            app.OnCommand(OwlosDemo.App.CmCalc);
            app.OnCommand(OwlosDemo.App.CmOptions);
            var f = owl.GetFrame();
            Check(f.Find("Reverse buttons") != null && f.Find("Medium") != null, "no options dialog", f);
            // Space turns the first box on; Tab, Down, Space picks Medium; Enter is OK.
            owl.Press(ConsoleKey.Spacebar, ch: ' ');
            owl.Press(ConsoleKey.Tab);
            owl.Press(ConsoleKey.DownArrow);
            owl.Press(ConsoleKey.Spacebar, ch: ' ');
            owl.Press(ConsoleKey.Enter); owl.Tick();
            app.OnCommand(owl.Take().pressed);
            var g = owl.GetFrame();
            // The answer wraps in its box; look for its parts.
            Check(g.Find("Reverse buttons: on") != null && g.Find("Show cursor: off") != null && g.Find("medium.") != null, "the dialog's answer is wrong", g);
            app.OnCommand(OwlosDemo.App.CmDismiss);

            // Next puts the calculator behind; Zoom fills the desktop.
            app.OnCommand(OwlosDemo.App.CmCalendar);
            Check(owl.Active() == app.Calendar, "the newest window should be in front");
            app.OnCommand(OwlosDemo.App.CmNext);
            Check(owl.Active() == app.Calculator, "Next did not bring the other window forward");
            // The calculator is a dialog: fixed size, so Zoom leaves it alone
            // - a window whose parts are placed by hand has nothing to gain
            // from the whole screen. A document window would fill it.
            var before = owl.GetFrame();
            app.OnCommand(OwlosDemo.App.CmZoom);
            var after = owl.GetFrame();
            Check(Locate(before, " Calculator ") == Locate(after, " Calculator "), "Zoom moved a fixed dialog", after);
            Check(!IsCorner(after, 79, 23), "a dialog should not zoom to the whole desktop", after);
            app.OnCommand(OwlosDemo.App.CmClose);
            Check(app.Calculator == 0, "Close did not close the front window");
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

        // ------------------------------------------------------ the window

        WindowCases.Run();

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

    /// <summary>The first cell showing a character, if any.</summary>
    private static (int x, int y)? Find(Owlosui.Frame f, char ch)
    {
        for (var y = 0; y < f.H; y++)
            for (var x = 0; x < f.W; x++)
                if (f.Char(x, y) == ch) return (x, y);
        return null;
    }

    private static (int x, int y) Locate(Owlosui.Frame f, string text) =>
        f.Find(text) ?? throw new Exception($"'{text}' is not on the screen\n{f}");

    /// <summary>A bottom-right frame corner: ╝ on a double frame, ┘ on a single one or a resize grip.</summary>
    internal static bool IsCorner(Owlosui.Frame f, int x, int y) =>
        x < f.W && y < f.H && f.Glyph(x, y) is 0xBC or 0xD9;

    internal static int Failed => failed;
    internal static (string l, string r) MakeTree() => TempTree();
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
    private static readonly string DemoExe = Path.Combine(AppContext.BaseDirectory, "OwlosDemo.exe");
    private static readonly string CommanderExe = Path.Combine(AppContext.BaseDirectory, "Commander.exe");

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

        Tests.RunCase("Console: F10 opens the demo's menu bar, and Alt+F10 shows the Commander's tree", () =>
        {
            using (var a = ConsoleAgent.Start(DemoExe, "", workDir))
            {
                a.WaitFor(s => s.Find("F1 Help") != null);
                a.Key(ConsoleKey.F10);
                var f = a.WaitFor(s => s.Find("Open...") != null);
                Tests.Require(f.Find("Open...") != null, "F10 did not open the File menu\n" + a.Trace(), f);
                a.Key(ConsoleKey.Escape);
                a.Key(ConsoleKey.X, alt: true);
                Tests.Require(a.WaitForExit(), "Alt+X did not end the demo\n" + a.Trace());
            }
            var (l, r) = Tests.MakeTree();
            using (var a = ConsoleAgent.Start(CommanderExe, $"\"{l}\" \"{r}\"", workDir))
            {
                a.WaitFor(s => s.Find("F10 Quit") != null);
                var root = Path.GetPathRoot(l)!.TrimEnd('\\');
                a.Key(ConsoleKey.F10, alt: true);
                var f = a.WaitFor(s => s.Find("a.txt") == null && s.Find(root) != null);
                Tests.Require(f.Find("a.txt") == null, "Alt+F10 did not turn the panel into a tree\n" + a.Trace(), f);
                a.Key(ConsoleKey.F10, alt: true);
                var g = a.WaitFor(s => s.Find("F10 Quit") != null && s.Find("a.txt") == null);
                Tests.Require(g.Find("Quit") != null, "Alt+F10 again did not come back\n" + a.Trace(), g);
                a.Key(ConsoleKey.F10);
                Tests.Require(a.WaitForExit(), "F10 did not quit the Commander\n" + a.Trace());
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
