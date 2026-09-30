/* ==========================================================================
   DEMO.C - the OWLOS UI demo, in C.

       RUN DEMO

   The shape every text-mode toolkit's demo has had, because it is the right
   shape: a menu bar, a status line, a desktop, and small windows that each
   use one part of the kit.

     File      New: an editor window of its own. Open: the file panel,
               the folder read through DOS, a file shown in a viewer. Exit.
     Tools     Calculator  - a keypad of button rows; typing works too,
                             because the window carries the keys it wants
               Calendar    - a canvas drawn again for every month
               ASCII table - every glyph of the font; a click names one
               Puzzle      - the fifteen puzzle: click a tile, or arrows
     Window    Zoom, Next, Previous, Close, List, Cascade, Tile
     Help      About

   The same demo is in Examples\DOS\Asm and Examples\DOS\Pascal, part for
   part. The toolkit - every frame, menu, button and the editor - is
   OWLOSRES, reached through INT 60h (lib\dos\c\OWLOSUI.C). What is here is
   the program: what to build, and what to do with each command.
   ========================================================================== */

#include <direct.h>
#include <dos.h>
#include <limits.h>
#include <stdio.h>
#include <string.h>
#include "OWLOSUI.H"

/* The program's own command numbers; 0 means none. */
enum {
    CM_NEW = 1, CM_EXIT = 2, CM_OPEN = 3,
    CM_CALC = 10, CM_CALENDAR = 11, CM_ASCII = 12, CM_PUZZLE = 13,
    CM_NEXT = 30, CM_ZOOM = 31, CM_CLOSE = 32, CM_CASCADE = 33, CM_TILE = 34,
    CM_PREVIOUS = 35, CM_LIST = 36,
    CM_ABOUT = 40, CM_HELP = 41, CM_DISMISS = 62,
    CM_FILE_OPEN = 63, CM_OPEN_CANCEL = 64,           /* the Open dialog's buttons */
    /* the calculator's keys */
    CM_DIGIT = 100,                     /* 100..109: 0..9 */
    CM_ADD = 110, CM_SUB = 111, CM_MUL = 112, CM_DIV = 113,
    CM_EQUALS = 114, CM_CLEAR = 115, CM_NEGATE = 116,
    /* the calendar's */
    CM_MONTH_BACK = 160, CM_MONTH_ON = 161, CM_TODAY = 162,
    /* the puzzle's */
    CM_SCRAMBLE = 180, CM_UP = 181, CM_DOWN = 182, CM_LEFT = 183, CM_RIGHT = 184
};

static owl_id box, calc_win, cal_win, ascii_win, puz_win;
static int editors;
static unsigned glyphs[256];            /* each glyph as Unicode */

/* ---- about, and the one message box ------------------------------------ */

static void box_close(void)
{
    if (box) owl_close(box);
    box = 0;
}

static void about(void)
{
    box_close();
    owl_message_box("OWLOS UI Demo",
        "A C program on DOS. Every window in it is drawn by the OWLOSUI core, "
        "a Rust program behind INT 60h - the same core that draws the C# demo "
        "on Windows and the JavaScript one in a browser.", 1);
    owl_button(CM_DISMISS, OWL_DEFAULT, "~O~K");
    box = owl_end();
}

/* ---- File > New: an editor, a window like any other -------------------- */

static void editor_new(void)
{
    char title[20];
    int n = ++editors;
    owl_id w;
    sprintf(title, "UNTITLED%d.TXT", n);
    w = owl_window(0, 2 + (n & 7), 1 + (n & 7), 56, 14, OWL_BLUE, title, CM_CLOSE);
    owl_text(w, "Type here. Shift and the arrows select, Ctrl+Z takes it back.");
}

/* ---- File > Open: the file panel, the folder read through DOS ---------- */

static owl_id open_dialog, open_panel;
static char open_dir[80];               /* the folder the panel shows */
static char file_buf[8192];             /* a file's first 8 KB, for the viewer */

