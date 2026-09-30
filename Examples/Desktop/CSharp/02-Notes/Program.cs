// Desktop, C# step 2 of 6 - an editor, a file, and a question before leaving.
// Before: 01-HelloWorld. Next: 03-Commander, two panels of files.
//
// An editor that saves to a file, and asks before it lets you leave.
//
// Two things this shows that HelloWorld does not:
//
//  * the editor is real - undo, selection, a clipboard, two keymaps, scroll
//    bars, the mouse placing the caret - and this program never sees a
//    keystroke of it. It reads the text back with GetText when it is time
//    to save, and that is all. Sixty lines, and most of them are comments;
//
//  * the "you have not saved" question is a modal box. While it is up, keys
//    and clicks on the editor behind it do nothing; the box can still be
//    dragged out of the way. Both of those come from the core, not from
//    anything written here.
//
// The window is an ordinary document window: drag its title to move it, its
// bottom-right corner to resize it, click [↑] to zoom, [■] to close. Resize
// the console and the window follows its edges. Alt+S saves; Alt+X, the
// Exit button, Escape, or the close box ask first if there is something
// unsaved.
//
// It is a class rather than a script so that `Examples/Desktop/CSharp/Tests` can run
// the same program headless and press its buttons by the wire - and in a
// real console, by an agent, without anybody at the keyboard.

namespace Notes;

public sealed class App
{
    // The program's commands. Four buttons in two windows; four numbers.
    // They only have to be distinct and not zero - the core does not know
    // what any of them means, and that is the point.
    public const ushort CmSave = 1;
    public const ushort CmExit = 2;
    public const ushort CmLeave = 3;
    public const ushort CmStay = 4;
    // Find and Replace: the dialogs, their buttons, and F3 for the next match.
    public const ushort CmFind = 5, CmReplace = 6, CmFindNext = 7;
    public const ushort CmFindOk = 8, CmReplaceOne = 9, CmReplaceAll = 10, CmSearchCancel = 11;

    private readonly Owlosui owl;
    private readonly string fileName;

    // What the file held when last read or written. "Modified" is the text
    // in the editor differing from this - computed, never stored, so it can
    // never be out of step with the text.
    private string saved;

    // The question box, while it is up. Needed to close it on Stay.
    private ushort box;

    // The search dialog, while it is up, and its fields; what was last
    // searched for, so F3 finds the next one without asking again.
    private ushort search, findField, withField, options;
    private string lastPattern = "", lastWith = "";
    private bool lastCase, lastWhole;
    public string LastPattern => lastPattern;

    // Handles, for the tests: the window and the editor inside it.
    public ushort Window { get; }
    public ushort Editor { get; }

    /// <summary>
    /// True when the file holds characters the session's code page has no
    /// glyph for. Such a file is shown with `?` where they were - the screen
    /// can do no better - and is *not* saved from here: writing the `?`s
    /// back would destroy the text. Start with a code page that has the
    /// letters (866 for Cyrillic) and it opens for editing.
    /// </summary>
    public bool ReadOnly { get; }

    public App(Owlosui owl, string fileName)
    {
        this.owl = owl;
        this.fileName = fileName;
        saved = File.Exists(fileName) ? File.ReadAllText(fileName) : "";
        ReadOnly = !owl.Fits(saved);

        // A document window - blue, movable, resizable, zoomable - almost as
        // big as the desktop, centred, with a two-cell margin all round so
        // that it reads as a window and not as the screen. Because its size
        // was set relative to the desktop, it keeps that relation when the
        // console is resized: the core moves a window's far edges with the
        // desktop's, which is what Turbo Vision called gfGrowAll.
        //
        // closeCmd: the close box [■] sends CmExit instead of closing. That
        // is Turbo Vision's cmClose bargain - the program hears about it and
        // decides - and it is how an editor gets to ask before losing text.
        Window = owl.Window(fileName, owl.Width - 4, owl.Height - 2, closeCmd: CmExit);

        // The editor fills the window - it is docked, so when the window is
        // resized or zoomed the editor follows and the scroll bars move with
        // the frame. It takes the focus first, so typing starts at once.
        // A file the screen cannot show whole opens as a viewer instead.
        Editor = owl.Text(Window, saved, readOnly: ReadOnly);
        if (ReadOnly)
            box = owl.MessageBox("Notes",
                $"{fileName} has characters this code page cannot show; they are drawn as ? " +
                "and the file is opened read-only, so nothing is lost. Start with a code page " +
                "that has them - 866 for Cyrillic - to edit it.",
                ("~O~K", CmStay));

        // Bottom right, over the editor's corner. Save is first and so the
        // default - but Enter inside an editor is a new line, and the editor
        // takes it before the button row gets a chance; the default only
        // matters when the focus is on something that does not want Enter.
        // Escape is not a thing an editor wants, so it does go to the last
        // button, Exit, and Exit asks. The hotkeys work from anywhere:
        // Alt+S, Alt+X.
        // Find and Replace are buttons too, so Alt+F and Alt+R reach them
        // from the text; the dialog remembers the last search, and OK on
        // it again is "find next".
        owl.Buttons(Window, ("~S~ave", CmSave), ("~F~ind", CmFind), ("~R~eplace", CmReplace), ("E~x~it", CmExit));
    }

