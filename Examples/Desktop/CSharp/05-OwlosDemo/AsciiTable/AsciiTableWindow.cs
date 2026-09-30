// Every glyph of the session's font below 256, in the shape the tables
// in the back of the manuals had: sixteen columns headed 0 to F, sixteen
// rows headed 00 to F0, so a glyph's place reads as its hex number.
//
//   ┌────────────────── ASCII table ─────────────────┐
//   │     0  1  2  3  4  5  6  7  8  9  A  B  C  D  E  F │
//   │ 00     ☺  ☻  ♥  ♦  ♣  ♠  •  ◘  ○  ◙  ♂  ♀  ♪  ♫  ☼ │
//   │ 10  ►  ◄  ↕  ‼  ¶  §  ▬  ↨  ↑  ↓  →  ←  ∟  ↔  ▲  ▼ │
//   │ ...                                                 │
//   │ 40  @  A  B  C  D  E  F  G  H  I  J  K  L  M  N  O │
//   │ ...                                                 │
//   │                                                     │
//   │ Glyph 65  dec 65  hex 41  oct 101  bin 01000001     │
//   └─────────────────────────────────────────────────────┘
//
// The whole table is one canvas, headers included: a static line would
// have folded the spaces the columns are made of. A click names the
// glyph under it in every base and lights its cell.
//
// The glyphs are shown by their index in the font the session runs on,
// whatever they look like there: 0x01 is a face on 437 and on 866 alike,
// 0x80 is Ç on one and А on the other. That is the point of the table.

namespace OwlosDemo.AsciiTable;

public sealed class AsciiTableWindow
{
    /// <summary>This window sends no commands; the range is empty.</summary>
    public const ushort CmFirst = 170, CmLast = 170;

    /// <summary>The window, while it is open. Zero is closed.</summary>
    public ushort Id { get; private set; }

    /// <summary>The glyph last clicked, if any.</summary>
    public int? Picked { get; private set; }

    // Three columns a cell, a three-wide row label first, one header row.
    private const int CellW = 3, Label = 3, Columns = 16, Rows = 16;
    private const int GridW = Label + CellW * Columns, GridH = 1 + Rows;

    private readonly Owlosui owl;
    private readonly ushort closeCmd;
    private ushort grid, caption;

    public AsciiTableWindow(Owlosui owl, ushort closeCmd)
    {
        this.owl = owl;
        this.closeCmd = closeCmd;
    }

    /// <summary>Open it, or bring it to the front if it is open.</summary>
    public void Show()
    {
        if (Id != 0) { owl.Activate(Id); return; }
        // A row of air under the title, the table, a row, the caption.
        Id = owl.Window("ASCII table", GridW + 4, GridH + 5, style: Owlosui.Style.Dialog, closeCmd: closeCmd);
        grid = owl.Canvas(Id, 1, 1, GridW, GridH);
        caption = owl.Static(Id, 1, GridH + 2, "Click a glyph.", GridW);
        Picked = null;
        Draw();
    }

    public bool Handles(ushort cmd) => false;

    public void OnCommand(ushort cmd) { }

    /// <summary>A click on a cell names the glyph.</summary>
    public void Poll()
    {
        if (Id == 0) return;
        if (owl.CanvasClick(grid) is not { } p || p.y < 1 || p.x < Label) return;
        var code = (p.y - 1) * Columns + (p.x - Label) / CellW;
        if (code >= 256) return;
        Picked = code;
        Draw();
        owl.SetText(caption, $"Glyph {code}  dec {code}  hex {code:X2}  oct {Convert.ToString(code, 8)}  bin {Convert.ToString(code, 2).PadLeft(8, '0')}");
    }

    private void Draw()
    {
        var text = new char[GridW * GridH];
        var attrs = new byte[GridW * GridH];
        var plain = Owlosui.Attr(ConsoleColor.Black, ConsoleColor.Gray);
        var head = Owlosui.Attr(ConsoleColor.DarkBlue, ConsoleColor.Gray);
        var lit = Owlosui.Attr(ConsoleColor.White, ConsoleColor.DarkCyan);
        Array.Fill(text, ' ');
        Array.Fill(attrs, plain);

        for (var c = 0; c < Columns; c++)
        {
            var x = Label + c * CellW + 1;
            text[x] = "0123456789ABCDEF"[c];
            attrs[x] = head;
        }
        for (var r = 0; r < Rows; r++)
        {
            var y = 1 + r;
            var label = (r * 16).ToString("X2");
            text[y * GridW] = label[0];
            text[y * GridW + 1] = label[1];
            attrs[y * GridW] = attrs[y * GridW + 1] = head;
            for (var c = 0; c < Columns; c++)
            {
                var code = r * 16 + c;
                var x = Label + c * CellW;
                // Cell 0 is the blank; a control code with no picture in the
                // font shows as whatever the font has there.
                var ch = code < owl.Glyphs.Length ? owl.Glyphs[code] : '?';
                text[y * GridW + x + 1] = code == 0 ? ' ' : ch;
                if (Picked == code)
                    for (var dx = 0; dx < CellW; dx++) attrs[y * GridW + x + dx] = lit;
            }
        }
        owl.Blit(grid, 0, 0, GridW, GridH, text, attrs);
    }

    /// <summary>The window has been closed by the program: forget it.</summary>
    public void Closed() => Id = 0;
}
