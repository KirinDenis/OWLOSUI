/* ==========================================================================
   OWLOSUI.C - the OWLOSUI toolkit, for a DOS program in C: the wire.

   Every function here builds a request of lib/PROTOCOL.md, calls INT 60h
   with it - DS:SI the request, ES:DI room for the reply, CX how much -
   and reads the reply. OWLOSRES answers; nothing is drawn here.
   ========================================================================== */

#include <dos.h>
#include <stdio.h>
#include <string.h>
#include "OWLOSUI.H"

#define OWL_INT 0x60

static unsigned char req[32768];        /* the request being built: a file's text fits */
static unsigned char rep[2048];         /* the reply */
static unsigned at;                     /* where the next byte of req goes */

/* ---- the wire ---- */

void owl_begin(unsigned char op) { req[0] = op; at = 3; }
void owl_u8(unsigned char v)     { req[at++] = v; }
void owl_u16(unsigned v)         { req[at++] = (unsigned char)v; req[at++] = (unsigned char)(v >> 8); }

void owl_str(const char *s)
{
    unsigned n = strlen(s);
    owl_u16(n);
    memcpy(req + at, s, n);
    at += n;
}

int owl_send(void)
{
    union REGS r;
    struct SREGS s;
    unsigned len = at - 3;
    req[1] = (unsigned char)len;
    req[2] = (unsigned char)(len >> 8);
    segread(&s);                        /* small model: both buffers are in DS */
    s.es = s.ds;
    r.x.si = (unsigned)req;
    r.x.di = (unsigned)rep;
    r.x.cx = sizeof rep;
    int86x(OWL_INT, &r, &r, &s);
    return r.h.al;
}

unsigned char *owl_reply(void) { return rep + 3; }

static unsigned reply_u16(int i) { return rep[3 + i] | (rep[4 + i] << 8); }

static owl_id send_id(void) { owl_send(); return reply_u16(0); }

static void rect(int x, int y, int w, int h) { owl_u16(x); owl_u16(y); owl_u16(w); owl_u16(h); }

static void simple(unsigned char op) { owl_begin(op); owl_send(); }

static void with_id(unsigned char op, owl_id id) { owl_begin(op); owl_u16(id); owl_send(); }

/* ---- the session ---- */

/* OWLOSRES's handler has "OWLOSUI" and a zero in the eight bytes before it. */
int owl_check(void)
{
    char __far *p = (char __far *)_dos_getvect(OWL_INT);
    if (FP_OFF(p) >= 8 && _fmemcmp(p - 8, "OWLOSUI", 8) == 0) return 1;
    puts("This program draws with OWLOSUI: start it with OWLOSRES, as RUN.BAT does.");
    return 0;
}

void owl_init(void) { owl_begin(0x01); owl_u16(80); owl_u16(25); owl_send(); }
void owl_quit(void) { simple(0x00); }

void owl_wait(unsigned *pressed, unsigned *command)
{
    simple(0x5C);
    *pressed = reply_u16(0);
    *command = reply_u16(2);
}

/* ---- views ---- */

owl_id owl_window(owl_id parent, int x, int y, int w, int h,
                  unsigned char flags, const char *title, unsigned close_cmd)
{
    owl_begin(0x10); owl_u16(parent); rect(x, y, w, h); owl_u8(flags); owl_str(title); owl_u16(close_cmd);
    return send_id();
}

owl_id owl_static(owl_id parent, int x, int y, int w, int h, const char *text)
{
    owl_begin(0x12); owl_u16(parent); rect(x, y, w, h); owl_str(text);
    return send_id();
}

owl_id owl_text(owl_id parent, const char *text)
{
    owl_begin(0x11); owl_u16(parent); rect(0, 0, 0, 0); owl_u8(0); owl_u8(0); owl_str(text);
    return send_id();
}

owl_id owl_canvas(owl_id parent, int x, int y, int w, int h)
{
    owl_begin(0x1B); owl_u16(parent); rect(x, y, w, h);
    return send_id();
}

/* ---- requests with parts ---- */

void owl_buttons(owl_id parent, int n) { owl_begin(0x14); owl_u16(parent); owl_u8(n); }

void owl_button_row(owl_id parent, int x, int y, int n)
{
    owl_begin(0x1E); owl_u16(parent); rect(x, y, 0, 0); owl_u8(1); owl_u8(n);
}

