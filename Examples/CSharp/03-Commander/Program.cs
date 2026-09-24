// A two-panel file manager, in the shape of Norton Commander.
//
// Two file panels side by side, Tab between them, Insert to mark. F3 views
// the file under the cursor, F4 edits it, F5 copies the marked files (or the
// one under the cursor) to the other panel, F6 moves them, F7 makes a
// folder, F8 deletes, F10 quits. Enter on a folder goes into it and `..`
// comes back out; Enter on a file views it.
//
// What this shows that Notes did not:
//
//  * the file panel is on the wire, but the file *system* is not. The server
//    reads no directories: this program reads them (`Owlosui.ReadDirectory`)
//    and hands the listing over, and reads them again when the person has
//    gone somewhere else. On DOS the same code would be INT 21h; in a
//    browser, a fetch. The panel does not know or care;
//
//  * not everything a person does is a command. Enter on a name is not a
//    button; it is a thing the panel remembers until asked, and `Poll` asks
//    after every keystroke - that is the second argument to `Run`;
//
//  * the status line binds the keys. F5 is "copy" because the status line
//    says so, and it says so on the screen as well;
//
//  * a viewer and an editor are the same control with one flag, and both are
//    ordinary windows: drag them aside, resize them, zoom them.
//
// It is a class so that `Examples/CSharp/Tests` can run the same program in
// a temporary folder and press its keys by the wire.

using System.Text;

namespace Commander;

public sealed class App
{
    // Command numbers. They are the program's own - the core never learns
    // what any of them means - and the only rules are that they are distinct
    // and none is zero, which means "nothing happened". Two groups: what the
    // status line sends, and what the dialogs answer with.
    public const ushort CmSwitch = 1;
    public const ushort CmHelp = 2;
    public const ushort CmView = 3;
    public const ushort CmEdit = 4;
    public const ushort CmCopy = 5;
    public const ushort CmMove = 6;
    public const ushort CmMkdir = 7;
    public const ushort CmDelete = 8;
    public const ushort CmQuit = 9;

    // The dialogs' answers.
    public const ushort CmNo = 20;      // and OK on a message, and Cancel
    public const ushort CmYes = 21;
    public const ushort CmMkdirOk = 22;
    public const ushort CmSave = 23;
    public const ushort CmCloseEdit = 24;
    public const ushort CmCloseView = 25;

    private readonly Owlosui owl;

    /// <summary>One side: its window, its panel, and where it is looking.</summary>
    public sealed class Panel
    {
        public ushort Window;
        public ushort Files;
        public string Dir = "";
        public string Mask = "*.*";
    }

    public Panel Left { get; }
    public Panel Right { get; }

    // The question box, while one is up, and what Yes means this time. A
    // message box does not close itself: the program that asked closes it,
    // when it has heard the answer. `onYes` is the deferred half of a
    // question - "delete these" - kept until the person says so.
    private ushort box;
    private Action? onYes;
    // The name field of the Make-directory dialog, read when OK is pressed.
    private ushort askInput;

    // The editor and the viewer, while they are open.
    private ushort editor, editorText, viewer;
    private string editorPath = "";

    public App(Owlosui owl, string leftDir, string rightDir)
    {
        this.owl = owl;

        // The status line takes the bottom row and binds these keys. Tab is
        // bound here on purpose: inside a panel Tab would walk between the
        // path line and the names, and in a two-panel program Tab has meant
        // "the other panel" since 1986. It is bound without a label because
        // the row is full - and everybody knows.
        owl.StatusLine(new Owlosui.StatusItem("~F1~ Help", CmHelp, ConsoleKey.F1),
                       new Owlosui.StatusItem("~F3~ View", CmView, ConsoleKey.F3),
                       new Owlosui.StatusItem("~F4~ Edit", CmEdit, ConsoleKey.F4),
                       new Owlosui.StatusItem("~F5~ Copy", CmCopy, ConsoleKey.F5),
                       new Owlosui.StatusItem("~F6~ Move", CmMove, ConsoleKey.F6),
                       new Owlosui.StatusItem("~F7~ MkDir", CmMkdir, ConsoleKey.F7),
                       new Owlosui.StatusItem("~F8~ Del", CmDelete, ConsoleKey.F8),
                       new Owlosui.StatusItem("~F10~ Quit", CmQuit, ConsoleKey.F10),
                       new Owlosui.StatusItem("", CmSwitch, ConsoleKey.Tab));

        // Two windows, each half the screen, above the status line. They are
        // ordinary document windows - drag one aside if you like - and
        // closing either is quitting: one panel is not a thing this program
        // means.
        var w = owl.Width;
        var h = owl.Height - 1;
        Left = Open(0, 0, w / 2, h, leftDir);
        Right = Open(w / 2, 0, w - w / 2, h, rightDir);
        owl.Activate(Left.Window);
    }

