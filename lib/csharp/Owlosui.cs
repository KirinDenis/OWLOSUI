// The C# side of the wire.
//
// This is a client of `owlosui-serve`, not a port of the toolkit: it starts
// the server, sends what happened, asks for the frame and puts the cells on
// System.Console. Every window, button and keystroke is handled by the same
// Rust core that drives the terminal demo, and this file knows nothing about
// how any of it works. That is the point. See lib/PROTOCOL.md.
//
// Use:
//
//     using var owl = new Owlosui();
//     var w = owl.Window("Hello", 40, 10);
//     owl.Static(w, 2, 1, "Hello, world!");
//     owl.Buttons(w, ("~O~K", 1));
//     owl.Run(cmd => cmd != 1);
//
// Handles are numbers (ushort), as they will be on DOS. Commands are your
// own ushorts; 0 means "nothing happened" and cannot be a button.
//
// Two halves. The first talks to the server and works anywhere, with no
// console at all — that is what a test uses. The second, `Run`, is the
// console: it reads keys and the mouse, draws frames, and is the only part
// that knows it is on Windows.

using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;
using static Win32;

public sealed class Owlosui : IDisposable
{
    // ------------------------------------------------------------ building

    /// <summary>
    /// Start the server and make a desktop the size of the console — or the
    /// size given, for a program with no console to ask: a test, or a client
    /// that will put the cells somewhere else.
    /// </summary>
    public Owlosui(string? serverPath = null, int width = 0, int height = 0)
    {
        var path = serverPath ?? FindServer();
        var psi = new ProcessStartInfo(path)
        {
            UseShellExecute = false,
            RedirectStandardInput = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
        };
        proc = Process.Start(psi) ?? throw new InvalidOperationException($"could not start {path}");
        proc.ErrorDataReceived += (_, e) => { if (e.Data != null) stderr.AppendLine(e.Data); };
        proc.BeginErrorReadLine();
        toServer = proc.StandardInput.BaseStream;
        fromServer = proc.StandardOutput.BaseStream;

        if (width <= 0 || height <= 0)
        {
            try
            {
                width = Console.WindowWidth;
                height = Console.WindowHeight;
            }
            catch (IOException)
            {
                // No console at all — output is a file, or there never was
                // one. Eighty by twenty-five is what a text screen is.
                (width, height) = (80, 25);
            }
        }
        Width = Math.Max(width, 20);
        Height = Math.Max(height, 5);
        Call(Op.Init, W.I16(Width), W.I16(Height));
    }

    public int Width { get; private set; }
    public int Height { get; private set; }

    /// <summary>The desktop changed size — the console window did, or a test says so.</summary>
    public void Resize(int width, int height)
    {
        Width = Math.Max(width, 20);
        Height = Math.Max(height, 5);
        Call(Op.Resize, W.I16(Width), W.I16(Height));
        prev = null;
    }

    /// <summary>
    /// A window. (-1, -1) — the default — means centred, now and after every
    /// resize until it is dragged. <paramref name="closeCmd"/>, if given, is
    /// what the close box sends instead of closing: the program then decides,
    /// which is how "save changes?" gets asked first.
    /// </summary>
    public ushort Window(string title, int w, int h, int x = -1, int y = -1,
                         Style style = Style.Document, ushort parent = 0, ushort closeCmd = 0)
    {
        var r = Call(Op.Window, W.U16(parent), W.Rect(x, y, w, h), new[] { (byte)style }, W.Str(title), W.U16(closeCmd));
        return R.U16(r);
    }

    /// <summary>A text editor filling its window. Read-only makes it a viewer.</summary>
    public ushort Text(ushort parent, string text = "", bool readOnly = false)
    {
        var flags = (byte)(readOnly ? 1 : 0);
        var r = Call(Op.Text, W.U16(parent), W.Rect(0, 0, 0, 0), new byte[] { 0, flags }, W.Str(text));
        return R.U16(r);
    }

    /// <summary>A boxed memo at a place of its own inside the window.</summary>
    public ushort Memo(ushort parent, int x, int y, int w, int h, string text = "", bool readOnly = false)
    {
        var flags = (byte)((readOnly ? 1 : 0) | 2);
        var r = Call(Op.Text, W.U16(parent), W.Rect(x, y, w, h), new byte[] { 1, flags }, W.Str(text));
        return R.U16(r);
    }

