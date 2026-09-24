// The part of Windows that still remembers text mode.
//
// Shared by the client (which reads the console it is running in) and the
// agent (which drives a console some other program is running in). One
// copy of every structure, so the two cannot disagree about a field.

using System.Runtime.InteropServices;

internal static class Win32
{
    public const int STD_INPUT_HANDLE = -10;
    public const int STD_OUTPUT_HANDLE = -11;
    public const uint ENABLE_VIRTUAL_TERMINAL_PROCESSING = 0x0004;

    public const uint ENABLE_PROCESSED_INPUT = 0x0001;
    public const uint ENABLE_LINE_INPUT = 0x0002;
    public const uint ENABLE_ECHO_INPUT = 0x0004;
    public const uint ENABLE_WINDOW_INPUT = 0x0008;
    public const uint ENABLE_MOUSE_INPUT = 0x0010;
    public const uint ENABLE_QUICK_EDIT_MODE = 0x0040;
    public const uint ENABLE_EXTENDED_FLAGS = 0x0080;
    public const uint ENABLE_VIRTUAL_TERMINAL_INPUT = 0x0200;

    public const ushort KEY_EVENT = 0x0001;
    public const ushort MOUSE_EVENT = 0x0002;
    public const ushort WINDOW_BUFFER_SIZE_EVENT = 0x0004;

    public const uint MOUSE_MOVED = 0x0001;
    public const uint MOUSE_WHEELED = 0x0004;
    public const uint MOUSE_HWHEELED = 0x0008;

    public const uint FROM_LEFT_1ST_BUTTON_PRESSED = 0x0001;

    public const uint RIGHT_ALT_PRESSED = 0x0001;
    public const uint LEFT_ALT_PRESSED = 0x0002;
    public const uint RIGHT_CTRL_PRESSED = 0x0004;
    public const uint LEFT_CTRL_PRESSED = 0x0008;
    public const uint SHIFT_PRESSED = 0x0010;

    public const uint GENERIC_READ = 0x80000000;
    public const uint GENERIC_WRITE = 0x40000000;
    public const uint FILE_SHARE_READ = 0x1;
    public const uint FILE_SHARE_WRITE = 0x2;
    public const uint OPEN_EXISTING = 3;

    public const uint CREATE_NEW_CONSOLE = 0x00000010;
    public const uint STARTF_USESHOWWINDOW = 0x00000001;
    public const ushort SW_HIDE = 0;

    [StructLayout(LayoutKind.Sequential)]
    public struct COORD
    {
        public short X;
        public short Y;
        public COORD(int x, int y) { X = (short)x; Y = (short)y; }
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct SMALL_RECT
    {
        public short Left, Top, Right, Bottom;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct KEY_EVENT_RECORD
    {
        public int bKeyDown;
        public ushort wRepeatCount;
        public ushort wVirtualKeyCode;
        public ushort wVirtualScanCode;
        public ushort UnicodeChar;
        public uint dwControlKeyState;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct MOUSE_EVENT_RECORD
    {
        public COORD dwMousePosition;
        public uint dwButtonState;
        public uint dwControlKeyState;
        public uint dwEventFlags;
    }

    // A tagged union: two bytes of type, two of padding, sixteen of
    // whichever record it is. Twenty bytes, as the header says.
    [StructLayout(LayoutKind.Explicit, Size = 20)]
    public struct INPUT_RECORD
    {
        [FieldOffset(0)] public ushort EventType;
        [FieldOffset(4)] public KEY_EVENT_RECORD KeyEvent;
        [FieldOffset(4)] public MOUSE_EVENT_RECORD MouseEvent;
        [FieldOffset(4)] public COORD WindowBufferSize;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct CHAR_INFO
    {
        public ushort UnicodeChar;
        public ushort Attributes;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct CONSOLE_SCREEN_BUFFER_INFO
    {
        public COORD dwSize;
        public COORD dwCursorPosition;
        public ushort wAttributes;
        public SMALL_RECT srWindow;
        public COORD dwMaximumWindowSize;
    }

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    public struct STARTUPINFOW
    {
        public int cb;
        public IntPtr lpReserved;
        public IntPtr lpDesktop;
        public IntPtr lpTitle;
        public uint dwX, dwY, dwXSize, dwYSize, dwXCountChars, dwYCountChars, dwFillAttribute;
        public uint dwFlags;
        public ushort wShowWindow;
        public ushort cbReserved2;
        public IntPtr lpReserved2;
        public IntPtr hStdInput, hStdOutput, hStdError;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct PROCESS_INFORMATION
    {
        public IntPtr hProcess;
        public IntPtr hThread;
        public uint dwProcessId;
        public uint dwThreadId;
    }

    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern IntPtr GetStdHandle(int nStdHandle);

    /// <summary>The OEM code page of the system locale - what a DOS window on this machine runs in.</summary>
    [DllImport("kernel32.dll")]
    public static extern uint GetOEMCP();

    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern bool GetConsoleMode(IntPtr hConsoleHandle, out uint lpMode);

    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern bool SetConsoleMode(IntPtr hConsoleHandle, uint dwMode);

    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern bool ReadConsoleInputW(IntPtr hConsoleInput, [Out] INPUT_RECORD[] lpBuffer,
                                                uint nLength, out uint lpNumberOfEventsRead);

    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern bool WriteConsoleInputW(IntPtr hConsoleInput, INPUT_RECORD[] lpBuffer,
                                                 uint nLength, out uint lpNumberOfEventsWritten);

    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern bool ReadConsoleOutputW(IntPtr hConsoleOutput, [Out] CHAR_INFO[] lpBuffer,
                                                 COORD dwBufferSize, COORD dwBufferCoord, ref SMALL_RECT lpReadRegion);

    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern bool GetConsoleScreenBufferInfo(IntPtr hConsoleOutput, out CONSOLE_SCREEN_BUFFER_INFO info);

    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern bool SetConsoleScreenBufferSize(IntPtr hConsoleOutput, COORD dwSize);

    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern bool SetConsoleWindowInfo(IntPtr hConsoleOutput, bool bAbsolute, ref SMALL_RECT lpConsoleWindow);

    [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    public static extern IntPtr CreateFileW(string lpFileName, uint dwDesiredAccess, uint dwShareMode,
                                            IntPtr lpSecurityAttributes, uint dwCreationDisposition,
                                            uint dwFlagsAndAttributes, IntPtr hTemplateFile);

    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern bool CloseHandle(IntPtr hObject);

    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern bool AttachConsole(uint dwProcessId);

    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern bool FreeConsole();

    [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    public static extern bool CreateProcessW(string? lpApplicationName, System.Text.StringBuilder lpCommandLine,
                                             IntPtr lpProcessAttributes, IntPtr lpThreadAttributes,
                                             bool bInheritHandles, uint dwCreationFlags, IntPtr lpEnvironment,
                                             string? lpCurrentDirectory, ref STARTUPINFOW lpStartupInfo,
                                             out PROCESS_INFORMATION lpProcessInformation);

    public static readonly IntPtr INVALID_HANDLE_VALUE = new(-1);
}
