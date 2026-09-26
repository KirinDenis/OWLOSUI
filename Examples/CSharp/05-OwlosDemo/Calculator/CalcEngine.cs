// The calculator's arithmetic, with no screen in it.
//
// A string goes in, a string comes out, and nothing here knows that a
// window exists: the engine can be used from a test, a script or another
// program's command line exactly as it is used from CalculatorWindow.
//
// It is a small expression language, the one every calculator that grew
// past four functions ends up with:
//
//   1 + 2 * 3          precedence: * and / before + and -
//   (1 + 2) * 3        parentheses
//   2 ^ 10             power, right-associative: 2^3^2 is 2^9
//   17 mod 5           remainder, also 17 % 5
//   -3 ^ 2             unary minus binds looser than ^: this is -9
//   sqrt(2)  sin(30)   functions; trig in degrees or radians, a switch
//   pi  e              constants
//   5!                 factorial
//   FF + 1             in hex mode; 1010 in binary mode; 17 in octal
//
// Numbers are read in the current base and the answer is written in it.
// In a base other than ten the answer is cut to its integer part, as every
// programmer's calculator does, because a hexadecimal fraction is a thing
// nobody wants to read.
//
// Errors are words, not exceptions, on the way out: "Divide by zero",
// "Invalid input", "Overflow", "Error". A calculator that crashes on 1/0
// is not a calculator.

using System.Globalization;

namespace OwlosDemo.Calculator;

public enum NumberBase { Dec = 10, Hex = 16, Bin = 2, Oct = 8 }

/// <summary>Thrown inside <see cref="CalcEngine.Eval"/>; <see cref="CalcEngine.Evaluate"/> turns it into words.</summary>
public sealed class CalcError : Exception
{
    public CalcError(string message) : base(message) { }
}

public sealed class CalcEngine
{
    /// <summary>The base numbers are read and written in.</summary>
    public NumberBase Base { get; set; } = NumberBase.Dec;

    /// <summary>Trigonometry in degrees (true) or radians (false).</summary>
    public bool Degrees { get; set; } = true;

    /// <summary>The one memory of a pocket calculator: M+, MR, MC.</summary>
    public double Memory { get; set; }

    public bool HasMemory => Memory != 0;

    // ---------------------------------------------------------------- results

    /// <summary>The answer as the display should show it, or the error as words.</summary>
    public string Evaluate(string expression)
    {
        try
        {
            return Format(Eval(expression));
        }
        catch (CalcError e)
        {
            return e.Message;
        }
    }

    /// <summary>The answer as a number; throws <see cref="CalcError"/> when there is none.</summary>
    public double Eval(string expression)
    {
        var p = new Parser(expression, this);
        var v = p.ParseAll();
        if (double.IsNaN(v)) throw new CalcError("Invalid input");
        if (double.IsInfinity(v)) throw new CalcError("Overflow");
        return v;
    }

    /// <summary>A number written in the current base.</summary>
    public string Format(double v)
    {
        if (Base == NumberBase.Dec)
        {
            // Fifteen significant digits is what a double honestly has, and
            // rounding there is what turns sin(pi)'s 1.2e-16 back into 0.
            // Anything closer to zero than a double's own noise is zero:
            // sin(180) comes out as 1.2e-16 and nobody wants to read that.
            if (Math.Abs(v) < 1e-14) return "0";
            var mag = Math.Abs(v);
            if (mag >= 1e15 || mag < 1e-9)
                return v.ToString("G15", CultureInfo.InvariantCulture);
            var digits = 14 - (int)Math.Floor(Math.Log10(mag));
            var r = Math.Round(v, Math.Clamp(digits, 0, 15), MidpointRounding.AwayFromZero);
            return r.ToString("0.###############", CultureInfo.InvariantCulture);
        }
        if (Math.Abs(v) >= 9.2e18) throw new CalcError("Overflow");
        var n = (long)Math.Truncate(v);
        var s = Convert.ToString(Math.Abs(n), (int)Base).ToUpperInvariant();
        return n < 0 ? "-" + s : s;
    }

    /// <summary>The characters a number may be made of in the current base.</summary>
    public bool IsDigit(char c) => Digit(c) is int d && d < (int)Base;

    private static int? Digit(char c)
    {
        if (c >= '0' && c <= '9') return c - '0';
        c = char.ToUpperInvariant(c);
        if (c >= 'A' && c <= 'F') return c - 'A' + 10;
        return null;
    }

    // ----------------------------------------------------------------- parser

    /// <summary>
    /// Recursive descent, one method per level of precedence. The grammar,
    /// loosest first:
    ///
    ///   sum     := product (('+' | '-') product)*
    ///   product := unary (('*' | '/' | 'mod' | '%') unary)*
    ///   unary   := ('-' | '+') unary | power
    ///   power   := postfix ('^' unary)?
    ///   postfix := atom ('!')*
    ///   atom    := number | '(' sum ')' | name '(' sum ')' | name
    /// </summary>
    private sealed class Parser
    {
        private readonly string s;
        private readonly CalcEngine e;
        private int i;