    /// <summary>
    /// The Find or the Replace dialog: a field (two for Replace), the two
    /// options every search has, and the buttons. Modal, so the editor
    /// behind it waits; the answer comes back as a command.
    /// </summary>
    private void ShowSearch(bool replace)
    {
        CloseSearch();
        var h = replace ? 12 : 10;
        search = owl.Window(replace ? "Replace" : "Find", 50, h, style: Owlosui.Style.ModalDialog, closeCmd: CmSearchCancel);
        findField = owl.Input(search, 2, 1, 44, "Find:", lastPattern);
        owl.SetHistory(findField, lastPattern.Length > 0 ? new[] { lastPattern } : Array.Empty<string>());
        var y = 2;
        if (replace)
        {
            withField = owl.Input(search, 2, 3, 44, "Replace with:", lastWith);
            y = 4;
        }
        options = owl.Cluster(search, 2, y + 1, 30, new[] { "~C~ase sensitive", "~W~hole words only" });
        if (replace)
            owl.Buttons(search, new Owlosui.Button("~R~eplace", CmReplaceOne, Default: true), ("Replace ~a~ll", CmReplaceAll),
                        new Owlosui.Button("~C~ancel", CmSearchCancel, Cancel: true));
        else
            owl.Buttons(search, new Owlosui.Button("~O~K", CmFindOk, Default: true), new Owlosui.Button("~C~ancel", CmSearchCancel, Cancel: true));
    }

    /// <summary>Read the dialog's fields into what F3 will use, and close it.</summary>
    private void TakeSearch()
    {
        lastPattern = owl.GetText(findField);
        if (withField != 0) lastWith = owl.GetText(withField);
        var (on, _) = owl.ClusterState(options);
        lastCase = on.Length > 0 && on[0];
        lastWhole = on.Length > 1 && on[1];
        CloseSearch();
    }

    private void CloseSearch()
    {
        if (search != 0) owl.Close(search);
        search = 0;
        withField = 0;
    }

    private void FindNext()
    {
        if (lastPattern.Length == 0) { ShowSearch(false); return; }
        if (!owl.Find(Editor, lastPattern, lastCase, lastWhole))
            box = owl.MessageBox("Find", $"\"{lastPattern}\" not found.", ("~O~K", CmStay));
    }

    public bool Modified => owl.GetText(Editor) != saved;

    /// <summary>
    /// What to do with a command. True to keep running, false to leave.
    /// Called by <see cref="Owlosui.Run"/> for every button pressed - in
    /// the editor window and in the question box alike; the numbers say
    /// which was which.
    /// </summary>
    public bool OnCommand(ushort cmd)
    {
        switch (cmd)
        {
            case CmSave:
                // A file opened read-only is not written, whatever was
                // pressed: the text on the screen is not the text in the file.
                if (ReadOnly)
                {
                    box = owl.MessageBox("Notes", "Opened read-only: this code page cannot hold the file.", ("~O~K", CmStay));
                    return true;
                }
                // The only time the program looks at the text. Everything
                // the person did to it since the last save - typing, undo,
                // paste - is already in it.
                saved = owl.GetText(Editor);
                File.WriteAllText(fileName, saved);
                return true;

            case CmExit:
                // Nothing to lose: just go.
                if (!Modified) return false;

                // Otherwise ask. MessageBox is a grey modal window sized to
                // its words; while it is up the editor behind it is deaf.
                //
                // The order of the buttons is the whole design of this
                // dialog. Stay is marked Default, so Enter keeps you here;
                // it is also last, so Escape does the same. Leaving - the
                // one answer that destroys something - has to be chosen on
                // purpose: Alt+L, or a click on the button.
                box = owl.MessageBox("Confirm", $"{fileName} has been changed. Leave without saving?",
                                     ("~L~eave", CmLeave),
                                     new Owlosui.Button("~S~tay", CmStay, Default: true));
                return true;

            case CmLeave:
                return false;

            case CmFind: ShowSearch(false); return true;
            case CmReplace: ShowSearch(true); return true;
            case CmFindNext: FindNext(); return true;
            case CmFindOk:
                TakeSearch();
                FindNext();
                return true;
            case CmReplaceOne:
            {
                // The dialog stays up, as it did in the editors people
                // learned on: each Replace does one and finds the next.
                lastPattern = owl.GetText(findField);
                lastWith = owl.GetText(withField);
                var (on, _) = owl.ClusterState(options);
                lastCase = on.Length > 0 && on[0];
                lastWhole = on.Length > 1 && on[1];
                var (_, found) = owl.Replace(Editor, lastPattern, lastWith, lastCase, lastWhole);
                if (!found) CloseSearch();
                return true;
            }
            case CmReplaceAll:
            {
                TakeSearch();
                var n = owl.ReplaceAll(Editor, lastPattern, lastWith, lastCase, lastWhole);
                box = owl.MessageBox("Replace", $"{n} replaced.", ("~O~K", CmStay));
                return true;
            }
            case CmSearchCancel:
                CloseSearch();
                return true;

            case CmStay:
                // Closing the box is enough. The editor behind it still has
                // the focus it had, and the text it had.
                owl.Close(box);
                return true;

            default:
                return true;
        }
    }

    public static void Main()
    {
        using var owl = new Owlosui();
        var app = new App(owl, "NOTES.TXT");
        owl.Run(app.OnCommand);
    }
}
