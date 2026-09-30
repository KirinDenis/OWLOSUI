{ ==========================================================================
  HELLO.PAS - a window, words, a button: the whole shape of a program.

      RUN HELLO

  Build the windows, then wait for the person until the answer is OK.
  Every window, frame and shadow is drawn by the OWLOSUI core; this program
  only says what it wants, through INT 60h (lib\dos\pascal\OWLOSUI.PAS).
  ========================================================================== }

program Hello;

uses OwlosUI;

const
  CmOk = 1;                             { the program's own command number }

var
  Win, Pressed, Command: Word;

begin
  if not OwlCheck then Halt(1);         { started by OWLOSRES? }
  OwlInit;                              { the desktop: 80 by 25, as DOS's screen is }

  Win := OwlWindow(0, -1, -1, 42, 9, OwlDialog, 'Hello', 0);   { centred }
  OwlStatic(Win, 2, 1, 36, 1, 'Hello, world!');
  OwlStatic(Win, 2, 3, 36, 2, 'This window is drawn by a Rust core and shown by Pascal.');
  OwlButtons(Win, 1);                   { one button, docked bottom right }
  OwlButton(CmOk, OwlDefault, '~O~K');
  OwlEnd;

  repeat
    OwlWait(Pressed, Command);          { a button, or a command }
  until (Pressed = CmOk) or (Command = CmOk);

  OwlQuit;
end.