void owl_message_box(const char *title, const char *text, int n)
{
    owl_begin(0x15); owl_str(title); owl_str(text); owl_u8(n);
}

void owl_button(unsigned cmd, unsigned char flags, const char *label)
{
    owl_u16(cmd); owl_u8(flags); owl_str(label);
}

void owl_status(int n) { owl_begin(0x16); owl_u8(n); }
void owl_window_status(owl_id window, int n) { owl_begin(0x4A); owl_u16(window); owl_u8(n); }

void owl_key(const char *label, unsigned cmd, unsigned char kind, unsigned value, unsigned char mods)
{
    owl_u16(cmd); owl_u8(kind); owl_u16(value); owl_u8(mods); owl_str(label);
}

void owl_menu_bar(int menus) { owl_begin(0x1C); owl_u8(menus); }

void owl_menu(const char *text, int items)
{
    owl_u8(0); owl_u16(0); owl_str(text); owl_str(""); owl_str(""); owl_u8(items);
}

void owl_item(const char *text, const char *shortcut, const char *hint, unsigned cmd)
{
    owl_u8(0); owl_u16(cmd); owl_str(text); owl_str(shortcut); owl_str(hint); owl_u8(0);
}

void owl_line(void) { owl_u8(1); owl_u16(0); owl_str(""); owl_str(""); owl_str(""); owl_u8(0); }

void owl_blit(owl_id canvas, int x, int y, int w, int h) { owl_begin(0x2C); owl_u16(canvas); rect(x, y, w, h); }
void owl_cell(unsigned ch, unsigned char attr) { owl_u16(ch); owl_u8(attr); }

owl_id owl_end(void) { return send_id(); }

/* ---- changing things ---- */

void owl_set_text(owl_id id, const char *text) { owl_begin(0x2B); owl_u16(id); owl_str(text); owl_send(); }
void owl_close(owl_id id)    { with_id(0x20, id); }
void owl_activate(owl_id id) { with_id(0x27, id); }
owl_id owl_active(void)      { owl_begin(0x2A); return send_id(); }
void owl_zoom(owl_id id)     { with_id(0x44, id); }
void owl_cycle(void)         { simple(0x43); }
void owl_cycle_back(void)    { simple(0x4D); }
void owl_cascade(void)       { simple(0x47); }
void owl_tile(void)          { simple(0x48); }
void owl_window_list(void)   { simple(0x4C); }

int owl_click(owl_id canvas, int *x, int *y)
{
    with_id(0x2F, canvas);
    if (rep[3] == 0) return 0;
    *x = (int)reply_u16(1);
    *y = (int)reply_u16(3);
    return 1;
}

void owl_glyphs(unsigned table[256])
{
    int i;
    simple(0x42);
    for (i = 0; i < 256; i++) table[i] = reply_u16(3 + i * 2);
}

/* ---- files: DOS reads the folder and the file, the panel shows them ---- */

#define ANY_ENTRY (_A_SUBDIR | _A_RDONLY | _A_HIDDEN | _A_SYSTEM | _A_ARCH)
#define NO_MORE_FILES 18

static unsigned dos_table[256];         /* the glyphs as Unicode, for text from DOS */
static int dos_table_loaded;

/* A folder made into DOS's pattern for everything in it: C:\DIR\*.* */
static void pattern_of(char *out, const char *dir)
{
    unsigned n = strlen(dir);
    strcpy(out, dir);
    if (n == 0 || dir[n - 1] != '\\') strcat(out, "\\");
    strcat(out, "*.*");
}

static void patch_u16(unsigned where, unsigned v)
{
    req[where] = (unsigned char)v;
    req[where + 1] = (unsigned char)(v >> 8);
}

/* The folder's entries after a request's header, as FILES, SET_FILES and
   ADD_FILES want them: a count, then a name, size, date, time and DOS
   attributes each, straight from FindFirst and FindNext. "." is left out;
   ".." is kept, as DOS gives it. A folder too big for one request goes on
   in ADD_FILES to the same panel - whose id, for a new panel, is the
   reply to the first part. */