    /// <summary>Words. Wrapped to the width given; one line if no height.</summary>
    public ushort Static(ushort parent, int x, int y, string text, int w = 0, int h = 1)
    {
        if (w <= 0) w = text.Length;
        var r = Call(Op.Static, W.U16(parent), W.Rect(x, y, w, h), W.Str(text));
        return R.U16(r);
    }

    /// <summary>A one-line field: <c>Name:</c> followed by what is typed.</summary>
    public ushort Input(ushort parent, int x, int y, int w, string label, string text = "", int max = 0)
    {
        var r = Call(Op.Input, W.U16(parent), W.Rect(x, y, w, 1), W.U16((ushort)max), W.Str(label), W.Str(text));
        return R.U16(r);
    }

    /// <summary>
    /// A button: a label with its hotkey between tildes, the command it
    /// sends, and whether Enter presses it from anywhere in the dialog.
    /// A plain tuple <c>("~O~K", 1)</c> converts to one.
    /// </summary>
    public readonly record struct Button(string Label, ushort Cmd, bool Default = false)
    {
        public static implicit operator Button((string label, ushort cmd) t) => new(t.label, t.cmd);
    }

    /// <summary>
    /// Buttons, bottom right. Enter presses the one marked Default, or the
    /// first if none is; Escape presses the last. So put Cancel last — and
    /// if the safe answer should take both keys, mark it Default and put it
    /// last.
    /// </summary>
    public ushort Buttons(ushort parent, params Button[] buttons)
    {
        var r = Call(Op.Buttons, W.U16(parent), W.Buttons(buttons));
        return R.U16(r);
    }

    /// <summary>The commonest dialog there is: some words and a row of buttons.</summary>
    public ushort MessageBox(string title, string text, params Button[] buttons)
    {
        var r = Call(Op.MessageBox, W.Str(title), W.Str(text), W.Buttons(buttons));
        return R.U16(r);
    }

    public void Close(ushort id) => Call(Op.Close, W.U16(id));

    /// <summary>What a Text, Memo or Input holds right now.</summary>
    public string GetText(ushort id) => R.Str(Call(Op.GetText, W.U16(id)));

    // -------------------------------------------------------------- events

    public enum MouseKind : byte { Down = 0, Up = 1, Drag = 2, Move = 3, WheelUp = 4, WheelDown = 5 }
    public enum MouseButton : byte { Left = 0, Right = 1, Middle = 2 }

    /// <summary>One mouse event, in screen cells.</summary>
    public void SendMouse(MouseKind kind, int x, int y, MouseButton button = MouseButton.Left)
    {
        Call(Op.Mouse, new[] { (byte)kind, (byte)button }, W.I16(x), W.I16(y));
    }

    /// <summary>Press and release at a cell. What a test means by "click".</summary>
    public void Click(int x, int y, MouseButton button = MouseButton.Left)
    {
        SendMouse(MouseKind.Down, x, y, button);
        SendMouse(MouseKind.Up, x, y, button);
    }

    /// <summary>Press at one cell, drag to another, release. Moving or resizing a window.</summary>
    public void Drag(int fromX, int fromY, int toX, int toY)
    {
        SendMouse(MouseKind.Down, fromX, fromY);
        SendMouse(MouseKind.Drag, toX, toY);
        SendMouse(MouseKind.Up, toX, toY);
    }