static void open_show(void)
{
    if (open_dialog) { owl_activate(open_dialog); return; }
    if (!open_dir[0]) getcwd(open_dir, sizeof open_dir);
    open_dialog = owl_window(0, -1, -1, 70, 20, OWL_DIALOG | OWL_MODAL, "Open", CM_OPEN_CANCEL);
    open_panel = owl_files(open_dialog, 1, open_dir);      /* a row of air above it */
    owl_buttons(open_dialog, 2);
    owl_button(CM_FILE_OPEN, OWL_DEFAULT, "~O~pen");
    owl_button(CM_OPEN_CANCEL, OWL_CANCEL, "~C~ancel");
    owl_end();
}

static void open_close(void)
{
    if (open_dialog) owl_close(open_dialog);
    open_dialog = open_panel = 0;
}

/* A name made into a whole path: a drive's path as it is, anything else
   from the folder the panel shows. */
static void join(char *out, const char *dir, const char *name)
{
    if (name[0] && name[1] == ':') { strcpy(out, name); return; }
    if (name[0] == '\\') { out[0] = dir[0]; out[1] = ':'; strcpy(out + 2, name); return; }
    strcpy(out, dir);
    if (out[strlen(out) - 1] != '\\') strcat(out, "\\");
    strcat(out, name);
}

/* The panel shows another folder - or says why it cannot. A path with a
   mask on the end, as the path line shows it, means its folder. */
static void go_to(const char *path)
{
    char dir[80], text[100], *cut;
    strcpy(dir, path);
    if (strchr(dir, '*') || strchr(dir, '?')) {
        cut = strrchr(dir, '\\');
        if (cut) { if (cut == dir + 2) cut[1] = 0; else *cut = 0; }
    }
    if (owl_set_files(open_panel, dir)) strcpy(open_dir, dir);
    else { sprintf(text, "Folder not found: %s", dir); owl_files_error(open_panel, text); }
}

/* "..": the folder above. C:\A\B becomes C:\A, and C:\A becomes C:\. */
static void go_up(void)
{
    char dir[80], *cut;
    strcpy(dir, open_dir);
    cut = strrchr(dir, '\\');
    if (!cut) return;
    if (cut == dir + 2) cut[1] = 0; else *cut = 0;
    go_to(dir);
}

/* A file into a viewer window of its own: its first 8 KB, read by DOS. */
static int view_file(const char *path, const char *name)
{
    FILE *fp = fopen(path, "rb");
    char title[48];
    unsigned n;
    long size;
    owl_id w;
    if (!fp) return 0;
    n = fread(file_buf, 1, sizeof file_buf, fp);
    fseek(fp, 0, SEEK_END);
    size = ftell(fp);
    fclose(fp);
    if (size > (long)n) sprintf(title, "%s - the first %u bytes", name, n);
    else strcpy(title, name);
    w = owl_window(0, 2, 2, 76, 20, OWL_BLUE, title, CM_CLOSE);
    owl_text_dos(w, file_buf, n, 1);    /* read-only: a viewer */
    return 1;
}

/* A name entered in the panel, or its path typed: a folder is walked into,
   a file is opened and the dialog closes. */
static void chosen(const char *name)
{
    char full[128], text[160];
    const char *base;
    unsigned attr;
    if (strcmp(name, "..") == 0) { go_up(); return; }
    join(full, open_dir, name);
    if (_dos_getfileattr(full, &attr) != 0) {
        sprintf(text, "Not found: %s", full);
        owl_files_error(open_panel, text);
        return;
    }
    if (attr & _A_SUBDIR) { go_to(full); return; }
    base = strrchr(full, '\\');
    if (view_file(full, base ? base + 1 : full)) open_close();
    else { sprintf(text, "Cannot read: %s", full); owl_files_error(open_panel, text); }
}

/* What the panel reports is not a command: asked for after every event. */
static void open_poll(void)
{
    char text[128], full[128];
    int kind;
    if (!open_dialog) return;
    kind = owl_take_files(open_panel, text, sizeof text);
    if (kind == 1) chosen(text);
    else if (kind == 2) {
        join(full, open_dir, text);
        if (strchr(full, '*') || strchr(full, '?')) go_to(full); else chosen(full);
    }
}

/* The Open button: whatever is under the cursor. */
static void open_button(void)
{
    char name[80];
    if (open_panel && owl_current_name(open_panel, name, sizeof name)) chosen(name);
}

/* ---- the calculator: a pocket calculator's arithmetic on longs --------- */

static owl_id calc_display;
static long acc, entry;
static int op, typing, error;

/* The display is a canvas one line high, so the number can stand at its
   right-hand end - a static line would fold the spaces away. */
