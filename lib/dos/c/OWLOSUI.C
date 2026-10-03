/* ==========================================================================
   OWLOSUI.C - the OWLOSUI toolkit, for a DOS program in C: the wire.

   Every function here builds a request of lib/PROTOCOL.md, calls INT 60h
   with it - DS:SI the request, ES:DI room for the reply, CX how much -
   and reads the reply. OWLOSRES answers; nothing is drawn here.
   ========================================================================== */

#include <dos.h>
#include <malloc.h>                     /* _fmalloc: a far block, for a file's text */
#include <process.h>                    /* spawnl: another program, for owl_run */
#include <stdio.h>
#include <stdlib.h>
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

/* The request out, its reply into buf - room bytes, anywhere in memory:
   rep for most, a far block for a file's text. The status: 0 OK. */
static int send_into(void __far *buf, unsigned room)
{
    union REGS r;
    struct SREGS s;
    unsigned len = at - 3;
    req[1] = (unsigned char)len;
    req[2] = (unsigned char)(len >> 8);
    segread(&s);                        /* small model: the request is in DS */
    s.es = FP_SEG(buf);
    r.x.si = (unsigned)req;
    r.x.di = FP_OFF(buf);
    r.x.cx = room;
    int86x(OWL_INT, &r, &r, &s);
    return r.h.al;
}

int owl_send(void) { return send_into(rep, sizeof rep); }

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

void owl_editor(owl_id text, unsigned char offers, unsigned char state)
{
    owl_begin(0x5D); owl_u16(text); owl_u8(offers); owl_u8(state); owl_send();
}

unsigned char owl_editor_state(owl_id text)
{
    owl_begin(0x5E); owl_u16(text); owl_send();
    return owl_reply()[1];
}

int owl_syntax(owl_id text, const char *language)
{
    owl_begin(0x5F); owl_u16(text); owl_str(language); owl_send();
    return owl_reply()[0];
}
void owl_close(owl_id id)    { with_id(0x20, id); }
void owl_activate(owl_id id) { with_id(0x27, id); }
owl_id owl_active(void)      { owl_begin(0x2A); return send_id(); }
void owl_zoom(owl_id id)     { with_id(0x44, id); }
void owl_minimize(owl_id id) { with_id(0x6C, id); }

void owl_window_tag(owl_id window, const char *tag)
{
    owl_begin(0x65); owl_u16(window); owl_str(tag); owl_send();
}

void owl_window_indicator(owl_id window, const char *text)
{
    owl_begin(0x66); owl_u16(window); owl_str(text); owl_send();
}

int owl_place(owl_id window, int *x, int *y, int *w, int *h)
{
    with_id(0x62, window);
    *x = (int)reply_u16(0); *y = (int)reply_u16(2);
    *w = (int)reply_u16(4); *h = (int)reply_u16(6);
    return rep[3 + 8] != 0;
}

owl_id owl_console(owl_id parent, unsigned scrollback)
{
    /* the rect is read and not used: a console fills its window */
    owl_begin(0x67); owl_u16(parent); rect(0, 0, 0, 0); owl_u16(scrollback);
    return send_id();
}

void owl_console_write(owl_id console, const char *text)
{
    owl_begin(0x68); owl_u16(console); owl_str(text); owl_send();
}
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

owl_id owl_files_ex(owl_id parent, int top, unsigned char flags, const char *dir)
{
    char pattern[80];
    struct find_t f;
    unsigned rc;
    pattern_of(pattern, dir);
    rc = _dos_findfirst(pattern, ANY_ENTRY, &f);
    owl_begin(0x1A); owl_u16(parent); rect(0, top, 0, 0); owl_u8(flags); owl_str("*.*"); owl_str(pattern);
    return listing(0, &f, rc, 1);       /* a folder DOS cannot read: an empty panel */
}

owl_id owl_files(owl_id parent, int top, const char *dir) { return owl_files_ex(parent, top, 0, dir); }

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
static void load_dos_table(void)
{
    if (!dos_table_loaded) { owl_glyphs(dos_table); dos_table_loaded = 1; }
}

/* How many bytes of UTF-8 a byte of a DOS text becomes on the wire. */
static unsigned cost(unsigned char b)
{
    unsigned c;
    if (b == '\r') return 0;
    if (b == '\n' || b == '\t') return 1;
    c = dos_table[b];
    return c < 0x80 ? 1 : c < 0x800 ? 2 : 3;
}