    /// <summary>Send one key. Returns false if it was one the wire has no name for.</summary>
    public bool SendKey(ConsoleKeyInfo k)
    {
        byte mods = 0;
        if (k.Modifiers.HasFlag(ConsoleModifiers.Shift)) mods |= 1;
        if (k.Modifiers.HasFlag(ConsoleModifiers.Control)) mods |= 2;
        if (k.Modifiers.HasFlag(ConsoleModifiers.Alt)) mods |= 4;

        static (byte kind, ushort value)? Named(ushort v) => ((byte)2, v);
        (byte kind, ushort value)? code = k.Key switch
        {
            ConsoleKey.Enter => Named(0),
            ConsoleKey.Escape => Named(1),
            ConsoleKey.Tab => Named((mods & 1) != 0 ? (ushort)3 : (ushort)2),
            ConsoleKey.Backspace => Named(4),
            ConsoleKey.Delete => Named(5),
            ConsoleKey.Insert => Named(6),
            ConsoleKey.Home => Named(7),
            ConsoleKey.End => Named(8),
            ConsoleKey.PageUp => Named(9),
            ConsoleKey.PageDown => Named(10),
            ConsoleKey.UpArrow => Named(11),
            ConsoleKey.DownArrow => Named(12),
            ConsoleKey.LeftArrow => Named(13),
            ConsoleKey.RightArrow => Named(14),
            >= ConsoleKey.F1 and <= ConsoleKey.F12 => ((byte)1, (ushort)(k.Key - ConsoleKey.F1 + 1)),
            _ => null,
        };
        if (code is null)
        {
            var ch = k.KeyChar;
            // Ctrl+letter arrives as a control character; the core wants the
            // letter and the modifier, as a keymap table would spell it.
            if (ch >= 1 && ch <= 26 && (mods & 2) != 0) ch = (char)('a' + ch - 1);
            if (ch == '\0' || char.IsControl(ch)) return false;
            // Shift is already in the character's case. Sending it as well
            // would make Shift+A a different key from A, which it is not.
            if ((mods & 6) == 0) mods = 0;
            code = (0, ch);
        }
        Call(Op.Key, new[] { code.Value.kind }, W.U16(code.Value.value), new[] { mods });
        return true;
    }

    /// <summary>A named key with modifiers, for a program or a test that has no keyboard in hand.</summary>
    public void Press(ConsoleKey key, bool shift = false, bool alt = false, bool ctrl = false, char ch = '\0')
    {
        SendKey(new ConsoleKeyInfo(ch, key, shift, alt, ctrl));
    }

    /// <summary>Type a string, one character at a time.</summary>
    public void Type(string text)
    {
        foreach (var ch in text)
            SendKey(new ConsoleKeyInfo(ch, ConsoleKey.NoName, false, false, false));
    }

    /// <summary>What the last event caused: a button pressed, a menu command chosen, or 0.</summary>
    public (ushort pressed, ushort command) Take()
    {
        var r = Call(Op.Take);
        return (R.U16(r, 0), R.U16(r, 2));
    }

    // --------------------------------------------------------------- frame

    /// <summary>The screen as the core drew it: two bytes per cell, glyph then attribute.</summary>
    public Frame GetFrame()
    {
        var r = Call(Op.Frame);
        return new Frame(R.I16(r, 0), R.I16(r, 2), R.I16(r, 4), R.I16(r, 6), r[8..]);
    }

    public readonly record struct Frame(int W, int H, int CursorX, int CursorY, byte[] Cells)
    {
        public byte Glyph(int x, int y) => Cells[(y * W + x) * 2];
        public byte Attr(int x, int y) => Cells[(y * W + x) * 2 + 1];
        public char Char(int x, int y) => Cp437[Glyph(x, y)];

        /// <summary>One row as text, for looking at and for searching.</summary>
        public string Row(int y)
        {
            var sb = new StringBuilder(W);
            for (var x = 0; x < W; x++) sb.Append(Char(x, y));
            return sb.ToString();
        }

        /// <summary>Where some words are on the screen, if they are. Top-left of the first match.</summary>
        public (int x, int y)? Find(string text)
        {
            for (var y = 0; y < H; y++)
            {
                var x = Row(y).IndexOf(text, StringComparison.Ordinal);
                if (x >= 0) return (x, y);
            }
            return null;
        }

        /// <summary>The whole screen as text, for a failing test to print.</summary>
        public override string ToString()
        {
            var sb = new StringBuilder();
            for (var y = 0; y < H; y++) sb.AppendLine(Row(y));
            return sb.ToString();
        }
    }

    // ------------------------------------------------------------- running

