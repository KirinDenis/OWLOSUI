// The C# side of the wire.
//
// This is a client of `owlosui-serve`, not a port of the toolkit: it starts
// the server, sends what happened, asks for the frame and puts the cells on
// System.Console. Every window, button and keystroke is handled by the same
// Rust core that drives the terminal demo, and this file knows nothing about
// how any of it works. That is the point. See lib/PROTOCOL.md.
//
// Use:
//
//     using var owl = new Owlosui();
//     var w = owl.Window("Hello", 40, 10);
//     owl.Static(w, 2, 1, "Hello, world!");
//     owl.Buttons(w, ("~O~K", 1));
//     owl.Run(cmd => cmd != 1);
//
// Handles are numbers (ushort), as they will be on DOS. Commands are your
// own ushorts; 0 means "nothing happened" and cannot be a button.
//
// Two halves. The first talks to the server and works anywhere, with no
// console at all — that is what a test uses. The second, `Run`, is the
// console: it reads keys and the mouse, draws frames, and is the only part
// that knows it is on Windows.

using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;
using static Win32;

public sealed class Owlosui : IDisposable
{
    // ------------------------------------------------------------ building

    /// <summary>
    /// Start the server and make a desktop the size of the console — or the
    /// size given, for a program with no console to ask: a test, or a client
    /// that will put the cells somewhere else.
    /// </summary>
    public Owlosui(string? serverPath = null, int width = 0, int height = 0, int codePage = 0)
    {
        // The code page: what the program asked for; else OWLOSUI_CODEPAGE in
        // the environment; else the OEM code page of the machine's locale,
        // which is what a DOS window here would have been running - 866 on
        // a Russian Windows - if it is one the server knows; else 437. So an
        // unchanged program shows Cyrillic names in Russia without being
        // told, and a person elsewhere gets the page DOS shipped with.
        if (codePage == 0)
        {
            var env = Environment.GetEnvironmentVariable("OWLOSUI_CODEPAGE");
            if (int.TryParse(env, out var n) && n > 0) codePage = n;
            else
            {
                // The system locale's OEM page (GetOEMCP), not the user's
                // regional format: it is the "language for non-Unicode
                // programs", the one a DOS window here would show.
                var oem = OperatingSystem.IsWindows() ? (int)GetOEMCP() : 437;
                codePage = oem is 437 or 866 ? oem : 437;
            }
        }
        var path = serverPath ?? FindServer();
        var psi = new ProcessStartInfo(path)
        {
            UseShellExecute = false,
            RedirectStandardInput = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
        };
        proc = Process.Start(psi) ?? throw new InvalidOperationException($"could not start {path}");
        proc.ErrorDataReceived += (_, e) => { if (e.Data != null) stderr.AppendLine(e.Data); };
        proc.BeginErrorReadLine();
        toServer = proc.StandardInput.BaseStream;
        fromServer = proc.StandardOutput.BaseStream;

        if (width <= 0 || height <= 0)
        {
            try
            {
                width = Console.WindowWidth;
                height = Console.WindowHeight;
            }
            catch (IOException)
            {
                // No console at all — output is a file, or there never was
                // one. Eighty by twenty-five is what a text screen is.
                (width, height) = (80, 25);
            }
        }
        Width = Math.Max(width, 20);
        Height = Math.Max(height, 5);
        Call(Op.Init, W.I16(Width), W.I16(Height), W.U16((ushort)codePage));
        Glyphs = FetchGlyphs();
    }

    /// <summary>
    /// What each of the 256 glyph indices looks like, as Unicode - the
    /// server's table for the session's code page, fetched once, so this
    /// client keeps no table of its own. Index it with a cell's glyph.
    /// </summary>
    public string Glyphs { get; private set; } = Cp437;

    private HashSet<char> glyphSet = new();

    /// <summary>Whether the session's font takes new characters as they come, or is the 256 it was born with.</summary>
    public bool FontGrows { get; private set; }

    private string FetchGlyphs()
    {
        var r = Call(Op.GetGlyphs);
        FontGrows = r[0] != 0;
        var n = R.U16(r, 1);
        var sb = new StringBuilder(n);
        for (var i = 0; i < n; i++) sb.Append((char)R.U16(r, 3 + i * 2));
        var g = sb.ToString();
        // Positions 0x20 up: the pictures below are not text.
        glyphSet = new HashSet<char>(g.Skip(0x20));
        return g;
    }

    /// <summary>
    /// Whether every character of a text has a glyph on the session's code
    /// page - so a program can know before the text goes in and comes back
    /// as `?`. An editor that cannot show a file must not save it.
    /// </summary>
    public bool Fits(string text) =>
        FontGrows || text.All(c => c < 128 || c == '\n' || c == '\r' || c == '\t' || glyphSet.Contains(c));

    /// <summary>Switch the session to another code page: 437 or 866.</summary>
    public void SetCodePage(int codePage)
    {
        Call(Op.CodePage, W.U16((ushort)codePage));
        Glyphs = FetchGlyphs();
        prev = null;
    }

    public int Width { get; private set; }
    public int Height { get; private set; }

    /// <summary>The desktop changed size — the console window did, or a test says so.</summary>
    public void Resize(int width, int height)
    {
        Width = Math.Max(width, 20);
        Height = Math.Max(height, 5);
        Call(Op.Resize, W.I16(Width), W.I16(Height));
        prev = null;
    }

    /// <summary>
    /// A window. (-1, -1) — the default — means centred, now and after every
    /// resize until it is dragged. <paramref name="closeCmd"/>, if given, is
    /// what the close box sends instead of closing: the program then decides,
    /// which is how "save changes?" gets asked first. Every window casts a
    /// <paramref name="shadow"/> unless told not to - a panel that tiles the
    /// screen with another has nothing to float above.
    /// </summary>
    public ushort Window(string title, int w, int h, int x = -1, int y = -1,
                         Style style = Style.Document, ushort parent = 0, ushort closeCmd = 0, bool shadow = true)
    {
        var flags = (byte)((byte)style | (shadow ? 0 : 0x40));
        var r = Call(Op.Window, W.U16(parent), W.Rect(x, y, w, h), new[] { flags }, W.Str(title), W.U16(closeCmd));
        return R.U16(r);
    }

    /// <summary>A text editor filling its window. Read-only makes it a viewer.</summary>
    public ushort Text(ushort parent, string text = "", bool readOnly = false)
    {
        var flags = (byte)(readOnly ? 1 : 0);
        var r = Call(Op.Text, W.U16(parent), W.Rect(0, 0, 0, 0), new byte[] { 0, flags }, W.Str(text));
        return R.U16(r);
    }

    /// <summary>A boxed memo at a place of its own inside the window.</summary>
    public ushort Memo(ushort parent, int x, int y, int w, int h, string text = "", bool readOnly = false)
    {
        var flags = (byte)((readOnly ? 1 : 0) | 2);
        var r = Call(Op.Text, W.U16(parent), W.Rect(x, y, w, h), new byte[] { 1, flags }, W.Str(text));
        return R.U16(r);
    }

