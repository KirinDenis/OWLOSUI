// Desktop, C# step 5 of 6 - everything the kit has, one tool per folder, each liftable.
// Before: 04-Basez-Sokoban. Next: 06-Window, this program in a window of its own.
//
// OWLOS UI Demo: everything the toolkit has, in one program.
//
// The shape is the one every text-mode toolkit's demo has had since the
// eighties, because it is the right shape: a menu bar, a status line, a
// desktop, and a set of small windows that each use one part of the kit.
// Nothing here is copied from anyone; the list of windows is a checklist.
//
//   File      Open a file; Exit. Any number of files can be open.
//   Tools     Calculator, Calendar, ASCII table, Puzzle.
//   Options   A dialog of check boxes and radio buttons; a ticked option;
//             the colour dialog (Colors/), every role of the palette live.
//   Window    Size/Move, Zoom, Next, Previous, Close, List, Cascade,
//             Tile - the desktop's own verbs. Alt+1..9 reach the numbered
//             windows; the number is in the frame.
//   Help      About.
//
// What each window shows of the kit:
//
//   Calculator   an input line, a keypad of buttons, radio buttons and a
//                check box for its modes (Calculator/, with the arithmetic
//                in a class of its own that has no window in it)
//   Calendar     a canvas drawn again for every month and chosen day, and
//                three buttons (Calendar/)
//   ASCII table  a canvas of every glyph with its hex headers; a click
//                names the glyph in every base (AsciiTable/)
//   Puzzle       a canvas game; clicks slide tiles, the rules live in a
//                Board class with no window in it (Puzzle/)
//   Options      clusters and labels in a modal dialog, read back on OK
//   Open         the file panel, and a FileWindow for the file (Editor/):
//                a viewer ([view] in its title) until F4 makes it an editor,
//                its bytes in a hex window on F7, a question on closing an
//                edited file. The window CARRIES those keys: F4 and F7 are
//                on the status line and in the Options menu only while a
//                file window is the active one
//
// Every window is an ordinary window: drag it, resize it, zoom it, put it
// behind another. Alt+X or Exit leaves; nothing here has anything to save.
//
// It is a class so that `Examples/Desktop/CSharp/Tests` can open every window by the
// wire and press its keys. Each tool is a class in a folder of its own,
// liftable into another program as it is: it owns a range of command
// numbers, answers `Handles` for them, and has the same four verbs -
// `Show`, `OnCommand`, `Poll`, `Closed` - so this file only routes.

using System.Text;
using OwlosDemo.AsciiTable;
using OwlosDemo.Calculator;
using OwlosDemo.Calendar;
using OwlosDemo.Colors;
using OwlosDemo.Editor;
using OwlosDemo.Puzzle;

namespace OwlosDemo;

public sealed class App
{
    // Commands. The program's own numbers; 0 means none.
    public const ushort CmOpen = 1, CmExit = 2;
    public const ushort CmCalc = 10, CmCalendar = 11, CmAscii = 12, CmPuzzle = 13;
    public const ushort CmOptions = 20, CmClock = 21, CmColors = 22;
    public const ushort CmNext = 30, CmZoom = 31, CmClose = 32, CmCascade = 33, CmTile = 34;
    public const ushort CmPrevious = 35, CmList = 36, CmSizeMove = 37;
    public const ushort CmAbout = 40, CmHelp = 41;
    // The windows' own buttons. The tools own their own ranges: the
    // calculator 100..159, the calendar 160..169, the puzzle 180..189.
    public const ushort CmOk = 60, CmCancel = 61, CmDismiss = 62;
    public const ushort CmFileOpen = 63;
    // The Open dialog's other two buttons: the panel as a tree of folders, and a drive.
    public const ushort CmOpenTree = 64, CmOpenDrive = 65, CmDriveOk = 66;
    // A file window's own commands are 200..209 (Editor/FileWindow.cs).

    private readonly Owlosui owl;

    // The tools, each a window of its own.
    public readonly CalculatorWindow Calc;
    public readonly CalendarWindow Cal;
    public readonly AsciiTableWindow Table;
    public readonly PuzzleWindow Game;
    public readonly ColorsWindow Palette;