    /// <summary>
    /// Draw, wait for input, send it, report what it caused — until
    /// <paramref name="onCommand"/> returns false. This is the whole event
    /// loop of an application; most programs never need anything else.
    /// </summary>
    public void Run(Func<ushort, bool> onCommand)
    {
        var enc = Console.OutputEncoding;
        Console.OutputEncoding = Encoding.UTF8;
        Console.CursorVisible = false;
        Console.Clear();
        prev = null;
        using var input = ConsoleInput.Open();
        // Two ways to ask for the mouse, because there are two kinds of
        // console. SetConsoleMode (in ConsoleInput) is what the classic
        // console listens to. A terminal on the other side of a pseudo
        // console - Windows Terminal, an editor's terminal pane - listens
        // for the VT request instead, and forwards mouse reports only after
        // it has seen one; the pseudo console then turns them into the same
        // input records. Ask both ways and every console answers.
        var vt = VtMouse.On();
        try
        {
            while (true)
            {
                Draw();
                foreach (var ev in input.Read())
                {
                    var sent = ev switch
                    {
                        KeyEv k => SendKey(k.Key),
                        MouseEv m => SendMouseEv(m),
                        ResizeEv r => SendResize(r),
                        _ => false,
                    };
                    if (!sent) continue;
                    var (pressed, command) = Take();
                    if (pressed != 0 && !onCommand(pressed)) return;
                    if (command != 0 && !onCommand(command)) return;
                }
            }
        }
        finally
        {
            vt.Off();
            Console.ResetColor();
            Console.Clear();
            Console.CursorVisible = true;
            Console.OutputEncoding = enc;
        }
    }

    /// <summary>
    /// The VT side of asking for the mouse: DECSET 1000 (buttons), 1002
    /// (drag), 1006 (SGR reports, so positions past column 223 survive).
    /// Written only if the console will interpret escape sequences rather
    /// than print them, which is what ENABLE_VIRTUAL_TERMINAL_PROCESSING
    /// on the output handle means.
    /// </summary>
    private readonly struct VtMouse
    {
        private readonly bool on;
        private VtMouse(bool on) => this.on = on;

        public static VtMouse On()
        {
            if (!OperatingSystem.IsWindows()) return new VtMouse(false);
            var h = GetStdHandle(STD_OUTPUT_HANDLE);
            if (!GetConsoleMode(h, out var mode)) return new VtMouse(false);
            if ((mode & ENABLE_VIRTUAL_TERMINAL_PROCESSING) == 0
                && !SetConsoleMode(h, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING))
                return new VtMouse(false);
            Console.Out.Write("\x1b[?1000h\x1b[?1002h\x1b[?1006h");
            Console.Out.Flush();
            return new VtMouse(true);
        }

        public void Off()
        {
            if (!on) return;
            Console.Out.Write("\x1b[?1006l\x1b[?1002l\x1b[?1000l");
            Console.Out.Flush();
        }
    }

    private bool SendMouseEv(MouseEv m)
    {
        SendMouse(m.Kind, m.X, m.Y, m.Button);
        return true;
    }

    private bool SendResize(ResizeEv r)
    {
        Console.Clear();
        Resize(r.W, r.H);
        return true;
    }

    private byte[]? prev;

    /// <summary>Fetch the frame and put what changed on the console.</summary>
    public void Draw()
    {
        try
        {
            DrawCells();
        }
        catch (ArgumentOutOfRangeException)
        {
            // The console changed size between the frame being asked for and
            // the cells being written - a person is dragging the window edge,
            // and the size event that explains it is already queued. Nothing
            // is lost: the next frame is drawn in full.
            prev = null;
        }
        catch (IOException)
        {
            prev = null;
        }
    }

    private void DrawCells()
    {
        var f = GetFrame();
        var full = prev is null || prev.Length != f.Cells.Length;
        // The console may be smaller than the frame for a moment (see Draw);
        // what does not fit is not written.
        var (bw, bh) = (Console.BufferWidth, Console.BufferHeight);
        var sb = new StringBuilder();
        for (var y = 0; y < Math.Min(f.H, bh); y++)
        {
            var x = 0;
            while (x < Math.Min(f.W, bw))
            {
                // The very last cell is left alone: writing it makes most
                // consoles scroll, and the toolkit's own terminal backend
                // skips it for the same reason.
                if (y == f.H - 1 && x == f.W - 1) break;
                if (y == bh - 1 && x == bw - 1) break;

                var i = (y * f.W + x) * 2;
                var changed = full || prev![i] != f.Cells[i] || prev[i + 1] != f.Cells[i + 1];
                if (!changed) { x++; continue; }

                // A run of changed cells with one attribute, written in one go.
                var attr = f.Cells[i + 1];
                sb.Clear();
                var start = x;
                while (x < Math.Min(f.W, bw) && !(y == f.H - 1 && x == f.W - 1) && !(y == bh - 1 && x == bw - 1))
                {
                    var j = (y * f.W + x) * 2;
                    if (f.Cells[j + 1] != attr) break;
                    sb.Append(Cp437[f.Cells[j]]);
                    x++;
                }
                Console.SetCursorPosition(start, y);
                Console.ForegroundColor = (ConsoleColor)(attr & 0x0F);
                Console.BackgroundColor = (ConsoleColor)(attr >> 4);
                Console.Write(sb);
            }
        }
        prev = f.Cells;

        if (f.CursorX >= 0 && f.CursorY >= 0 && f.CursorX < Math.Min(f.W, bw) && f.CursorY < Math.Min(f.H, bh))
        {
            Console.SetCursorPosition(f.CursorX, f.CursorY);
            Console.CursorVisible = true;
        }
        else
        {
            Console.CursorVisible = false;
        }
    }

