// BASE-Z 47, on the wire.
//
// A cell is two glyphs, the way the small picture in view.rs is two glyphs:
// wall ▓▓, floor ░░, mark ··, box ▒▒, box on a mark ██, keeper /\.
// The level is the original 30 by 20, and it is placed with the same
// centering as custom_draw: half the screen, minus half the content, times
// the cell width. Static text collapses spaces, which is why a map drawn
// with ' ' came apart and the keeper looked as if he were pushing walls.
// There is no space in a row here, so a row cannot be reflowed.
//
// Stepping is do_step from model.rs. A wall is not a floor and not a box,
// so it is never entered and never overwritten. The cell the keeper leaves
// is put back from the original map, where his start and every box were
// stored as floor, and a mark was kept. That is how a mark reappears.

using System.Text;

namespace BaseZ47;

public sealed class App
{
    const ushort CmPlay = 1;
    const ushort CmExit = 2;
    const ushort CmMenu = 3;
    const ushort CmUp = 4;
    const ushort CmDown = 5;
    const ushort CmLeft = 6;
    const ushort CmRight = 7;
    const ushort CmRestart = 8;
    const ushort CmUndo = 9;
    const ushort CmNext = 10;
    const ushort CmPrev = 11;
    const ushort CmHelp = 12;
    const ushort CmDismiss = 13;

    readonly Owlosui owl;
    readonly Warehouse[] maps;
    readonly Stack<Shot> undo = new();

    Warehouse play = null!;
    int level;
    int ox, oy, cols, contentRows;

    ushort menu;
    ushort menuList;
    ushort window;
    // The board is a canvas: cells with colours, put there in one Blit per
    // move. The line above it is a static, given new words with SetText.
    ushort board;
    ushort caption;
    ushort box;
    bool locked;
    bool finished;

    public App(Owlosui owl)
    {
        this.owl = owl;
        maps = new Warehouse[Maps.All.Length];
        for (var i = 0; i < maps.Length; i++) maps[i] = Warehouse.Load(Maps.All[i]);
        ShowMenu();
        MenuKeys();
    }

    public bool OnCommand(ushort cmd)
    {
        if (locked && cmd is not (CmDismiss or CmExit)) return true;

        switch (cmd)
        {
            case CmHelp:
                Tell("Push every box onto a mark. A box can only be pushed, and only one at a time. A wall does not move. Arrows or WASD. R restarts, Z takes the step back, N and P change the level. Q or Esc returns to the list.");
                return true;
            case CmDismiss:
                CloseBox();
                if (finished)
                {
                    finished = false;
                    locked = false;
                    CloseGame();
                    ShowMenu();
                    MenuKeys();
                }
                return true;

            case CmPlay:
                level = owl.Current(menuList);
                if ((uint)level >= (uint)maps.Length) level = 0;
                CloseMenu();
                StartLevel();
                GameKeys();
                return true;
            case CmMenu:
                CloseBox();
                CloseGame();
                ShowMenu();
                MenuKeys();
                return true;
            case CmExit:
                return false;

            case CmUp: return Move(0, -1);
            case CmDown: return Move(0, 1);
            case CmLeft: return Move(-1, 0);
            case CmRight: return Move(1, 0);
            case CmRestart:
                StartLevel();
                return true;
            case CmUndo:
                Undo();
                return true;
            case CmNext:
                if (level + 1 < maps.Length) { level++; StartLevel(); }
                return true;
            case CmPrev:
                if (level > 0) { level--; StartLevel(); }
                return true;
            default:
                return true;
        }
    }

    bool Move(int dx, int dy)
    {
        if (window == 0) return true;
        if (!play.TryMove(dy, dx, undo)) return true;
        if (play.Won)
        {
            if (level + 1 < maps.Length)
            {
                level++;
                StartLevel();
            }
            else
            {
                locked = true;
                finished = true;
                Tell("All 60 levels are clear.");
            }
        }
        else Redraw();
        return true;
    }

    void Undo()
    {
        if (window == 0 || undo.Count == 0) return;
        var shot = undo.Pop();
        play.Cur = shot.Cur;
        play.Hero = shot.Hero;
        play.Moves = shot.Moves;
        Redraw();
    }