unsigned owl_dos_fits(const char *bytes, unsigned n)
{
    unsigned used = 17, i = 0;          /* TEXT's header, as owl_text_dos_ex writes it */
    load_dos_table();
    while (i < n && used + 4 < sizeof req) used += cost((unsigned char)bytes[i++]);
    return i;
}

owl_id owl_text_dos(owl_id parent, const char *bytes, unsigned n, unsigned char flags)
{
    unsigned taken;
    return owl_text_dos_ex(parent, bytes, n, flags, &taken);
}

owl_id owl_text_dos_ex(owl_id parent, const char *bytes, unsigned n, unsigned char flags, unsigned *taken)
{
    unsigned len_at, i;
    load_dos_table();
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
    *taken = i;
    return send_id();
}

/* The byte of DOS's code page that shows a Unicode character: the glyph
   table read backwards. ASCII is itself; what the table does not have is
   a question mark. */
static unsigned char byte_of(unsigned c)
{
    int i;
    if (c >= 32 && c < 127) return (unsigned char)c;
    for (i = 0; i < 256; i++) if (dos_table[i] == c) return (unsigned char)i;
    return '?';
}

/* GET_TEXT into a far block of its own - a file's text is far more than
   rep holds, and this small model has one 64K segment for all its data -
   and back from UTF-8 into the code page, a line ending as CR LF, the way
   DOS keeps a text file. */
unsigned owl_get_text_dos(owl_id id, char *bytes, unsigned room)
{
    const unsigned size = 65000u;       /* an int is 16 bits here: an enum would not hold it */
    unsigned char __far *p;
    unsigned len, i, put = 0, c;
    load_dos_table();
    p = (unsigned char __far *)_fmalloc(size);
    if (p == 0) return 0xFFFF;
    owl_begin(0x21); owl_u16(id);
    if (send_into(p, size) != 0) { _ffree(p); return 0xFFFF; }
    len = p[3] | (p[4] << 8);
    for (i = 5; i < 5 + len && put <= room;) {
        unsigned char b = p[i];
        if (b < 0x80) { c = b; i += 1; }
        else if (b < 0xE0) { c = ((b & 0x1F) << 6) | (p[i + 1] & 0x3F); i += 2; }
        else if (b < 0xF0) { c = ((unsigned)(b & 0x0F) << 12) | ((p[i + 1] & 0x3F) << 6) | (p[i + 2] & 0x3F); i += 3; }
        else { c = '?'; i += 4; }       /* beyond what a code page has */
        if (c == '\n') {
            if (put < room) bytes[put] = '\r';
            put++;
            if (put < room) bytes[put] = '\n';
            put++;
        } else {
            if (put < room) bytes[put] = (char)byte_of(c);
            put++;
        }
    }
    _ffree(p);
    return put <= room ? put : 0xFFFF;
}

/* MARKED_NAMES, kept: owl_marked says how many, owl_marked_name gives them. */
static unsigned marked_count;

unsigned owl_marked(owl_id panel)
{
    owl_begin(0x28); owl_u16(panel);
    marked_count = owl_send() != 0 ? 0 : reply_u16(0);
    return marked_count;
}

int owl_marked_name(unsigned i, char *name, int room)
{
    unsigned pos = 5, k, n;              /* past the header and the count */
    name[0] = 0;
    if (i < 1 || i > marked_count) return 0;
    for (k = 2; k <= i; k++) pos += 2 + (rep[pos] | (rep[pos + 1] << 8));
    n = rep[pos] | (rep[pos + 1] << 8);
    if (n >= (unsigned)room) n = room - 1;
    memcpy(name, rep + pos + 2, n);
    name[n] = 0;
    return 1;
}

void owl_unmark(owl_id panel) { with_id(0x61, panel); }

/* ---- the session, more ---- */

/* FRAME: on DOS the resident draws the screen for it. The frame's own bytes
   are not wanted here, so the room is only a header: the status says it was
   too small, and that is fine. */
void owl_draw(void)
{
    unsigned char head[3];
    owl_begin(0x40);
    send_into(head, sizeof head);
}

/* SUSPEND and RESUME are the resident's own: the session - every window -
   kept whole while another program has the screen. */
void owl_suspend(void) { simple(0x63); }
void owl_resume(void)  { owl_begin(0x64); owl_u8(0); owl_send(); }