    private Panel Open(int x, int y, int w, int h, string dir)
    {
        dir = Path.GetFullPath(dir);
        var p = new Panel { Dir = dir };
        p.Window = owl.Window(Fit(dir, w - 12), w, h, x, y, closeCmd: CmQuit);
        // The panel fills the window and marks are allowed: that is what
        // makes it a file manager rather than an Open dialog. And no path
        // line above the names: people who grew up on Norton read a `*.*`
        // at the top of a panel as something gone wrong. They navigate by
        // Enter, and the foot of the panel says where they are.
        p.Files = owl.Files(p.Window, dir, Owlosui.ReadDirectory(dir), p.Mask, multi: true, pathLine: false);
        return p;
    }

    /// <summary>
    /// The panel whose window is in front, and the other one. Asked of the
    /// core each time rather than remembered here: a click on the other
    /// panel activates it without telling this program, and a remembered
    /// answer would be wrong from that moment on.
    /// </summary>
    public Panel Active => owl.Active() == Right.Window ? Right : Left;
    public Panel Other => Active == Left ? Right : Left;

    /// <summary>True to keep running, false to leave.</summary>
    public bool OnCommand(ushort cmd)
    {
        switch (cmd)
        {
            case CmSwitch:
                // From a viewer or an editor, Tab is not "the other panel";
                // those windows have their own keys.
                if (editor == 0 && viewer == 0) owl.Activate(Other.Window);
                return true;

            case CmHelp:
                Tell("Enter opens the folder under the cursor, or views the file. Insert marks. " +
                     "F5, F6 and F8 use the marks, or the cursor if there are none; copy and move go " +
                     "to the other panel. Tab or a click changes panel. F10 quits.");
                return true;

            case CmView:
                if (Cursor(Active) is { } toView && File.Exists(toView)) ViewFile(toView);
                return true;

            case CmEdit:
                if (Cursor(Active) is { } toEdit && File.Exists(toEdit)) EditFile(toEdit);
                return true;

            case CmCopy:
            case CmMove:
            {
                var (src, dst) = (Active, Other);
                var items = Selected(src);
                if (items.Length == 0)
                {
                    Tell("Nothing to copy: mark some files with Insert.");
                    return true;
                }
                var verb = cmd == CmCopy ? "Copy" : "Move";
                Ask($"{verb} {items.Length} file(s) to {dst.Dir}?", () => Transfer(items, dst.Dir, move: cmd == CmMove));
                return true;
            }

            case CmMkdir:
                CloseBox();
                box = owl.Window("Make directory", 46, 8, style: Owlosui.Style.ModalDialog, closeCmd: CmNo);
                askInput = owl.Input(box, 2, 2, 40, "Name:", "", 64);
                owl.Buttons(box, new Owlosui.Button("~O~K", CmMkdirOk, Default: true), ("~C~ancel", CmNo));
                return true;

            case CmMkdirOk:
                MakeDir();
                return true;

            case CmDelete:
            {
                var src = Active;
                var items = Selected(src);
                if (items.Length == 0)
                {
                    Tell("Nothing to delete.");
                    return true;
                }
                var shown = string.Join(", ", items.Take(3).Select(Path.GetFileName));
                if (items.Length > 3) shown += $", +{items.Length - 3}";
                Ask($"Delete {items.Length} file(s): {shown}?", () =>
                {
                    foreach (var path in items) DeletePath(path);
                });
                return true;
            }

            case CmYes:
                CloseBox();
                var act = onYes;
                onYes = null;
                try { act?.Invoke(); }
                catch (Exception e) when (e is IOException or UnauthorizedAccessException) { Tell(e.Message); }
                Refresh();
                return true;

            case CmNo:
                CloseBox();
                onYes = null;
                return true;

            case CmSave:
                try { File.WriteAllText(editorPath, owl.GetText(editorText)); }
                catch (Exception e) when (e is IOException or UnauthorizedAccessException) { Tell(e.Message); return true; }
                CloseEditor();
                Refresh();
                return true;

            case CmCloseEdit:
                CloseEditor();
                return true;

            case CmCloseView:
                if (viewer != 0) owl.Close(viewer);
                viewer = 0;
                return true;

            case CmQuit:
                return false;

            default:
                return true;
        }
    }