    /// <summary>Words. Wrapped to the width given; one line if no height.</summary>
    public ushort Static(ushort parent, int x, int y, string text, int w = 0, int h = 1)
    {
        if (w <= 0) w = text.Length;
        var r = Call(Op.Static, W.U16(parent), W.Rect(x, y, w, h), W.Str(text));
        return R.U16(r);
    }

    /// <summary>A one-line field: <c>Name:</c> followed by what is typed.</summary>
    public ushort Input(ushort parent, int x, int y, int w, string label, string text = "", int max = 0)
    {
        var r = Call(Op.Input, W.U16(parent), W.Rect(x, y, w, 1), W.U16((ushort)max), W.Str(label), W.Str(text));
        return R.U16(r);
    }

    /// <summary>
    /// A button: a label with its hotkey between tildes, the command it
    /// sends, and whether Enter presses it from anywhere in the dialog.
    /// A plain tuple <c>("~O~K", 1)</c> converts to one.
    /// </summary>
    public readonly record struct Button(string Label, ushort Cmd, bool Default = false)
    {
        public static implicit operator Button((string label, ushort cmd) t) => new(t.label, t.cmd);
    }

    /// <summary>
    /// Buttons, bottom right. Enter presses the one marked Default, or the
    /// first if none is; Escape presses the last. So put Cancel last — and
    /// if the safe answer should take both keys, mark it Default and put it
    /// last.
    /// </summary>
    public ushort Buttons(ushort parent, params Button[] buttons)
    {
        var r = Call(Op.Buttons, W.U16(parent), W.Buttons(buttons));
        return R.U16(r);
    }

    /// <summary>The commonest dialog there is: some words and a row of buttons.</summary>
    public ushort MessageBox(string title, string text, params Button[] buttons)
    {
        var r = Call(Op.MessageBox, W.Str(title), W.Str(text), W.Buttons(buttons));
        return R.U16(r);
    }

    /// <summary>
    /// One entry of the status line: what it says, what it sends, and the
    /// key that sends it. A plain tuple <c>("~F1~ Help", 1)</c> is an entry
    /// with no key - clickable only.
    /// </summary>
    public readonly record struct StatusItem(string Label, ushort Cmd, ConsoleKey Key = ConsoleKey.NoName,
                                             bool Alt = false, bool Ctrl = false, bool Shift = false, char Ch = '\0')
    {
        public static implicit operator StatusItem((string label, ushort cmd) t) => new(t.label, t.cmd);
    }

    /// <summary>
    /// The bottom row: keys and what they do. It shows them and it binds
    /// them - press the key, or click the words, and the command comes back
    /// through <see cref="Run"/> like a button's. One per program; calling
    /// this again replaces it.
    /// </summary>
    public ushort StatusLine(params StatusItem[] items)
    {
        var v = new List<byte> { (byte)items.Length };
        foreach (var it in items)
        {
            v.AddRange(W.U16(it.Cmd));
            var ch = it.Ch;
            if (ch == '\0' && it.Key >= ConsoleKey.A && it.Key <= ConsoleKey.Z)
                ch = (char)('a' + (it.Key - ConsoleKey.A));
            var k = it.Key == ConsoleKey.NoName ? null : EncodeKey(it.Key, ch, it.Shift, it.Alt, it.Ctrl);
            if (k is { } key)
            {
                v.Add(key.kind);
                v.AddRange(W.U16(key.value));
                v.Add(key.mods);
            }
            else
            {
                v.Add(0xFF);
                v.AddRange(W.U16(0));
                v.Add(0);
            }
            v.AddRange(W.Str(it.Label));
        }
        return R.U16(Call(Op.Status, v.ToArray()));
    }

    /// <summary>
    /// Words with a hotkey beside a control: <c>~N~ame:</c> next to an
    /// input. Alt+N, or a click on the words, puts the focus on the target.
    /// </summary>
    public ushort Label(ushort parent, int x, int y, string text, ushort target = 0)
    {
        var w = text.Replace("~", "").Length;
        var r = Call(Op.Label, W.U16(parent), W.Rect(x, y, w, 1), W.U16(target), W.Str(text));
        return R.U16(r);
    }

    /// <summary>A bar that fills up: <see cref="SetProgress"/> moves it.</summary>
    public ushort Progress(ushort parent, int x, int y, int w, uint max = 100, bool percent = true)
    {
        var r = Call(Op.Progress, W.U16(parent), W.Rect(x, y, w, 1), W.U32(max), new[] { (byte)(percent ? 1 : 0) });
        return R.U16(r);
    }

    public void SetProgress(ushort id, uint value) => Call(Op.SetProgress, W.U16(id), W.U32(value));

    /// <summary>
    /// A list of strings. With <paramref name="multi"/>, Insert marks the
    /// item under the cursor and moves down; <see cref="Marked"/> reads the
    /// marks back and <see cref="Current"/> the cursor.
    /// </summary>
    public ushort List(ushort parent, int x, int y, int w, int h, IEnumerable<string> items, bool multi = false)
    {
        var list = items.ToList();
        var v = new List<byte> { (byte)(multi ? 1 : 0) };
        v.AddRange(W.U16((ushort)list.Count));
        foreach (var s in list) v.AddRange(W.Str(s));
        var r = Call(Op.List, W.U16(parent), W.Rect(x, y, w, h), v.ToArray());
        return R.U16(r);
    }

    public int[] Marked(ushort id)
    {
        var r = Call(Op.GetMarked, W.U16(id));
        var n = R.U16(r, 0);
        var out_ = new int[n];
        for (var i = 0; i < n; i++) out_[i] = R.U16(r, 2 + i * 2);
        return out_;
    }

    public int Current(ushort id) => R.U16(Call(Op.GetCurrent, W.U16(id)));

    // -------------------------------------------------------------- files

    /// <summary>
    /// One line of a directory listing, as the panel shows it. The bits in
    /// <see cref="Attrs"/> are DOS's: 0x10 directory, 0x01 read-only, 0x02
    /// hidden, 0x04 system, 0x20 archive.
    /// </summary>
    public readonly record struct FileEntry(string Name, uint Size, DateTime Date, byte Attrs)
    {
        public const byte Dir = 0x10, ReadOnly = 0x01, Hidden = 0x02, System = 0x04, Archive = 0x20;
        public bool IsDir => (Attrs & Dir) != 0;
    }

    /// <summary>
    /// Read a directory the way a file panel wants it: `..` first unless
    /// this is a root, then everything else; the panel sorts. This is the
    /// whole of what the client has to know about a file system - the
    /// server never opens one. A directory that cannot be read throws, and
    /// the program decides what to show for it.
    /// </summary>
    public static List<FileEntry> ReadDirectory(string path)
    {
        var dir = new DirectoryInfo(path);
        var list = new List<FileEntry>();
        if (dir.Parent != null)
            list.Add(new FileEntry("..", 0, dir.LastWriteTime, FileEntry.Dir));
        foreach (var e in dir.EnumerateFileSystemInfos())
        {
            byte attrs = 0;
            if ((e.Attributes & FileAttributes.Directory) != 0) attrs |= FileEntry.Dir;
            if ((e.Attributes & FileAttributes.ReadOnly) != 0) attrs |= FileEntry.ReadOnly;
            if ((e.Attributes & FileAttributes.Hidden) != 0) attrs |= FileEntry.Hidden;
            if ((e.Attributes & FileAttributes.System) != 0) attrs |= FileEntry.System;
            if ((e.Attributes & FileAttributes.Archive) != 0) attrs |= FileEntry.Archive;
            var size = e is FileInfo f ? (uint)Math.Min(f.Length, uint.MaxValue) : 0u;
            list.Add(new FileEntry(e.Name, size, e.LastWriteTime, attrs));
        }
        return list;
    }

