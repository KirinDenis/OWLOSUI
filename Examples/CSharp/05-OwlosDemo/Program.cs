// OWLOS UI Demo: everything the toolkit has, in one program.
//
// The shape is the one every text-mode toolkit's demo has had since the
// eighties, because it is the right shape: a menu bar, a status line, a
// desktop, and a set of small windows that each use one part of the kit.
// Nothing here is copied from anyone; the list of windows is a checklist.
//
//   File      Open a file into a viewer; Exit.
//   Tools     Calculator, Calendar, ASCII table, Puzzle.
//   Options   A dialog of check boxes and radio buttons; a ticked option.
//   Window    Next, Zoom, Close - the desktop's own verbs.
//   Help      About.
//
// What each window shows of the kit:
//
//   Calculator   an input line, a canvas keypad the mouse can press, buttons
//   Calendar     a canvas drawn once a month, two buttons that redraw it
//   ASCII table  a canvas of every glyph; a click says which one
//   Puzzle       a canvas game; clicks move tiles, the program keeps score
//   Options      clusters and labels in a modal dialog, read back on OK
//   Open         the file panel, and a read-only text window for the file
//
// Every window is an ordinary window: drag it, resize it, zoom it, put it
// behind another. Alt+X or Exit leaves; nothing here has anything to save.
//
// It is a class so that `Examples/CSharp/Tests` can open every window by the
// wire and press its keys.

using System.Globalization;
using System.Text;

namespace OwlosDemo;

public sealed class App
{
    // Commands. The program's own numbers; 0 means none.
    public const ushort CmOpen = 1, CmExit = 2;
    public const ushort CmCalc = 10, CmCalendar = 11, CmAscii = 12, CmPuzzle = 13;
    public const ushort CmOptions = 20, CmClock = 21;
    public const ushort CmNext = 30, CmZoom = 31, CmClose = 32;
    public const ushort CmAbout = 40, CmHelp = 41;
    // The windows' own buttons.
    public const ushort CmCalcEval = 50, CmCalcClear = 51;
    public const ushort CmCalPrev = 52, CmCalNext = 53;
    public const ushort CmScramble = 54;
    public const ushort CmOk = 60, CmCancel = 61, CmDismiss = 62;
    public const ushort CmFileOpen = 63;

    private readonly Owlosui owl;

    // The windows, while they are open. Zero is closed.
    public ushort Calculator, Calendar, Ascii, Puzzle, Options, OpenDialog, Viewer;
    private ushort calcInput, calcKeys, calDays, calTitle, asciiGrid, asciiCaption, puzzleBoard, puzzleCaption;
    private ushort optChecks, optRadio, files, box;
    private DateTime month = new(DateTime.Today.Year, DateTime.Today.Month, 1);
    private readonly int[] tiles = Enumerable.Range(1, 15).Append(0).ToArray();
    private int puzzleMoves;
    private bool clock = true;
    private string openDir = Directory.GetCurrentDirectory();

    public App(Owlosui owl)
    {
        this.owl = owl;

        owl.MenuBar(
            Owlosui.MenuItem.Sub("~F~ile",
                ("~O~pen...", CmOpen, "F3"),
                Owlosui.MenuItem.Line(),
                ("E~x~it", CmExit, "Alt+X")),
            Owlosui.MenuItem.Sub("~T~ools",
                ("~C~alculator", CmCalc),
                ("Ca~l~endar", CmCalendar),
                ("~A~SCII table", CmAscii),
                ("~P~uzzle", CmPuzzle)),
            Owlosui.MenuItem.Sub("~O~ptions",
                ("~M~ouse...", CmOptions),
                new Owlosui.MenuItem("~C~lock on status line", CmClock, Checked: clock)),
            Owlosui.MenuItem.Sub("~W~indow",
                ("~N~ext", CmNext, "F6"),
                ("~Z~oom", CmZoom, "F5"),
                ("~C~lose", CmClose, "Alt+F3")),
            Owlosui.MenuItem.Sub("~H~elp",
                ("~A~bout", CmAbout)));

        owl.StatusLine(new Owlosui.StatusItem("~F1~ Help", CmHelp, ConsoleKey.F1),
                       new Owlosui.StatusItem("~F3~ Open", CmOpen, ConsoleKey.F3),
                       new Owlosui.StatusItem("~F5~ Zoom", CmZoom, ConsoleKey.F5),
                       new Owlosui.StatusItem("~F6~ Next", CmNext, ConsoleKey.F6),
                       new Owlosui.StatusItem("~Alt-F3~ Close", CmClose, ConsoleKey.F3, Alt: true),
                       new Owlosui.StatusItem("~Alt-X~ Exit", CmExit, ConsoleKey.X, Alt: true));
    }