    /// <summary>
    /// What the panels have to say that is not a command: a name entered, a
    /// path typed. Called after every event by <see cref="Owlosui.Run(Func{ushort, bool}, Action?)"/>.
    /// </summary>
    public void Poll()
    {
        foreach (var p in new[] { Left, Right })
        {
            var (kind, text) = owl.TakeFiles(p.Files);
            switch (kind)
            {
                case Owlosui.FilesEvent.Chosen:
                {
                    if (text == "..")
                    {
                        var up = Directory.GetParent(p.Dir);
                        if (up != null) Go(p, up.FullName, p.Mask);
                        break;
                    }
                    var full = Path.Combine(p.Dir, text);
                    if (Directory.Exists(full)) Go(p, full, p.Mask);
                    else if (File.Exists(full)) ViewFile(full);
                    break;
                }

                case Owlosui.FilesEvent.Path:
                {
                    // Not reachable without a path line, but a panel that
                    // had one would come here: `C:\WORK\*.TXT` is a folder
                    // and a mask; `C:\WORK` is a folder and the mask stays.
                    var last = Path.GetFileName(text);
                    if (last.Contains('*') || last.Contains('?'))
                        Go(p, Path.GetDirectoryName(text) ?? text, last);
                    else
                        Go(p, text, p.Mask);
                    break;
                }
            }
        }
    }

    // ---------------------------------------------------------------- panels

    /// <summary>Show another folder in a panel - or say why not, and stay.</summary>
    private void Go(Panel p, string dir, string mask)
    {
        try
        {
            dir = Path.GetFullPath(dir);
            var entries = Owlosui.ReadDirectory(dir);
            p.Dir = dir;
            p.Mask = mask;
            owl.SetFiles(p.Files, dir, entries, mask);
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException or ArgumentException)
        {
            // The old listing stays: the person has a typo to fix, not a
            // program to restart.
            owl.SetFilesError(p.Files, e is DirectoryNotFoundException
                ? $"Folder not found: {dir}"
                : $"Cannot read {dir}: {e.Message}");
        }
    }

    /// <summary>Both panels read again: something was made, copied or deleted.</summary>
    public void Refresh()
    {
        foreach (var p in new[] { Left, Right })
        {
            try { owl.SetFiles(p.Files, p.Dir, Owlosui.ReadDirectory(p.Dir), p.Mask); }
            catch (Exception e) when (e is IOException or UnauthorizedAccessException) { owl.SetFilesError(p.Files, e.Message); }
        }
    }

    /// <summary>
    /// The full path under the cursor, or null on `..` or nothing. For F3 and
    /// F4, which act on one file: with nothing marked the panel answers with
    /// the cursor, and with marks it answers with the marks - so a single
    /// answer is the cursor, and more than one means "no, mark less".
    /// </summary>
    private string? Cursor(Panel p)
    {
        var names = owl.MarkedNames(p.Files);
        if (names.Length != 1 || names[0] == "..") return null;
        return Path.Combine(p.Dir, names[0]);
    }