    /// <summary>
    /// A file panel filling its window: the path line, the names in
    /// columns, the details pane. With <paramref name="multi"/>, Insert
    /// marks files. What the person does in it comes back through
    /// <see cref="TakeFiles"/>.
    /// </summary>
    public ushort Files(ushort parent, string path, IEnumerable<FileEntry> entries, string mask = "*.*",
                        bool multi = false, bool pathLabel = true, bool pathLine = true)
    {
        var flags = (byte)((multi ? 1 : 0) | (pathLabel ? 0 : 2) | (pathLine ? 0 : 4));
        var r = Call(Op.Files, W.U16(parent), W.Rect(0, 0, 0, 0), new[] { flags },
                     W.Str(mask), W.Str(Path.Combine(path, mask)), W.Entries(entries));
        return R.U16(r);
    }

    /// <summary>A new listing for a panel: the person went somewhere else, or something changed.</summary>
    public void SetFiles(ushort id, string path, IEnumerable<FileEntry> entries, string mask = "*.*") =>
        Call(Op.SetFiles, W.U16(id), W.Str(Path.Combine(path, mask)), W.Str(mask), W.Entries(entries));

    /// <summary>Something to say in the panel's pane instead of details: a folder that could not be read.</summary>
    public void SetFilesError(ushort id, string text) => Call(Op.SetFilesError, W.U16(id), W.Str(text));

    public enum FilesEvent : byte { None = 0, Chosen = 1, Path = 2 }

    /// <summary>
    /// What the person did in a panel since last asked: pressed Enter on a
    /// name (<see cref="FilesEvent.Chosen"/>, with the name), or typed a
    /// path and pressed Enter (<see cref="FilesEvent.Path"/>, with the
    /// text). Read once, like <see cref="Take"/>.
    /// </summary>
    public (FilesEvent kind, string text) TakeFiles(ushort id)
    {
        var r = Call(Op.TakeFiles, W.U16(id));
        return ((FilesEvent)r[0], R.Str(r, 1));
    }

    /// <summary>The marked names of a panel - or the one under the cursor, if none are marked.</summary>
    public string[] MarkedNames(ushort id)
    {
        var r = Call(Op.MarkedNames, W.U16(id));
        var n = R.U16(r, 0);
        var names = new string[n];
        var at = 2;
        for (var i = 0; i < n; i++)
        {
            names[i] = R.Str(r, at);
            at += 2 + Encoding.UTF8.GetByteCount(names[i]);
        }
        return names;
    }

    /// <summary>Bring a window to the front and give it the keys.</summary>
    public void Activate(ushort window) => Call(Op.Activate, W.U16(window));

    /// <summary>The window in front, or 0.</summary>
    public ushort Active() => R.U16(Call(Op.Active));

    // ---------------------------------------------------------------- menus

    /// <summary>
    /// One entry of a menu: words with the hotkey between tildes, the
    /// command it sends (or a submenu instead), a shortcut shown at the
    /// right as a reminder - the key itself is bound elsewhere. A plain
    /// tuple <c>("~O~pen", 1)</c> is an entry; <see cref="Line"/> is a
    /// separator; <see cref="Sub"/> nests.
    /// </summary>
    public sealed record MenuItem(string Label, ushort Cmd, string Shortcut = "", bool Checked = false,
                                  bool Enabled = true, bool Separator = false, MenuItem[]? Items = null)
    {
        public static implicit operator MenuItem((string label, ushort cmd) t) => new(t.label, t.cmd);
        public static implicit operator MenuItem((string label, ushort cmd, string shortcut) t) => new(t.label, t.cmd, t.shortcut);
        public static MenuItem Line() => new("", 0, Separator: true);
        public static MenuItem Sub(string label, params MenuItem[] items) => new(label, 0, Items: items);
    }

    /// <summary>
    /// The menu bar across the top: each argument is a menu, made with
    /// <see cref="MenuItem.Sub"/>. Chosen items come back through
    /// <see cref="Run"/> as commands. Calling this again replaces the bar.
    /// </summary>
    public ushort MenuBar(params MenuItem[] menus) => R.U16(Call(Op.MenuBar, W.Menu(menus)));

    /// <summary>Tick or untick the menu item that sends a command - an option that is on.</summary>
    public void MenuCheck(ushort cmd, bool on) => Call(Op.MenuCheck, W.U16(cmd), new[] { (byte)(on ? 1 : 0) });

    /// <summary>The front window goes to the back: Turbo Vision's F6.</summary>
    public void NextWindow() => Call(Op.Cycle);

    /// <summary>A window fills the work area, or goes back to its size: F5.</summary>
    public void Zoom(ushort window) => Call(Op.Zoom, W.U16(window));

    // ------------------------------------------------------------- clusters

    /// <summary>
    /// Check boxes (any number on) or, with <paramref name="single"/>,
    /// radio buttons (exactly one, the first to begin with). One row per
    /// label; Space toggles, the hotkey letters jump.
    /// </summary>
    public ushort Cluster(ushort parent, int x, int y, int w, IEnumerable<string> labels, bool single = false)
    {
        var list = labels.ToList();
        var v = new List<byte> { (byte)(single ? 1 : 0), (byte)list.Count };
        foreach (var l in list) v.AddRange(W.Str(l));
        return R.U16(Call(Op.Cluster, W.U16(parent), W.Rect(x, y, w, list.Count), v.ToArray()));
    }

    /// <summary>Which boxes are on, and where the cursor is.</summary>
    public (bool[] on, int current) ClusterState(ushort id)
    {
        var r = Call(Op.GetCluster, W.U16(id));
        var n = r[0];
        var on = new bool[n];
        for (var i = 0; i < n; i++) on[i] = r[1 + i] != 0;
        return (on, r[1 + n]);
    }

    // -------------------------------------------------------------- canvas

    /// <summary>
    /// Where the mouse last went down on a canvas, in its cells, or null.
    /// Read once: the canvas does not know what its cells mean, so the
    /// program looks up what was hit.
    /// </summary>
    public (int x, int y)? CanvasClick(ushort id)
    {
        var r = Call(Op.GetClick, W.U16(id));
        return r[0] != 0 ? (R.I16(r, 1), R.I16(r, 3)) : null;
    }