    /// <summary>True to keep running, false to leave.</summary>
    public bool OnCommand(ushort cmd)
    {
        switch (cmd)
        {
            case CmExit: return false;
            case CmHelp:
            case CmAbout:
                Tell("OWLOS UI Demo", "A Turbo Vision-shaped toolkit with one portable core: this program is C#, " +
                     "every window in it is drawn by a Rust core behind a pipe, and the same core is " +
                     "meant for a terminal, a browser and a DOS machine.");
                return true;
            case CmDismiss:
                CloseBox();
                return true;

            case CmNext: owl.NextWindow(); return true;
            case CmZoom: { var a = owl.Active(); if (a != 0) owl.Zoom(a); return true; }
            case CmClose: { var a = owl.Active(); if (a != 0) Closed(a); return true; }

            case CmClock:
                clock = !clock;
                owl.MenuCheck(CmClock, clock);
                return true;

            case CmCalc: ShowCalculator(); return true;
            case CmCalcEval: Evaluate(); return true;
            case CmCalcClear: owl.SetText(calcInput, ""); return true;

            case CmCalendar: ShowCalendar(); return true;
            case CmCalPrev: month = month.AddMonths(-1); DrawCalendar(); return true;
            case CmCalNext: month = month.AddMonths(1); DrawCalendar(); return true;

            case CmAscii: ShowAscii(); return true;

            case CmPuzzle: ShowPuzzle(); return true;
            case CmScramble: Scramble(); DrawPuzzle(); return true;

            case CmOptions: ShowOptions(); return true;
            case CmOk:
            {
                var (on, _) = owl.ClusterState(optChecks);
                var (speed, _) = owl.ClusterState(optRadio);
                var which = Array.IndexOf(speed, true);
                owl.Close(Options);
                Options = 0;
                Tell("Options", $"Reverse buttons: {(on[0] ? "on" : "off")}. Show cursor: {(on[1] ? "on" : "off")}. " +
                                $"Speed: {new[] { "slow", "medium", "fast" }[Math.Max(0, which)]}.");
                return true;
            }
            case CmCancel:
                if (Options != 0) { owl.Close(Options); Options = 0; }
                if (OpenDialog != 0) { owl.Close(OpenDialog); OpenDialog = 0; }
                return true;

            case CmOpen: ShowOpen(); return true;
            case CmFileOpen:
            {
                // The button: whatever is under the cursor in the panel.
                var names = owl.MarkedNames(files);
                if (names.Length == 1) Chosen(names[0]);
                return true;
            }

            default:
                return true;
        }
    }

    /// <summary>What is not a command: clicks on canvases, a name entered in the file panel.</summary>
    public void Poll()
    {
        if (Calculator != 0 && owl.CanvasClick(calcKeys) is { } k)
        {
            // The keypad is four columns of three cells: which key was hit.
            var col = k.x / 4;
            var row = k.y;
            if (col < 4 && row < 4)
            {
                var key = Keypad[row][col];
                if (key == '=') Evaluate();
                else if (key == 'C') owl.SetText(calcInput, "");
                else owl.SetText(calcInput, owl.GetText(calcInput) + key);
            }
        }
        if (Ascii != 0 && owl.CanvasClick(asciiGrid) is { } a)
        {
            var code = a.y * 32 + a.x;
            if (code < 256)
                owl.SetText(asciiCaption, $"Glyph {code}  dec {code}  hex {code:X2}  oct {Convert.ToString(code, 8)}");
        }
        if (Puzzle != 0 && owl.CanvasClick(puzzleBoard) is { } p)
        {
            var (col, row) = (p.x / 4, p.y / 2);
            if (col < 4 && row < 4 && Slide(row * 4 + col)) DrawPuzzle();
        }
        if (OpenDialog != 0)
        {
            var (kind, text) = owl.TakeFiles(files);
            if (kind == Owlosui.FilesEvent.Chosen) Chosen(text);
            else if (kind == Owlosui.FilesEvent.Path) GoTo(text);
        }
    }