static void calc_print(long value)
{
    char text[16];
    int i, n, pad;
    if (error) strcpy(text, "Error"); else sprintf(text, "%ld", value);
    n = strlen(text);
    pad = 40 - n;
    owl_blit(calc_display, 0, 0, 40, 1);
    for (i = 0; i < 40; i++) owl_cell(i < pad ? ' ' : text[i - pad], 0x1F);
    owl_end();
}

/* acc = acc (op) entry; an overflow or a division by nought is an error. */
static void calc_apply(void)
{
    long r = acc;
    if (!op) r = entry;                 /* nothing waiting: the number is the total */
    else if (typing) {
        switch (op) {
        case CM_ADD: r = acc + entry; if ((entry > 0 && r < acc) || (entry < 0 && r > acc)) error = 1; break;
        case CM_SUB: r = acc - entry; if ((entry < 0 && r < acc) || (entry > 0 && r > acc)) error = 1; break;
        case CM_MUL:                    /* the one product whose check would itself overflow first */
            if ((acc == -1 && entry == LONG_MIN) || (entry == -1 && acc == LONG_MIN)) { error = 1; break; }
            r = acc * entry;
            if (acc != 0 && r / acc != entry) error = 1;
            break;
        case CM_DIV: if (entry == 0 || (entry == -1 && acc == LONG_MIN)) error = 1; else r = acc / entry; break;
        }
    }
    if (!error) acc = r;
    typing = 0;
}

static void calc_key(unsigned cmd)
{
    if (!calc_win) return;
    if (cmd == CM_CLEAR) {
        acc = entry = 0;
        op = typing = error = 0;
        calc_print(acc);
        return;
    }
    if (error) return;                  /* after an error only C does anything */
    if (cmd < CM_ADD) {                 /* a digit */
        long d = cmd - CM_DIGIT;
        if (!typing) { entry = 0; typing = 1; }
        if (entry <= 99999999L && entry >= -99999999L)   /* nine digits are enough */
            entry = entry < 0 ? entry * 10 - d : entry * 10 + d;
        calc_print(entry);
    } else if (cmd == CM_EQUALS) {
        calc_apply();
        op = 0;
        entry = acc;
        calc_print(acc);
    } else if (cmd == CM_NEGATE) {
        if (typing) { entry = -entry; calc_print(entry); }
        else { acc = -acc; entry = acc; calc_print(acc); }
    } else {                            /* + - * /: what waited is done, this one waits */
        calc_apply();
        op = cmd;
        calc_print(acc);
    }
}

static void calc_show(void)
{
    int i;
    if (calc_win) { owl_activate(calc_win); return; }
    calc_win = owl_window(0, -1, -1, 46, 13, OWL_DIALOG, "Calculator", CM_CLOSE);
    calc_display = owl_canvas(calc_win, 2, 1, 40, 1);
    /* The keypad: four placed rows, labels padded to three so the columns
       line up; a keypad is for the mouse and for typing, not for Tab. */
    owl_button_row(calc_win, 2, 3, 5);
    owl_button(CM_DIGIT + 7, 0, " 7 "); owl_button(CM_DIGIT + 8, 0, " 8 "); owl_button(CM_DIGIT + 9, 0, " 9 ");
    owl_button(CM_DIV, OWL_ACCENT, " / "); owl_button(CM_CLEAR, OWL_DANGER, " C ");
    owl_end();
    owl_button_row(calc_win, 2, 5, 4);
    owl_button(CM_DIGIT + 4, 0, " 4 "); owl_button(CM_DIGIT + 5, 0, " 5 "); owl_button(CM_DIGIT + 6, 0, " 6 ");
    owl_button(CM_MUL, OWL_ACCENT, " * ");
    owl_end();
    owl_button_row(calc_win, 2, 7, 4);
    owl_button(CM_DIGIT + 1, 0, " 1 "); owl_button(CM_DIGIT + 2, 0, " 2 "); owl_button(CM_DIGIT + 3, 0, " 3 ");
    owl_button(CM_SUB, OWL_ACCENT, " - ");
    owl_end();
    owl_button_row(calc_win, 2, 9, 4);
    owl_button(CM_DIGIT + 0, 0, " 0 "); owl_button(CM_NEGATE, OWL_ACCENT, "+/-");
    owl_button(CM_EQUALS, OWL_ACCENT | OWL_DEFAULT, " = "); owl_button(CM_ADD, OWL_ACCENT, " + ");
    owl_end();
    /* The keys the window carries: while it is in front, a digit or an
       operator typed is its command. No labels: the status line keeps its
       own words. Enter is the default button, =. */
    owl_window_status(calc_win, 17);
    for (i = 0; i < 10; i++) owl_key("", CM_DIGIT + i, OWL_KEY_CHAR, '0' + i, 0);
    owl_key("", CM_ADD, OWL_KEY_CHAR, '+', 0);
    owl_key("", CM_SUB, OWL_KEY_CHAR, '-', 0);
    owl_key("", CM_MUL, OWL_KEY_CHAR, '*', 0);
    owl_key("", CM_DIV, OWL_KEY_CHAR, '/', 0);
    owl_key("", CM_EQUALS, OWL_KEY_CHAR, '=', 0);
    owl_key("", CM_CLEAR, OWL_KEY_NAMED, 1, 0);         /* Esc */
    owl_key("", CM_CLEAR, OWL_KEY_CHAR, 'c', 0);
    owl_end();
    calc_key(CM_CLEAR);
}

