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
            // The server needs no console: its stdio is these pipes. Without
            // this a program built as a Windows application would start it
            // with a black console window of its own.
            CreateNoWindow = true,
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

    /// <summary>
    /// A viewer becomes an editor, or the other way round. The window's
    /// title says <c>[view]</c> while it is one; the colour does not
    /// change, as it did not in the file managers people learned on.
    /// </summary>
    public void SetReadOnly(ushort text, bool on) => Call(Op.SetReadOnly, W.U16(text), new[] { (byte)(on ? 1 : 0) });

    /// <summary>
    /// What an editor offers, for <see cref="Editor"/>: each a whole feature
    /// the core then runs by itself while the editor's window is active -
    /// its items in the Edit menu, its keys, and for Find and Replace the
    /// dialogs. None of them sends the program a command.
    /// </summary>
    [Flags]
    public enum Offer : byte
    {
        None = 0,
        Edit = 1,        // Undo, Redo, Cut, Copy, Paste, Select all
        Find = 2,        // Find... (Ctrl+F) and Find next (Ctrl+L)
        Replace = 4,     // Replace... (Ctrl+H)
        Wrap = 8,        // Word wrap, ticked while on
        ReadOnly = 16,   // Read only, ticked while on
        Hex = 32,        // the same text as bytes
        Keys = 64,       // Classic keys: the WordStar arrangement
        Syntax = 128,    // Syntax: the language the text is coloured as
        All = 255,
    }

    /// <summary>How an editor is set, and where its caret is: line and column from 0.</summary>
    public readonly record struct EditorState(Offer Offers, bool Wrap, bool ReadOnly, bool Classic, bool Hex, bool Syntax, int Line, int Col);

    /// <summary>
    /// Colour a text as a language: its name ("Pascal"), an extension ("PAS")
    /// or a file name ("DEMO.PAS"). The language's name, or null when there
    /// is none - and then the text is plain.
    /// </summary>
    public string? Syntax(ushort text, string language)
    {
        var r = Call(Op.Syntax, W.U16(text), W.Str(language));
        return r[0] != 0 ? R.Str(r, 1) : null;
    }

    /// <summary>Languages of the program's own, in the format of lib/core/src/syntax.ini; how many.</summary>
    public int DefineSyntax(string ini) => Call(Op.SyntaxDefine, W.Str(ini))[0];

    /// <summary>
    /// What a text offers and how it starts. The person can change each
    /// setting from the Edit menu; <see cref="GetEditor"/> reads them back.
    /// </summary>
    public void Editor(ushort text, Offer offers, bool wrap = false, bool readOnly = false, bool classic = false, bool hex = false)
    {
        var state = (byte)((wrap ? 1 : 0) | (readOnly ? 2 : 0) | (classic ? 4 : 0) | (hex ? 8 : 0));
        Call(Op.Editor, W.U16(text), new[] { (byte)offers, state });
    }

    public EditorState GetEditor(ushort text)
    {
        var r = Call(Op.GetEditor, W.U16(text));
        var s = r[1];
        return new EditorState((Offer)r[0], (s & 1) != 0, (s & 2) != 0, (s & 4) != 0, (s & 8) != 0, (s & 16) != 0, R.U16(r, 2), R.U16(r, 4));
    }

    /// <summary>
    /// What an input line has been given before, newest first. Down, or
    /// the ▼ at the field's end, lists them; a pick fills the field. Enter
    /// in the field adds to the list.
    /// </summary>
    public void SetHistory(ushort input, params string[] items)
    {
        var v = new List<byte> { (byte)Math.Min(items.Length, 255) };
        foreach (var it in items.Take(255)) v.AddRange(W.Str(it));
        Call(Op.SetHistory, W.U16(input), v.ToArray());
    }

    /// <summary>
    /// Find the next match after the caret in a text view and select it.
    /// False when there is none; the search does not wrap.
    /// </summary>
    public bool Find(ushort text, string pattern, bool caseSensitive = false, bool wholeWord = false)
    {
        var flags = (byte)((caseSensitive ? 1 : 0) | (wholeWord ? 2 : 0));
        return Call(Op.Find, W.U16(text), new[] { flags }, W.Str(pattern))[0] != 0;
    }

    /// <summary>Replace the selected match and find the next: (replaced, found).</summary>
    public (bool replaced, bool found) Replace(ushort text, string pattern, string with, bool caseSensitive = false, bool wholeWord = false)
    {
        var flags = (byte)((caseSensitive ? 1 : 0) | (wholeWord ? 2 : 0));
        var r = Call(Op.Replace, W.U16(text), new[] { flags }, W.Str(pattern), W.Str(with));
        return (r[0] != 0, r[1] != 0);
    }

    /// <summary>Replace every match; how many.</summary>
    public int ReplaceAll(ushort text, string pattern, string with, bool caseSensitive = false, bool wholeWord = false)
    {
        var flags = (byte)((caseSensitive ? 1 : 0) | (wholeWord ? 2 : 0));
        return R.U16(Call(Op.ReplaceAll, W.U16(text), new[] { flags }, W.Str(pattern), W.Str(with)));
    }

    /// <summary>
    /// A node of a tree. <c>Lazy</c> says children exist but are given
    /// only when the node is opened - <see cref="TreeExpand"/> then names
    /// it and <see cref="TreeChildren"/> fills it. A disk is not read whole.
    /// </summary>
    public sealed record TreeNode(string Text, TreeNode[]? Children = null, bool Open = false, bool Lazy = false);

    /// <summary>A tree filling its window.</summary>
    public ushort Tree(ushort parent, params TreeNode[] nodes) =>
        R.U16(Call(Op.Tree, W.U16(parent), W.Rect(0, 0, 0, 0), W.Nodes(nodes)));

    /// <summary>The children of the node at <paramref name="path"/>; the node opens.</summary>
    public void TreeChildren(ushort tree, int[] path, params TreeNode[] nodes)
    {
        var v = new List<byte> { (byte)path.Length };
        foreach (var i in path) v.AddRange(W.U16((ushort)i));
        Call(Op.TreeChildren, W.U16(tree), v.ToArray(), W.Nodes(nodes));
    }

    /// <summary>
    /// The lazy node somebody opened, once: its path as indices and as
    /// the texts along it, or null. Answer with <see cref="TreeChildren"/>.
    /// </summary>
    public (int[] path, string[] texts)? TreeExpand(ushort tree)
    {
        var r = Call(Op.TreeExpand, W.U16(tree));
        var n = r[0];
        if (n == 0) return null;
        var path = new int[n];
        var texts = new string[n];
        var at = 1;
        for (var i = 0; i < n; i++)
        {
            path[i] = r[at] | (r[at + 1] << 8);
            at += 2;
            var len = r[at] | (r[at + 1] << 8);
            texts[i] = System.Text.Encoding.UTF8.GetString(r, at + 2, len);
            at += 2 + len;
        }
        return (path, texts);
    }

    /// <summary>The texts from the root down to the tree's current row.</summary>
    public string[] TreePath(ushort tree)
    {
        var r = Call(Op.TreePath, W.U16(tree));
        var n = r[0];
        var texts = new string[n];
        var at = 1;
        for (var i = 0; i < n; i++)
        {
            var len = r[at] | (r[at + 1] << 8);
            texts[i] = System.Text.Encoding.UTF8.GetString(r, at + 2, len);
            at += 2 + len;
        }
        return texts;
    }

    /// <summary>One colour of the palette: its group, its name, and the attribute it wears.</summary>
    public readonly record struct PaletteEntry(string Group, string Name, byte Attr);

    /// <summary>
    /// Every colour the toolkit draws with, by role, in the order
    /// <see cref="SetColor"/> indexes. Colour comes from the palette by
    /// role, never from the element, so changing an entry here changes
    /// everything that plays that role.
    /// </summary>
    public PaletteEntry[] Palette()
    {
        var r = Call(Op.Palette);
        var n = r[0];
        var items = new PaletteEntry[n];
        var at = 1;
        string Str()
        {
            var len = r[at] | (r[at + 1] << 8);
            var s = System.Text.Encoding.UTF8.GetString(r, at + 2, len);
            at += 2 + len;
            return s;
        }
        for (var i = 0; i < n; i++)
        {
            var group = Str();
            var name = Str();
            items[i] = new PaletteEntry(group, name, r[at++]);
        }
        return items;
    }

    /// <summary>Change one colour of the palette; the next frame wears it.</summary>
    public void SetColor(int index, byte attr) => Call(Op.SetColor, new[] { (byte)index, attr });

    /// <summary>The history of an input line, newest first, to keep for next time.</summary>
    public string[] GetHistory(ushort input)
    {
        var r = Call(Op.GetHistory, W.U16(input));
        var n = r[0];
        var items = new string[n];
        var at = 1;
        for (var i = 0; i < n; i++)
        {
            var len = r[at] | (r[at + 1] << 8);
            items[i] = System.Text.Encoding.UTF8.GetString(r, at + 2, len);
            at += 2 + len;
        }
        return items;
    }

    /// <summary>
    /// A hex dump filling its window: offsets, bytes, characters. At most
    /// 64K of bytes cross the wire in one call; send the part to be seen.
    /// </summary>
    public ushort Hex(ushort parent, byte[] bytes)
    {
        if (bytes.Length > 60000) bytes = bytes[..60000];
        var r = Call(Op.Hex, W.U16(parent), W.Rect(0, 0, 0, 0), bytes);
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
    public enum ButtonStyle : byte { Normal = 0, Accent = 1, Danger = 2 }

    /// <summary>
    /// A button: its label with the hotkey between tildes, the command it
    /// sends, whether Enter presses it (<c>Default</c>) and whether Escape
    /// does (<c>Cancel</c>), its colour and whether it can be pressed.
    /// </summary>
    public readonly record struct Button(string Label, ushort Cmd, bool Default = false,
                                         ButtonStyle Style = ButtonStyle.Normal, bool Enabled = true,
                                         bool Cancel = false)
    {
        public static implicit operator Button((string label, ushort cmd) t) => new(t.label, t.cmd);
        public static implicit operator Button((string label, ushort cmd, ButtonStyle style) t) => new(t.label, t.cmd, Style: t.style);
    }

    /// <summary>
    /// A row of buttons placed by hand, from <paramref name="x"/>,
    /// <paramref name="y"/>: one row of a keypad. Enter finds a Default in
    /// it; a click presses without moving the focus. Two rows tall (the
    /// second is the shadow).
    /// </summary>
    public ushort ButtonRow(ushort parent, int x, int y, params Button[] buttons) =>
        ButtonRow(parent, x, y, true, buttons);

    /// <summary>
    /// The same row, and whether Tab stops at it. A keypad says no: it is
    /// pressed with the mouse or with Enter, and the keys typed go on to
    /// the display without a detour through thirty buttons.
    /// </summary>
    public ushort ButtonRow(ushort parent, int x, int y, bool selectable, params Button[] buttons)
    {
        var flags = (byte)(selectable ? 0 : 1);
        var r = Call(Op.ButtonRow, W.U16(parent), W.Rect(x, y, 0, 2), new[] { flags }, W.Buttons(buttons, implyDefault: false));
        return R.U16(r);
    }

    /// <summary>Put the focus on one control of a window.</summary>
    public void Focus(ushort id) => Call(Op.Focus, W.U16(id));

    /// <summary>Turn one button of a row on or off, by its place in the row.</summary>
    public void EnableButton(ushort row, int index, bool on) =>
        Call(Op.SetButton, W.U16(row), new[] { (byte)index, (byte)(on ? 1 : 0) });

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
    public ushort StatusLine(params StatusItem[] items) => R.U16(Call(Op.Status, StatusBytes(items)));

    /// <summary>
    /// The keys a window carries: shown on the status line, and bound,
    /// only while that window is the active one. An editor's F4 does not
    /// belong to the program - it belongs to the editor, and it goes when
    /// the editor is behind something else.
    /// </summary>
    public void WindowStatus(ushort window, params StatusItem[] items) =>
        Call(Op.WindowStatus, W.U16(window), StatusBytes(items));

    /// <summary>
    /// The menus a window carries, on the bar only while it is active. A
    /// submenu named like one already on the bar (<c>"~O~ptions"</c>) puts
    /// its items into that menu after a line; any other becomes a new
    /// menu at the end.
    /// </summary>
    public void WindowMenu(ushort window, params MenuItem[] menus) =>
        Call(Op.WindowMenu, W.U16(window), W.Menu(menus));

    private static byte[] StatusBytes(StatusItem[] items)
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
        return v.ToArray();
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
                        bool multi = false, bool pathLabel = true, bool pathLine = true, int top = 0,
                        bool detailsOnly = false)
    {
        // The panel fills the window; `top` rows are left free above it.
        // detailsOnly: a one-row foot of what the cursor is on - size, date,
        // attributes - without the path or the name, for a commander that
        // puts the folder in its window's title (SetText on the window).
        var flags = (byte)((multi ? 1 : 0) | (pathLabel ? 0 : 2) | (pathLine ? 0 : 4) | (detailsOnly ? 8 : 0));
        var parts = W.EntryChunks(entries);
        var r = Call(Op.Files, W.U16(parent), W.Rect(0, top, 0, 0), new[] { flags },
                     W.Str(mask), W.Str(Path.Combine(path, mask)), parts[0]);
        var id = R.U16(r);
        foreach (var more in parts.Skip(1)) Call(Op.AddFiles, W.U16(id), more);
        return id;
    }

    /// <summary>A new listing for a panel: the person went somewhere else, or something changed.</summary>
    /// <remarks>
    /// A request is at most 64K, and a folder of a few thousand names is
    /// more: the listing goes in pieces, the first with the request and the
    /// rest added to it. Whoever calls this never sees the seams.
    /// </remarks>
    public void SetFiles(ushort id, string path, IEnumerable<FileEntry> entries, string mask = "*.*")
    {
        var parts = W.EntryChunks(entries);
        Call(Op.SetFiles, W.U16(id), W.Str(Path.Combine(path, mask)), W.Str(mask), parts[0]);
        foreach (var more in parts.Skip(1)) Call(Op.AddFiles, W.U16(id), more);
    }

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

    /// <summary>Every mark off a panel: the marked files were copied, moved or deleted.</summary>
    public void Unmark(ushort id) => Call(Op.Unmark, W.U16(id));

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
    /// <summary>
    /// A menu item: its label with the hotkey between tildes, the command,
    /// the shortcut shown at the right, and a <c>Hint</c> - one line that
    /// the status line shows while the cursor stands on the item.
    /// </summary>
    public sealed record MenuItem(string Label, ushort Cmd, string Shortcut = "", bool Checked = false,
                                  bool Enabled = true, bool Separator = false, MenuItem[]? Items = null,
                                  string Hint = "")
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

    /// <summary>The front window goes to the back: the classic F6.</summary>
    public void NextWindow() => Call(Op.Cycle);

    /// <summary>The window at the back comes to the front: Shift+F6.</summary>
    public void PreviousWindow() => Call(Op.CycleBack);

    /// <summary>The list of windows, a modal dialog; Enter brings the chosen one to the front: Alt+0.</summary>
    public void WindowList() => Call(Op.WindowList);

    /// <summary>
    /// Move or resize the active window from the keyboard: arrows move,
    /// Shift+arrows resize, Enter keeps, Escape puts it back. Ctrl+F5.
    /// </summary>
    public void SizeMove() => Call(Op.SizeMove);

    /// <summary>The windows along the diagonal, every title bar showing.</summary>
    public void Cascade() => Call(Op.Cascade);

    /// <summary>The windows share the desktop in cells; a fixed-size one stands in its cell at its own size.</summary>
    public void Tile() => Call(Op.Tile);

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
    /// A console filling its window: text that keeps arriving, coloured by
    /// the ANSI sequences in it. It keeps <paramref name="scrollback"/>
    /// lines (0, a thousand), folds long ones to its width, and follows the
    /// newest line until it is scrolled back; End follows again. Not called
    /// Console: inside this class that name is System.Console's.
    /// </summary>
    public ushort ConsoleView(ushort parent, int scrollback = 0) =>
        R.U16(Call(Op.Console, W.U16(parent), W.Rect(0, 0, 0, 0), W.U16((ushort)scrollback)));

    /// <summary>
    /// Text at the end of a console. SGR colours, CR, tab, backspace,
    /// erase-line and clear are obeyed and never shown; a sequence cut in
    /// two between calls is still one. <see cref="GetText"/> gives the
    /// whole record back without its colours.
    /// </summary>
    public void ConsoleWrite(ushort id, string text)
    {
        // A request carries 64K; a character is at most four bytes of UTF-8.
        for (int i = 0; i < text.Length; i += 16000)
            Call(Op.ConsoleWrite, W.U16(id), W.Str(text.Substring(i, Math.Min(16000, text.Length - i))));
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

    public enum MouseKind : byte { Down = 0, Up = 1, Drag = 2, Move = 3, WheelUp = 4, WheelDown = 5, Double = 6 }
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

    /// <summary>
    /// Two clicks on a cell, the second one a double. A title bar zooms,
    /// a file name is chosen as Enter would choose it, a list item presses
    /// the default button.
    /// </summary>
    public void DoubleClick(int x, int y)
    {
        Click(x, y);
        SendMouse(MouseKind.Double, x, y);
        SendMouse(MouseKind.Up, x, y);
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
        if (InWindow)
        {
            RunWindow(onCommand, afterInput);
            return;
        }
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
                    // happen. The classic menus did the same for an item.
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

    // ---------------------------------------------------------- the window

    /// <summary>Whether <see cref="OpenWindow"/> has moved the desktop into a window.</summary>
    public bool InWindow { get; private set; }

    /// <summary>
    /// Show the desktop in a native window instead of the console. The
    /// window is the server's: it draws the cells and reads the keys and
    /// the mouse itself (lib/window, the same code a Rust program links),
    /// so this program draws nothing. Call it once, before
    /// <see cref="Run(Func{ushort, bool}, Action?)"/>; everything else - windows,
    /// buttons, commands - is unchanged. Build the program as
    /// <c>&lt;OutputType&gt;WinExe&lt;/OutputType&gt;</c> and there is no
    /// console at all. Windows only.
    /// </summary>
    /// <param name="activate">False opens it without taking the focus: for a
    /// test or an agent, so a person typing elsewhere keeps typing there.</param>
    public void OpenWindow(string title = "OWLOSUI", bool activate = true)
    {
        Call(Op.OpenWindow, W.Str(title), new[] { (byte)(activate ? 0 : 1) });
        InWindow = true;
    }

    /// <summary>
    /// Run's loop when the window is the screen: nothing to draw, and WAIT
    /// in place of reading the console. WAIT answers after one key or click
    /// has been through the core, with what it caused and the desktop's
    /// size - the person may have resized the window.
    /// </summary>
    private void RunWindow(Func<ushort, bool> onCommand, Action? afterInput)
    {
        while (true)
        {
            var r = Call(Op.Wait);
            ushort pressed = R.U16(r, 0), command = R.U16(r, 2);
            Width = R.I16(r, 4);
            Height = R.I16(r, 6);
            if (pressed != 0 && !onCommand(pressed)) return;
            if (command != 0 && !onCommand(command)) return;
            afterInput?.Invoke();
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

            private long lastDown;
            private (int x, int y, MouseButton b) lastDownAt;

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
                    if (kind == MouseKind.Down)
                    {
                        // The console says "double" itself; a terminal that
                        // does not is timed: a second press on the same cell
                        // within half a second is the same gesture.
                        var t = Environment.TickCount64;
                        var quick = (m.dwEventFlags & DOUBLE_CLICK) != 0
                                    || (t - lastDown < 500 && lastDownAt == (x, y, button));
                        lastDown = quick ? 0 : t;
                        lastDownAt = (x, y, button);
                        if (quick) kind = MouseKind.Double;
                    }
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
        public const byte Status = 0x16, Label = 0x17, Progress = 0x18, List = 0x19, Files = 0x1A, Canvas = 0x1B, MenuBar = 0x1C, Cluster = 0x1D, ButtonRow = 0x1E, Hex = 0x1F;
        public const byte Close = 0x20, GetText = 0x21, SetProgress = 0x22, GetMarked = 0x23, GetCurrent = 0x24;
        public const byte SetFiles = 0x25, TakeFiles = 0x26, Activate = 0x27, MarkedNames = 0x28, SetFilesError = 0x29, Active = 0x2A;
        public const byte SetText = 0x2B, Blit = 0x2C, MenuCheck = 0x2D, GetCluster = 0x2E, GetClick = 0x2F;
        public const byte Cycle = 0x43, Zoom = 0x44, SetButton = 0x45, Focus = 0x46, Cascade = 0x47, Tile = 0x48, SetReadOnly = 0x49;
        public const byte WindowStatus = 0x4A, WindowMenu = 0x4B, WindowList = 0x4C, CycleBack = 0x4D, SizeMove = 0x4E;
        public const byte SetHistory = 0x4F, GetHistory = 0x50, Palette = 0x51, SetColor = 0x52, AddFiles = 0x5A;
        public const byte Tree = 0x53, TreeChildren = 0x54, TreeExpand = 0x55, TreePath = 0x56;
        public const byte Find = 0x57, Replace = 0x58, ReplaceAll = 0x59, Editor = 0x5D, GetEditor = 0x5E, Syntax = 0x5F, SyntaxDefine = 0x60, Unmark = 0x61;
        public const byte OpenWindow = 0x5B, Wait = 0x5C;
        public const byte Console = 0x67, ConsoleWrite = 0x68;
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
        // A payload is at most 64K: its length is a u16 on the wire. Sent
        // anyway, the length would wrap and the server would read the rest
        // as requests of its own - and a stray 0x00 is QUIT.
        if (len > ushort.MaxValue)
            throw new OwlosuiException($"op {op:X2}: {len} bytes is more than one request can carry (65535)");
        var head = new byte[3];
        head[0] = op;
        BitConverter.TryWriteBytes(head.AsSpan(1), (ushort)len);
        try
        {
            toServer.Write(head);
            foreach (var p in parts) toServer.Write(p);
            toServer.Flush();
        }
        catch (IOException)
        {
            // The server is gone already. What it last said - a Rust panic
            // names its file and line - is worth more than "pipe ended".
            proc.WaitForExit(500);
            throw new OwlosuiException("owlosui-serve went away" + (stderr.Length > 0 ? ":\n" + stderr : ""));
        }

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
    /// <summary>
    /// Where the core is, in this order:
    ///
    ///   1. OWLOSUI_SERVE in the environment - a person who says where it
    ///      is, is right;
    ///   2. beside the program - a folder that ships the two side by side;
    ///   3. the repository's own cargo build, walking up from the program:
    ///      target\debug or target\release, whichever was built last, so
    ///      `cargo build` after a change is seen at once and a stale release
    ///      build never hides a fresh debug one;
    ///   4. the copy inside this assembly, written to the temp folder -
    ///      what a published single .exe runs on;
    ///   5. the PATH.
    /// </summary>
    private static string FindServer()
    {
        var env = Environment.GetEnvironmentVariable("OWLOSUI_SERVE");
        if (!string.IsNullOrEmpty(env)) return env;

        var exe = OperatingSystem.IsWindows() ? "owlosui-serve.exe" : "owlosui-serve";
        var beside = Path.Combine(AppContext.BaseDirectory, exe);
        if (File.Exists(beside)) return beside;

        for (var dir = new DirectoryInfo(AppContext.BaseDirectory); dir != null; dir = dir.Parent)
        {
            var built = new[] { "debug", "release" }
                .Select(profile => new FileInfo(Path.Combine(dir.FullName, "target", profile, exe)))
                .Where(f => f.Exists)
                .OrderByDescending(f => f.LastWriteTimeUtc)
                .FirstOrDefault();
            if (built != null) return built.FullName;
        }

        if (Unpack(exe) is { } unpacked) return unpacked;

        // Let the OS search the PATH; a clear message if it cannot.
        return "owlosui-serve";
    }

    /// <summary>
    /// The server carried inside this assembly, written out where it can be
    /// run: %TEMP%\owlosui\&lt;hash&gt;\owlosui-serve.exe. The folder is named
    /// by the bytes, so two versions never overwrite each other, and a copy
    /// already there - perhaps running - is used as it is.
    /// </summary>
    private static string? Unpack(string exe)
    {
        using var res = typeof(Owlosui).Assembly.GetManifestResourceStream("owlosui-serve.exe");
        if (res == null) return null;
        var bytes = new byte[res.Length];
        res.ReadExactly(bytes);
        var hash = Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(bytes))[..16];
        var dir = Path.Combine(Path.GetTempPath(), "owlosui", hash);
        var path = Path.Combine(dir, exe);
        if (File.Exists(path) && new FileInfo(path).Length == bytes.Length) return path;
        Directory.CreateDirectory(dir);
        // Written beside and moved into place, so a second program starting
        // at the same moment never runs half a file.
        var tmp = path + "." + Environment.ProcessId + ".tmp";
        File.WriteAllBytes(tmp, bytes);
        try { File.Move(tmp, path, overwrite: false); }
        catch (IOException) { File.Delete(tmp); }
        return path;
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
                v.AddRange(Str(it.Hint));
                v.AddRange(Menu(it.Items ?? Array.Empty<MenuItem>()));
            }
            return v.ToArray();
        }

        public static byte[] Nodes(TreeNode[] nodes)
        {
            var v = new List<byte> { (byte)Math.Min(nodes.Length, 255) };
            foreach (var n in nodes.Take(255))
            {
                v.Add((byte)((n.Open ? 1 : 0) | (n.Lazy ? 2 : 0)));
                v.AddRange(Str(n.Text));
                v.AddRange(Nodes(n.Children ?? Array.Empty<TreeNode>()));
            }
            return v.ToArray();
        }

        /// <summary>
        /// A listing as pieces that each fit in one request with room to
        /// spare: 60000 bytes of entries at most, each piece its own count.
        /// Always at least one piece, empty if the folder is.
        /// </summary>
        public static List<byte[]> EntryChunks(IEnumerable<FileEntry> entries)
        {
            var parts = new List<byte[]>();
            var piece = new List<FileEntry>();
            var size = 0;
            foreach (var e in entries)
            {
                var n = Encoding.UTF8.GetByteCount(e.Name) + 2 + 4 + 7;
                if (piece.Count > 0 && (size + n > 60000 || piece.Count == ushort.MaxValue))
                {
                    parts.Add(Entries(piece));
                    piece.Clear();
                    size = 0;
                }
                piece.Add(e);
                size += n;
            }
            parts.Add(Entries(piece));
            return parts;
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
        /// <summary>
        /// A docked row makes its first button the default when none is
        /// marked - Enter in a one-button box presses the one button. A
        /// placed row does not: a keypad with a default in every row would
        /// give Enter six answers, and the first would be Clear.
        /// </summary>
        public static byte[] Buttons(Button[] buttons, bool implyDefault = true)
        {
            if (buttons.Length == 0) throw new ArgumentException("a button row needs at least one button");
            var dflt = Array.FindIndex(buttons, b => b.Default);
            if (dflt < 0 && implyDefault) dflt = 0;
            var v = new List<byte> { (byte)buttons.Length };
            for (var i = 0; i < buttons.Length; i++)
            {
                if (buttons[i].Cmd == 0) throw new ArgumentException($"button '{buttons[i].Label}' has command 0, which means none");
                v.AddRange(U16(buttons[i].Cmd));
                var flags = (i == dflt ? 1 : 0) | ((int)buttons[i].Style << 1) | (buttons[i].Enabled ? 0 : 8) | (buttons[i].Cancel ? 16 : 0);
                v.Add((byte)flags);
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
