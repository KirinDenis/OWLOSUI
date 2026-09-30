// Desktop, C# step 6 of 6 - step 5's demo in a native window instead of the console.
// Before: 05-OwlosDemo. The same window from Rust is Examples/Desktop/Rust/02-Window.
//
// One line differs from 05-OwlosDemo's Main: OpenWindow. After it the
// server shows the desktop in a window of its own - CreateWindow and GDI,
// every cell drawn by hand, lib/window - and reads the keys and the mouse
// there. This program draws nothing and knows nothing about Win32: the
// App below is step 5's class, unchanged, and Run is the same Run.
//
// The project is a WinExe, so no console opens with it. Drag the window's
// edge and the desktop grows with it; the close box asks the program to
// exit, as Alt+X does.

using OwlosDemo;

namespace DemoWindow;

public static class Program
{
    public static void Main()
    {
        // A window has no console to take its size from: say it.
        using var owl = new Owlosui(width: 100, height: 32);
        owl.OpenWindow("OWLOS UI Demo");
        var app = new App(owl);
        owl.Run(app.OnCommand, app.Poll);
    }
}
