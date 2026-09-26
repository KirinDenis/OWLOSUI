// The calculator's window: a display, a keypad of real buttons, and the
// modes down the side. The arithmetic is in CalcEngine; this file only
// decides what a key puts into the display and when to ask the engine.
//
//   ┌──────────────────────── Calculator ────────────────────────┐
//   │                                                            │
//   │ [ 12+34*2                                     ]  M         │
//   │                                                            │
//   │   C     MC     MR     M+     (      )        ( ) Dec       │
//   │  sin    cos    tan    √      x²     ^        (•) Hex       │
//   │   7      8      9     A      B      /        ( ) Bin       │
//   │   4      5      6     C      D      *        ( ) Oct       │
//   │   1      2      3     E      F      -                      │
//   │   0      .      ±     1/x     =      +        [ ] Radians   │
//   └────────────────────────────────────────────────────────────┘
//
// The digits of every base stand together and the four operations make
// the right-hand column, where a hand that knows a pocket calculator
// looks for them.
//
// Three things about the keys, each a rule of the toolkit rather than of
// this program:
//
//   * The keys are buttons, so they look like every other button, go down
//     when pressed, and can be reached with Tab. A keypad drawn on a
//     canvas would have had to reinvent all three.
//   * Clear is red and top left: the one key that throws work away wears
//     the colour that says so and stands where the hand does not fall by
//     accident. Operators and functions are cyan, digits green, so the
//     eye finds a number in a row of signs.
//   * A click does not move the focus, and the keypad is not a Tab stop.
//     The display keeps the caret, so digits and signs can simply be typed,
//     Enter is =, Escape is C, and "7 [+] 8 [=]" typed and clicked in any
//     mixture reads as one line. Choosing a mode hands the caret back to
//     the display for the same reason.
//
// Digits a base does not have are disabled rather than hidden: A to F
// outside hex, 8 and 9 in octal, and so on. Switching base converts the
// number on the display, as a programmer's calculator does.
//
// The class owns a range of command numbers and answers `Handles` for
// them, so a program can drop it in beside its own commands.

using System.Text;

namespace OwlosDemo.Calculator;

public sealed class CalculatorWindow
{
    /// <summary>The commands this window sends: [First, Last].</summary>
    public const ushort CmFirst = 100, CmLast = 159;

    private const ushort CmClear = 100, CmMemClear = 101, CmMemRecall = 102, CmMemAdd = 103;
    private const ushort CmEquals = 104, CmNegate = 105, CmInverse = 106;
    /// <summary>Keys that put text into the display: CmKey + their index in <see cref="Keys"/>.</summary>
    private const ushort CmKey = 110;

    /// <summary>The window, while it is open. Zero is closed.</summary>
    public ushort Id { get; private set; }

    public readonly CalcEngine Engine = new();

    private readonly Owlosui owl;
    private readonly ushort closeCmd;
    private ushort display, memoryMark, baseRadio, radiansCheck;
    private readonly List<(ushort row, int index)> keyPlaces = new();
    private NumberBase shownBase = NumberBase.Dec;
    private bool shownRadians;

    /// <summary>What a text key types. The order is the order of the commands.</summary>
    private static readonly (string label, string text, Owlosui.ButtonStyle style)[] Keys =
    {
        ("(", "(", Owlosui.ButtonStyle.Accent), (")", ")", Owlosui.ButtonStyle.Accent),
        ("sin", "sin(", Owlosui.ButtonStyle.Accent), ("cos", "cos(", Owlosui.ButtonStyle.Accent),
        ("tan", "tan(", Owlosui.ButtonStyle.Accent), ("√", "sqrt(", Owlosui.ButtonStyle.Accent),
        ("x²", "^2", Owlosui.ButtonStyle.Accent), ("^", "^", Owlosui.ButtonStyle.Accent),
        ("7", "7", Owlosui.ButtonStyle.Normal), ("8", "8", Owlosui.ButtonStyle.Normal), ("9", "9", Owlosui.ButtonStyle.Normal),
        ("A", "A", Owlosui.ButtonStyle.Normal), ("B", "B", Owlosui.ButtonStyle.Normal), ("/", "/", Owlosui.ButtonStyle.Accent),
        ("4", "4", Owlosui.ButtonStyle.Normal), ("5", "5", Owlosui.ButtonStyle.Normal), ("6", "6", Owlosui.ButtonStyle.Normal),
        ("C", "C", Owlosui.ButtonStyle.Normal), ("D", "D", Owlosui.ButtonStyle.Normal), ("*", "*", Owlosui.ButtonStyle.Accent),
        ("1", "1", Owlosui.ButtonStyle.Normal), ("2", "2", Owlosui.ButtonStyle.Normal), ("3", "3", Owlosui.ButtonStyle.Normal),
        ("E", "E", Owlosui.ButtonStyle.Normal), ("F", "F", Owlosui.ButtonStyle.Normal), ("-", "-", Owlosui.ButtonStyle.Accent),
        ("0", "0", Owlosui.ButtonStyle.Normal), (".", ".", Owlosui.ButtonStyle.Normal),
        ("+", "+", Owlosui.ButtonStyle.Accent),
    };