/* ---- the calendar ------------------------------------------------------ */

static owl_id cal_title, cal_canvas;
static int cal_year, cal_month, today_year, today_month, today_day;

static const char *month_names[12] = {
    "January", "February", "March", "April", "May", "June",
    "July", "August", "September", "October", "November", "December"
};

/* 0 Sunday .. 6 Saturday. Sakamoto's: January and February counted in the
   year before. */
static int day_of_week(int y, int m, int d)
{
    static const int t[12] = { 0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4 };
    if (m < 3) y--;
    return (y + y / 4 - y / 100 + y / 400 + t[m - 1] + d) % 7;
}

static int days_in_month(int y, int m)
{
    static const int days[12] = { 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31 };
    int leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    return days[m - 1] + (m == 2 && leap);
}

/* The month into the canvas: a row of day names, then up to six weeks. */
static void calendar_draw(void)
{
    static const char names[] = "Su Mo Tu We Th Fr Sa ";
    char ch[7][21];
    unsigned char at[7][21];
    char title[24];
    int first, days, d, row, col, i, j;

    sprintf(title, "%s %d", month_names[cal_month - 1], cal_year);
    owl_set_text(cal_title, title);

    memset(ch, ' ', sizeof ch);
    memset(at, 0x70, sizeof at);        /* black on grey */
    for (i = 0; i < 21; i++) { ch[0][i] = names[i]; at[0][i] = 0x71; }
    first = day_of_week(cal_year, cal_month, 1);
    days = days_in_month(cal_year, cal_month);
    for (d = 1; d <= days; d++) {
        unsigned char colour;
        row = 1 + (first + d - 1) / 7;
        col = ((first + d - 1) % 7) * 3;
        ch[row][col] = d < 10 ? ' ' : '0' + d / 10;
        ch[row][col + 1] = '0' + d % 10;
        colour = col == 0 ? 0x74 : 0x70;                  /* Sundays red */
        if (cal_year == today_year && cal_month == today_month && d == today_day)
            colour = 0x2F;                                /* today white on green */
        at[row][col] = at[row][col + 1] = colour;
    }
    owl_blit(cal_canvas, 0, 0, 21, 7);
    for (i = 0; i < 7; i++)
        for (j = 0; j < 21; j++) owl_cell((unsigned char)ch[i][j], at[i][j]);
    owl_end();
}

static void calendar_key(unsigned cmd)
{
    if (!cal_win) return;
    if (cmd == CM_TODAY) { cal_year = today_year; cal_month = today_month; }
    else if (cmd == CM_MONTH_BACK) { if (--cal_month == 0) { cal_month = 12; cal_year--; } }
    else { if (++cal_month == 13) { cal_month = 1; cal_year++; } }
    calendar_draw();
}

