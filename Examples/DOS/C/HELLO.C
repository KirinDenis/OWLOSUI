/* ==========================================================================
   HELLO.C - a window, words, a button: the whole shape of a program.

       RUN HELLO

   Build the windows, then wait for the person until the answer is OK.
   Every window, frame and shadow is drawn by the OWLOSUI core; this
   program only says what it wants, through INT 60h (lib\dos\c\OWLOSUI.C).
   ========================================================================== */

#include "OWLOSUI.H"

#define CM_OK 1                         /* the program's own command number */

int main(void)
{
    owl_id win;
    unsigned pressed, command;

    if (!owl_check()) return 1;         /* started by OWLOSRES? */
    owl_init();                         /* the desktop: 80 by 25, as DOS's screen is */

    win = owl_window(0, -1, -1, 42, 9, OWL_DIALOG, "Hello", 0);   /* centred */
    owl_static(win, 2, 1, 36, 1, "Hello, world!");
    owl_static(win, 2, 3, 36, 2, "This window is drawn by a Rust core and shown by C.");
    owl_buttons(win, 1);                /* one button, docked bottom right */
    owl_button(CM_OK, OWL_DEFAULT, "~O~K");
    owl_end();

    do {
        owl_wait(&pressed, &command);   /* a button, or a command */
    } while (pressed != CM_OK && command != CM_OK);

    owl_quit();
    return 0;
}
