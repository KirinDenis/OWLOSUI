// A test agent for console programs.
//
// It starts a program in a console of its own - hidden, so nothing appears
// on anybody's desktop - attaches to that console, puts keys and mouse
// events straight into its input buffer and reads its screen back as cells.
// The program under test cannot tell the difference from a person: the same
// INPUT_RECORDs arrive through the same ReadConsoleInput.
//
// The headless tests prove the wire; this proves the layer between the wire
// and the console, which is the layer that broke first. It is not specific
// to OWLOSUI programs - it will drive anything that reads a console.
//
// One process may be attached to one console at a time, and attaching needs
// the process to have none of its own. So an agent runs in a process whose
// stdout is a pipe, never in one that is talking to a console - see
// Examples/CSharp/Tests, which spawns itself with `--agent` for the purpose.
//
// Windows only, by nature. Elsewhere `Start` throws, and a test suite
// should say "skipped" rather than fail.

using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;
using static Win32;

public sealed class ConsoleAgent : IDisposable
{
    private readonly Process process;
    private IntPtr input = INVALID_HANDLE_VALUE;
    private IntPtr output = INVALID_HANDLE_VALUE;
    private bool attached;

    /// <summary>Where the program's own trace went, if it wrote one (OWLOSUI_TRACE).</summary>
    public string TracePath { get; }

    private ConsoleAgent(Process process, string tracePath)
    {
        this.process = process;
        TracePath = tracePath;
    }

    /// <summary>
    /// Give up this process's own console. An agent process calls it once,
    /// first thing: a process attached to a console cannot attach to another.
    /// Anything written to stdout afterwards must be going to a pipe or a file.
    /// </summary>
    public static void ReleaseOwnConsole() => FreeConsole();

    /// <summary>
    /// Start a program in a hidden console and attach to it. Waits until the
    /// console exists; the program itself may still be starting - use
    /// <see cref="WaitFor"/> for the first thing it draws.
    /// </summary>
    public static ConsoleAgent Start(string exe, string args = "", string? workDir = null)
    {
        if (!OperatingSystem.IsWindows())
            throw new PlatformNotSupportedException("ConsoleAgent drives a Windows console");

        // The client writes its trace where this says. Set before the child
        // is made, since it inherits our environment.
        var trace = Path.Combine(Path.GetTempPath(), $"owlosui-agent-{Environment.ProcessId}-{DateTime.Now:HHmmss}.log");
        Environment.SetEnvironmentVariable("OWLOSUI_TRACE", trace);

        var si = new STARTUPINFOW
        {
            cb = Marshal.SizeOf<STARTUPINFOW>(),
            dwFlags = STARTF_USESHOWWINDOW,
            wShowWindow = SW_HIDE,
        };
        var cmd = new StringBuilder($"\"{exe}\" {args}");
        if (!CreateProcessW(null, cmd, IntPtr.Zero, IntPtr.Zero, false, CREATE_NEW_CONSOLE, IntPtr.Zero,
                            workDir, ref si, out var pi))
            throw new OwlosuiException($"could not start {exe}: error {Marshal.GetLastWin32Error()}");
        CloseHandle(pi.hThread);
        CloseHandle(pi.hProcess);

        var agent = new ConsoleAgent(Process.GetProcessById((int)pi.dwProcessId), trace);
        agent.Attach();
        return agent;
    }

    private void Attach()
    {
        // The console comes into being a moment after the process does.
        for (var tries = 0; tries < 100; tries++)
        {
            if (AttachConsole((uint)process.Id))
            {
                attached = true;
                break;
            }
            if (process.HasExited)
                throw new OwlosuiException("the program exited before it had a console");
            Thread.Sleep(50);
        }
        if (!attached)
            throw new OwlosuiException($"could not attach to the program's console: error {Marshal.GetLastWin32Error()}");

        input = CreateFileW("CONIN$", GENERIC_READ | GENERIC_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE,
                            IntPtr.Zero, OPEN_EXISTING, 0, IntPtr.Zero);
        output = CreateFileW("CONOUT$", GENERIC_READ | GENERIC_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE,
                             IntPtr.Zero, OPEN_EXISTING, 0, IntPtr.Zero);
        if (input == INVALID_HANDLE_VALUE || output == INVALID_HANDLE_VALUE)
            throw new OwlosuiException("could not open the program's console buffers");
    }