    /// <summary>
    /// What F5, F6 and F8 act on: the marked names, or the one under the
    /// cursor if none are marked - Norton's rule - as full paths. Never
    /// `..`, and only things that still exist, since a listing can be older
    /// than the folder.
    /// </summary>
    private string[] Selected(Panel p) =>
        owl.MarkedNames(p.Files)
            .Where(n => n != "..")
            .Select(n => Path.Combine(p.Dir, n))
            .Where(f => File.Exists(f) || Directory.Exists(f))
            .ToArray();

    // ----------------------------------------------------------------- files

    /// <summary>
    /// Copy or move items into a folder. Folders go whole; a folder cannot
    /// go into itself; a thing already where it is going is left alone. On
    /// the first failure the rest is not attempted and the message says
    /// what went wrong - a half-done copy is at least an honest one.
    /// </summary>
    private void Transfer(string[] items, string to, bool move)
    {
        // A progress bar in a small modal window while the files go across.
        // The bar is moved after each item and the screen redrawn; on a fast
        // disk it is a flicker, on a floppy it was the whole point.
        var win = owl.Window(move ? "Move" : "Copy", 50, 5, style: Owlosui.Style.ModalDialog);
        var bar = owl.Progress(win, 2, 1, 44, (uint)items.Length);
        try
        {
            for (var i = 0; i < items.Length; i++)
            {
                var src = items[i];
                var dest = Path.Combine(to, Path.GetFileName(src));
                if (Same(src, dest)) continue;
                if (Directory.Exists(src) && Contains(src, to))
                    throw new IOException("Cannot put a folder inside itself.");
                if (move) MovePath(src, dest);
                else if (Directory.Exists(src)) CopyTree(src, dest);
                else File.Copy(src, dest, overwrite: true);
                owl.SetProgress(bar, (uint)(i + 1));
                owl.Draw();
            }
        }
        finally
        {
            owl.Close(win);
        }
    }

