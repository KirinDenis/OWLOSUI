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
#define OWL_BLACK     0x30              /* a terminal: a console's window, frame and all */
#define OWL_NO_SHADOW 0x40              /* windows that tile the screen cast none */
#define OWL_NO_MINIMIZE 0x80            /* no minimize box; a modal window has none anyway */
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

/* ---- what an editor offers (owl_editor): the core then runs each by itself ---- */
#define OWL_OFFER_EDIT     0x01         /* Undo, Redo, Cut, Copy, Paste, Select all */
#define OWL_OFFER_FIND     0x02         /* Find... Ctrl+F, Find next Ctrl+L */
#define OWL_OFFER_REPLACE  0x04         /* Replace... Ctrl+H */
#define OWL_OFFER_WRAP     0x08         /* Word wrap */
#define OWL_OFFER_READONLY 0x10         /* Read only */
#define OWL_OFFER_HEX      0x20         /* the same text as bytes */
#define OWL_OFFER_KEYS     0x40         /* Classic keys: the WordStar arrangement */
#define OWL_OFFER_SYNTAX   0x80         /* Syntax: the language the text is coloured as */
#define OWL_OFFER_ALL      0xFF
/* ---- and how it is set ---- */
#define OWL_EDIT_WRAP      0x01
#define OWL_EDIT_READONLY  0x02
#define OWL_EDIT_CLASSIC   0x04
#define OWL_EDIT_HEX       0x08
#define OWL_EDIT_SYNTAX    0x10         /* read back only: set a language with owl_syntax */

/* ---- a file panel's flags (owl_files_ex) ---- */
#define OWL_FILES_MARKS        0x01     /* Insert marks names */
#define OWL_FILES_NO_PATH_WORD 0x02     /* the path line without its "Path:" */
#define OWL_FILES_NO_PATH_LINE 0x04     /* no path line: a commander's panel */
#define OWL_FILES_DETAILS      0x08     /* a one-row foot: size, date, attributes of the entry
                                           under the cursor - no path, no name */

/* ---- a tree node's flags (owl_node) ---- */
#define OWL_NODE_OPEN 0x01              /* shown open */
#define OWL_NODE_LAZY 0x02              /* has children, asked for when opened */

/* ---- the session ---- */
int    owl_check(void);                 /* started by OWLOSRES? says how, if not */
void   owl_init(void);                  /* the desktop: 80 by 25 */
void   owl_quit(void);
void   owl_wait(unsigned *pressed, unsigned *command);
void   owl_draw(void);                  /* the screen drawn now, between waits: a progress bar */
void   owl_suspend(void);               /* the windows put aside, the screen DOS's: before another program */
void   owl_resume(void);                /* the windows back, drawn: after it */
int    owl_run(const char *path, const char *args, int pause);
                                        /* another program - .COM, .EXE, or through COMMAND.COM a
                                           .BAT - with the windows put aside while it runs; DOS's
                                           error, 0 if it ran. pause: a key to come back. */

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
void   owl_editor(owl_id text, unsigned char offers, unsigned char state);
                                        /* OWL_OFFER_ bits; OWL_EDIT_ bits */
unsigned char owl_editor_state(owl_id text);  /* OWL_EDIT_ bits as the person left them */
int    owl_syntax(owl_id text, const char *language);
                                        /* colour as C, PAS, DEMO.ASM ...; 0 if no such language */
void   owl_close(owl_id id);
void   owl_activate(owl_id id);
owl_id owl_active(void);
void   owl_zoom(owl_id id);
void   owl_minimize(owl_id id);         /* into a bar in the bottom right corner; owl_activate brings it back */
void   owl_window_tag(owl_id window, const char *tag);
                                        /* a word in brackets after the title, as [modal]; "" for none */
void   owl_window_indicator(owl_id window, const char *text);
                                        /* words at the right of the status line while it is in
                                           front; a part between tildes lit green: "[~A:~ B:]" */
int    owl_place(owl_id window, int *x, int *y, int *w, int *h);
                                        /* its inside, in cells, as last drawn; 1 if anything is
                                           over it - a window, a shadow, an open menu */
void   owl_cycle(void);
void   owl_cycle_back(void);
void   owl_cascade(void);
void   owl_tile(void);
void   owl_window_list(void);
int    owl_click(owl_id canvas, int *x, int *y);    /* 1 if the canvas was clicked */
void   owl_glyphs(unsigned table[256]);             /* each glyph as Unicode */