        public Parser(string text, CalcEngine engine)
        {
            s = text;
            e = engine;
        }

        public double ParseAll()
        {
            Skip();
            if (i >= s.Length) throw new CalcError("");
            var v = Sum();
            Skip();
            if (i < s.Length) throw new CalcError("Error");
            return v;
        }

        private void Skip()
        {
            while (i < s.Length && s[i] == ' ') i++;
        }

        private bool Take(char c)
        {
            Skip();
            if (i < s.Length && s[i] == c)
            {
                i++;
                return true;
            }
            return false;
        }

        private bool TakeWord(string w)
        {
            Skip();
            if (string.Compare(s, i, w, 0, w.Length, StringComparison.OrdinalIgnoreCase) != 0) return false;
            var end = i + w.Length;
            if (end < s.Length && char.IsLetterOrDigit(s[end])) return false;
            i = end;
            return true;
        }

        private double Sum()
        {
            var v = Product();
            while (true)
            {
                if (Take('+')) v += Product();
                else if (Take('-')) v -= Product();
                else return v;
            }
        }

        private double Product()
        {
            var v = Unary();
            while (true)
            {
                if (Take('*')) v *= Unary();
                else if (Take('/'))
                {
                    var d = Unary();
                    if (d == 0) throw new CalcError("Divide by zero");
                    v /= d;
                }
                else if (Take('%') || TakeWord("mod"))
                {
                    var d = Unary();
                    if (d == 0) throw new CalcError("Divide by zero");
                    v %= d;
                }
                else return v;
            }
        }

        private double Unary()
        {
            if (Take('-')) return -Unary();
            if (Take('+')) return Unary();
            return Power();
        }

        private double Power()
        {
            var v = Postfix();
            // Right-associative, and the exponent may carry its own sign:
            // 2^-1 is a half.
            if (Take('^')) v = Math.Pow(v, Unary());
            return v;
        }

        private double Postfix()
        {
            var v = Atom();
            while (Take('!'))
            {
                if (v < 0 || v != Math.Floor(v)) throw new CalcError("Invalid input");
                if (v > 170) throw new CalcError("Overflow");
                double f = 1;
                for (var k = 2; k <= (int)v; k++) f *= k;
                v = f;
            }
            return v;
        }

        private double Atom()
        {
            Skip();
            if (i >= s.Length) throw new CalcError("Error");
            if (Take('('))
            {
                var v = Sum();
                if (!Take(')')) throw new CalcError("Error");
                return v;
            }

            // A run of letters and digits is a number if every character is
            // a digit of the base - which in hex includes A to F, so "ace"
            // is a number there and "cos" is not - and a name otherwise.
            var start = i;
            while (i < s.Length && (char.IsLetterOrDigit(s[i]) || s[i] == '.')) i++;
            if (i == start) throw new CalcError("Error");
            var word = s[start..i];
            if (word.All(c => e.IsDigit(c) || c == '.')) return Number(word);
            return Name(word.ToLowerInvariant());
        }

        private double Number(string word)
        {
            if (e.Base == NumberBase.Dec)
            {
                if (double.TryParse(word, NumberStyles.Float, CultureInfo.InvariantCulture, out var d)) return d;
                throw new CalcError("Error");
            }
            if (word.Contains('.')) throw new CalcError("Invalid input");
            try
            {
                return Convert.ToInt64(word, (int)e.Base);
            }
            catch (OverflowException)
            {
                throw new CalcError("Overflow");
            }
        }

        private double Name(string name)
        {
            switch (name)
            {
                case "pi": return Math.PI;
                case "e": return Math.E;
                case "m": return e.Memory;
            }
            if (!Take('(')) throw new CalcError("Error");
            var x = Sum();
            if (!Take(')')) throw new CalcError("Error");
            return name switch
            {
                "sin" => Math.Sin(ToRad(x)),
                "cos" => Math.Cos(ToRad(x)),
                "tan" => Math.Tan(ToRad(x)),
                "asin" => FromRad(Math.Asin(x)),
                "acos" => FromRad(Math.Acos(x)),
                "atan" => FromRad(Math.Atan(x)),
                "sqrt" => x < 0 ? throw new CalcError("Invalid input") : Math.Sqrt(x),
                "ln" => x <= 0 ? throw new CalcError("Invalid input") : Math.Log(x),
                "log" => x <= 0 ? throw new CalcError("Invalid input") : Math.Log10(x),
                "log2" => x <= 0 ? throw new CalcError("Invalid input") : Math.Log2(x),
                "exp" => Math.Exp(x),
                "abs" => Math.Abs(x),
                "floor" => Math.Floor(x),
                "ceil" => Math.Ceiling(x),
                "round" => Math.Round(x, MidpointRounding.AwayFromZero),
                "sqr" => x * x,
                _ => throw new CalcError("Error"),
            };
        }

        private double ToRad(double x) => e.Degrees ? x * Math.PI / 180 : x;
        private double FromRad(double x) => e.Degrees ? x * 180 / Math.PI : x;
    }
}
