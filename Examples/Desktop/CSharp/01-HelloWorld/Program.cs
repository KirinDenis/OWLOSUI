// Desktop, C# step 1 of 6 - a window, words, a button: the whole shape of a program.
// Next: 02-Notes, an editor that saves and asks before you leave.
//
// The smallest OWLOSUI program there is.
//
// A window in the middle of the screen, a line of words, one button. The
// window can be dragged by its title and closed with the button, Escape, or
// a mouse click on OK.
//
// What is worth noticing in forty lines:
//
//  * Nothing here draws. `Owlosui` starts `owlosui-serve`, a Rust process,
//    and the core inside it decides what every cell on the screen looks
//    like - colours, frames, shadows, where the button sits. This program
//    only says what it wants and waits to hear which button was pressed.
//    Take the same calls to a browser or to DOS and the picture is the same.
//
//  * There are no callbacks. A button does not "have an OnClick"; it has a
//    number, and `Run` hands that number back when the button is pressed.
//    This is Turbo Vision's design and it is what lets the same core sit
//    behind a pipe today and behind a DOS interrupt later - a number crosses
//    both, a closure crosses neither.
//
//  * Handles are numbers too. `Window` returns a `ushort` and every other
//    call takes it. Nothing is an object on this side of the wire.
//
// The scene is a method rather than a script so that `Examples/Desktop/CSharp/Tests`
// can build exactly this window, headless, and press its button by the wire.
//
// Build the server once, from the repository root:
//
//     cargo build -p owlosui-serve
//
// then, from this folder:
//
//     dotnet run

namespace HelloWorld;

public static class App
{
    // Command numbers are the program's own. Any ushort but 0, which means
    // "nothing happened" and cannot be a button.
    public const ushort CmOk = 1;

    /// <summary>Everything on the screen. Called once, before <see cref="Owlosui.Run"/>.</summary>
    public static void Build(Owlosui owl)
    {
        // 40 cells wide, 9 tall, and centred - (-1, -1) is the default
        // position and it means "in the middle", now and after the console
        // is resized. Style.Dialog is grey, fixed in size and closable: the
        // look Turbo Vision gave to something that asks a question.
        var w = owl.Window("Hello", 40, 9, style: Owlosui.Style.Dialog);

        // Static text is placed by hand, in cells, relative to the inside of
        // the frame: column 2, row 1. Dialog contents were always laid out
        // this way, and for a dialog whose parts are known it is the honest
        // tool; docking is for things that must follow a window's size.
        owl.Static(w, 2, 1, "Hello, world!");

        // Longer text wraps at the width given, on word boundaries, into the
        // number of rows given. 34 wide leaves a margin on each side of the
        // 40-cell window; 2 rows is what the sentence needs.
        owl.Static(w, 2, 3, "This window is drawn by a Rust core and shown by C#.", w: 34, h: 2);

        // Buttons go bottom-right, as a row - the toolkit places them; you
        // do not say where. The tildes mark the hotkey: Alt+O presses it
        // from anywhere. The first button is the default, so Enter presses
        // it; Escape presses the last, and with one button that is the same
        // one. So Enter, Escape, Alt+O and a mouse click all mean OK here.
        owl.Buttons(w, ("~O~K", CmOk));
    }

    /// <summary>
    /// What to do with a command. Returns true to keep running, false to
    /// stop. `Run` calls this with every button pressed or menu item chosen;
    /// there is exactly one command here and it means "we are done".
    /// </summary>
    public static bool OnCommand(ushort cmd) => cmd != CmOk;

    public static void Main()
    {
        // `using`: when this goes out of scope the server is told to quit and
        // the console is put back the way it was found - colours, cursor,
        // whatever was on the screen.
        using var owl = new Owlosui();
        Build(owl);

        // The whole event loop: draw, wait for a key or the mouse, send it
        // over, ask what it caused, call back with the answer. Repeat until
        // OnCommand says stop.
        owl.Run(OnCommand);
    }
}