    // ------------------------------------------------------- console input

    private abstract record Ev;
    private sealed record KeyEv(ConsoleKeyInfo Key) : Ev;
    private sealed record MouseEv(MouseKind Kind, MouseButton Button, int X, int Y) : Ev;
    private sealed record ResizeEv(int W, int H) : Ev;

    /// <summary>
    /// Where keys and the mouse come from.
    ///
    /// <c>System.Console</c> has no idea a mouse exists, which is why the
    /// first C# programs could not press a button or drag a corner with one.
    /// On Windows the console *does* report the mouse — through
    /// <c>ReadConsoleInput</c>, which also says when the window changes size
    /// — so on Windows that is what is used, and keys come through it too.
    /// Anywhere else, keys only, until somebody needs more.
    /// </summary>
    private abstract class ConsoleInput : IDisposable
    {
        public static ConsoleInput Open() =>
            OperatingSystem.IsWindows() ? new WindowsInput() : new KeysOnly();

        public abstract IEnumerable<Ev> Read();
        public virtual void Dispose() { }

        private sealed class KeysOnly : ConsoleInput
        {
            public override IEnumerable<Ev> Read()
            {
                yield return new KeyEv(Console.ReadKey(intercept: true));
            }
        }

        private sealed class WindowsInput : ConsoleInput
        {
            private readonly IntPtr handle;
            private readonly uint savedMode;
            private readonly INPUT_RECORD[] records = new INPUT_RECORD[32];
            private uint buttons;   // held right now, for Down/Up and Drag

            public WindowsInput()
            {
                handle = GetStdHandle(STD_INPUT_HANDLE);
                if (!GetConsoleMode(handle, out savedMode))
                    throw new OwlosuiException("no console input handle");
                // Mouse and window events on; the line editor, echo, Ctrl+C
                // processing and quick-edit (which steals the mouse for
                // selecting text) off. Virtual-terminal input off too, or
                // the mouse would arrive as escape sequences instead.
                var mode = savedMode;
                mode &= ~(ENABLE_PROCESSED_INPUT | ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT
                          | ENABLE_QUICK_EDIT_MODE | ENABLE_VIRTUAL_TERMINAL_INPUT);
                mode |= ENABLE_WINDOW_INPUT | ENABLE_MOUSE_INPUT | ENABLE_EXTENDED_FLAGS;
                var ok = SetConsoleMode(handle, mode);
                GetConsoleMode(handle, out var now);
                Trace($"console mode: was {savedMode:X4}, asked {mode:X4}, set {(ok ? "ok" : "FAILED " + Marshal.GetLastWin32Error())}, now {now:X4}");
                if (!ok)
                    throw new OwlosuiException($"SetConsoleMode failed ({Marshal.GetLastWin32Error()}); no mouse");
            }

            // OWLOSUI_TRACE=<file> writes every input record here, raw, and
            // every event made from it. For the day the mouse "does nothing"
            // and the question is whether the console delivered anything at
            // all - which cannot be answered from inside the program any
            // other way.
            private static readonly string? trace = Environment.GetEnvironmentVariable("OWLOSUI_TRACE");