    // The windows, while they are open. Zero is closed.
    public ushort Options, OpenDialog;
    /// <summary>The open files, each a window (or two) of its own.</summary>
    public readonly List<FileWindow> Docs = new();
    /// <summary>The file whose window is in front, if a file's is.</summary>
    public FileWindow? ActiveDoc => Docs.FirstOrDefault(d => d.Owns(owl.Active()));
    public ushort Calculator => Calc.Id;
    public ushort Calendar => Cal.Id;
    public ushort Ascii => Table.Id;
    public ushort Puzzle => Game.Id;
    private ushort optChecks, optRadio, files, box;
    // The Open dialog's tree, while the panel is one, and the drive dialog's list.
    private ushort openTree, driveBox, driveList;
    private string openTreeRoot = "";
    private bool clock = true;
    private string openDir = Directory.GetCurrentDirectory();

    public App(Owlosui owl)
    {
        this.owl = owl;
        Calc = new CalculatorWindow(owl, CmClose);
        Cal = new CalendarWindow(owl, CmClose);
        Table = new AsciiTableWindow(owl, CmClose);
        Game = new PuzzleWindow(owl, CmClose);
        Palette = new ColorsWindow(owl);

        owl.MenuBar(
            // A Hint is one line the status line shows while the cursor
            // stands on the item, in place of the keys.
            Owlosui.MenuItem.Sub("~F~ile",
                new Owlosui.MenuItem("~O~pen...", CmOpen, "F3", Hint: "Open a file in a window of its own"),
                Owlosui.MenuItem.Line(),
                new Owlosui.MenuItem("E~x~it", CmExit, "Alt+X", Hint: "Leave the program")),
            Owlosui.MenuItem.Sub("~T~ools",
                new Owlosui.MenuItem("~C~alculator", CmCalc, Hint: "Arithmetic, trigonometry, hex and binary"),
                new Owlosui.MenuItem("Ca~l~endar", CmCalendar, Hint: "A month at a time; click a day"),
                new Owlosui.MenuItem("~A~SCII table", CmAscii, Hint: "Every glyph of the font, by its number"),
                new Owlosui.MenuItem("~P~uzzle", CmPuzzle, Hint: "The fifteen puzzle")),
            Owlosui.MenuItem.Sub("~O~ptions",
                new Owlosui.MenuItem("~M~ouse...", CmOptions, Hint: "Check boxes and radio buttons, read back on OK"),
                new Owlosui.MenuItem("Co~l~ors...", CmColors, Hint: "Every role's colour, changed live"),
                new Owlosui.MenuItem("~C~lock on status line", CmClock, Checked: clock, Hint: "A ticked option: on or off")),
            Owlosui.MenuItem.Sub("~W~indow",
                new Owlosui.MenuItem("~S~ize/Move", CmSizeMove, "Ctrl+F5", Hint: "Arrows move the window, Shift+arrows resize it; Enter keeps, Esc puts back"),
                new Owlosui.MenuItem("~Z~oom", CmZoom, "F5", Hint: "The window fills the desktop, or goes back to its size"),
                new Owlosui.MenuItem("~N~ext", CmNext, "F6", Hint: "The front window goes to the back"),
                new Owlosui.MenuItem("~P~revious", CmPrevious, "Shift+F6", Hint: "The window at the back comes to the front"),
                new Owlosui.MenuItem("~C~lose", CmClose, "Alt+F3", Hint: "Close the front window"),
                new Owlosui.MenuItem("~L~ist...", CmList, "Alt+0", Hint: "Every window by number; Enter brings one to the front"),
                Owlosui.MenuItem.Line(),
                new Owlosui.MenuItem("C~a~scade", CmCascade, Hint: "The windows along the diagonal, every title showing"),
                new Owlosui.MenuItem("~T~ile", CmTile, Hint: "The windows share the desktop with no overlap")),
            Owlosui.MenuItem.Sub("~H~elp",
                new Owlosui.MenuItem("~A~bout", CmAbout, Hint: "What this program is and what draws it")));

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
                Tell("OWLOS UI Demo", "A text mode toolkit in the classic DOS style, with one portable core: this program is C#, " +
                     "every window in it is drawn by a Rust core behind a pipe, and the same core is " +
                     "meant for a terminal, a browser and a DOS machine.");
                return true;
            case CmDismiss:
                CloseBox();
                if (driveBox != 0) { owl.Close(driveBox); driveBox = 0; }
                return true;

            case CmNext: owl.NextWindow(); return true;
            case CmZoom: { var a = owl.Active(); if (a != 0) owl.Zoom(a); return true; }
            case CmClose: { var a = owl.Active(); if (a != 0) Closed(a); return true; }
            // The desktop's own verbs: it knows the windows, we only ask.
            case CmCascade: owl.Cascade(); return true;
            case CmPrevious: owl.PreviousWindow(); return true;
            case CmList: owl.WindowList(); return true;
            case CmSizeMove: owl.SizeMove(); return true;
            case CmTile: owl.Tile(); return true;

            case CmClock:
                clock = !clock;
                owl.MenuCheck(CmClock, clock);
                return true;

            case CmCalc: Calc.Show(); return true;
            case CmCalendar: Cal.Show(); return true;
            case CmAscii: Table.Show(); return true;
            case CmPuzzle: Game.Show(); return true;

            case CmOptions: ShowOptions(); return true;
            case CmColors: Palette.Show(); return true;
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
                if (OpenDialog != 0) { owl.Close(OpenDialog); OpenDialog = 0; openTree = 0; }
                return true;

            case CmOpen: ShowOpen(); return true;
            case CmOpenTree: ToggleOpenTree(); return true;
            case CmOpenDrive: if (OpenDialog != 0) ShowDrives(); return true;
            case CmDriveOk:
            {
                var drives = DriveInfo.GetDrives().Where(d => d.IsReady).Select(d => d.RootDirectory.FullName).ToArray();
                var ix = owl.Current(driveList);
                owl.Close(driveBox);
                driveBox = 0;
                if (ix >= 0 && ix < drives.Length)
                {
                    if (openTree != 0) ToggleOpenTree();
                    GoTo(drives[ix]);
                }
                return true;
            }
            case CmFileOpen:
            {
                // In tree mode Open means "the files of this folder".
                if (openTree != 0) { ToggleOpenTree(); return true; }
                // The button: whatever is under the cursor in the panel.
                var names = owl.MarkedNames(files);
                if (names.Length == 1) Chosen(names[0]);
                return true;
            }

            default:
                // A file window's keys are bound only while it is active,
                // so the command goes to the file that owns the front window.
                if (cmd >= FileWindow.CmFirst && cmd <= FileWindow.CmLast)
                {
                    var doc = ActiveDoc;
                    doc?.OnCommand(cmd);
                    if (doc is { Gone: true }) Docs.Remove(doc);
                    return true;
                }
                // The tools' own buttons.
                if (Palette.Handles(cmd)) Palette.OnCommand(cmd);
                else if (Calc.Handles(cmd)) Calc.OnCommand(cmd);
                else if (Cal.Handles(cmd)) Cal.OnCommand(cmd);
                else if (Game.Handles(cmd)) Game.OnCommand(cmd);
                return true;
        }
    }

    /// <summary>What is not a command: clicks on canvases, a name entered in the file panel.</summary>
    public void Poll()
    {
        Calc.Poll();
        Palette.Poll();
        Cal.Poll();
        Table.Poll();
        Game.Poll();
        if (OpenDialog != 0 && openTree != 0)
        {
            // The tree fills itself as it is opened.
            if (owl.TreeExpand(openTree) is { } ask)
            {
                var dir = ask.texts.Length <= 1 ? openTreeRoot : Path.Combine(new[] { openTreeRoot }.Concat(ask.texts.Skip(1)).ToArray());
                owl.TreeChildren(openTree, ask.path, Subfolders(dir));
            }
        }
        else if (OpenDialog != 0)
        {
            var (kind, text) = owl.TakeFiles(files);
            if (kind == Owlosui.FilesEvent.Chosen) Chosen(text);
            else if (kind == Owlosui.FilesEvent.Path) GoTo(text);
        }
    }

    // --------------------------------------------------------------- options

    private void ShowOptions()
    {
        if (Options != 0) { owl.Activate(Options); return; }
        // A row of air under the title in every window: content against
        // the title bar looks pinned to it.
        Options = owl.Window("Mouse", 40, 13, style: Owlosui.Style.ModalDialog, closeCmd: CmCancel);
        owl.Label(Options, 1, 1, "~B~uttons:", 0);
        optChecks = owl.Cluster(Options, 2, 2, 30, new[] { "~R~everse buttons", "~S~how cursor" });
        owl.Label(Options, 1, 5, "~D~ouble-click speed:", 0);
        optRadio = owl.Cluster(Options, 2, 6, 30, new[] { "S~l~ow", "~M~edium", "~F~ast" }, single: true);
        owl.Buttons(Options, new Owlosui.Button("~O~K", CmOk, Default: true), ("~C~ancel", CmCancel));
    }

    // ------------------------------------------------------------------ open

    private void ShowOpen()
    {
        if (OpenDialog != 0) { owl.Activate(OpenDialog); return; }
        OpenDialog = owl.Window("Open", Math.Min(owl.Width - 6, 70), Math.Min(owl.Height - 4, 20),
                                style: Owlosui.Style.ModalDialog, closeCmd: CmCancel);
        files = owl.Files(OpenDialog, openDir, Owlosui.ReadDirectory(openDir), top: 1);
        openTree = 0;
        // Tree turns the panel into the folders of its drive, filled as
        // they are opened, and back; Drive picks another drive.
        owl.Buttons(OpenDialog, new Owlosui.Button("~O~pen", CmFileOpen, Default: true), ("~T~ree", CmOpenTree),
                    ("~D~rive", CmOpenDrive), new Owlosui.Button("~C~ancel", CmCancel, Cancel: true));
    }

    /// <summary>Tree: the Open dialog's panel shows the drive's folders; again, and it shows the files of the folder under the cursor.</summary>
    private void ToggleOpenTree()
    {
        if (OpenDialog == 0) return;
        if (openTree == 0)
        {
            openTreeRoot = Path.GetPathRoot(openDir) ?? openDir;
            owl.Close(files);
            files = 0;
            openTree = owl.Tree(OpenDialog, new Owlosui.TreeNode(openTreeRoot.TrimEnd('\\'), Subfolders(openTreeRoot), Open: true));
            return;
        }
        var texts = owl.TreePath(openTree);
        var dir = texts.Length <= 1 ? openTreeRoot : Path.Combine(new[] { openTreeRoot }.Concat(texts.Skip(1)).ToArray());
        owl.Close(openTree);
        openTree = 0;
        files = owl.Files(OpenDialog, openDir, Owlosui.ReadDirectory(openDir), top: 1);
        GoTo(dir);
    }

    /// <summary>A folder's subfolders as lazy nodes: each is read when opened.</summary>
    private static Owlosui.TreeNode[] Subfolders(string dir)
    {
        try
        {
            return Directory.EnumerateDirectories(dir)
                .Select(Path.GetFileName)
                .Where(n => n != null)
                .OrderBy(n => n, StringComparer.OrdinalIgnoreCase)
                .Select(n => new Owlosui.TreeNode(n!, Lazy: true))
                .ToArray();
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException)
        {
            return Array.Empty<Owlosui.TreeNode>();
        }
    }

    /// <summary>Drive: a modal list of the drives over the Open dialog.</summary>
    private void ShowDrives()
    {
        var drives = DriveInfo.GetDrives().Where(d => d.IsReady)
            .Select(d => $"{d.RootDirectory.FullName.TrimEnd('\\')}  {d.DriveType}  {d.VolumeLabel}".TrimEnd())
            .ToArray();
        if (driveBox != 0) owl.Close(driveBox);
        driveBox = owl.Window("Drive", 40, 6 + Math.Min(drives.Length, 8), style: Owlosui.Style.ModalDialog, closeCmd: CmDismiss);
        driveList = owl.List(driveBox, 1, 1, 34, Math.Min(drives.Length, 8), drives);
        owl.Buttons(driveBox, new Owlosui.Button("~O~K", CmDriveOk, Default: true), new Owlosui.Button("~C~ancel", CmDismiss, Cancel: true));
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
        if (!Open(full)) return;
        owl.Close(OpenDialog);
        OpenDialog = 0;
    }

    /// <summary>A file into a window of its own. False, with the panel saying why, if it cannot be read.</summary>
    public bool Open(string path)
    {
        var doc = FileWindow.Open(owl, path, CmClose, out var error);
        if (doc == null)
        {
            if (OpenDialog != 0) owl.SetFilesError(files, error ?? "cannot read"); else Tell("Open", error ?? "cannot read");
            return false;
        }
        Docs.Add(doc);
        return true;
    }

    // -------------------------------------------------------------- closing

    /// <summary>A window closed by its box or by Window > Close: forget it.</summary>
    private void Closed(ushort id)
    {
        // A file's window: the file decides - a hex window just goes, an
        // edited text asks first.
        if (Docs.FirstOrDefault(d => d.Owns(id)) is { } doc)
        {
            if (doc.Closing(id)) Docs.Remove(doc);
            return;
        }
        owl.Close(id);
        if (id == Calculator) Calc.Closed();
        else if (id == Calendar) Cal.Closed();
        else if (id == Ascii) Table.Closed();
        else if (id == Puzzle) Game.Closed();
        else if (id == Options) Options = 0;
        else if (id == Palette.Id) Palette.Closed();
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