    // ------------------------------------------------------------ calculator

    private static readonly string[] Keypad = { "789/", "456*", "123-", "C0=+" };

    private void ShowCalculator()
    {
        if (Calculator != 0) { owl.Activate(Calculator); return; }
        Calculator = owl.Window("Calculator", 26, 10, style: Owlosui.Style.Dialog, closeCmd: CmClose);
        calcInput = owl.Input(Calculator, 1, 0, 22, "", "");
        // A keypad drawn on a canvas: three cells a key, a space between,
        // and the mouse finds the key by where it landed.
        calcKeys = owl.Canvas(Calculator, 1, 2, 15, 4);
        for (var row = 0; row < 4; row++)
            for (var col = 0; col < 4; col++)
                owl.Blit(calcKeys, col * 4, row, $" {Keypad[row][col]} ", Owlosui.Attr(ConsoleColor.Black, ConsoleColor.Cyan));
        owl.Buttons(Calculator, new Owlosui.Button("~=~", CmCalcEval, Default: true), ("~C~lear", CmCalcClear));
    }

    /// <summary>
    /// `12+34*2` left to right, no precedence - the four-function pocket
    /// calculator, which is what a text screen is the size of. Errors are
    /// words in the display, not exceptions.
    /// </summary>
    public static string Calculate(string expr)
    {
        var s = expr.Replace(" ", "");
        if (s.Length == 0) return "";
        double acc = 0;
        var op = '+';
        var i = 0;
        while (i < s.Length)
        {
            var start = i;
            while (i < s.Length && (char.IsDigit(s[i]) || s[i] == '.')) i++;
            if (start == i) return "Error";
            if (!double.TryParse(s[start..i], NumberStyles.Float, CultureInfo.InvariantCulture, out var n)) return "Error";
            acc = op switch
            {
                '+' => acc + n,
                '-' => acc - n,
                '*' => acc * n,
                '/' => n == 0 ? double.NaN : acc / n,
                _ => n,
            };
            if (i < s.Length)
            {
                op = s[i];
                if (op is not ('+' or '-' or '*' or '/')) return "Error";
                i++;
                if (i == s.Length) return "Error";
            }
        }
        if (double.IsNaN(acc)) return "Divide by zero";
        return acc.ToString("0.########", CultureInfo.InvariantCulture);
    }

    private void Evaluate() => owl.SetText(calcInput, Calculate(owl.GetText(calcInput)));

    // -------------------------------------------------------------- calendar

    private void ShowCalendar()
    {
        if (Calendar != 0) { owl.Activate(Calendar); return; }
        Calendar = owl.Window("Calendar", 26, 12, style: Owlosui.Style.Dialog, closeCmd: CmClose);
        calTitle = owl.Static(Calendar, 1, 0, "", 22);
        calDays = owl.Canvas(Calendar, 1, 1, 21, 7);
        owl.Buttons(Calendar, ("~<~", CmCalPrev), ("~>~", CmCalNext));
        DrawCalendar();
    }

    private void DrawCalendar()
    {
        owl.SetText(calTitle, month.ToString("MMMM yyyy", CultureInfo.InvariantCulture));
        var text = new char[21 * 7];
        var attrs = new byte[21 * 7];
        var plain = Owlosui.Attr(ConsoleColor.Black, ConsoleColor.Gray);
        Array.Fill(text, ' ');
        Array.Fill(attrs, plain);
        void Put(int x, int y, string s, byte a)
        {
            for (var i = 0; i < s.Length && x + i < 21; i++) { text[y * 21 + x + i] = s[i]; attrs[y * 21 + x + i] = a; }
        }
        Put(0, 0, "Su Mo Tu We Th Fr Sa", Owlosui.Attr(ConsoleColor.DarkBlue, ConsoleColor.Gray));
        var first = (int)month.DayOfWeek;
        var days = DateTime.DaysInMonth(month.Year, month.Month);
        for (var d = 1; d <= days; d++)
        {
            var slot = first + d - 1;
            var (x, y) = ((slot % 7) * 3, 1 + slot / 7);
            var today = month.Year == DateTime.Today.Year && month.Month == DateTime.Today.Month && d == DateTime.Today.Day;
            Put(x, y, d.ToString().PadLeft(2), today ? Owlosui.Attr(ConsoleColor.White, ConsoleColor.DarkGreen) : plain);
        }
        owl.Blit(calDays, 0, 0, 21, 7, text, attrs);
    }

