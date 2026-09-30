// The fifteen puzzle's window: the board on a canvas, tiles six wide and
// three tall, a line that counts the moves, and a button that starts
// over. The rules are in Board; this file only draws and listens.
//
//   ┌──────────── Puzzle ────────────┐
//   │                                │
//   │      ┌─────┐┌─────┐┌─────┐     │
//   │      │  1  ││  2  ││  3  │     │
//   │      ...                       │
//   │ Moves: 12                      │
//   │                    [Scramble]  │
//   └────────────────────────────────┘
//
// A click on a tile beside the hole slides it there; a click anywhere
// else is a click on a tile that cannot move, and nothing happens, which
// is what a tile that cannot move does.

namespace OwlosDemo.Puzzle;

public sealed class PuzzleWindow
{
    /// <summary>The commands this window sends: [First, Last].</summary>
    public const ushort CmFirst = 180, CmLast = 189;
    public const ushort CmScramble = 180;

    /// <summary>The window, while it is open. Zero is closed.</summary>
    public ushort Id { get; private set; }

    public readonly Board Board = new();

    private const int TileW = 6, TileH = 3;
    private const int GridW = TileW * Board.Size, GridH = TileH * Board.Size;

    private readonly Owlosui owl;
    private readonly ushort closeCmd;
    private ushort grid, caption;

    public PuzzleWindow(Owlosui owl, ushort closeCmd)
    {
        this.owl = owl;
        this.closeCmd = closeCmd;
    }

    /// <summary>Open it, or bring it to the front if it is open.</summary>
    public void Show()
    {
        if (Id != 0) { owl.Activate(Id); return; }
        Id = owl.Window("Puzzle", GridW + 12, GridH + 7, style: Owlosui.Style.Dialog, closeCmd: closeCmd);
        grid = owl.Canvas(Id, 5, 1, GridW, GridH);
        caption = owl.Static(Id, 1, GridH + 2, "", GridW + 8);
        owl.Buttons(Id, ("~S~cramble", CmScramble));
        Board.Scramble();
        Draw();
    }

    public bool Handles(ushort cmd) => Id != 0 && cmd >= CmFirst && cmd <= CmLast;

    public void OnCommand(ushort cmd)
    {
        if (Id == 0 || cmd != CmScramble) return;
        Board.Scramble();
        Draw();
    }

    /// <summary>A click on a tile beside the hole slides it.</summary>
    public void Poll()
    {
        if (Id == 0) return;
        if (owl.CanvasClick(grid) is not { } p) return;
        var (col, row) = (p.x / TileW, p.y / TileH);
        if (col < Board.Size && row < Board.Size && Board.Slide(row * Board.Size + col)) Draw();
    }

    private void Draw()
    {
        var text = new char[GridW * GridH];
        var attrs = new byte[GridW * GridH];
        var hole = Owlosui.Attr(ConsoleColor.Gray, ConsoleColor.Gray);
        var even = Owlosui.Attr(ConsoleColor.White, ConsoleColor.DarkBlue);
        var odd = Owlosui.Attr(ConsoleColor.White, ConsoleColor.DarkCyan);
        var done = Owlosui.Attr(ConsoleColor.White, ConsoleColor.DarkGreen);
        var solved = Board.Solved;
        for (var i = 0; i < Board.Tiles.Length; i++)
        {
            var (r, c) = (i / Board.Size, i % Board.Size);
            var tile = Board.Tiles[i];
            var a = tile == 0 ? hole : solved ? done : tile % 2 == 0 ? even : odd;
            // The number on the middle line, centred in the tile.
            var label = tile == 0 ? "" : tile.ToString();
            var lx = (TileW - label.Length) / 2;
            for (var dy = 0; dy < TileH; dy++)
                for (var dx = 0; dx < TileW; dx++)
                {
                    var at = (r * TileH + dy) * GridW + c * TileW + dx;
                    var k = dx - lx;
                    text[at] = dy == TileH / 2 && k >= 0 && k < label.Length ? label[k] : ' ';
                    attrs[at] = a;
                }
        }
        owl.Blit(grid, 0, 0, GridW, GridH, text, attrs);
        owl.SetText(caption, solved ? $"Solved in {Board.Moves} moves!" : $"Moves: {Board.Moves}");
    }

    /// <summary>The window has been closed by the program: forget it.</summary>
    public void Closed() => Id = 0;
}