    /// <summary>
    /// A rectangle of cells the program draws itself - a game board, a
    /// chart, a piece of ANSI art. Fill it with <see cref="Blit"/>; it is
    /// shown as it is, never wrapped or collapsed.
    /// </summary>
    public ushort Canvas(ushort parent, int x, int y, int w, int h) =>
        R.U16(Call(Op.Canvas, W.U16(parent), W.Rect(x, y, w, h)));

    /// <summary>
    /// Put a block of cells into a canvas: <paramref name="chars"/> and
    /// <paramref name="attrs"/> row by row, <c>w*h</c> of each. An
    /// attribute is <see cref="Attr"/> of two console colours.
    /// </summary>
    public void Blit(ushort id, int x, int y, int w, int h, ReadOnlySpan<char> chars, ReadOnlySpan<byte> attrs)
    {
        if (chars.Length < w * h || attrs.Length < w * h)
            throw new ArgumentException($"a {w}x{h} block needs {w * h} cells");
        var v = new byte[10 + w * h * 3];
        var at = 0;
        foreach (var part in new[] { W.U16(id), W.I16(x), W.I16(y), W.I16(w), W.I16(h) })
        {
            part.CopyTo(v, at);
            at += 2;
        }
        for (var i = 0; i < w * h; i++)
        {
            v[at++] = (byte)(chars[i] & 0xFF);
            v[at++] = (byte)(chars[i] >> 8);
            v[at++] = attrs[i];
        }
        Call(Op.Blit, v);
    }

    /// <summary>One row of text into a canvas, in one colour.</summary>
    public void Blit(ushort id, int x, int y, string text, byte attr)
    {
        var attrs = new byte[text.Length];
        Array.Fill(attrs, attr);
        Blit(id, x, y, text.Length, 1, text, attrs);
    }

    /// <summary>The IBM attribute byte for a foreground and a background: the same order <see cref="ConsoleColor"/> uses.</summary>
    public static byte Attr(ConsoleColor fg, ConsoleColor bg) => (byte)(((int)bg << 4) | (int)fg);

    /// <summary>
    /// A canvas cell that is not drawn: the window shows through it. Send
    /// <see cref="ClearChar"/> with <see cref="ClearAttr"/>. A new canvas
    /// is all clear.
    /// </summary>
    public const char ClearChar = '\0';
    public const byte ClearAttr = 0xFF;

    /// <summary>
    /// New words for a Static (keeps its place and width), an Input (keeps
    /// its label) or a Text (starts over). Cheaper than closing and making
    /// another, and the view keeps its handle.
    /// </summary>
    public void SetText(ushort id, string text) => Call(Op.SetText, W.U16(id), W.Str(text));

    public void Close(ushort id) => Call(Op.Close, W.U16(id));

    /// <summary>What a Text, Memo or Input holds right now.</summary>
    public string GetText(ushort id) => R.Str(Call(Op.GetText, W.U16(id)));

    // -------------------------------------------------------------- events

    public enum MouseKind : byte { Down = 0, Up = 1, Drag = 2, Move = 3, WheelUp = 4, WheelDown = 5 }
    public enum MouseButton : byte { Left = 0, Right = 1, Middle = 2 }

    /// <summary>One mouse event, in screen cells.</summary>
    public void SendMouse(MouseKind kind, int x, int y, MouseButton button = MouseButton.Left)
    {
        Call(Op.Mouse, new[] { (byte)kind, (byte)button }, W.I16(x), W.I16(y));
    }

    /// <summary>Press and release at a cell. What a test means by "click".</summary>
    public void Click(int x, int y, MouseButton button = MouseButton.Left)
    {
        SendMouse(MouseKind.Down, x, y, button);
        SendMouse(MouseKind.Up, x, y, button);
    }

    /// <summary>Press at one cell, drag to another, release. Moving or resizing a window.</summary>
    public void Drag(int fromX, int fromY, int toX, int toY)
    {
        SendMouse(MouseKind.Down, fromX, fromY);
        SendMouse(MouseKind.Drag, toX, toY);
        SendMouse(MouseKind.Up, toX, toY);
    }

    /// <summary>Send one key. Returns false if it was one the wire has no name for.</summary>
    public bool SendKey(ConsoleKeyInfo k)
    {
        var shift = k.Modifiers.HasFlag(ConsoleModifiers.Shift);
        var ctrl = k.Modifiers.HasFlag(ConsoleModifiers.Control);
        var alt = k.Modifiers.HasFlag(ConsoleModifiers.Alt);
        if (EncodeKey(k.Key, k.KeyChar, shift, alt, ctrl) is not { } code) return false;
        Call(Op.Key, new[] { code.kind }, W.U16(code.value), new[] { code.mods });
        return true;
    }

    /// <summary>
    /// A key as the wire spells it: a named key, a function key, or a
    /// character with its modifiers. Null for a key the wire has no name
    /// for. The one place this is decided, so the status line and a
    /// keystroke cannot disagree about what Alt+X is.
    /// </summary>
    private static (byte kind, ushort value, byte mods)? EncodeKey(ConsoleKey key, char ch, bool shift, bool alt, bool ctrl)
    {
        byte mods = 0;
        if (shift) mods |= 1;
        if (ctrl) mods |= 2;
        if (alt) mods |= 4;

        static (byte kind, ushort value)? Named(ushort v) => ((byte)2, v);
        (byte kind, ushort value)? code = key switch
        {
            ConsoleKey.Enter => Named(0),
            ConsoleKey.Escape => Named(1),
            ConsoleKey.Tab => Named((mods & 1) != 0 ? (ushort)3 : (ushort)2),
            ConsoleKey.Backspace => Named(4),
            ConsoleKey.Delete => Named(5),
            ConsoleKey.Insert => Named(6),
            ConsoleKey.Home => Named(7),
            ConsoleKey.End => Named(8),
            ConsoleKey.PageUp => Named(9),
            ConsoleKey.PageDown => Named(10),
            ConsoleKey.UpArrow => Named(11),
            ConsoleKey.DownArrow => Named(12),
            ConsoleKey.LeftArrow => Named(13),
            ConsoleKey.RightArrow => Named(14),
            >= ConsoleKey.F1 and <= ConsoleKey.F12 => ((byte)1, (ushort)(key - ConsoleKey.F1 + 1)),
            _ => null,
        };
        if (code is null)
        {
            // Ctrl+letter arrives as a control character; the core wants the
            // letter and the modifier, as a keymap table would spell it.
            if (ch >= 1 && ch <= 26 && (mods & 2) != 0) ch = (char)('a' + ch - 1);
            if (ch == '\0' || char.IsControl(ch)) return null;
            // Shift is already in the character's case. Sending it as well
            // would make Shift+A a different key from A, which it is not.
            if ((mods & 6) == 0) mods = 0;
            code = (0, ch);
        }
        return (code.Value.kind, code.Value.value, mods);
    }

    /// <summary>A named key with modifiers, for a program or a test that has no keyboard in hand.</summary>
    public void Press(ConsoleKey key, bool shift = false, bool alt = false, bool ctrl = false, char ch = '\0')
    {
        SendKey(new ConsoleKeyInfo(ch, key, shift, alt, ctrl));
    }