    void ShowMenu()
    {
        menu = owl.Window("BASE-Z 47", 44, 20, style: Owlosui.Style.Dialog, closeCmd: CmExit);
        owl.Static(menu, 1, 0, "Sixty warehouses. Enter plays.", 40);
        var names = new string[maps.Length];
        for (var i = 0; i < names.Length; i++) names[i] = $"Level {i + 1,2}";
        menuList = owl.List(menu, 0, 1, 40, 14, names);
        owl.Buttons(menu,
            new Owlosui.Button("~P~lay", CmPlay, Default: true),
            ("~Q~uit", CmExit));
    }

    void CloseMenu()
    {
        if (menu == 0) return;
        owl.Close(menu);
        menu = 0;
        menuList = 0;
    }

    void StartLevel()
    {
        undo.Clear();
        play = maps[level].Copy();
        if (window != 0) owl.Close(window);

        // The window is the screen. The maze is centered inside it, which
        // is what custom_draw does on the 200 by 48 terminal.
        var winW = Math.Max(owl.Width, 20);
        var winH = Math.Max(owl.Height - 1, 8);
        window = owl.Window("BASE-Z 47", winW, winH, 0, 0, closeCmd: CmMenu);

        var clientW = winW - 2;
        var clientH = winH - 2;
        play.Measure(out var right, out contentRows);
        cols = right + 1;
        // ((screen / cell) / 2 - content / 2) * cell, from view.rs.
        // There the cell is 5 wide and 2 tall. Here it is 2 wide and 1 tall.
        ox = ((clientW / Warehouse.CellW) / 2 - right / 2) * Warehouse.CellW;
        oy = clientH / 2 - contentRows / 2;
        if (ox < 0) ox = 0;
        if (oy < 1) oy = 1;

        caption = owl.Static(window, 1, Math.Max(0, oy - 1), "", clientW - 2);
        board = owl.Canvas(window, ox, oy, cols * Warehouse.CellW, contentRows);
        Redraw();
    }

    void CloseGame()
    {
        if (window == 0) return;
        owl.Close(window);
        window = 0;
        board = 0;
        caption = 0;
        undo.Clear();
    }

    void Redraw()
    {
        owl.SetText(caption, $"Level {level + 1}  moves {play.Moves}  {play.OnGoals}/{play.Goals}");
        // The whole board in one block: every cell is two glyphs wide, and
        // each kind of cell has its colour. A wall is grey stone, the floor
        // is dark, a mark is yellow, a box brown until it sits on a mark and
        // turns green, and the keeper is white.
        var w = cols * Warehouse.CellW;
        var chars = new char[w * contentRows];
        var attrs = new byte[w * contentRows];
        for (var y = 0; y < contentRows; y++)
            for (var x = 0; x < cols; x++)
            {
                var (glyphs, attr) = play.Cell(y * Warehouse.W + x);
                var at = y * w + x * Warehouse.CellW;
                chars[at] = glyphs[0];
                chars[at + 1] = glyphs[1];
                attrs[at] = attr;
                attrs[at + 1] = attr;
            }
        owl.Blit(board, 0, 0, w, contentRows, chars, attrs);
    }

    void MenuKeys() => owl.StatusLine(
        new Owlosui.StatusItem("~F1~ Help", CmHelp, ConsoleKey.F1));

    void GameKeys() => owl.StatusLine(
        new Owlosui.StatusItem("^", CmUp, ConsoleKey.UpArrow),
        new Owlosui.StatusItem("v", CmDown, ConsoleKey.DownArrow),
        new Owlosui.StatusItem("<", CmLeft, ConsoleKey.LeftArrow),
        new Owlosui.StatusItem(">", CmRight, ConsoleKey.RightArrow),
        new Owlosui.StatusItem("W", CmUp, ConsoleKey.W),
        new Owlosui.StatusItem("A", CmLeft, ConsoleKey.A),
        new Owlosui.StatusItem("S", CmDown, ConsoleKey.S),
        new Owlosui.StatusItem("D", CmRight, ConsoleKey.D),
        new Owlosui.StatusItem("~R~estart", CmRestart, ConsoleKey.R),
        new Owlosui.StatusItem("~Z~undo", CmUndo, ConsoleKey.Z),
        new Owlosui.StatusItem("~N~ext", CmNext, ConsoleKey.N),
        new Owlosui.StatusItem("~P~rev", CmPrev, ConsoleKey.P),
        new Owlosui.StatusItem("~Q~uit", CmMenu, ConsoleKey.Q),
        new Owlosui.StatusItem("Esc", CmMenu, ConsoleKey.Escape),
        new Owlosui.StatusItem("~F1~", CmHelp, ConsoleKey.F1));