int owl_run(const char *path, const char *args, int pause)
{
    unsigned n = strlen(path);
    int rc, err = 0;
    /* The far heap keeps what was freed - owl_get_text_dos's 65000 bytes -
       and DOS would have no room left for the program: give it back first. */
    _fheapshrink();
    owl_suspend();
    if (n >= 4 && (stricmp(path + n - 4, ".BAT") == 0)) {
        /* A batch file is COMMAND.COM's to run, not DOS's EXEC's. */
        const char *shell = getenv("COMSPEC");
        char line[130];
        if (shell == 0) shell = "C:\\COMMAND.COM";
        sprintf(line, "%s %s", path, args);
        rc = spawnl(P_WAIT, shell, shell, "/C", line, (char *)0);
    } else {
        rc = spawnl(P_WAIT, path, path, args, (char *)0);
    }
    if (rc == -1) err = _doserrno ? _doserrno : 2;
    /* Pause: what a program printed stays until a key. The resident decides
       whether there is anything to read - not after a toolkit program, whose
       windows were its answer. */
    owl_begin(0x64); owl_u8((unsigned char)(pause && err == 0)); owl_send();
    return err;
}

/* ---- controls ---- */

static void u32(unsigned long v) { owl_u16((unsigned)v); owl_u16((unsigned)(v >> 16)); }

owl_id owl_input(owl_id parent, int x, int y, int w, unsigned max, const char *label, const char *text)
{
    owl_begin(0x13); owl_u16(parent); rect(x, y, w, 1); owl_u16(max); owl_str(label); owl_str(text);
    return send_id();
}

int owl_get_text(owl_id id, char *text, int room)
{
    unsigned n;
    owl_begin(0x21); owl_u16(id);
    n = owl_send() != 0 ? 0 : reply_u16(0);
    if (n >= (unsigned)room) n = room - 1;
    memcpy(text, rep + 5, n);
    text[n] = 0;
    return (int)n;
}

void owl_list(owl_id parent, int x, int y, int w, int h, unsigned n)
{
    owl_begin(0x19); owl_u16(parent); rect(x, y, w, h); owl_u8(0); owl_u16(n);
}

void owl_list_item(const char *text) { owl_str(text); }

unsigned owl_current(owl_id list) { with_id(0x24, list); return reply_u16(0); }

owl_id owl_progress(owl_id parent, int x, int y, int w, unsigned long max)
{
    owl_begin(0x18); owl_u16(parent); rect(x, y, w, 1); u32(max); owl_u8(1);
    return send_id();
}

void owl_set_progress(owl_id bar, unsigned long value)
{
    owl_begin(0x22); owl_u16(bar); u32(value); owl_send();
}

/* ---- a tree ---- */

void owl_tree(owl_id parent, int n) { owl_begin(0x53); owl_u16(parent); rect(0, 0, 0, 0); owl_u8(n); }

void owl_node(unsigned char flags, const char *text, int children)
{
    owl_u8(flags); owl_str(text); owl_u8(children);
}

/* The texts of a reply, one after another from pos, into path->text. */
static void read_texts(owl_path *path, unsigned pos, int with_index)
{
    int k;
    for (k = 0; k < path->depth; k++) {
        unsigned n, l;
        if (with_index) { path->index[k] = rep[pos] | (rep[pos + 1] << 8); pos += 2; }
        n = rep[pos] | (rep[pos + 1] << 8);
        l = n > 12 ? 12 : n;            /* a DOS name is at most 8.3 */
        memcpy(path->text[k], rep + pos + 2, l);
        path->text[k][l] = 0;
        pos += 2 + n;
    }
}

int owl_tree_expand(owl_id tree, owl_path *path)
{
    with_id(0x55, tree);
    path->depth = rep[3] > 16 ? 16 : rep[3];
    read_texts(path, 4, 1);
    return path->depth > 0;
}

void owl_tree_children(owl_id tree, const owl_path *path, int n)
{
    int k;
    owl_begin(0x54); owl_u16(tree); owl_u8(path->depth);
    for (k = 0; k < path->depth; k++) owl_u16(path->index[k]);
    owl_u8(n);
}

void owl_tree_path(owl_id tree, owl_path *path)
{
    with_id(0x56, tree);
    path->depth = rep[3] > 16 ? 16 : rep[3];
    read_texts(path, 4, 0);
}
