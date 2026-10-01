// The colour dialog: every role the palette has, and what it wears.
//
//   ┌────────────── Colors [modal] ───────────────┐
//   │                                             │
//   │  Desktop: desktop          Foreground       │
//   │  Desktop: shadow           ■ ■ ■ ■ ■ ■ ■ ■  │
//   │  Menus: menu               ■ ■ ■ ■ ■ ■ ■ ■  │
//   │  Menus: menu key                            │
//   │  ...                       Background       │
//   │                            ■ ■ ■ ■ ■ ■ ■ ■  │
//   │                            ■ ■ ■ ■ ■ ■ ■ ■  │
//   │                                             │
//   │                            Sample text      │
//   │                        [  OK  ] [ Cancel ]  │
//   └─────────────────────────────────────────────┘
//
// Colour comes from the palette by role, never from the element - the
// first rule the toolkit keeps - and this is the dialog that rule makes
// possible: pick a role on the left, click a colour on the right, and
// everything that plays the role changes on the next frame, this dialog
// included. The classic toolkits' demos had the same dialog for the same reason.
//
// Cancel puts back what was there when the dialog opened; OK keeps it.
// Nothing is written anywhere - a program that wants a theme kept reads
// `Palette()` and stores it.

namespace OwlosDemo.Colors;

public sealed class ColorsWindow
{
    public const ushort CmFirst = 210, CmLast = 219;
    public const ushort CmOk = 210, CmCancel = 211;

    public ushort Id { get; private set; }

    private readonly Owlosui owl;
    private ushort list, fore, back, sample;
    private Owlosui.PaletteEntry[] entries = Array.Empty<Owlosui.PaletteEntry>();
    private byte[] before = Array.Empty<byte>();
    private int shown = -1;

    // The sixteen colours in two rows of eight, three cells each.
    private const int CellW = 3, GridW = 8 * CellW, GridH = 2;

    public ColorsWindow(Owlosui owl)
    {
        this.owl = owl;
    }

    public void Show()
    {
        if (Id != 0) { owl.Activate(Id); return; }
        entries = owl.Palette();
        before = entries.Select(e => e.Attr).ToArray();
        Id = owl.Window("Colors", 66, 20, style: Owlosui.Style.ModalDialog, closeCmd: CmCancel);
        list = owl.List(Id, 1, 1, 34, 15, entries.Select(e => $"{e.Group}: {e.Name}"));
        owl.Static(Id, 38, 1, "Foreground");
        fore = owl.Canvas(Id, 38, 2, GridW, GridH);
        owl.Static(Id, 38, 5, "Background");
        back = owl.Canvas(Id, 38, 6, GridW, GridH);
        sample = owl.Canvas(Id, 38, 10, 22, 1);
        owl.Buttons(Id, new Owlosui.Button("~O~K", CmOk, Default: true), new Owlosui.Button("~C~ancel", CmCancel, Cancel: true));
        shown = -1;
        Poll();
    }

    public bool Handles(ushort cmd) => Id != 0 && cmd >= CmFirst && cmd <= CmLast;

    public void OnCommand(ushort cmd)
    {
        if (Id == 0) return;
        if (cmd == CmCancel)
        {
            // Everything back as it was when the dialog opened.
            for (var i = 0; i < before.Length; i++)
                if (entries[i].Attr != before[i]) owl.SetColor(i, before[i]);
        }
        Closed();
    }

    /// <summary>The list's cursor and the grids' clicks: looked at after every input.</summary>
    public void Poll()
    {
        if (Id == 0) return;
        var ix = owl.Current(list);
        if (ix != shown)
        {
            shown = ix;
            Draw();
        }
        if (owl.CanvasClick(fore) is { } f) Pick(f, foreground: true);
        if (owl.CanvasClick(back) is { } b) Pick(b, foreground: false);
    }

    private void Pick((int x, int y) at, bool foreground)
    {
        if (shown < 0 || shown >= entries.Length) return;
        var colour = at.y * 8 + at.x / CellW;
        if (colour is < 0 or > 15) return;
        var old = entries[shown].Attr;
        var attr = (byte)(foreground ? (old & 0xF0) | colour : (colour << 4) | (old & 0x0F));
        entries[shown] = entries[shown] with { Attr = attr };
        owl.SetColor(shown, attr);
        Draw();
    }

    /// <summary>The two grids with the chosen cells marked, and the sample in the role's colour.</summary>
    private void Draw()
    {
        if (shown < 0 || shown >= entries.Length) return;
        var attr = entries[shown].Attr;
        var (fg, bg) = (attr & 0x0F, attr >> 4);
        Grid(fore, fg);
        Grid(back, bg);
        var text = " Sample text          ".ToCharArray();
        var attrs = Enumerable.Repeat(attr, text.Length).ToArray();
        owl.Blit(sample, 0, 0, text.Length, 1, text, attrs);
    }

    private void Grid(ushort canvas, int chosen)
    {
        var text = new char[GridW * GridH];
        var attrs = new byte[GridW * GridH];
        for (var c = 0; c < 16; c++)
        {
            var (x0, y) = ((c % 8) * CellW, c / 8);
            // A block of the colour; the chosen one carries a mark that
            // reads on light and dark alike.
            var mark = c == chosen ? 'X' : ' ';
            var ink = c < 8 ? 15 : 0;
            for (var dx = 0; dx < CellW; dx++)
            {
                text[y * GridW + x0 + dx] = dx == 1 ? mark : ' ';
                attrs[y * GridW + x0 + dx] = (byte)((c << 4) | ink);
            }
        }
        owl.Blit(canvas, 0, 0, GridW, GridH, text, attrs);
    }

    public void Closed()
    {
        if (Id != 0) owl.Close(Id);
        Id = 0;
    }
}