    /// <summary>Type a string, one character at a time.</summary>
    public void Type(string text)
    {
        foreach (var ch in text)
            SendKey(new ConsoleKeyInfo(ch, ConsoleKey.NoName, false, false, false));
    }

    /// <summary>What the last event caused: a button pressed, a menu command chosen, or 0.</summary>
    public (ushort pressed, ushort command) Take()
    {
        var r = Call(Op.Take);
        return (R.U16(r, 0), R.U16(r, 2));
    }

    // --------------------------------------------------------------- frame

    /// <summary>The screen as the core drew it: two bytes per cell, glyph then attribute.</summary>
    public Frame GetFrame()
    {
        var r = Call(Op.Frame);
        // The font may have grown since the table was fetched: a frame
        // says how long it is now, and a shorter table is fetched again
        // before any cell is looked up in it.
        var fontLen = R.U16(r, 9);
        if (fontLen > Glyphs.Length) Glyphs = FetchGlyphs();
        return new Frame(R.I16(r, 0), R.I16(r, 2), R.I16(r, 4), R.I16(r, 6), r[8] != 0, r[11..], Glyphs);
    }

    /// <summary>
    /// The moment has passed: deliver what the last frame was holding - a
    /// button a key put down, a menu item chosen. `Run` sends this itself
    /// after showing such a frame for 90 ms; a headless client sends it
    /// when it has "looked".
    /// </summary>
    public void Tick() => Call(Op.Tick);

    /// <summary>
    /// The screen as the core drew it: two bytes per cell, glyph then
    /// attribute. <see cref="Hold"/> means "show this one for a moment, then
    /// <see cref="Tick"/>": something chosen is being shown before it happens.
    /// </summary>
    /// <summary>
    /// The screen as the core drew it: three bytes per cell, a 16-bit
    /// glyph index then the attribute. <see cref="Hold"/> means "show this
    /// one for a moment, then <see cref="Tick"/>".
    /// </summary>
    public readonly record struct Frame(int W, int H, int CursorX, int CursorY, bool Hold, byte[] Cells, string Glyphs)
    {
        public ushort Glyph(int x, int y) => BitConverter.ToUInt16(Cells, (y * W + x) * 3);
        public byte Attr(int x, int y) => Cells[(y * W + x) * 3 + 2];
        /// <summary>The cell as a character, through the table the server gave for its font.</summary>
        public char Char(int x, int y)
        {
            var g = Glyph(x, y);
            return g < Glyphs.Length ? Glyphs[g] : '?';
        }

        /// <summary>One row as text, for looking at and for searching.</summary>
        public string Row(int y)
        {
            var sb = new StringBuilder(W);
            for (var x = 0; x < W; x++) sb.Append(Char(x, y));
            return sb.ToString();
        }

        /// <summary>Where some words are on the screen, if they are. Top-left of the first match.</summary>
        public (int x, int y)? Find(string text)
        {
            for (var y = 0; y < H; y++)
            {
                var x = Row(y).IndexOf(text, StringComparison.Ordinal);
                if (x >= 0) return (x, y);
            }
            return null;
        }

        /// <summary>The whole screen as text, for a failing test to print.</summary>
        public override string ToString()
        {
            var sb = new StringBuilder();
            for (var y = 0; y < H; y++) sb.AppendLine(Row(y));
            return sb.ToString();
        }
    }

    // ------------------------------------------------------------- running

    /// <summary>
    /// Draw, wait for input, send it, report what it caused — until
    /// <paramref name="onCommand"/> returns false. This is the whole event
    /// loop of an application; most programs never need anything else.
    /// </summary>
    public void Run(Func<ushort, bool> onCommand) => Run(onCommand, null);

    /// <summary>
    /// As <see cref="Run(Func{ushort, bool})"/>, with <paramref name="afterInput"/>
    /// called after every event as well - for what a program has to poll
    /// rather than be told: a name chosen in a file panel is not a command.
    /// </summary>
    public void Run(Func<ushort, bool> onCommand, Action? afterInput)
    {
        var enc = Console.OutputEncoding;
        Console.OutputEncoding = Encoding.UTF8;
        Console.CursorVisible = false;
        Console.Clear();
        prev = null;
        using var input = ConsoleInput.Open();
        // Two ways to ask for the mouse, because there are two kinds of
        // console. SetConsoleMode (in ConsoleInput) is what the classic
        // console listens to. A terminal on the other side of a pseudo
        // console - Windows Terminal, an editor's terminal pane - listens
        // for the VT request instead, and forwards mouse reports only after
        // it has seen one; the pseudo console then turns them into the same
        // input records. Ask both ways and every console answers.
        var vt = VtMouse.On();
        try
        {
            while (true)
            {
                var hold = Draw();
                if (hold)
                {
                    // A button a key just pressed is on the screen, down.
                    // Leave it there long enough to be seen, then let it
                    // happen. Turbo Vision did the same for a menu item.
                    Thread.Sleep(90);
                    Tick();
                    var (p, c) = Take();
                    if (p != 0 && !onCommand(p)) return;
                    if (c != 0 && !onCommand(c)) return;
                    continue;
                }
                foreach (var ev in input.Read())
                {
                    var sent = ev switch
                    {
                        KeyEv k => SendKey(k.Key),
                        MouseEv m => SendMouseEv(m),
                        ResizeEv r => SendResize(r),
                        _ => false,
                    };
                    if (!sent) continue;
                    var (pressed, command) = Take();
                    if (pressed != 0 && !onCommand(pressed)) return;
                    if (command != 0 && !onCommand(command)) return;
                    afterInput?.Invoke();
                }
            }
        }
        finally
        {
            vt.Off();
            Console.ResetColor();
            Console.Clear();
            Console.CursorVisible = true;
            Console.OutputEncoding = enc;
        }
    }

    /// <summary>
    /// The VT side of asking for the mouse: DECSET 1000 (buttons), 1002
    /// (drag), 1006 (SGR reports, so positions past column 223 survive).
    /// Written only if the console will interpret escape sequences rather
    /// than print them, which is what ENABLE_VIRTUAL_TERMINAL_PROCESSING
    /// on the output handle means.
    /// </summary>
    private readonly struct VtMouse
    {
        private readonly bool on;
        private VtMouse(bool on) => this.on = on;

        public static VtMouse On()
        {
            if (!OperatingSystem.IsWindows()) return new VtMouse(false);
            var h = GetStdHandle(STD_OUTPUT_HANDLE);
            if (!GetConsoleMode(h, out var mode)) return new VtMouse(false);
            if ((mode & ENABLE_VIRTUAL_TERMINAL_PROCESSING) == 0
                && !SetConsoleMode(h, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING))
                return new VtMouse(false);
            Console.Out.Write("\x1b[?1000h\x1b[?1002h\x1b[?1006h");
            Console.Out.Flush();
            return new VtMouse(true);
        }

        public void Off()
        {
            if (!on) return;
            Console.Out.Write("\x1b[?1006l\x1b[?1002l\x1b[?1000l");
            Console.Out.Flush();
        }
    }