            // Opened and closed for every line, so the file is readable by
            // whoever is watching while the program is still running - a
            // test agent, most likely. A held-open writer showed nothing
            // until the process ended, which is exactly when nobody was
            // looking any more.
            private static void Trace(string line)
            {
                if (string.IsNullOrEmpty(trace)) return;
                try { File.AppendAllText(trace, $"{DateTime.Now:HH:mm:ss.fff} {line}{Environment.NewLine}"); }
                catch (IOException) { }
            }

            public override void Dispose() => SetConsoleMode(handle, savedMode);

            public override IEnumerable<Ev> Read()
            {
                if (!ReadConsoleInputW(handle, records, (uint)records.Length, out var n))
                    throw new OwlosuiException("ReadConsoleInput failed");
                for (var i = 0; i < n; i++)
                {
                    var r = records[i];
                    if (trace != null)
                        Trace(r.EventType switch
                        {
                            KEY_EVENT => $"key   down={r.KeyEvent.bKeyDown} vk={r.KeyEvent.wVirtualKeyCode:X2} ch={r.KeyEvent.UnicodeChar:X4} ctl={r.KeyEvent.dwControlKeyState:X4}",
                            MOUSE_EVENT => $"mouse at=({r.MouseEvent.dwMousePosition.X},{r.MouseEvent.dwMousePosition.Y}) buttons={r.MouseEvent.dwButtonState:X8} flags={r.MouseEvent.dwEventFlags:X2} top={Console.WindowTop}",
                            _ => $"type {r.EventType}",
                        });
                    switch (r.EventType)
                    {
                        case KEY_EVENT:
                            if (Key(r.KeyEvent) is { } k) yield return k;
                            break;
                        case MOUSE_EVENT:
                            foreach (var m in Mouse(r.MouseEvent)) yield return m;
                            break;
                        case WINDOW_BUFFER_SIZE_EVENT:
                            // The record carries the buffer size, which can
                            // include scrollback; the window is what we draw on.
                            yield return new ResizeEv(Console.WindowWidth, Console.WindowHeight);
                            break;
                    }
                }
            }

            private static KeyEv? Key(KEY_EVENT_RECORD k)
            {
                if (k.bKeyDown == 0) return null;
                var vk = k.wVirtualKeyCode;
                // Modifiers on their own are not keys.
                if (vk is 0x10 or 0x11 or 0x12 or 0x14 or 0x5B or 0x5C or 0x90 or 0x91) return null;

                var shift = (k.dwControlKeyState & SHIFT_PRESSED) != 0;
                var ctrl = (k.dwControlKeyState & (LEFT_CTRL_PRESSED | RIGHT_CTRL_PRESSED)) != 0;
                var alt = (k.dwControlKeyState & (LEFT_ALT_PRESSED | RIGHT_ALT_PRESSED)) != 0;
                var ch = (char)k.UnicodeChar;
                // With Alt held the console reports no character for a letter
                // or digit; the key code says which it was.
                if (ch == '\0' && vk >= 0x30 && vk <= 0x5A)
                    ch = shift ? (char)vk : char.ToLowerInvariant((char)vk);
                return new KeyEv(new ConsoleKeyInfo(ch, (ConsoleKey)vk, shift, alt, ctrl));
            }

            private IEnumerable<Ev> Mouse(MOUSE_EVENT_RECORD m)
            {
                var x = m.dwMousePosition.X;
                var y = m.dwMousePosition.Y - Console.WindowTop;

                if ((m.dwEventFlags & MOUSE_WHEELED) != 0)
                {
                    var delta = (short)(m.dwButtonState >> 16);
                    yield return new MouseEv(delta > 0 ? MouseKind.WheelUp : MouseKind.WheelDown, MouseButton.Left, x, y);
                    yield break;
                }
                if ((m.dwEventFlags & MOUSE_HWHEELED) != 0) yield break;

                if ((m.dwEventFlags & MOUSE_MOVED) != 0)
                {
                    // Motion with a button held is a drag; motion without one
                    // is nothing the toolkit acts on, and not worth a round
                    // trip per cell.
                    if (buttons != 0)
                        yield return new MouseEv(MouseKind.Drag, MouseButton.Left, x, y);
                    yield break;
                }

                // A press or a release: whichever bits changed.
                var now = m.dwButtonState & 0x7;
                var changed = now ^ buttons;
                buttons = now;
                for (var b = 0; b < 3; b++)
                {
                    var bit = 1u << b;
                    if ((changed & bit) == 0) continue;
                    var button = b switch { 0 => MouseButton.Left, 1 => MouseButton.Right, _ => MouseButton.Middle };
                    var kind = (now & bit) != 0 ? MouseKind.Down : MouseKind.Up;
                    Trace($"  -> {kind} {button} at ({x},{y})");
                    yield return new MouseEv(kind, button, x, y);
                }
            }
        }
    }