    public CalculatorWindow(Owlosui owl, ushort closeCmd)
    {
        this.owl = owl;
        this.closeCmd = closeCmd;
    }

    /// <summary>Open it, or bring it to the front if it is open.</summary>
    public void Show()
    {
        if (Id != 0) { owl.Activate(Id); return; }
        Id = owl.Window("Calculator", 70, 18, style: Owlosui.Style.Dialog, closeCmd: closeCmd);
        // A blank row above the display: a display against the title bar
        // looks pinned to it.
        display = owl.Input(Id, 1, 1, 52, "", "");
        memoryMark = owl.Static(Id, 56, 1, " ", w: 1);
        keyPlaces.Clear();

        // The keypad: six rows of six, every label padded to three so the
        // columns line up. A key is either a text key, by its index in
        // Keys, or one of the few that do something to the whole display.
        Row(3, new Owlosui.Button(Pad("C"), CmClear, Style: Owlosui.ButtonStyle.Danger, Cancel: true),
               Key("MC", CmMemClear, Owlosui.ButtonStyle.Accent),
               Key("MR", CmMemRecall, Owlosui.ButtonStyle.Accent), Key("M+", CmMemAdd, Owlosui.ButtonStyle.Accent),
               Text(0), Text(1));
        Row(5, Text(2), Text(3), Text(4), Text(5), Text(6), Text(7));
        Row(7, Text(8), Text(9), Text(10), Text(11), Text(12), Text(13));
        Row(9, Text(14), Text(15), Text(16), Text(17), Text(18), Text(19));
        Row(11, Text(20), Text(21), Text(22), Text(23), Text(24), Text(25));
        Row(13, Text(26), Text(27), Key("±", CmNegate, Owlosui.ButtonStyle.Accent), Key("1/x", CmInverse, Owlosui.ButtonStyle.Accent),
                Key("=", CmEquals, Owlosui.ButtonStyle.Accent, isDefault: true), Text(28));

        // The modes: radio buttons for the base, a check box for degrees.
        baseRadio = owl.Cluster(Id, 56, 3, 12, new[] { "~D~ec", "~H~ex", "~B~in", "~O~ct" }, single: true);
        // Degrees are the default, and a check box starts clear, so the
        // box is the other one: ticked means radians.
        radiansCheck = owl.Cluster(Id, 56, 8, 12, new[] { "~R~adians" });
        shownBase = NumberBase.Dec;
        shownRadians = false;
        Engine.Base = NumberBase.Dec;
        Engine.Degrees = true;
        EnableDigits();
    }

    private Owlosui.Button Text(int ix)
    {
        var (label, _, style) = Keys[ix];
        return new Owlosui.Button(Pad(label), (ushort)(CmKey + ix), Style: style);
    }

    private static Owlosui.Button Key(string label, ushort cmd, Owlosui.ButtonStyle style, bool isDefault = false) =>
        new(Pad(label), cmd, Default: isDefault, Style: style);

    /// <summary>Three characters wide, the label in the middle.</summary>
    private static string Pad(string label) => label.Length switch
    {
        1 => " " + label + " ",
        2 => " " + label,
        _ => label,
    };

    private void Row(int y, params Owlosui.Button[] buttons)
    {
        var row = owl.ButtonRow(Id, 1, y, selectable: false, buttons);
        for (var i = 0; i < buttons.Length; i++)
        {
            var cmd = buttons[i].Cmd;
            if (cmd >= CmKey && cmd < CmKey + Keys.Length) keyPlaces.Add((row, i));
        }
    }

