// The fifteen puzzle with no screen in it: sixteen places, fifteen tiles
// and a hole, and one rule - a tile beside the hole slides into it.
//
// Kept apart from the window for the same reason the calculator's
// arithmetic is: a test can play it, and another program can put it on
// a different board.

namespace OwlosDemo.Puzzle;

public sealed class Board
{
    public const int Size = 4;

    /// <summary>Tiles by place, row by row; 0 is the hole.</summary>
    public readonly int[] Tiles = new int[Size * Size];

    /// <summary>Slides made since the last scramble.</summary>
    public int Moves { get; private set; }

    public Board()
    {
        Reset();
    }

    /// <summary>The solved board: 1 to 15, then the hole.</summary>
    public void Reset()
    {
        for (var i = 0; i < Tiles.Length; i++) Tiles[i] = (i + 1) % Tiles.Length;
        Moves = 0;
    }

    public int Hole => Array.IndexOf(Tiles, 0);

    public bool Solved
    {
        get
        {
            for (var i = 0; i < Tiles.Length - 1; i++)
                if (Tiles[i] != i + 1) return false;
            return true;
        }
    }

    /// <summary>The tile at a place slides into the hole if it is beside it.</summary>
    public bool Slide(int at)
    {
        if (at < 0 || at >= Tiles.Length || Tiles[at] == 0) return false;
        var hole = Hole;
        var (r1, c1, r2, c2) = (at / Size, at % Size, hole / Size, hole % Size);
        if (Math.Abs(r1 - r2) + Math.Abs(c1 - c2) != 1) return false;
        (Tiles[at], Tiles[hole]) = (0, Tiles[at]);
        Moves++;
        return true;
    }

    /// <summary>
    /// Some hundreds of legal slides from the solved board: always
    /// solvable, which a shuffle of the tiles is only half the time.
    /// Seeded, so a test can know the board it gets.
    /// </summary>
    public void Scramble(int seed = 47, int slides = 200)
    {
        Reset();
        var rnd = new Random(seed);
        for (var n = 0; n < slides; n++)
        {
            var hole = Hole;
            var (r, c) = (hole / Size, hole % Size);
            var moves = new List<int>();
            if (r > 0) moves.Add(hole - Size);
            if (r < Size - 1) moves.Add(hole + Size);
            if (c > 0) moves.Add(hole - 1);
            if (c < Size - 1) moves.Add(hole + 1);
            Slide(moves[rnd.Next(moves.Count)]);
        }
        Moves = 0;
    }
}