    // ------------------------------------------------------------- the wire

    public enum Style : byte
    {
        Document = 0x00,          // blue, resizable, zoomable, closable
        Help = 0x10,              // cyan
        Dialog = 0x20 | 0x02 | 0x04,  // grey, fixed size
        ModalDialog = Dialog | 0x01,
    }

    private static class Op
    {
        public const byte Quit = 0x00, Init = 0x01, Resize = 0x02;
        public const byte Window = 0x10, Text = 0x11, Static = 0x12, Input = 0x13, Buttons = 0x14, MessageBox = 0x15;
        public const byte Close = 0x20, GetText = 0x21;
        public const byte Key = 0x30, Mouse = 0x31;
        public const byte Frame = 0x40, Take = 0x41;
    }

    private readonly Process proc;
    private readonly Stream toServer;
    private readonly Stream fromServer;
    private readonly StringBuilder stderr = new();

    /// <summary>One request, one reply. An error reply becomes an exception that says what the server said.</summary>
    private byte[] Call(byte op, params byte[][] parts)
    {
        var len = parts.Sum(p => p.Length);
        var head = new byte[3];
        head[0] = op;
        BitConverter.TryWriteBytes(head.AsSpan(1), (ushort)len);
        toServer.Write(head);
        foreach (var p in parts) toServer.Write(p);
        toServer.Flush();

        var reply = new byte[3];
        ReadExactly(reply);
        var n = BitConverter.ToUInt16(reply, 1);
        var body = new byte[n];
        ReadExactly(body);
        if (reply[0] != 0)
            throw new OwlosuiException($"op {op:X2}: {R.Str(body)}");
        return body;
    }

    private void ReadExactly(byte[] buf)
    {
        var got = 0;
        while (got < buf.Length)
        {
            var n = fromServer.Read(buf, got, buf.Length - got);
            if (n <= 0)
                throw new OwlosuiException("owlosui-serve went away" + (stderr.Length > 0 ? ":\n" + stderr : ""));
            got += n;
        }
    }

    public void Dispose()
    {
        try
        {
            if (!proc.HasExited)
            {
                Call(Op.Quit);
                proc.WaitForExit(1000);
            }
        }
        catch (OwlosuiException) { }
        proc.Dispose();
    }

    /// <summary>
    /// Where the server is. In order: OWLOSUI_SERVE in the environment, then
    /// the repository's own build next to this source tree, then the PATH.
    /// </summary>
    private static string FindServer()
    {
        var env = Environment.GetEnvironmentVariable("OWLOSUI_SERVE");
        if (!string.IsNullOrEmpty(env)) return env;

        var exe = OperatingSystem.IsWindows() ? "owlosui-serve.exe" : "owlosui-serve";
        for (var dir = new DirectoryInfo(AppContext.BaseDirectory); dir != null; dir = dir.Parent)
        {
            foreach (var profile in new[] { "release", "debug" })
            {
                var candidate = Path.Combine(dir.FullName, "target", profile, exe);
                if (File.Exists(candidate)) return candidate;
            }
        }
        // Let the OS search the PATH; a clear message if it cannot.
        return "owlosui-serve";
    }

    // ---------------------------------------------------------- byte helpers

    private static class W
    {
        public static byte[] U16(ushort v) => BitConverter.GetBytes(v);
        public static byte[] I16(int v) => BitConverter.GetBytes((short)v);
        public static byte[] Rect(int x, int y, int w, int h) =>
            I16(x).Concat(I16(y)).Concat(I16(w)).Concat(I16(h)).ToArray();
        public static byte[] Str(string s)
        {
            var b = Encoding.UTF8.GetBytes(s);
            if (b.Length > ushort.MaxValue) throw new ArgumentException("string too long for the wire");
            return U16((ushort)b.Length).Concat(b).ToArray();
        }
        public static byte[] Buttons(Button[] buttons)
        {
            if (buttons.Length == 0) throw new ArgumentException("a button row needs at least one button");
            var dflt = Array.FindIndex(buttons, b => b.Default);
            if (dflt < 0) dflt = 0;
            var v = new List<byte> { (byte)buttons.Length };
            for (var i = 0; i < buttons.Length; i++)
            {
                if (buttons[i].Cmd == 0) throw new ArgumentException($"button '{buttons[i].Label}' has command 0, which means none");
                v.AddRange(U16(buttons[i].Cmd));
                v.Add((byte)(i == dflt ? 1 : 0));
                v.AddRange(Str(buttons[i].Label));
            }
            return v.ToArray();
        }
    }