    // ------------------------------------------------------------- commands

    public bool Handles(ushort cmd) => Id != 0 && cmd >= CmFirst && cmd <= CmLast;

    public void OnCommand(ushort cmd)
    {
        if (Id == 0) return;
        var text = owl.GetText(display);
        switch (cmd)
        {
            case CmClear:
                owl.SetText(display, "");
                break;
            case CmEquals:
                owl.SetText(display, Engine.Evaluate(text));
                break;
            case CmNegate:
                // A leading minus comes and goes; anything else is wrapped.
                if (text.Length == 0) break;
                owl.SetText(display, text.StartsWith('-') ? text[1..] : "-" + text);
                break;
            case CmInverse:
                if (text.Length > 0) owl.SetText(display, "1/(" + text + ")");
                break;
            case CmMemClear:
                Engine.Memory = 0;
                ShowMemory();
                break;
            case CmMemRecall:
                owl.SetText(display, text + Engine.Format(Engine.Memory));
                break;
            case CmMemAdd:
                try
                {
                    Engine.Memory += Engine.Eval(text);
                    ShowMemory();
                }
                catch (CalcError e)
                {
                    owl.SetText(display, e.Message);
                }
                break;
            default:
                if (cmd >= CmKey && cmd < CmKey + Keys.Length)
                {
                    // Typing after an answer starts a new line, as a pocket
                    // calculator does, unless the key is an operator that
                    // wants the answer as its left side.
                    var key = Keys[cmd - CmKey].text;
                    if (IsError(text)) text = "";
                    owl.SetText(display, text + key);
                }
                break;
        }
    }

    private static bool IsError(string s) => s is "Error" or "Divide by zero" or "Invalid input" or "Overflow";

    private void ShowMemory() => owl.SetText(memoryMark, Engine.HasMemory ? "M" : " ");

    /// <summary>The modes are clusters, which send nothing: look at them after every input.</summary>
    public void Poll()
    {
        if (Id == 0) return;
        // The one with the dot, not the one under the cursor: Tab can
        // park the cursor on Hex while the answer is still Dec.
        var (chosen, _) = owl.ClusterState(baseRadio);
        var which = Array.IndexOf(chosen, true);
        var wanted = which switch { 1 => NumberBase.Hex, 2 => NumberBase.Bin, 3 => NumberBase.Oct, _ => NumberBase.Dec };
        if (wanted != shownBase)
        {
            // Convert what is on the display, if it is a number.
            var text = owl.GetText(display);
            double? value = null;
            if (text.Length > 0)
            {
                try { value = Engine.Eval(text); } catch (CalcError) { }
            }
            Engine.Base = wanted;
            shownBase = wanted;
            if (value is { } v)
            {
                try { owl.SetText(display, Engine.Format(v)); }
                catch (CalcError e) { owl.SetText(display, e.Message); }
            }
            EnableDigits();
            // The mode is chosen; the hand goes back to the number.
            owl.Focus(display);
        }
        var (on, _) = owl.ClusterState(radiansCheck);
        var radians = on.Length > 0 && on[0];
        Engine.Degrees = !radians;
        if (radians != shownRadians)
        {
            shownRadians = radians;
            owl.Focus(display);
        }
    }

    /// <summary>Only the digits of the current base can be pressed; the point only in decimal.</summary>
    private void EnableDigits()
    {
        for (var i = 0; i < Keys.Length; i++)
        {
            var label = Keys[i].label;
            if (label.Length != 1) continue;
            var c = label[0];
            bool on;
            if (c == '.') on = Engine.Base == NumberBase.Dec;
            else if (char.IsLetterOrDigit(c)) on = Engine.IsDigit(c);
            else continue;
            var (row, index) = keyPlaces[i];
            owl.EnableButton(row, index, on);
        }
    }

    /// <summary>The window has been closed by the program: forget it.</summary>
    public void Closed()
    {
        Id = 0;
        keyPlaces.Clear();
    }

    /// <summary>Everything in the display, for a test to read.</summary>
    public string Display => Id == 0 ? "" : owl.GetText(display);

    /// <summary>The display's id: its history is what was evaluated.</summary>
    public ushort DisplayId => display;
}