    public bool HasExited => process.HasExited;

    /// <summary>The input mode the program set on its console - whether it asked for the mouse.</summary>
    public uint InputMode
    {
        get
        {
            GetConsoleMode(input, out var mode);
            return mode;
        }
    }

    public bool MouseEnabled => (InputMode & ENABLE_MOUSE_INPUT) != 0 && (InputMode & ENABLE_QUICK_EDIT_MODE) == 0;

    // --------------------------------------------------------------- screen

    private static readonly Dictionary<char, byte> glyphOf = BuildGlyphMap();

    private static Dictionary<char, byte> BuildGlyphMap()
    {
        var m = new Dictionary<char, byte>();
        for (var i = 255; i >= 0; i--) m[Owlosui.Cp437[i]] = (byte)i;
        return m;
    }

    /// <summary>The visible window of the program's console, as the same kind of frame the wire returns.</summary>
    public Owlosui.Frame Screen()
    {
        if (!GetConsoleScreenBufferInfo(output, out var info))
            throw new OwlosuiException($"GetConsoleScreenBufferInfo failed: {Marshal.GetLastWin32Error()}");
        var win = info.srWindow;
        int w = win.Right - win.Left + 1, h = win.Bottom - win.Top + 1;
        var buf = new CHAR_INFO[w * h];
        var region = win;
        if (!ReadConsoleOutputW(output, buf, new COORD(w, h), new COORD(0, 0), ref region))
            throw new OwlosuiException($"ReadConsoleOutput failed: {Marshal.GetLastWin32Error()}");

        var cells = new byte[w * h * 2];
        for (var i = 0; i < w * h; i++)
        {
            cells[i * 2] = glyphOf.TryGetValue((char)buf[i].UnicodeChar, out var g) ? g : (byte)'?';
            cells[i * 2 + 1] = (byte)(buf[i].Attributes & 0xFF);
        }
        var cx = info.dwCursorPosition.X - win.Left;
        var cy = info.dwCursorPosition.Y - win.Top;
        return new Owlosui.Frame(w, h, cx, cy, cells);
    }

    /// <summary>Poll the screen until it satisfies the test, or give up. Returns the last screen seen.</summary>
    public Owlosui.Frame WaitFor(Func<Owlosui.Frame, bool> test, int timeoutMs = 5000)
    {
        var until = Environment.TickCount64 + timeoutMs;
        Owlosui.Frame last;
        do
        {
            last = Screen();
            if (test(last)) return last;
            Thread.Sleep(40);
        } while (Environment.TickCount64 < until);
        return last;
    }

    /// <summary>Wait for the program to end. True if it did.</summary>
    public bool WaitForExit(int timeoutMs = 5000) => process.WaitForExit(timeoutMs);

    /// <summary>
    /// Change the console's window and buffer to a size, as dragging the
    /// window's edge would. The program sees a WINDOW_BUFFER_SIZE_EVENT.
    /// The buffer may never be smaller than the window, so the order of the
    /// two calls depends on which way the size is going.
    /// </summary>
    public void Resize(int w, int h)
    {
        GetConsoleScreenBufferInfo(output, out var info);
        var window = new SMALL_RECT { Left = 0, Top = 0, Right = (short)(w - 1), Bottom = (short)(h - 1) };
        var size = new COORD(w, h);
        var shrinking = w <= info.dwSize.X && h <= info.dwSize.Y;
        if (shrinking)
        {
            if (!SetConsoleWindowInfo(output, true, ref window))
                throw new OwlosuiException($"SetConsoleWindowInfo failed: {Marshal.GetLastWin32Error()}");
            if (!SetConsoleScreenBufferSize(output, size))
                throw new OwlosuiException($"SetConsoleScreenBufferSize failed: {Marshal.GetLastWin32Error()}");
        }
        else
        {
            // Growing in one direction and shrinking in the other needs the
            // window brought inside first, then the buffer, then the window.
            var inside = new SMALL_RECT
            {
                Left = 0, Top = 0,
                Right = (short)(Math.Min(w, info.dwSize.X) - 1),
                Bottom = (short)(Math.Min(h, info.dwSize.Y) - 1),
            };
            SetConsoleWindowInfo(output, true, ref inside);
            if (!SetConsoleScreenBufferSize(output, size))
                throw new OwlosuiException($"SetConsoleScreenBufferSize failed: {Marshal.GetLastWin32Error()}");
            if (!SetConsoleWindowInfo(output, true, ref window))
                throw new OwlosuiException($"SetConsoleWindowInfo failed: {Marshal.GetLastWin32Error()}");
        }
    }