static void calendar_show(void)
{
    struct dosdate_t today;
    if (cal_win) { owl_activate(cal_win); return; }
    _dos_getdate(&today);
    today_year = cal_year = today.year;
    today_month = cal_month = today.month;
    today_day = today.day;
    cal_win = owl_window(0, -1, -1, 40, 15, OWL_DIALOG, "Calendar", CM_CLOSE);
    cal_title = owl_static(cal_win, 9, 1, 22, 1, " ");
    cal_canvas = owl_canvas(cal_win, 8, 3, 21, 7);
    owl_buttons(cal_win, 3);
    owl_button(CM_MONTH_BACK, 0, "< ~B~ack");
    owl_button(CM_TODAY, OWL_DEFAULT, "~T~oday");
    owl_button(CM_MONTH_ON, 0, "~O~n >");
    owl_end();
    owl_window_status(cal_win, 3);
    owl_key("", CM_MONTH_BACK, OWL_KEY_NAMED, 9, 0);    /* PgUp */
    owl_key("", CM_MONTH_ON, OWL_KEY_NAMED, 10, 0);     /* PgDn */
    owl_key("", CM_TODAY, OWL_KEY_NAMED, 7, 0);         /* Home */
    owl_end();
    calendar_draw();
}

/* ---- the ASCII table --------------------------------------------------- */

static owl_id ascii_canvas, ascii_info;

static void ascii_show(void)
{
    static const char hex[] = "0123456789ABCDEF";
    int x, y;
    if (ascii_win) { owl_activate(ascii_win); return; }
    ascii_win = owl_window(0, -1, -1, 39, 21, OWL_DIALOG, "ASCII table", CM_CLOSE);
    ascii_canvas = owl_canvas(ascii_win, 1, 1, 35, 17);
    ascii_info = owl_static(ascii_win, 2, 18, 33, 1, "Click a glyph");
    /* 35 by 17: the column numbers across the top, the row numbers down
       the side, and every glyph two columns apart - sent as the character
       the glyph table says it is, so the server finds the glyph again. */
    owl_blit(ascii_canvas, 0, 0, 35, 17);
    for (y = 0; y < 17; y++)
        for (x = 0; x < 35; x++) {
            if (y == 0) {
                if (x >= 3 && (x - 3) % 2 == 0) owl_cell(hex[(x - 3) / 2], 0x74);
                else owl_cell(' ', 0x70);
            } else if (x < 3) {
                owl_cell(x == 0 ? hex[y - 1] : x == 1 ? '0' : ' ', 0x74);
            } else if ((x - 3) % 2 == 0) {
                owl_cell(glyphs[(y - 1) * 16 + (x - 3) / 2], 0x1E);
            } else {
                owl_cell(' ', 0x1E);
            }
        }
    owl_end();
}

/* A character as UTF-8, onto s. */
static char *utf8(char *s, unsigned c)
{
    if (c < 0x80) *s++ = (char)c;
    else if (c < 0x800) { *s++ = (char)(0xC0 | c >> 6); *s++ = (char)(0x80 | (c & 0x3F)); }
    else { *s++ = (char)(0xE0 | c >> 12); *s++ = (char)(0x80 | ((c >> 6) & 0x3F)); *s++ = (char)(0x80 | (c & 0x3F)); }
    return s;
}

/* A click on the table: which glyph, in every base. */
static void ascii_poll(void)
{
    int x, y, code;
    char text[40], *end;
    if (!ascii_win || !owl_click(ascii_canvas, &x, &y)) return;
    if (x < 3 || (x - 3) % 2 != 0 || y < 1 || y > 16) return;
    code = (y - 1) * 16 + (x - 3) / 2;
    end = text + sprintf(text, "dec %d  hex %02X  ", code, code);
    end = utf8(end, glyphs[code]);
    *end = 0;
    owl_set_text(ascii_info, text);
}

/* ---- the fifteen puzzle ------------------------------------------------ */

static owl_id puz_canvas, puz_moves_text;
static unsigned char board[16];         /* tiles 1..15, 0 the hole */
static unsigned moves, seed;

static int hole(void)
{
    int i;
    for (i = 0; i < 15 && board[i]; i++) ;
    return i;
}

/* A tile next to the hole slides into it. */
static int slide(int t)
{
    int h = hole(), d = t - h;
    if (t < 0 || t > 15) return 0;
    if (d == 4 || d == -4 || ((d == 1 || d == -1) && t / 4 == h / 4)) {
        board[h] = board[t];
        board[t] = 0;
        moves++;
        return 1;
    }
    return 0;
}

