// The desktop in a native window (Owlosui.OpenWindow), driven the way the
// console cases drive a console: by messages to the program's own window,
// never by keys typed on the machine's keyboard. The window opens without
// taking the focus, so a person working at the same desktop keeps typing
// where they were.
//
// It also leaves a picture of the window in the temp folder,
// owlosui-window.bmp, for a person who wants to see what was tested.

using System.Runtime.InteropServices;
using HW = HelloWorld.App;

internal static class WindowCases
{
    public static void Run()
    {
        if (!OperatingSystem.IsWindows())
        {
            Console.WriteLine("skip  window cases: the window is a Windows one");
            return;
        }

        Tests.RunCase("Window: the desktop opens in a window, the pipe still answers, Enter there reaches Run", () =>
        {
            var title = "OWLOSUI test " + Guid.NewGuid().ToString("N")[..8];
            using var owl = new Owlosui(width: 60, height: 20);
            HW.Build(owl);
            owl.OpenWindow(title, activate: false);
            var hwnd = WaitForWindow(title);
            Tests.Require(hwnd != 0, $"no window titled \"{title}\" appeared");

            // Requests are served on the window's thread now; a frame still comes back.
            var f = owl.GetFrame();
            Tests.Require(f.Find("Hello, world!") != null, "the greeting is not in the window's frame", f);

            // The window made wider: the core takes the new size, and the
            // next answer to Run carries it.
            SetWindowPos(hwnd, 0, 0, 0, 1100, 600, SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE);
            Thread.Sleep(300);
            Capture(hwnd, Path.Combine(Path.GetTempPath(), "owlosui-window.bmp"));

            // Enter, posted before Run is waiting: it is queued, and the
            // first WAIT gets it. OK is the default button, OK ends Run.
            PostMessageW(hwnd, WM_KEYDOWN, VK_RETURN, 0);
            var run = Task.Run(() => owl.Run(HW.OnCommand));
            Tests.Require(run.Wait(5000), "Enter in the window never reached Run");
            Tests.Require(owl.Width > 60, $"the desktop is still {owl.Width} wide after the window grew");
        });
    }

    private static nint WaitForWindow(string title)
    {
        for (var i = 0; i < 60; i++)
        {
            var h = FindWindowW("OWLOSUI", title);
            if (h != 0) return h;
            Thread.Sleep(50);
        }
        return 0;
    }

    /// <summary>The window's client area as a 32-bit BMP, whatever lies on top of it.</summary>
    private static void Capture(nint hwnd, string path)
    {
        GetClientRect(hwnd, out var r);
        int w = r.Right, h = r.Bottom;
        var dc = GetDC(hwnd);
        var mem = CreateCompatibleDC(dc);
        var bmp = CreateCompatibleBitmap(dc, w, h);
        var old = SelectObject(mem, bmp);
        // PW_CLIENTONLY | PW_RENDERFULLCONTENT: the compositor's copy, so a
        // window behind another still gives its own picture.
        PrintWindow(hwnd, mem, 3);
        SelectObject(mem, old);
        var info = new BITMAPINFOHEADER { Size = 40, Width = w, Height = -h, Planes = 1, BitCount = 32 };
        var px = new byte[w * h * 4];
        GetDIBits(mem, bmp, 0, (uint)h, px, ref info, 0);
        DeleteObject(bmp);
        DeleteDC(mem);
        ReleaseDC(hwnd, dc);

        using var s = new BinaryWriter(File.Create(path));
        s.Write((byte)'B'); s.Write((byte)'M');
        s.Write(14 + 40 + px.Length); s.Write(0); s.Write(14 + 40);
        s.Write(40); s.Write(w); s.Write(-h); s.Write((short)1); s.Write((short)32);
        s.Write(0); s.Write(px.Length); s.Write(2835); s.Write(2835); s.Write(0); s.Write(0);
        s.Write(px);
    }

    private const uint WM_KEYDOWN = 0x0100;
    private const nint VK_RETURN = 0x0D;
    private const uint SWP_NOMOVE = 0x0002, SWP_NOZORDER = 0x0004, SWP_NOACTIVATE = 0x0010;

    [StructLayout(LayoutKind.Sequential)]
    private struct RECT { public int Left, Top, Right, Bottom; }

    [StructLayout(LayoutKind.Sequential)]
    private struct BITMAPINFOHEADER
    {
        public int Size, Width, Height;
        public short Planes, BitCount;
        public int Compression, SizeImage, XPelsPerMeter, YPelsPerMeter, ClrUsed, ClrImportant;
    }

    [DllImport("user32.dll", CharSet = CharSet.Unicode)] private static extern nint FindWindowW(string cls, string title);
    [DllImport("user32.dll")] private static extern bool PostMessageW(nint h, uint msg, nint w, nint l);
    [DllImport("user32.dll")] private static extern bool SetWindowPos(nint h, nint after, int x, int y, int cx, int cy, uint flags);
    [DllImport("user32.dll")] private static extern bool GetClientRect(nint h, out RECT r);
    [DllImport("user32.dll")] private static extern nint GetDC(nint h);
    [DllImport("user32.dll")] private static extern int ReleaseDC(nint h, nint dc);
    [DllImport("user32.dll")] private static extern bool PrintWindow(nint h, nint dc, uint flags);
    [DllImport("gdi32.dll")] private static extern nint CreateCompatibleDC(nint dc);
    [DllImport("gdi32.dll")] private static extern nint CreateCompatibleBitmap(nint dc, int w, int h);
    [DllImport("gdi32.dll")] private static extern nint SelectObject(nint dc, nint o);
    [DllImport("gdi32.dll")] private static extern bool DeleteObject(nint o);
    [DllImport("gdi32.dll")] private static extern bool DeleteDC(nint dc);
    [DllImport("gdi32.dll")] private static extern int GetDIBits(nint dc, nint bmp, uint start, uint lines, byte[] bits, ref BITMAPINFOHEADER info, uint usage);
}