    private static class R
    {
        public static ushort U16(byte[] b, int at = 0) => BitConverter.ToUInt16(b, at);
        public static short I16(byte[] b, int at = 0) => BitConverter.ToInt16(b, at);
        public static string Str(byte[] b, int at = 0)
        {
            var n = BitConverter.ToUInt16(b, at);
            return Encoding.UTF8.GetString(b, at + 2, n);
        }
    }

    /// <summary>Code page 437, glyph index to Unicode. Same table as the Rust side.</summary>
    public static readonly string Cp437 = "\u0020\u263A\u263B\u2665\u2666\u2663\u2660\u2022\u25D8\u25CB\u25D9\u2642\u2640\u266A\u266B\u263C" +
        "\u25BA\u25C4\u2195\u203C\u00B6\u00A7\u25AC\u21A8\u2191\u2193\u2192\u2190\u221F\u2194\u25B2\u25BC" +
        "\u0020\u0021\u0022\u0023\u0024\u0025\u0026\u0027\u0028\u0029\u002A\u002B\u002C\u002D\u002E\u002F" +
        "\u0030\u0031\u0032\u0033\u0034\u0035\u0036\u0037\u0038\u0039\u003A\u003B\u003C\u003D\u003E\u003F" +
        "\u0040\u0041\u0042\u0043\u0044\u0045\u0046\u0047\u0048\u0049\u004A\u004B\u004C\u004D\u004E\u004F" +
        "\u0050\u0051\u0052\u0053\u0054\u0055\u0056\u0057\u0058\u0059\u005A\u005B\u005C\u005D\u005E\u005F" +
        "\u0060\u0061\u0062\u0063\u0064\u0065\u0066\u0067\u0068\u0069\u006A\u006B\u006C\u006D\u006E\u006F" +
        "\u0070\u0071\u0072\u0073\u0074\u0075\u0076\u0077\u0078\u0079\u007A\u007B\u007C\u007D\u007E\u2302" +
        "\u00C7\u00FC\u00E9\u00E2\u00E4\u00E0\u00E5\u00E7\u00EA\u00EB\u00E8\u00EF\u00EE\u00EC\u00C4\u00C5" +
        "\u00C9\u00E6\u00C6\u00F4\u00F6\u00F2\u00FB\u00F9\u00FF\u00D6\u00DC\u00A2\u00A3\u00A5\u20A7\u0192" +
        "\u00E1\u00ED\u00F3\u00FA\u00F1\u00D1\u00AA\u00BA\u00BF\u2310\u00AC\u00BD\u00BC\u00A1\u00AB\u00BB" +
        "\u2591\u2592\u2593\u2502\u2524\u2561\u2562\u2556\u2555\u2563\u2551\u2557\u255D\u255C\u255B\u2510" +
        "\u2514\u2534\u252C\u251C\u2500\u253C\u255E\u255F\u255A\u2554\u2569\u2566\u2560\u2550\u256C\u2567" +
        "\u2568\u2564\u2565\u2559\u2558\u2552\u2553\u256B\u256A\u2518\u250C\u2588\u2584\u258C\u2590\u2580" +
        "\u03B1\u00DF\u0393\u03C0\u03A3\u03C3\u00B5\u03C4\u03A6\u0398\u03A9\u03B4\u221E\u03C6\u03B5\u2229" +
        "\u2261\u00B1\u2265\u2264\u2320\u2321\u00F7\u2248\u00B0\u2219\u00B7\u221A\u207F\u00B2\u25A0\u0020";
}

public sealed class OwlosuiException : Exception
{
    public OwlosuiException(string message) : base(message) { }
}