/* An arrow: the tile beside the hole on the far side moves into it. */
static void arrow(unsigned cmd)
{
    int h = hole();
    if (cmd == CM_UP && h < 12) slide(h + 4);
    else if (cmd == CM_DOWN && h >= 4) slide(h - 4);
    else if (cmd == CM_LEFT && h % 4 != 3) slide(h + 1);
    else if (cmd == CM_RIGHT && h % 4 != 0) slide(h - 1);
}

static int solved(void)
{
    int i;
    for (i = 0; i < 15; i++) if (board[i] != i + 1) return 0;
    return 1;
}

/* Solved, then three hundred random slides: every scramble can be undone. */
static void scramble(void)
{
    int i;
    for (i = 0; i < 15; i++) board[i] = (unsigned char)(i + 1);
    board[15] = 0;
    seed = *(unsigned __far *)MK_FP(0x40, 0x6C);        /* the BIOS clock */
    for (i = 0; i < 300; i++) {
        seed = seed * 25173u + 13849u;
        arrow(CM_UP + ((seed >> 8) & 3));
    }
    moves = 0;
}

/* A tile is five by two in blue, with a column and a row of the window's
   grey between tiles. */
static void puzzle_draw(void)
{
    char text[32];
    int x, y;
    owl_blit(puz_canvas, 0, 0, 24, 12);
    for (y = 0; y < 12; y++)
        for (x = 0; x < 24; x++) {
            int t = board[(y / 3) * 4 + x / 6], dx = x % 6, dy = y % 3;
            if (dx == 5 || dy == 2 || t == 0) owl_cell(' ', 0x70);
            else if (dy == 1 && dx == 2 && t >= 10) owl_cell('1', 0x1F);
            else if (dy == 1 && dx == 3) owl_cell('0' + t % 10, 0x1F);
            else owl_cell(' ', 0x1F);
        }
    owl_end();
    sprintf(text, solved() ? "Solved! Moves: %u" : "Moves: %u", moves);
    owl_set_text(puz_moves_text, text);
}

static void puzzle_key(unsigned cmd)
{
    if (!puz_win) return;
    if (cmd == CM_SCRAMBLE) scramble(); else arrow(cmd);
    puzzle_draw();
}

static void puzzle_show(void)
{
    if (puz_win) { owl_activate(puz_win); return; }
    puz_win = owl_window(0, -1, -1, 30, 18, OWL_DIALOG, "Puzzle", CM_CLOSE);
    puz_canvas = owl_canvas(puz_win, 3, 1, 24, 12);
    puz_moves_text = owl_static(puz_win, 3, 13, 24, 1, " ");
    owl_buttons(puz_win, 1);
    owl_button(CM_SCRAMBLE, OWL_DEFAULT, "~S~cramble");
    owl_end();
    owl_window_status(puz_win, 4);
    owl_key("", CM_UP, OWL_KEY_NAMED, 11, 0);
    owl_key("", CM_DOWN, OWL_KEY_NAMED, 12, 0);
    owl_key("", CM_LEFT, OWL_KEY_NAMED, 13, 0);
    owl_key("", CM_RIGHT, OWL_KEY_NAMED, 14, 0);
    owl_end();
    puzzle_key(CM_SCRAMBLE);
}

/* A click on the board: the tile under it, slid if it can go. */
static void puzzle_poll(void)
{
    int x, y;
    if (!puz_win || !owl_click(puz_canvas, &x, &y)) return;
    if (slide((y / 3) * 4 + x / 6)) puzzle_draw();
}

/* ---- the desktop's own verbs ------------------------------------------- */

/* The front window is being closed: whoever owns it forgets it. */
static void window_close(void)
{
    owl_id a = owl_active();
    if (!a) return;
    if (a == calc_win) calc_win = 0;
    if (a == cal_win) cal_win = 0;
    if (a == ascii_win) ascii_win = 0;
    if (a == puz_win) puz_win = 0;
    if (a == box) box = 0;
    if (a == open_dialog) open_dialog = open_panel = 0;
    owl_close(a);
}

/* ---- the bars, and the commands ----------------------------------------- */