    /// <summary>OK in the Make-directory dialog: the name, checked, made on the active side.</summary>
    private void MakeDir()
    {
        var name = owl.GetText(askInput).Trim();
        CloseBox();
        if (name.Length == 0 || name.IndexOfAny(new[] { '/', '\\' }) >= 0 || name is "." or "..")
        {
            Tell("Bad name.");
            return;
        }
        try { Directory.CreateDirectory(Path.Combine(Active.Dir, name)); }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException) { Tell(e.Message); return; }
        Refresh();
    }

    private static void MovePath(string src, string dest)
    {
        if (Directory.Exists(src)) Directory.Move(src, dest);
        else File.Move(src, dest, overwrite: true);
    }

    private static void CopyTree(string src, string dest)
    {
        Directory.CreateDirectory(dest);
        foreach (var file in Directory.EnumerateFiles(src))
            File.Copy(file, Path.Combine(dest, Path.GetFileName(file)), overwrite: true);
        foreach (var dir in Directory.EnumerateDirectories(src))
            CopyTree(dir, Path.Combine(dest, Path.GetFileName(dir)));
    }

    private static void DeletePath(string path)
    {
        if (Directory.Exists(path)) Directory.Delete(path, recursive: true);
        else File.Delete(path);
    }

    private static bool Contains(string parent, string path) =>
        Norm(path).StartsWith(Norm(parent), StringComparison.OrdinalIgnoreCase);

    private static bool Same(string a, string b) =>
        string.Equals(Norm(a), Norm(b), StringComparison.OrdinalIgnoreCase);

    private static string Norm(string path) =>
        Path.GetFullPath(path).TrimEnd(Path.DirectorySeparatorChar) + Path.DirectorySeparatorChar;

    /// <summary>A path cut to fit a title, keeping its tail - the end is the part that says where you are.</summary>
    private static string Fit(string text, int width)
    {
        if (width < 4 || text.Length <= width) return text;
        return "..." + text[^(width - 3)..];
    }

    // ------------------------------------------------------ viewer and editor

    private void ViewFile(string path)
    {
        try
        {
            if (Binary(path)) { Tell("Not a text file."); return; }
            var text = ReadText(path);
            if (viewer != 0) owl.Close(viewer);
            // Cyan, like a help window: something to read, not to change.
            viewer = owl.Window(Path.GetFileName(path), owl.Width - 8, owl.Height - 4,
                                style: Owlosui.Style.Help, closeCmd: CmCloseView);
            owl.Text(viewer, text, readOnly: true);
            owl.Buttons(viewer, ("~C~lose", CmCloseView));
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException) { Tell(e.Message); }
    }

    private void EditFile(string path)
    {
        try
        {
            if (Binary(path)) { Tell("Not a text file."); return; }
            var text = ReadText(path);
            CloseEditor();
            editorPath = path;
            editor = owl.Window(Path.GetFileName(path), owl.Width - 8, owl.Height - 4, closeCmd: CmCloseEdit);
            editorText = owl.Text(editor, text);
            // Enter is a new line inside the editor, so Save is Alt+S. Escape
            // reaches the last button, which drops the text - the same
            // bargain as Notes, without the question.
            owl.Buttons(editor, ("~S~ave", CmSave), ("~C~lose", CmCloseEdit));
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException) { Tell(e.Message); }
    }

    /// <summary>
    /// A file with a zero byte in its first 4K is not text, whatever its
    /// name says. The viewer and the editor refuse it: a text view of a
    /// binary is not a view of anything, and saving it back would be worse.
    /// </summary>
    private static bool Binary(string path)
    {
        var buf = new byte[4096];
        int n;
        using (var s = File.OpenRead(path)) n = s.Read(buf, 0, buf.Length);
        for (var i = 0; i < n; i++)
            if (buf[i] == 0) return true;
        return false;
    }

    // The editor's caret is an i16, and a screen is a screen. A file is not
    // read past its first 64K, and what is handed over is cut to 400 lines
    // of 180 characters - enough to look at, and the viewer says `...` where
    // it stopped. A real editor would page; this one manages files.
    private static string ReadText(string path)
    {
        var buf = new char[64 * 1024];
        using var reader = new StreamReader(path, new UTF8Encoding(encoderShouldEmitUTF8Identifier: false));
        var n = reader.Read(buf, 0, buf.Length);
        return Clip(new string(buf, 0, n));
    }

    /// <summary>Line endings made one kind, then at most 400 lines of 180 characters, with `...` if there was more.</summary>
    private static string Clip(string text)
    {
        var lines = text.Replace("\r\n", "\n").Replace('\r', '\n').Split('\n');
        var sb = new StringBuilder();
        var n = Math.Min(lines.Length, 400);
        for (var i = 0; i < n; i++)
        {
            var line = lines[i];
            sb.Append(line.Length > 180 ? line[..180] : line);
            if (i != n - 1) sb.Append('\n');
        }
        if (lines.Length > n) sb.Append("\n...");
        return sb.ToString();
    }

    // --------------------------------------------------------------- dialogs

    private void Ask(string text, Action yes)
    {
        CloseBox();
        onYes = yes;
        // No is the default and the last: Enter and Escape both keep things
        // as they are. Doing something to files has to be chosen on purpose.
        box = owl.MessageBox("Confirm", text, ("~Y~es", CmYes), new Owlosui.Button("~N~o", CmNo, Default: true));
    }

    private void Tell(string text)
    {
        CloseBox();
        onYes = null;
        box = owl.MessageBox("Commander", text, ("~O~K", CmNo));
    }

    private void CloseBox()
    {
        if (box == 0) return;
        owl.Close(box);
        box = 0;
        askInput = 0;
    }

    private void CloseEditor()
    {
        if (editor == 0) return;
        owl.Close(editor);
        editor = 0;
        editorText = 0;
    }

    public static void Main(string[] args)
    {
        var left = args.Length > 0 ? args[0] : Directory.GetCurrentDirectory();
        var right = args.Length > 1 ? args[1] : left;
        using var owl = new Owlosui();
        var app = new App(owl, left, right);
        owl.Run(app.OnCommand, app.Poll);
    }
}