static owl_id listing(owl_id panel, struct find_t *f, unsigned rc, int is_new)
{
    unsigned count_at = at, count = 0;
    owl_u16(0);
    for (; rc == 0; rc = _dos_findnext(f)) {
        if (strcmp(f->name, ".") == 0) continue;
        if (at + 64 > sizeof req) {
            patch_u16(count_at, count);
            owl_send();
            if (is_new) { panel = reply_u16(0); is_new = 0; }
            owl_begin(0x5A); owl_u16(panel);
            count_at = at;
            count = 0;
            owl_u16(0);
        }
        owl_str(f->name);
        owl_u16((unsigned)f->size); owl_u16((unsigned)(f->size >> 16));
        owl_u16(1980 + (f->wr_date >> 9));
        owl_u8((unsigned char)((f->wr_date >> 5) & 15));
        owl_u8((unsigned char)(f->wr_date & 31));
        owl_u8((unsigned char)(f->wr_time >> 11));
        owl_u8((unsigned char)((f->wr_time >> 5) & 63));
        owl_u8(f->attrib);
        count++;
    }
    patch_u16(count_at, count);
    owl_send();
    return is_new ? reply_u16(0) : panel;
}

owl_id owl_files(owl_id parent, int top, const char *dir)
{
    char pattern[80];
    struct find_t f;
    unsigned rc;
    pattern_of(pattern, dir);
    rc = _dos_findfirst(pattern, ANY_ENTRY, &f);
    owl_begin(0x1A); owl_u16(parent); rect(0, top, 0, 0); owl_u8(0); owl_str("*.*"); owl_str(pattern);
    return listing(0, &f, rc, 1);       /* a folder DOS cannot read: an empty panel */
}

int owl_set_files(owl_id panel, const char *dir)
{
    char pattern[80];
    struct find_t f;
    unsigned rc;
    pattern_of(pattern, dir);
    rc = _dos_findfirst(pattern, ANY_ENTRY, &f);
    if (rc != 0 && rc != NO_MORE_FILES) return 0;       /* no such folder, or no disk */
    owl_begin(0x25); owl_u16(panel); owl_str(pattern); owl_str("*.*");
    listing(panel, &f, rc, 0);
    return 1;
}

void owl_files_error(owl_id panel, const char *text)
{
    owl_begin(0x29); owl_u16(panel); owl_str(text);
    owl_send();
}

int owl_take_files(owl_id panel, char *text, int room)
{
    unsigned n;
    with_id(0x26, panel);
    n = reply_u16(1);
    if (n >= (unsigned)room) n = room - 1;
    memcpy(text, rep + 6, n);
    text[n] = 0;
    return rep[3];
}

int owl_current_name(owl_id panel, char *name, int room)
{
    unsigned n;
    with_id(0x28, panel);
    if (reply_u16(0) == 0) return 0;
    n = reply_u16(2);
    if (n >= (unsigned)room) n = room - 1;
    memcpy(name, rep + 7, n);
    name[n] = 0;
    return 1;
}

/* Bytes in the machine's code page, as an editor: each byte through the
   glyph table into Unicode and on as UTF-8, which is what the wire
   carries. A line ends at LF; CR is dropped; a tab is a space. What does
   not fit in one request is left out. */
owl_id owl_text_dos(owl_id parent, const char *bytes, unsigned n, unsigned char flags)
{
    unsigned len_at, i;
    if (!dos_table_loaded) { owl_glyphs(dos_table); dos_table_loaded = 1; }
    owl_begin(0x11); owl_u16(parent); rect(0, 0, 0, 0); owl_u8(0); owl_u8(flags);
    len_at = at;
    owl_u16(0);
    for (i = 0; i < n && at + 4 < sizeof req; i++) {
        unsigned char b = (unsigned char)bytes[i];
        unsigned c;
        if (b == '\r') continue;
        c = b == '\n' ? '\n' : b == '\t' ? ' ' : dos_table[b];
        if (c < 0x80) req[at++] = (unsigned char)c;
        else if (c < 0x800) {
            req[at++] = (unsigned char)(0xC0 | (c >> 6));
            req[at++] = (unsigned char)(0x80 | (c & 0x3F));
        } else {
            req[at++] = (unsigned char)(0xE0 | (c >> 12));
            req[at++] = (unsigned char)(0x80 | ((c >> 6) & 0x3F));
            req[at++] = (unsigned char)(0x80 | (c & 0x3F));
        }
    }
    patch_u16(len_at, at - len_at - 2);
    return send_id();
}