    private bool SendMouseEv(MouseEv m)
    {
        SendMouse(m.Kind, m.X, m.Y, m.Button);
        return true;
    }

    private bool SendResize(ResizeEv r)
    {
        Console.Clear();
        Resize(r.W, r.H);
        return true;
    }

    private byte[]? prev;

    /// <summary>
    /// Fetch the frame and put what changed on the console. Returns whether
    /// the frame asked to be held - see <see cref="Frame.Hold"/>.
    /// </summary>
    public bool Draw()
    {
        try
        {
            return DrawCells();
        }
        catch (ArgumentOutOfRangeException)
        {
            // The console changed size between the frame being asked for and
            // the cells being written - a person is dragging the window edge,
            // and the size event that explains it is already queued. Nothing
            // is lost: the next frame is drawn in full.
            prev = null;
            return false;
        }
        catch (IOException)
        {
            prev = null;
            return false;
        }
    }

    private bool DrawCells()
    {
        var f = GetFrame();
        var full = prev is null || prev.Length != f.Cells.Length;
        // The console may be smaller than the frame for a moment (see Draw);
        // what does not fit is not written.
        var (bw, bh) = (Console.BufferWidth, Console.BufferHeight);
        var sb = new StringBuilder();
        for (var y = 0; y < Math.Min(f.H, bh); y++)
        {
            var x = 0;
            while (x < Math.Min(f.W, bw))
            {
                // The very last cell is left alone: writing it makes most
                // consoles scroll, and the toolkit's own terminal backend
                // skips it for the same reason.
                if (y == f.H - 1 && x == f.W - 1) break;
                if (y == bh - 1 && x == bw - 1) break;

                var i = (y * f.W + x) * 3;
                var changed = full || prev![i] != f.Cells[i] || prev[i + 1] != f.Cells[i + 1] || prev[i + 2] != f.Cells[i + 2];
                if (!changed) { x++; continue; }

                // A run of changed cells with one attribute, written in one go.
                var attr = f.Cells[i + 2];
                sb.Clear();
                var start = x;
                while (x < Math.Min(f.W, bw) && !(y == f.H - 1 && x == f.W - 1) && !(y == bh - 1 && x == bw - 1))
                {
                    var j = (y * f.W + x) * 3;
                    if (f.Cells[j + 2] != attr) break;
                    sb.Append(f.Char(x, y));
                    x++;
                }
                Console.SetCursorPosition(start, y);
                Console.ForegroundColor = (ConsoleColor)(attr & 0x0F);
                Console.BackgroundColor = (ConsoleColor)(attr >> 4);
                Console.Write(sb);
            }
        }
        prev = f.Cells;

        if (f.CursorX >= 0 && f.CursorY >= 0 && f.CursorX < Math.Min(f.W, bw) && f.CursorY < Math.Min(f.H, bh))
        {
            Console.SetCursorPosition(f.CursorX, f.CursorY);
            Console.CursorVisible = true;
        }
        else
        {
            Console.CursorVisible = false;
        }
        return f.Hold;
    }

    // ------------------------------------------------------- console input

    private abstract record Ev;
    private sealed record KeyEv(ConsoleKeyInfo Key) : Ev;
    private sealed record MouseEv(MouseKind Kind, MouseButton Button, int X, int Y) : Ev;
    private sealed record ResizeEv(int W, int H) : Ev;

    /// <summary>
    /// Where keys and the mouse come from.
    ///
    /// <c>System.Console</c> has no idea a mouse exists, which is why the
    /// first C# programs could not press a button or drag a corner with one.
    /// On Windows the console *does* report the mouse — through
    /// <c>ReadConsoleInput</c>, which also says when the window changes size
    /// — so on Windows that is what is used, and keys come through it too.
    /// Anywhere else, keys only, until somebody needs more.
    /// </summary>
    private abstract class ConsoleInput : IDisposable
    {
        public static ConsoleInput Open() =>
            OperatingSystem.IsWindows() ? new WindowsInput() : new KeysOnly();

        public abstract IEnumerable<Ev> Read();
        public virtual void Dispose() { }

        private sealed class KeysOnly : ConsoleInput
        {
            public override IEnumerable<Ev> Read()
            {
                yield return new KeyEv(Console.ReadKey(intercept: true));
            }
        }

        private sealed class WindowsInput : ConsoleInput
        {
            private readonly IntPtr handle;
            private readonly uint savedMode;
            private readonly INPUT_RECORD[] records = new INPUT_RECORD[32];
            private uint buttons;   // held right now, for Down/Up and Drag

            public WindowsInput()
            {
                handle = GetStdHandle(STD_INPUT_HANDLE);
                if (!GetConsoleMode(handle, out savedMode))
                    throw new OwlosuiException("no console input handle");
                // Mouse and window events on; the line editor, echo, Ctrl+C
                // processing and quick-edit (which steals the mouse for
                // selecting text) off. Virtual-terminal input off too, or
                // the mouse would arrive as escape sequences instead.
                var mode = savedMode;
                mode &= ~(ENABLE_PROCESSED_INPUT | ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT
                          | ENABLE_QUICK_EDIT_MODE | ENABLE_VIRTUAL_TERMINAL_INPUT);
                mode |= ENABLE_WINDOW_INPUT | ENABLE_MOUSE_INPUT | ENABLE_EXTENDED_FLAGS;
                var ok = SetConsoleMode(handle, mode);
                GetConsoleMode(handle, out var now);
                Trace($"console mode: was {savedMode:X4}, asked {mode:X4}, set {(ok ? "ok" : "FAILED " + Marshal.GetLastWin32Error())}, now {now:X4}");
                if (!ok)
                    throw new OwlosuiException($"SetConsoleMode failed ({Marshal.GetLastWin32Error()}); no mouse");
            }

            // OWLOSUI_TRACE=<file> writes every input record here, raw, and
            // every event made from it. For the day the mouse "does nothing"
            // and the question is whether the console delivered anything at
            // all - which cannot be answered from inside the program any
            // other way.
            private static readonly string? trace = Environment.GetEnvironmentVariable("OWLOSUI_TRACE");

            // Opened and closed for every line, so the file is readable by
            // whoever is watching while the program is still running - a
            // test agent, most likely. A held-open writer showed nothing
            // until the process ended, which is exactly when nobody was
            // looking any more.
            private static void Trace(string line)
            {
                if (string.IsNullOrEmpty(trace)) return;
                try { File.AppendAllText(trace, $"{DateTime.Now:HH:mm:ss.fff} {line}{Environment.NewLine}"); }
                catch (IOException) { }
            }

            public override void Dispose() => SetConsoleMode(handle, savedMode);