static void build_bars(void)
{
    owl_menu_bar(4);
    owl_menu("~F~ile", 4);
    owl_item("~N~ew", "", "An editor window of its own", CM_NEW);
    owl_item("~O~pen...", "F3", "A file from the disk, in a viewer", CM_OPEN);
    owl_line();
    owl_item("E~x~it", "Alt+X", "Leave the program", CM_EXIT);
    owl_menu("~T~ools", 4);
    owl_item("~C~alculator", "", "Add, take, times, share: type or click", CM_CALC);
    owl_item("Ca~l~endar", "", "A month at a time; PgUp and PgDn page", CM_CALENDAR);
    owl_item("~A~SCII table", "", "Every glyph of the font; click one", CM_ASCII);
    owl_item("~P~uzzle", "", "The fifteen puzzle: click a tile, or arrows", CM_PUZZLE);
    owl_menu("~W~indow", 8);
    owl_item("~Z~oom", "F5", "The window fills the desktop, or goes back", CM_ZOOM);
    owl_item("~N~ext", "F6", "The front window goes to the back", CM_NEXT);
    owl_item("~P~revious", "Shift+F6", "The window at the back comes to the front", CM_PREVIOUS);
    owl_item("~C~lose", "Alt+F3", "Close the front window", CM_CLOSE);
    owl_item("~L~ist...", "Alt+0", "Every window by number", CM_LIST);
    owl_line();
    owl_item("C~a~scade", "", "The windows along the diagonal", CM_CASCADE);
    owl_item("~T~ile", "", "The windows share the desktop", CM_TILE);
    owl_menu("~H~elp", 1);
    owl_item("~A~bout", "", "What this program is and what draws it", CM_ABOUT);
    owl_end();

    owl_status(6);
    owl_key("~F1~ Help", CM_HELP, OWL_KEY_F, 1, 0);
    owl_key("~F3~ Open", CM_OPEN, OWL_KEY_F, 3, 0);
    owl_key("~F5~ Zoom", CM_ZOOM, OWL_KEY_F, 5, 0);
    owl_key("~F6~ Next", CM_NEXT, OWL_KEY_F, 6, 0);
    owl_key("~Alt-F3~ Close", CM_CLOSE, OWL_KEY_F, 3, OWL_ALT);
    owl_key("~Alt-X~ Exit", CM_EXIT, OWL_KEY_CHAR, 'x', OWL_ALT);
    owl_end();
}

/* One command. 0 means leave. */
static int command(unsigned cmd)
{
    if (cmd >= CM_DIGIT && cmd <= CM_NEGATE) { calc_key(cmd); return 1; }
    if (cmd >= CM_MONTH_BACK && cmd <= CM_TODAY) { calendar_key(cmd); return 1; }
    if (cmd >= CM_SCRAMBLE && cmd <= CM_RIGHT) { puzzle_key(cmd); return 1; }
    switch (cmd) {
    case CM_EXIT: return 0;
    case CM_NEW: editor_new(); break;
    case CM_OPEN: open_show(); break;
    case CM_FILE_OPEN: open_button(); break;
    case CM_OPEN_CANCEL: open_close(); break;
    case CM_CALC: calc_show(); break;
    case CM_CALENDAR: calendar_show(); break;
    case CM_ASCII: ascii_show(); break;
    case CM_PUZZLE: puzzle_show(); break;
    case CM_ABOUT: case CM_HELP: about(); break;
    case CM_DISMISS: box_close(); break;
    case CM_NEXT: owl_cycle(); break;
    case CM_PREVIOUS: owl_cycle_back(); break;
    case CM_ZOOM: { owl_id a = owl_active(); if (a) owl_zoom(a); } break;
    case CM_CLOSE: window_close(); break;
    case CM_CASCADE: owl_cascade(); break;
    case CM_TILE: owl_tile(); break;
    case CM_LIST: owl_window_list(); break;
    }
    return 1;
}

int main(void)
{
    unsigned pressed, chosen;
    if (!owl_check()) return 1;
    owl_init();
    owl_glyphs(glyphs);
    build_bars();
    about();                            /* say hello */
    for (;;) {
        owl_wait(&pressed, &chosen);    /* a button pressed, a command chosen */
        if (pressed && !command(pressed)) break;
        if (chosen && !command(chosen)) break;
        ascii_poll();                   /* what is not a command: a click on a canvas */
        puzzle_poll();
        open_poll();                    /* and a name entered in the file panel */
    }
    owl_quit();
    return 0;
}