    // ----------------------------------------------------------- ASCII table

    private void ShowAscii()
    {
        if (Ascii != 0) { owl.Activate(Ascii); return; }
        Ascii = owl.Window("ASCII table", 36, 12, style: Owlosui.Style.Dialog, closeCmd: CmClose);
        asciiGrid = owl.Canvas(Ascii, 1, 0, 32, 8);
        var text = new char[32 * 8];
        var attrs = new byte[32 * 8];
        for (var i = 0; i < 256; i++)
        {
            // The glyphs by their index in the session's font, whatever
            // they look like there: 0x01 is a face, 0xC4 a line.
            text[i] = i < owl.Glyphs.Length ? owl.Glyphs[i] : '?';
            attrs[i] = Owlosui.Attr(ConsoleColor.Black, ConsoleColor.Gray);
        }
        owl.Blit(asciiGrid, 0, 0, 32, 8, text, attrs);
        asciiCaption = owl.Static(Ascii, 1, 9, "Click a glyph.", 32);
    }

    // ---------------------------------------------------------------- puzzle

    private void ShowPuzzle()
    {
        if (Puzzle != 0) { owl.Activate(Puzzle); return; }
        Puzzle = owl.Window("Puzzle", 24, 13, style: Owlosui.Style.Dialog, closeCmd: CmClose);
        puzzleBoard = owl.Canvas(Puzzle, 1, 0, 16, 8);
        puzzleCaption = owl.Static(Puzzle, 1, 8, "", 20);
        owl.Buttons(Puzzle, ("~S~cramble", CmScramble));
        Scramble();
        DrawPuzzle();
    }

    /// <summary>The tile at a place, if it is beside the hole, slides into it.</summary>
    public bool Slide(int at)
    {
        var hole = Array.IndexOf(tiles, 0);
        var (r1, c1, r2, c2) = (at / 4, at % 4, hole / 4, hole % 4);
        if (Math.Abs(r1 - r2) + Math.Abs(c1 - c2) != 1) return false;
        (tiles[at], tiles[hole]) = (0, tiles[at]);
        puzzleMoves++;
        return true;
    }

    public bool Solved => tiles.Take(15).Select((t, i) => t == i + 1).All(x => x);

    private void Scramble()
    {
        // Two hundred legal slides from the solved board: always solvable,
        // unlike a shuffle. Seeded, so a test can know the board.
        for (var i = 0; i < 16; i++) tiles[i] = (i + 1) % 16;
        var rnd = new Random(47);
        for (var n = 0; n < 200; n++)
        {
            var hole = Array.IndexOf(tiles, 0);
            var (r, c) = (hole / 4, hole % 4);
            var moves = new List<int>();
            if (r > 0) moves.Add(hole - 4);
            if (r < 3) moves.Add(hole + 4);
            if (c > 0) moves.Add(hole - 1);
            if (c < 3) moves.Add(hole + 1);
            Slide(moves[rnd.Next(moves.Count)]);
        }
        puzzleMoves = 0;
    }

    private void DrawPuzzle()
    {
        var text = new char[16 * 8];
        var attrs = new byte[16 * 8];
        for (var i = 0; i < 16; i++)
        {
            var (r, c) = (i / 4, i % 4);
            var tile = tiles[i];
            var a = tile == 0 ? Owlosui.Attr(ConsoleColor.Gray, ConsoleColor.Gray)
                              : Owlosui.Attr(ConsoleColor.White, (tile % 2 == 0) ? ConsoleColor.DarkBlue : ConsoleColor.DarkCyan);
            var label = tile == 0 ? "    " : $" {tile,2} ";
            for (var dy = 0; dy < 2; dy++)
                for (var dx = 0; dx < 4; dx++)
                {
                    var at = (r * 2 + dy) * 16 + c * 4 + dx;
                    text[at] = dy == 0 ? label[dx] : ' ';
                    attrs[at] = a;
                }
        }
        owl.Blit(puzzleBoard, 0, 0, 16, 8, text, attrs);
        owl.SetText(puzzleCaption, Solved ? $"Solved in {puzzleMoves} moves!" : $"Moves: {puzzleMoves}");
    }

