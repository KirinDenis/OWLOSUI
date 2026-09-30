// A month on a canvas, with the day names over it and three buttons under
// it. The canvas is drawn again whenever the month or the chosen day
// changes; nothing is remembered on the far side of the wire but the
// cells.
//
//   ┌────────────── Calendar ───────────────┐
//   │     September 2026                    │
//   │     Su  Mo  Tu  We  Th  Fr  Sa        │
//   │             1   2   3   4   5         │
//   │      6   7   8   9  10  11  12        │
//   │     13  14  15  16  17  18  19        │
//   │     ...                               │
//   │ Wednesday, 23 September 2026          │
//   │            [< Prev]  [Today]  [Next >] │
//   └───────────────────────────────────────┘
//
// Today is a green block, the chosen day a cyan one, weekends in red. A
// click on a day chooses it and names it in full under the grid; the
// buttons turn the page or come back to today.
//
// Like the calculator, the class owns a range of command numbers and
// answers Handles for them, so it drops into any program beside the
// program's own.

using System.Globalization;

namespace OwlosDemo.Calendar;

public sealed class CalendarWindow
{
    /// <summary>The commands this window sends: [First, Last].</summary>
    public const ushort CmFirst = 160, CmLast = 169;
    public const ushort CmPrev = 160, CmNext = 161, CmToday = 162;

    /// <summary>The window, while it is open. Zero is closed.</summary>
    public ushort Id { get; private set; }

    /// <summary>The first day of the month on show.</summary>
    public DateTime Month { get; private set; }

    /// <summary>The day the person clicked, if any.</summary>
    public DateTime? Selected { get; private set; }

    // A day is a cell four wide and two tall; seven across, six down, and
    // one row of names above them.
    private const int CellW = 4, CellH = 2, Columns = 7, Rows = 6;
    private const int GridW = CellW * Columns, GridH = 1 + CellH * Rows;

    private readonly Owlosui owl;
    private readonly ushort closeCmd;
    private ushort title, grid, caption;

    public CalendarWindow(Owlosui owl, ushort closeCmd)
    {
        this.owl = owl;
        this.closeCmd = closeCmd;
        Month = FirstOf(DateTime.Today);
    }

    private static DateTime FirstOf(DateTime d) => new(d.Year, d.Month, 1);

    /// <summary>Open it, or bring it to the front if it is open.</summary>
    public void Show()
    {
        if (Id != 0) { owl.Activate(Id); return; }
        // A row of air under the title, then the month, the grid, a
        // row, the chosen day, and the buttons.
        Id = owl.Window("Calendar", 40, 21, style: Owlosui.Style.Dialog, closeCmd: closeCmd);
        title = owl.Static(Id, 5, 1, "", 30);
        grid = owl.Canvas(Id, 5, 2, GridW, GridH);
        caption = owl.Static(Id, 1, 16, "", 36);
        owl.Buttons(Id, ("~<~ Prev", CmPrev), ("~T~oday", CmToday), ("~N~ext ~>~", CmNext));
        Draw();
    }

    public bool Handles(ushort cmd) => Id != 0 && cmd >= CmFirst && cmd <= CmLast;

    public void OnCommand(ushort cmd)
    {
        if (Id == 0) return;
        switch (cmd)
        {
            case CmPrev: Month = Month.AddMonths(-1); Selected = null; break;
            case CmNext: Month = Month.AddMonths(1); Selected = null; break;
            case CmToday: Month = FirstOf(DateTime.Today); Selected = DateTime.Today; break;
            default: return;
        }
        Draw();
    }

    /// <summary>A click on the grid chooses a day.</summary>
    public void Poll()
    {
        if (Id == 0) return;
        if (owl.CanvasClick(grid) is not { } p || p.y < 1) return;
        var slot = ((p.y - 1) / CellH) * Columns + p.x / CellW;
        var day = slot - (int)Month.DayOfWeek + 1;
        if (day < 1 || day > DateTime.DaysInMonth(Month.Year, Month.Month)) return;
        Selected = Month.AddDays(day - 1);
        Draw();
    }

    private void Draw()
    {
        owl.SetText(title, Month.ToString("MMMM yyyy", CultureInfo.InvariantCulture));
        owl.SetText(caption, Selected?.ToString("dddd, d MMMM yyyy", CultureInfo.InvariantCulture) ?? "");

        var text = new char[GridW * GridH];
        var attrs = new byte[GridW * GridH];
        var plain = Owlosui.Attr(ConsoleColor.Black, ConsoleColor.Gray);
        var names = Owlosui.Attr(ConsoleColor.DarkBlue, ConsoleColor.Gray);
        var weekend = Owlosui.Attr(ConsoleColor.DarkRed, ConsoleColor.Gray);
        var today = Owlosui.Attr(ConsoleColor.White, ConsoleColor.DarkGreen);
        var chosen = Owlosui.Attr(ConsoleColor.Black, ConsoleColor.DarkCyan);
        var both = Owlosui.Attr(ConsoleColor.Yellow, ConsoleColor.DarkGreen);
        Array.Fill(text, ' ');
        Array.Fill(attrs, plain);

        // A cell: its top line carries the words, both lines the colour.
        void Cell(int col, int row, string s, byte a)
        {
            var x0 = col * CellW;
            var y0 = 1 + row * CellH;
            for (var dy = 0; dy < CellH; dy++)
                for (var dx = 0; dx < CellW; dx++)
                {
                    var at = (y0 + dy) * GridW + x0 + dx;
                    text[at] = dy == 0 && dx - 1 < s.Length && dx >= 1 ? s[dx - 1] : ' ';
                    attrs[at] = a;
                }
        }

        var dayNames = new[] { "Su", "Mo", "Tu", "We", "Th", "Fr", "Sa" };
        for (var c = 0; c < Columns; c++)
        {
            var x = c * CellW + 1;
            text[x] = dayNames[c][0];
            text[x + 1] = dayNames[c][1];
            attrs[x] = attrs[x + 1] = c is 0 or 6 ? weekend : names;
        }

        var first = (int)Month.DayOfWeek;
        var days = DateTime.DaysInMonth(Month.Year, Month.Month);
        for (var d = 1; d <= days; d++)
        {
            var slot = first + d - 1;
            var (col, row) = (slot % Columns, slot / Columns);
            var date = Month.AddDays(d - 1);
            var isToday = date == DateTime.Today;
            var isChosen = Selected == date;
            var a = (isToday, isChosen) switch
            {
                (true, true) => both,
                (true, false) => today,
                (false, true) => chosen,
                _ => col is 0 or 6 ? weekend : plain,
            };
            Cell(col, row, d.ToString().PadLeft(2), a);
        }
        owl.Blit(grid, 0, 0, GridW, GridH, text, attrs);
    }

    /// <summary>The window has been closed by the program: forget it.</summary>
    public void Closed() => Id = 0;
}