            public override IEnumerable<Ev> Read()
            {
                if (!ReadConsoleInputW(handle, records, (uint)records.Length, out var n))
                    throw new OwlosuiException("ReadConsoleInput failed");
                for (var i = 0; i < n; i++)
                {
                    var r = records[i];
                    if (trace != null)
                        Trace(r.EventType switch
                        {
                            KEY_EVENT => $"key   down={r.KeyEvent.bKeyDown} vk={r.KeyEvent.wVirtualKeyCode:X2} ch={r.KeyEvent.UnicodeChar:X4} ctl={r.KeyEvent.dwControlKeyState:X4}",
                            MOUSE_EVENT => $"mouse at=({r.MouseEvent.dwMousePosition.X},{r.MouseEvent.dwMousePosition.Y}) buttons={r.MouseEvent.dwButtonState:X8} flags={r.MouseEvent.dwEventFlags:X2} top={Console.WindowTop}",
                            _ => $"type {r.EventType}",
                        });
                    switch (r.EventType)
                    {
                        case KEY_EVENT:
                            if (Key(r.KeyEvent) is { } k) yield return k;
                            break;
                        case MOUSE_EVENT:
                            foreach (var m in Mouse(r.MouseEvent)) yield return m;
                            break;
                        case WINDOW_BUFFER_SIZE_EVENT:
                            // The record carries the buffer size, which can
                            // include scrollback; the window is what we draw on.
                            yield return new ResizeEv(Console.WindowWidth, Console.WindowHeight);
                            break;
                    }
                }
            }

            private static KeyEv? Key(KEY_EVENT_RECORD k)
            {
                if (k.bKeyDown == 0) return null;
                var vk = k.wVirtualKeyCode;
                // Modifiers on their own are not keys.
                if (vk is 0x10 or 0x11 or 0x12 or 0x14 or 0x5B or 0x5C or 0x90 or 0x91) return null;

                var shift = (k.dwControlKeyState & SHIFT_PRESSED) != 0;
                var ctrl = (k.dwControlKeyState & (LEFT_CTRL_PRESSED | RIGHT_CTRL_PRESSED)) != 0;
                var alt = (k.dwControlKeyState & (LEFT_ALT_PRESSED | RIGHT_ALT_PRESSED)) != 0;
                var ch = (char)k.UnicodeChar;
                // With Alt held the console reports no character for a letter
                // or digit; the key code says which it was.
                if (ch == '\0' && vk >= 0x30 && vk <= 0x5A)
                    ch = shift ? (char)vk : char.ToLowerInvariant((char)vk);
                return new KeyEv(new ConsoleKeyInfo(ch, (ConsoleKey)vk, shift, alt, ctrl));
            }

            private IEnumerable<Ev> Mouse(MOUSE_EVENT_RECORD m)
            {
                var x = m.dwMousePosition.X;
                var y = m.dwMousePosition.Y - Console.WindowTop;

                if ((m.dwEventFlags & MOUSE_WHEELED) != 0)
                {
                    var delta = (short)(m.dwButtonState >> 16);
                    yield return new MouseEv(delta > 0 ? MouseKind.WheelUp : MouseKind.WheelDown, MouseButton.Left, x, y);
                    yield break;
                }
                if ((m.dwEventFlags & MOUSE_HWHEELED) != 0) yield break;

                if ((m.dwEventFlags & MOUSE_MOVED) != 0)
                {
                    // Motion with a button held is a drag; motion without one
                    // is nothing the toolkit acts on, and not worth a round
                    // trip per cell.
                    if (buttons != 0)
                        yield return new MouseEv(MouseKind.Drag, MouseButton.Left, x, y);
                    yield break;
                }

                // A press or a release: whichever bits changed.
                var now = m.dwButtonState & 0x7;
                var changed = now ^ buttons;
                buttons = now;
                for (var b = 0; b < 3; b++)
                {
                    var bit = 1u << b;
                    if ((changed & bit) == 0) continue;
                    var button = b switch { 0 => MouseButton.Left, 1 => MouseButton.Right, _ => MouseButton.Middle };
                    var kind = (now & bit) != 0 ? MouseKind.Down : MouseKind.Up;
                    Trace($"  -> {kind} {button} at ({x},{y})");
                    yield return new MouseEv(kind, button, x, y);
                }
            }
        }
    }

    // ------------------------------------------------------------- the wire

    public enum Style : byte
    {
        Document = 0x00,          // blue, resizable, zoomable, closable
        Help = 0x10,              // cyan
        Dialog = 0x20 | 0x02 | 0x04,  // grey, fixed size
        ModalDialog = Dialog | 0x01,
    }

    private static class Op
    {
        public const byte Quit = 0x00, Init = 0x01, Resize = 0x02, CodePage = 0x03;
        public const byte Window = 0x10, Text = 0x11, Static = 0x12, Input = 0x13, Buttons = 0x14, MessageBox = 0x15;
        public const byte Status = 0x16, Label = 0x17, Progress = 0x18, List = 0x19, Files = 0x1A, Canvas = 0x1B, MenuBar = 0x1C, Cluster = 0x1D;
        public const byte Close = 0x20, GetText = 0x21, SetProgress = 0x22, GetMarked = 0x23, GetCurrent = 0x24;
        public const byte SetFiles = 0x25, TakeFiles = 0x26, Activate = 0x27, MarkedNames = 0x28, SetFilesError = 0x29, Active = 0x2A;
        public const byte SetText = 0x2B, Blit = 0x2C, MenuCheck = 0x2D, GetCluster = 0x2E, GetClick = 0x2F;
        public const byte Cycle = 0x43, Zoom = 0x44;
        public const byte Key = 0x30, Mouse = 0x31, Tick = 0x32;
        public const byte Frame = 0x40, Take = 0x41, GetGlyphs = 0x42;
    }

    private readonly Process proc;
    private readonly Stream toServer;
    private readonly Stream fromServer;
    private readonly StringBuilder stderr = new();

    /// <summary>One request, one reply. An error reply becomes an exception that says what the server said.</summary>
    private byte[] Call(byte op, params byte[][] parts)
    {
        var len = parts.Sum(p => p.Length);
        var head = new byte[3];
        head[0] = op;
        BitConverter.TryWriteBytes(head.AsSpan(1), (ushort)len);
        toServer.Write(head);
        foreach (var p in parts) toServer.Write(p);
        toServer.Flush();

        var reply = new byte[3];
        ReadExactly(reply);
        var n = BitConverter.ToUInt16(reply, 1);
        var body = new byte[n];
        ReadExactly(body);
        if (reply[0] != 0)
            throw new OwlosuiException($"op {op:X2}: {R.Str(body)}");
        return body;
    }

    private void ReadExactly(byte[] buf)
    {
        var got = 0;
        while (got < buf.Length)
        {
            var n = fromServer.Read(buf, got, buf.Length - got);
            if (n <= 0)
                throw new OwlosuiException("owlosui-serve went away" + (stderr.Length > 0 ? ":\n" + stderr : ""));
            got += n;
        }
    }

    public void Dispose()
    {
        try
        {
            if (!proc.HasExited)
            {
                Call(Op.Quit);
                proc.WaitForExit(1000);
            }
        }
        catch (OwlosuiException) { }
        proc.Dispose();
    }