    // --------------------------------------------------------------- options

    private void ShowOptions()
    {
        if (Options != 0) { owl.Activate(Options); return; }
        Options = owl.Window("Mouse", 40, 12, style: Owlosui.Style.ModalDialog, closeCmd: CmCancel);
        owl.Label(Options, 1, 0, "~B~uttons:", 0);
        optChecks = owl.Cluster(Options, 2, 1, 30, new[] { "~R~everse buttons", "~S~how cursor" });
        owl.Label(Options, 1, 4, "~D~ouble-click speed:", 0);
        optRadio = owl.Cluster(Options, 2, 5, 30, new[] { "S~l~ow", "~M~edium", "~F~ast" }, single: true);
        owl.Buttons(Options, new Owlosui.Button("~O~K", CmOk, Default: true), ("~C~ancel", CmCancel));
    }

    // ------------------------------------------------------------------ open

    private void ShowOpen()
    {
        if (OpenDialog != 0) { owl.Activate(OpenDialog); return; }
        OpenDialog = owl.Window("Open", Math.Min(owl.Width - 6, 70), Math.Min(owl.Height - 4, 20),
                                style: Owlosui.Style.ModalDialog, closeCmd: CmCancel);
        files = owl.Files(OpenDialog, openDir, Owlosui.ReadDirectory(openDir));
        owl.Buttons(OpenDialog, new Owlosui.Button("~O~pen", CmFileOpen, Default: true), ("~C~ancel", CmCancel));
    }

    private void GoTo(string text)
    {
        var last = Path.GetFileName(text);
        var (dir, mask) = last.Contains('*') || last.Contains('?') ? (Path.GetDirectoryName(text) ?? text, last) : (text, "*.*");
        try
        {
            openDir = Path.GetFullPath(dir);
            owl.SetFiles(files, openDir, Owlosui.ReadDirectory(openDir), mask);
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException or ArgumentException)
        {
            owl.SetFilesError(files, e is DirectoryNotFoundException ? $"Folder not found: {dir}" : e.Message);
        }
    }

    private void Chosen(string name)
    {
        var full = Path.Combine(openDir, name);
        if (name == "..") { var up = Directory.GetParent(openDir); if (up != null) GoTo(up.FullName); return; }
        if (Directory.Exists(full)) { GoTo(full); return; }
        if (!File.Exists(full)) return;
        string text;
        try { text = File.ReadAllText(full); }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException) { owl.SetFilesError(files, e.Message); return; }
        owl.Close(OpenDialog);
        OpenDialog = 0;
        // Cut as Notes does: a caret is an i16, and this is a viewer.
        var lines = text.Replace("\r\n", "\n").Split('\n').Take(400).Select(l => l.Length > 180 ? l[..180] : l);
        if (Viewer != 0) owl.Close(Viewer);
        Viewer = owl.Window(name, owl.Width - 10, owl.Height - 6, style: Owlosui.Style.Help, closeCmd: CmClose);
        owl.Text(Viewer, string.Join("\n", lines), readOnly: true);
    }

    // -------------------------------------------------------------- closing

    /// <summary>A window closed by its box or by Window > Close: forget it.</summary>
    private void Closed(ushort id)
    {
        owl.Close(id);
        if (id == Calculator) Calculator = 0;
        else if (id == Calendar) Calendar = 0;
        else if (id == Ascii) Ascii = 0;
        else if (id == Puzzle) Puzzle = 0;
        else if (id == Viewer) Viewer = 0;
        else if (id == Options) Options = 0;
        else if (id == OpenDialog) OpenDialog = 0;
    }

    private void Tell(string title, string text)
    {
        CloseBox();
        box = owl.MessageBox(title, text, ("~O~K", CmDismiss));
    }

    private void CloseBox()
    {
        if (box == 0) return;
        owl.Close(box);
        box = 0;
    }

    public static void Main()
    {
        using var owl = new Owlosui();
        var app = new App(owl);
        owl.Run(app.OnCommand, app.Poll);
    }
}