    // ---------------------------------------------------------------- input

    private int WindowTop()
    {
        GetConsoleScreenBufferInfo(output, out var info);
        return info.srWindow.Top;
    }

    private void Put(params INPUT_RECORD[] records)
    {
        if (!WriteConsoleInputW(input, records, (uint)records.Length, out var n) || n != records.Length)
            throw new OwlosuiException($"WriteConsoleInput failed: {Marshal.GetLastWin32Error()}");
    }

    private INPUT_RECORD MouseRecord(int x, int y, uint buttons, uint flags) => new()
    {
        EventType = MOUSE_EVENT,
        MouseEvent = new MOUSE_EVENT_RECORD
        {
            dwMousePosition = new COORD(x, y + WindowTop()),
            dwButtonState = buttons,
            dwControlKeyState = 0,
            dwEventFlags = flags,
        },
    };

    /// <summary>Press and release the left button at a cell of the visible window.</summary>
    public void Click(int x, int y)
    {
        Put(MouseRecord(x, y, FROM_LEFT_1ST_BUTTON_PRESSED, 0));
        Put(MouseRecord(x, y, 0, 0));
    }

    /// <summary>Press at one cell, move to another with the button held, release there.</summary>
    public void Drag(int fromX, int fromY, int toX, int toY)
    {
        Put(MouseRecord(fromX, fromY, FROM_LEFT_1ST_BUTTON_PRESSED, 0));
        Put(MouseRecord(toX, toY, FROM_LEFT_1ST_BUTTON_PRESSED, MOUSE_MOVED));
        Put(MouseRecord(toX, toY, 0, 0));
    }

    private static INPUT_RECORD KeyRecord(bool down, ushort vk, char ch, uint control) => new()
    {
        EventType = KEY_EVENT,
        KeyEvent = new KEY_EVENT_RECORD
        {
            bKeyDown = down ? 1 : 0,
            wRepeatCount = 1,
            wVirtualKeyCode = vk,
            wVirtualScanCode = 0,
            UnicodeChar = ch,
            dwControlKeyState = control,
        },
    };

    /// <summary>A key by virtual-key code, with modifiers. The character is what the console would report for it.</summary>
    public void Key(ConsoleKey key, char ch = '\0', bool shift = false, bool alt = false, bool ctrl = false)
    {
        uint control = 0;
        if (shift) control |= SHIFT_PRESSED;
        if (alt) control |= LEFT_ALT_PRESSED;
        if (ctrl) control |= LEFT_CTRL_PRESSED;
        var vk = (ushort)key;
        Put(KeyRecord(true, vk, ch, control), KeyRecord(false, vk, ch, control));
    }

    /// <summary>Type text, a character at a time, as the console would deliver it.</summary>
    public void Type(string text)
    {
        foreach (var ch in text)
        {
            var upper = char.ToUpperInvariant(ch);
            var vk = (ushort)(upper >= 'A' && upper <= 'Z' || upper >= '0' && upper <= '9' ? upper : 0);
            Put(KeyRecord(true, vk, ch, char.IsUpper(ch) ? SHIFT_PRESSED : 0),
                KeyRecord(false, vk, ch, char.IsUpper(ch) ? SHIFT_PRESSED : 0));
        }
    }

    /// <summary>The program's trace, for a failing test to print. Empty if it wrote none.</summary>
    public string Trace()
    {
        try { return File.Exists(TracePath) ? File.ReadAllText(TracePath) : ""; }
        catch (IOException) { return ""; }
    }

    public void Dispose()
    {
        if (input != INVALID_HANDLE_VALUE) CloseHandle(input);
        if (output != INVALID_HANDLE_VALUE) CloseHandle(output);
        if (attached) FreeConsole();
        try
        {
            if (!process.HasExited)
            {
                process.Kill();
                process.WaitForExit(2000);
            }
        }
        catch (InvalidOperationException) { }
        process.Dispose();
        try { File.Delete(TracePath); } catch (IOException) { }
    }
}