    void Tell(string text)
    {
        CloseBox();
        box = owl.MessageBox("BASE-Z 47", text, ("~O~K", CmDismiss));
    }

    void CloseBox()
    {
        if (box == 0) return;
        owl.Close(box);
        box = 0;
    }

    public static void Main()
    {
        using var owl = new Owlosui();
        var app = new App(owl);
        owl.Run(app.OnCommand);
    }
}

public sealed class Warehouse
{
    public const int W = 30;
    public const int H = 20;
    public const int CellW = 2;

    public char[] Cur = Array.Empty<char>();
    public char[] Orig = Array.Empty<char>();
    public int Hero;
    public int Moves;
    public int Goals;

    public static Warehouse Load(string src)
    {
        var lines = src.Split('\n');
        var w = new Warehouse { Cur = new char[W * H], Orig = new char[W * H] };
        for (var i = 0; i < w.Cur.Length; i++)
        {
            w.Cur[i] = ' ';
            w.Orig[i] = ' ';
        }
        for (var y = 0; y < H && y < lines.Length; y++)
        {
            var line = lines[y];
            for (var x = 0; x < W && x < line.Length; x++)
            {
                var c = line[x];
                var i = y * W + x;
                w.Cur[i] = c;
                // load_level: the keeper and the boxes are floor in the
                // original, everything else is kept. A mark under a box
                // would be lost; these maps do not have one.
                if (c == '@')
                {
                    w.Hero = i;
                    w.Orig[i] = ' ';
                }
                else if (c == '$')
                {
                    w.Orig[i] = ' ';
                }
                else
                {
                    w.Orig[i] = c;
                    if (c == '.') w.Goals++;
                }
            }
        }
        return w;
    }

    public Warehouse Copy() => new()
    {
        Cur = (char[])Cur.Clone(),
        Orig = Orig,
        Hero = Hero,
        Goals = Goals,
    };

    /// <summary>
    /// Rightmost column that is not floor, and the first all-floor row.
    /// view.rs, the loop that feeds the centering.
    /// </summary>
    public void Measure(out int right, out int rows)
    {
        right = 0;
        var bottom = H;
        for (var y = 0; y < H; y++)
        {
            var cxr = 0;
            for (var x = W - 1; x >= 0; x--)
            {
                if (Cur[y * W + x] != ' ') { cxr = x; break; }
            }
            if (cxr != 0)
            {
                if (right < cxr) right = cxr;
            }
            else if (bottom > y)
            {
                bottom = y;
            }
        }
        if (bottom <= 0) bottom = 1;
        rows = bottom;
    }

    public int OnGoals
    {
        get
        {
            var n = 0;
            for (var i = 0; i < Cur.Length; i++)
                if (Cur[i] == '$' && Orig[i] == '.') n++;
            return n;
        }
    }

    public bool Won
    {
        get
        {
            if (Goals == 0) return false;
            for (var i = 0; i < Cur.Length; i++)
                if (Cur[i] == '$' && Orig[i] != '.') return false;
            return true;
        }
    }

    public string Row(int y, int cols)
    {
        var sb = new StringBuilder(cols * CellW);
        for (var x = 0; x < cols; x++) sb.Append(Glyph(y * W + x));
        return sb.ToString();
    }

    static string Glyph(char c, bool goal) => c switch
    {
        '#' => "\u2593\u2593",           // ▓▓  wall
        '$' => goal ? "\u2588\u2588"     // ██  box sitting on a mark
                    : "\u2592\u2592",     // ▒▒  box
        '@' => "/\\",                    // keeper, two glyphs like the small picture
        '.' => "\u00b7\u00b7",           // ··  mark
        _ => "\u2591\u2591",             // ░░  floor, never a space
    };

    string Glyph(int i) => Glyph(Cur[i], Orig[i] == '.');