    /// <summary>
    /// Where the server is. In order: OWLOSUI_SERVE in the environment, then
    /// the repository's own build next to this source tree, then the PATH.
    /// </summary>
    private static string FindServer()
    {
        var env = Environment.GetEnvironmentVariable("OWLOSUI_SERVE");
        if (!string.IsNullOrEmpty(env)) return env;

        var exe = OperatingSystem.IsWindows() ? "owlosui-serve.exe" : "owlosui-serve";
        for (var dir = new DirectoryInfo(AppContext.BaseDirectory); dir != null; dir = dir.Parent)
        {
            foreach (var profile in new[] { "release", "debug" })
            {
                var candidate = Path.Combine(dir.FullName, "target", profile, exe);
                if (File.Exists(candidate)) return candidate;
            }
        }
        // Let the OS search the PATH; a clear message if it cannot.
        return "owlosui-serve";
    }

    // ---------------------------------------------------------- byte helpers

    private static class W
    {
        public static byte[] U16(ushort v) => BitConverter.GetBytes(v);
        public static byte[] U32(uint v) => BitConverter.GetBytes(v);
        public static byte[] Menu(MenuItem[] items)
        {
            var v = new List<byte> { (byte)items.Length };
            foreach (var it in items)
            {
                var flags = (byte)((it.Separator ? 1 : 0) | (it.Enabled ? 0 : 2) | (it.Checked ? 4 : 0));
                v.Add(flags);
                v.AddRange(U16(it.Cmd));
                v.AddRange(Str(it.Label));
                v.AddRange(Str(it.Shortcut));
                v.AddRange(Menu(it.Items ?? Array.Empty<MenuItem>()));
            }
            return v.ToArray();
        }

        public static byte[] Entries(IEnumerable<FileEntry> entries)
        {
            var list = entries.ToList();
            var v = new List<byte>();
            v.AddRange(U16((ushort)Math.Min(list.Count, ushort.MaxValue)));
            foreach (var e in list.Take(ushort.MaxValue))
            {
                v.AddRange(Str(e.Name));
                v.AddRange(U32(e.Size));
                v.AddRange(U16((ushort)e.Date.Year));
                v.Add((byte)e.Date.Month);
                v.Add((byte)e.Date.Day);
                v.Add((byte)e.Date.Hour);
                v.Add((byte)e.Date.Minute);
                v.Add(e.Attrs);
            }
            return v.ToArray();
        }
        public static byte[] I16(int v) => BitConverter.GetBytes((short)v);
        public static byte[] Rect(int x, int y, int w, int h) =>
            I16(x).Concat(I16(y)).Concat(I16(w)).Concat(I16(h)).ToArray();
        public static byte[] Str(string s)
        {
            var b = Encoding.UTF8.GetBytes(s);
            if (b.Length > ushort.MaxValue) throw new ArgumentException("string too long for the wire");
            return U16((ushort)b.Length).Concat(b).ToArray();
        }
        public static byte[] Buttons(Button[] buttons)
        {
            if (buttons.Length == 0) throw new ArgumentException("a button row needs at least one button");
            var dflt = Array.FindIndex(buttons, b => b.Default);
            if (dflt < 0) dflt = 0;
            var v = new List<byte> { (byte)buttons.Length };
            for (var i = 0; i < buttons.Length; i++)
            {
                if (buttons[i].Cmd == 0) throw new ArgumentException($"button '{buttons[i].Label}' has command 0, which means none");
                v.AddRange(U16(buttons[i].Cmd));
                v.Add((byte)(i == dflt ? 1 : 0));
                v.AddRange(Str(buttons[i].Label));
            }
            return v.ToArray();
        }
    }

    private static class R
    {
        public static ushort U16(byte[] b, int at = 0) => BitConverter.ToUInt16(b, at);
        public static short I16(byte[] b, int at = 0) => BitConverter.ToInt16(b, at);
        public static string Str(byte[] b, int at = 0)
        {
            var n = BitConverter.ToUInt16(b, at);
            return Encoding.UTF8.GetString(b, at + 2, n);
        }
    }

    /// <summary>
    /// Code page 437, glyph index to Unicode: the default before a session
    /// has told us its own (see <see cref="Glyphs"/>), and what the console
    /// agent reads a screen with unless told otherwise.
    /// </summary>
    public static readonly string Cp437 = "\u0020\u263A\u263B\u2665\u2666\u2663\u2660\u2022\u25D8\u25CB\u25D9\u2642\u2640\u266A\u266B\u263C" +
        "\u25BA\u25C4\u2195\u203C\u00B6\u00A7\u25AC\u21A8\u2191\u2193\u2192\u2190\u221F\u2194\u25B2\u25BC" +
        "\u0020\u0021\u0022\u0023\u0024\u0025\u0026\u0027\u0028\u0029\u002A\u002B\u002C\u002D\u002E\u002F" +
        "\u0030\u0031\u0032\u0033\u0034\u0035\u0036\u0037\u0038\u0039\u003A\u003B\u003C\u003D\u003E\u003F" +
        "\u0040\u0041\u0042\u0043\u0044\u0045\u0046\u0047\u0048\u0049\u004A\u004B\u004C\u004D\u004E\u004F" +
        "\u0050\u0051\u0052\u0053\u0054\u0055\u0056\u0057\u0058\u0059\u005A\u005B\u005C\u005D\u005E\u005F" +
        "\u0060\u0061\u0062\u0063\u0064\u0065\u0066\u0067\u0068\u0069\u006A\u006B\u006C\u006D\u006E\u006F" +
        "\u0070\u0071\u0072\u0073\u0074\u0075\u0076\u0077\u0078\u0079\u007A\u007B\u007C\u007D\u007E\u2302" +
        "\u00C7\u00FC\u00E9\u00E2\u00E4\u00E0\u00E5\u00E7\u00EA\u00EB\u00E8\u00EF\u00EE\u00EC\u00C4\u00C5" +
        "\u00C9\u00E6\u00C6\u00F4\u00F6\u00F2\u00FB\u00F9\u00FF\u00D6\u00DC\u00A2\u00A3\u00A5\u20A7\u0192" +
        "\u00E1\u00ED\u00F3\u00FA\u00F1\u00D1\u00AA\u00BA\u00BF\u2310\u00AC\u00BD\u00BC\u00A1\u00AB\u00BB" +
        "\u2591\u2592\u2593\u2502\u2524\u2561\u2562\u2556\u2555\u2563\u2551\u2557\u255D\u255C\u255B\u2510" +
        "\u2514\u2534\u252C\u251C\u2500\u253C\u255E\u255F\u255A\u2554\u2569\u2566\u2560\u2550\u256C\u2567" +
        "\u2568\u2564\u2565\u2559\u2558\u2552\u2553\u256B\u256A\u2518\u250C\u2588\u2584\u258C\u2590\u2580" +
        "\u03B1\u00DF\u0393\u03C0\u03A3\u03C3\u00B5\u03C4\u03A6\u0398\u03A9\u03B4\u221E\u03C6\u03B5\u2229" +
        "\u2261\u00B1\u2265\u2264\u2320\u2321\u00F7\u2248\u00B0\u2219\u00B7\u221A\u207F\u00B2\u25A0\u0020";
}

public sealed class OwlosuiException : Exception
{
    public OwlosuiException(string message) : base(message) { }
}
