/* ==========================================================================
   OWLOSUI.H - the OWLOSUI toolkit, for a DOS program in C.

   The toolkit is not in here. It is OWLOSRES, the resident, which runs a
   program with the toolkit behind INT 60h; OWLOSUI.C is how a C program
   talks to it. A request is built the way it goes over the wire - the
   bytes of lib/PROTOCOL.md, the same ones the C# client sends down a
   pipe - and a reply comes back:

       win = owl_window(0, -1, -1, 40, 9, OWL_DIALOG, "Hello", 0);
       owl_static(win, 2, 1, 34, 1, "Hello, world!");
       owl_buttons(win, 1);            a request with parts: begin it,
       owl_button(CM_OK, OWL_DEFAULT, "~O~K");   add the parts,
       owl_end();                      and send it

   and a program's main loop is

       for (;;) { owl_wait(&pressed, &command); ... }

   Written for Open Watcom's 16-bit DOS compiler, small model:
       wcl -ms -bt=dos DEMO.C OWLOSUI.C
   Text is sent as it is: keep it ASCII, or UTF-8.
   ========================================================================== */

#ifndef OWLOSUI_H
#define OWLOSUI_H

typedef unsigned short owl_id;          /* a view; 0 is the desktop */

/* ---- window flags ---- */
#define OWL_MODAL     0x01
#define OWL_FIXED     0x02              /* not resizable */
#define OWL_NO_ZOOM   0x04
#define OWL_NO_CLOSE  0x08
#define OWL_BLUE      0x00              /* a document */
#define OWL_CYAN      0x10              /* help */
#define OWL_GREY      0x20              /* a dialog */
#define OWL_DIALOG    (OWL_GREY | OWL_FIXED | OWL_NO_ZOOM)

/* ---- a button's flags ---- */
#define OWL_DEFAULT   0x01              /* the one Enter presses */
#define OWL_ACCENT    0x02              /* cyan */
#define OWL_DANGER    0x04              /* red */
#define OWL_DISABLED  0x08
#define OWL_CANCEL    0x10              /* the one Escape presses */

/* ---- a key, for the status line: kind, value, mods ---- */
#define OWL_KEY_CHAR  0                 /* value: the character */
#define OWL_KEY_F     1                 /* value: 1 for F1 */
#define OWL_KEY_NAMED 2                 /* 0 Enter 1 Esc 2 Tab ... 7 Home 9 PgUp 10 PgDn 11 Up 12 Down 13 Left 14 Right */
#define OWL_KEY_NONE  0xFF
#define OWL_SHIFT     1
#define OWL_CTRL      2
#define OWL_ALT       4

/* ---- the session ---- */
int    owl_check(void);                 /* started by OWLOSRES? says how, if not */
void   owl_init(void);                  /* the desktop: 80 by 25 */
void   owl_quit(void);
void   owl_wait(unsigned *pressed, unsigned *command);

/* ---- views ---- */
owl_id owl_window(owl_id parent, int x, int y, int w, int h,
                  unsigned char flags, const char *title, unsigned close_cmd);
owl_id owl_static(owl_id parent, int x, int y, int w, int h, const char *text);
owl_id owl_text(owl_id parent, const char *text);       /* an editor filling it */
owl_id owl_canvas(owl_id parent, int x, int y, int w, int h);

/* ---- requests with parts: begin, add, owl_end() sends and gives the id ---- */
void   owl_buttons(owl_id parent, int n);                     /* docked bottom right */
void   owl_button_row(owl_id parent, int x, int y, int n);    /* placed, not a Tab stop */
void   owl_message_box(const char *title, const char *text, int n);
void   owl_button(unsigned cmd, unsigned char flags, const char *label);

void   owl_status(int n);
void   owl_window_status(owl_id window, int n);     /* keys the window carries */
void   owl_key(const char *label, unsigned cmd,
               unsigned char kind, unsigned value, unsigned char mods);

void   owl_menu_bar(int menus);
void   owl_menu(const char *text, int items);
void   owl_item(const char *text, const char *shortcut, const char *hint, unsigned cmd);
void   owl_line(void);

void   owl_blit(owl_id canvas, int x, int y, int w, int h);
void   owl_cell(unsigned ch, unsigned char attr);   /* ch: a Unicode character */

owl_id owl_end(void);

/* ---- changing things ---- */
void   owl_set_text(owl_id id, const char *text);
void   owl_close(owl_id id);
void   owl_activate(owl_id id);
owl_id owl_active(void);
void   owl_zoom(owl_id id);
void   owl_cycle(void);
void   owl_cycle_back(void);
void   owl_cascade(void);
void   owl_tile(void);
void   owl_window_list(void);
int    owl_click(owl_id canvas, int *x, int *y);    /* 1 if the canvas was clicked */
void   owl_glyphs(unsigned table[256]);             /* each glyph as Unicode */

/* ---- files: DOS reads the folder (FindFirst, FindNext), the panel shows it ---- */
owl_id owl_files(owl_id parent, int top, const char *dir);  /* a panel filling the window, top rows free */
int    owl_set_files(owl_id panel, const char *dir);        /* 0 if DOS cannot read that folder */
void   owl_files_error(owl_id panel, const char *text);     /* a line in the panel saying what went wrong */
int    owl_take_files(owl_id panel, char *text, int room);  /* 0 nothing, 1 a name entered, 2 a path typed */
int    owl_current_name(owl_id panel, char *name, int room);/* the name under the cursor; 0 if none */
owl_id owl_text_dos(owl_id parent, const char *bytes, unsigned n, unsigned char flags);
                                        /* an editor holding bytes in DOS's code page; flags 1 = read-only */

/* ---- the wire itself, for what the above does not cover ---- */
void   owl_begin(unsigned char op);
void   owl_u8(unsigned char v);
void   owl_u16(unsigned v);
void   owl_str(const char *s);
int    owl_send(void);                  /* the status: 0 OK */
unsigned char *owl_reply(void);         /* the reply's body */

#endif