/* ---- a console: text that keeps arriving, coloured by its ANSI sequences
   ("\33[31m" red, "\33[0m" back) - a log. It fills its window, keeps
   scrollback lines (0: a thousand), follows the newest until scrolled
   back; the mouse selects, Ctrl+C copies. ---- */
owl_id owl_console(owl_id parent, unsigned scrollback);
void   owl_console_write(owl_id console, const char *text);

/* ---- files: DOS reads the folder (FindFirst, FindNext), the panel shows it ---- */
owl_id owl_files(owl_id parent, int top, const char *dir);  /* a panel filling the window, top rows free */
int    owl_set_files(owl_id panel, const char *dir);        /* 0 if DOS cannot read that folder */
void   owl_files_error(owl_id panel, const char *text);     /* a line in the panel saying what went wrong */
int    owl_take_files(owl_id panel, char *text, int room);  /* 0 nothing, 1 a name entered, 2 a path typed */
int    owl_current_name(owl_id panel, char *name, int room);/* the name under the cursor; 0 if none */
owl_id owl_text_dos(owl_id parent, const char *bytes, unsigned n, unsigned char flags);
                                        /* an editor holding bytes in DOS's code page; flags 1 = read-only */
owl_id owl_text_dos_ex(owl_id parent, const char *bytes, unsigned n, unsigned char flags, unsigned *taken);
                                        /* the same, and taken: how many bytes fitted in one request */
unsigned owl_dos_fits(const char *bytes, unsigned n);
                                        /* how many of n bytes owl_text_dos would take - asked first,
                                           so a title can say "the first 9000 bytes" */
unsigned owl_get_text_dos(owl_id id, char *bytes, unsigned room);
                                        /* an editor's text back as DOS bytes, CR LF lines;
                                           0xFFFF if it does not fit in room */
owl_id owl_files_ex(owl_id parent, int top, unsigned char flags, const char *dir);
                                        /* a panel with OWL_FILES_ flags: marks, no path line */
unsigned owl_marked(owl_id panel);      /* how many names are marked - or 1, the one under the
                                           cursor, when none are; then owl_marked_name */
int    owl_marked_name(unsigned i, char *name, int room);   /* name i (from 1) of the last owl_marked */
void   owl_unmark(owl_id panel);        /* every mark off: the marked files were dealt with */

/* ---- controls ---- */
owl_id owl_input(owl_id parent, int x, int y, int w, unsigned max, const char *label, const char *text);
int    owl_get_text(owl_id id, char *text, int room);   /* an input's text, or a short editor's; its length */
void   owl_list(owl_id parent, int x, int y, int w, int h, unsigned n);  /* then n x owl_list_item, owl_end */
void   owl_list_item(const char *text);
unsigned owl_current(owl_id list);      /* the row under the cursor, from 0 */
owl_id owl_progress(owl_id parent, int x, int y, int w, unsigned long max);
void   owl_set_progress(owl_id bar, unsigned long value);

/* ---- a tree: owl_tree, then its nodes with owl_node, each followed by its
   own children, then owl_end ---- */
typedef struct {                        /* a place in a tree: how deep, and at each level */
    unsigned char depth;                /* which child and its text - owl_tree_expand fills */
    unsigned index[16];                 /* it with the node somebody opened, owl_tree_path */
    char text[16][13];                  /* with the texts from the root down to the cursor */
} owl_path;
void   owl_tree(owl_id parent, int n);
void   owl_node(unsigned char flags, const char *text, int children);
int    owl_tree_expand(owl_id tree, owl_path *path);    /* 1 if a lazy node was opened */
void   owl_tree_children(owl_id tree, const owl_path *path, int n);
                                        /* then n x owl_node, owl_end: its children */
void   owl_tree_path(owl_id tree, owl_path *path);      /* root to the cursor, texts only */

/* ---- the wire itself, for what the above does not cover ---- */
void   owl_begin(unsigned char op);
void   owl_u8(unsigned char v);
void   owl_u16(unsigned v);
void   owl_str(const char *s);
int    owl_send(void);                  /* the status: 0 OK */
unsigned char *owl_reply(void);         /* the reply's body */

#endif