    // The colours of the original Rust game, in the sixteen we have and in
    // their darker halves: the bright ones read as acid on a text screen,
    // and the originals are pastel. A wall is blue textured over dark
    // blue, the floor dark green, a mark an orange square on it, a box
    // brown, the keeper dark red. Outside the warehouse: nothing - clear
    // cells, so the window's own colour shows through, whatever it is.
    public static readonly byte WallAttr = Owlosui.Attr(ConsoleColor.Blue, ConsoleColor.DarkBlue);
    public static readonly byte FloorAttr = Owlosui.Attr(ConsoleColor.DarkGreen, ConsoleColor.DarkGreen);
    public static readonly byte MarkAttr = Owlosui.Attr(ConsoleColor.DarkYellow, ConsoleColor.DarkGreen);
    public static readonly byte BoxAttr = Owlosui.Attr(ConsoleColor.DarkYellow, ConsoleColor.DarkYellow);
    public static readonly byte PlacedAttr = Owlosui.Attr(ConsoleColor.DarkYellow, ConsoleColor.DarkGreen);
    public static readonly byte KeeperAttr = Owlosui.Attr(ConsoleColor.DarkRed, ConsoleColor.DarkGreen);
    public static readonly byte OutsideAttr = Owlosui.ClearAttr;

    /// <summary>
    /// Which cells are inside the warehouse. The map says ' ' for the floor
    /// and for the nothing around the walls alike; what tells them apart is
    /// that the keeper can reach the one and not the other. A flood from
    /// where he starts, stopped by walls.
    /// </summary>
    bool[]? inside;

    bool[] Inside()
    {
        if (inside != null) return inside;
        var seen = new bool[W * H];
        var todo = new Stack<int>();
        todo.Push(Hero);
        seen[Hero] = true;
        while (todo.Count > 0)
        {
            var i = todo.Pop();
            var (r, c) = (i / W, i % W);
            foreach (var (dr, dc) in new[] { (-1, 0), (1, 0), (0, -1), (0, 1) })
            {
                var (nr, nc) = (r + dr, c + dc);
                if (nr < 0 || nc < 0 || nr >= H || nc >= W) continue;
                var j = nr * W + nc;
                if (seen[j] || Orig[j] == '#') continue;
                seen[j] = true;
                todo.Push(j);
            }
        }
        inside = seen;
        return seen;
    }

    /// <summary>The two glyphs and the colour of a cell, for the canvas.</summary>
    public (string glyphs, byte attr) Cell(int i)
    {
        var c = Cur[i];
        var goal = Orig[i] == '.';
        return c switch
        {
            '#' => ("\u2593\u2593", WallAttr),        // ▓▓  blue over dark blue: stone, not paint
            '$' => goal ? ("\u2592\u2592", PlacedAttr) : ("  ", BoxAttr),
            '@' => ("\u2590\u258c", KeeperAttr),      // ▐▌  a figure, narrower than its cell
            '.' => ("\u25a0\u25a0", MarkAttr),        // ■■  the orange square on the floor
            _ => Inside()[i] ? ("  ", FloorAttr) : (new string(Owlosui.ClearChar, 2), OutsideAttr),
        };
    }

    /// <summary>
    /// model.rs can_step, then do_step. Row delta first, the way do_step is
    /// called. A wall returns false and the grid is left alone.
    /// </summary>
    public bool TryMove(int drow, int dcol, Stack<Shot> undo)
    {
        var r = Hero / W;
        var c = Hero % W;
        var nr = r + drow;
        var nc = c + dcol;
        if ((uint)nr >= H || (uint)nc >= W) return false;

        var ni = nr * W + nc;
        var dest = Cur[ni];
        var bi = -1;
        if (dest == ' ' || dest == '.')
        {
            // empty floor, or a mark
        }
        else if (dest == '$')
        {
            var br = nr + drow;
            var bc = nc + dcol;
            if ((uint)br >= H || (uint)bc >= W) return false;
            bi = br * W + bc;
            if (Cur[bi] != ' ' && Cur[bi] != '.') return false;
        }
        else
        {
            return false;
        }

        undo.Push(new Shot((char[])Cur.Clone(), Hero, Moves));
        if (bi >= 0) Cur[bi] = '$';
        var hi = Hero;
        if (Orig[hi] != '$') Cur[hi] = Orig[hi];
        Cur[ni] = '@';
        Hero = ni;
        Moves++;
        return true;
    }
}

public readonly record struct Shot(char[] Cur, int Hero, int Moves);
