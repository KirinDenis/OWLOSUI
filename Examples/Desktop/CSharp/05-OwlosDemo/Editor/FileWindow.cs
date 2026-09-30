// One open file: a text window that is a viewer until F4 makes it an
// editor, and a hex window of the same bytes on F7. Any number of these
// can be open at once, each with its own state, and the keys belong to
// the window and not to the program:
//
//   * The window CARRIES its keys. `WindowStatus` puts "F4 Edit  F7 Hex"
//     on the status line and binds them only while this window is the
//     active one; `WindowMenu` puts "Edit / view" and "Hex" into the
//     Options menu the same way. Put another window in front and both
//     are gone - there is nothing for them to act on. Two file windows
//     share the same command numbers for that reason: only the active
//     one's keys are bound, so the program routes the command to whoever
//     owns the active window.
//   * A viewer is blue like an editor and says [view] in its title. The
//     colour does not change, as it did not in the file managers people
//     learned on; the word and the missing caret are the difference.
//   * Closing an edited file asks first. The question is a modal box,
//     and its Yes and No are this window's commands too.
//   * The Edit menu - Find, Replace, Word wrap, Classic keys - is not
//     here at all: `Editor` switches it on and the core answers it.

namespace OwlosDemo.Editor;

public sealed class FileWindow
{
    /// <summary>The commands a file window sends: [First, Last]. Shared by every instance.</summary>
    public const ushort CmFirst = 200, CmLast = 209;
    public const ushort CmEditView = 200, CmHex = 201, CmSaveYes = 202, CmSaveNo = 203, CmDismiss = 204;

    /// <summary>The text window. Zero once it is closed.</summary>
    public ushort Id { get; private set; }

    /// <summary>The hex window, while it is open.</summary>
    public ushort HexId { get; private set; }

    public string Path { get; }

    public bool ReadOnly { get; private set; } = true;

    private readonly Owlosui owl;
    private readonly ushort closeCmd;
    private readonly ushort text;
    private readonly string original;
    private ushort box;

    private FileWindow(Owlosui owl, string path, string contents, ushort closeCmd)
    {
        this.owl = owl;
        this.closeCmd = closeCmd;
        Path = path;
        // Cut as Notes does: a caret is an i16.
        var lines = contents.Replace("\r\n", "\n").Split('\n').Take(400).Select(l => l.Length > 180 ? l[..180] : l);
        original = string.Join("\n", lines);
        Id = owl.Window(System.IO.Path.GetFileName(path), owl.Width - 10, owl.Height - 6, closeCmd: closeCmd);
        text = owl.Text(Id, original, readOnly: true);
        // What the core does by itself: an Edit menu with Find, Replace, Word
        // wrap and the two keymaps, while this window is in front. Not Read
        // only or Hex - this window does those itself, with F4 and F7, which
        // is what it is here to show.
        owl.Editor(text, Owlosui.Offer.Edit | Owlosui.Offer.Find | Owlosui.Offer.Replace | Owlosui.Offer.Wrap | Owlosui.Offer.Keys,
                   readOnly: true);
        // The keys and menu items this window brings with it.
        owl.WindowStatus(Id, new Owlosui.StatusItem("~F4~ Edit", CmEditView, ConsoleKey.F4),
                             new Owlosui.StatusItem("~F7~ Hex", CmHex, ConsoleKey.F7));
        owl.WindowMenu(Id, Owlosui.MenuItem.Sub("~O~ptions",
                               new Owlosui.MenuItem("~E~dit / view", CmEditView, "F4", Hint: "The file takes typing, or stops taking it"),
                               new Owlosui.MenuItem("~H~ex", CmHex, "F7", Hint: "The file's bytes in a window of their own")));
    }

    /// <summary>Open a file, or say why not.</summary>
    public static FileWindow? Open(Owlosui owl, string path, ushort closeCmd, out string? error)
    {
        try
        {
            var contents = File.ReadAllText(path);
            error = null;
            return new FileWindow(owl, path, contents, closeCmd);
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException)
        {
            error = e.Message;
            return null;
        }
    }

    /// <summary>Whether a window id is one of this file's: its text, its hex, its question.</summary>
    public bool Owns(ushort id) => id != 0 && (id == Id || id == HexId || id == box);

    public bool Handles(ushort cmd) => Id != 0 && cmd >= CmFirst && cmd <= CmLast;

    /// <summary>Whether the text differs from what was read.</summary>
    public bool Changed => Id != 0 && owl.GetText(text) != original;

    /// <summary>True when the file is closed and the program may forget it.</summary>
    public bool Gone => Id == 0;

    public void OnCommand(ushort cmd)
    {
        if (Id == 0) return;
        switch (cmd)
        {
            case CmEditView:
                if (owl.Active() != Id) { owl.Activate(Id); return; }
                ReadOnly = !ReadOnly;
                owl.SetReadOnly(text, ReadOnly);
                break;
            case CmHex:
                // From the hex window, F7 goes back to the text.
                if (owl.Active() == HexId) { owl.Activate(Id); return; }
                ShowHex();
                break;
            case CmSaveYes:
                CloseBox();
                try { File.WriteAllText(Path, owl.GetText(text)); }
                catch (Exception e) when (e is IOException or UnauthorizedAccessException)
                {
                    box = owl.MessageBox("Save", e.Message, ("~O~K", CmDismiss));
                    return;
                }
                Drop();
                break;
            case CmSaveNo:
                CloseBox();
                Drop();
                break;
            case CmDismiss:
                CloseBox();
                break;
        }
    }

    /// <summary>
    /// One of this file's windows is being closed. True when the file is
    /// gone; false when only the hex window went, or the file is asking.
    /// </summary>
    public bool Closing(ushort id)
    {
        if (id == HexId)
        {
            owl.Close(HexId);
            HexId = 0;
            return false;
        }
        if (id != Id) return false;
        if (Changed)
        {
            CloseBox();
            box = owl.MessageBox("Save", $"Save changes to {System.IO.Path.GetFileName(Path)}?",
                                 new Owlosui.Button("~Y~es", CmSaveYes, Default: true), ("~N~o", CmSaveNo));
            return false;
        }
        Drop();
        return true;
    }

    private void ShowHex()
    {
        if (HexId != 0) { owl.Activate(HexId); return; }
        byte[] bytes;
        try { bytes = File.ReadAllBytes(Path); }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException)
        {
            box = owl.MessageBox("Hex", e.Message, ("~O~K", CmDismiss));
            return;
        }
        HexId = owl.Window(System.IO.Path.GetFileName(Path), owl.Width - 14, owl.Height - 8, closeCmd: closeCmd);
        owl.Hex(HexId, bytes);
        owl.WindowStatus(HexId, new Owlosui.StatusItem("~F7~ Text", CmHex, ConsoleKey.F7));
    }

    private void CloseBox()
    {
        if (box == 0) return;
        owl.Close(box);
        box = 0;
    }

    private void Drop()
    {
        CloseBox();
        if (HexId != 0) owl.Close(HexId);
        HexId = 0;
        if (Id != 0) owl.Close(Id);
        Id = 0;
    }
}
