//! A file list, in the shape Volkov Commander drew one.
//!
//! **The core does not read a directory and never will.** It describes what a
//! directory entry looks like and draws a list of them; filling that list is
//! the application's job. This is the same bargain as a help page's `pending`
//! link and a menu's command number, and for the same reason: a callback out
//! of here would have to survive an interrupt or a WebAssembly import one day,
//! and neither carries one.
//!
//! So the portability layer is one function per platform — DOS calls INT 21h
//! 4Eh/4Fh, Windows and Unix call their own `read_dir`, a browser gets a list
//! from wherever it can — and each of them produces `Vec<FileEntry>`. There is
//! no trait, because nothing in here would call it.
//!
//! **The attribute byte is the common denominator**, and it is DOS's:
//!
//! ```text
//!   01 read-only   02 hidden   04 system
//!   08 volume      10 directory   20 archive
//! ```
//!
//! Windows has exactly these bits. Unix maps read-only from the write
//! permission and hidden from a leading dot, and simply never sets archive.
//! The rule cuts both ways, as it does everywhere else here: there is no POSIX
//! mode field, because DOS could not fill one.

// `no_std` needs these named; with `std` they are the prelude's.
#[allow(unused_imports)]
use alloc::{boxed::Box, string::{String, ToString}, vec::Vec};
use crate::geom::Rect;

pub const ATTR_READONLY: u8 = 0x01;
pub const ATTR_HIDDEN: u8 = 0x02;
pub const ATTR_SYSTEM: u8 = 0x04;
pub const ATTR_DIR: u8 = 0x10;
pub const ATTR_ARCHIVE: u8 = 0x20;

#[derive(Clone, Debug)]
pub struct FileEntry {
    pub name: String,
    pub size: u32,
    /// Year, month, day, hour, minute. Not a packed DOS word: unpacking it in
    /// three different backends is three chances to get the epoch wrong.
    pub date: (u16, u8, u8, u8, u8),
    pub attrs: u8,
}

impl FileEntry {
    pub fn is_dir(&self) -> bool {
        self.attrs & ATTR_DIR != 0
    }

    pub fn ext(&self) -> &str {
        match self.name.rfind('.') {
            Some(i) if i > 0 => &self.name[i + 1..],
            _ => "",
        }
    }

    /// `r a h s` — the letters Volkov showed, in that order, a dash where the
    /// bit is clear so the column stays put and the eye can scan it.
    pub fn attr_string(&self) -> String {
        let f = |bit: u8, c: char| {
            if self.attrs & bit != 0 {
                c
            } else {
                '-'
            }
        };
        [
            f(ATTR_READONLY, 'r'),
            f(ATTR_ARCHIVE, 'a'),
            f(ATTR_HIDDEN, 'h'),
            f(ATTR_SYSTEM, 's'),
        ]
        .iter()
        .collect()
    }
}

/// What a file is, for colouring.
///
/// The order the cases are tried in is the whole design, and it is Far's:
/// **hidden and system come first, before anything else**, so a hidden
/// directory reads as hidden rather than as a directory. Colour the directory
/// first instead and the one attribute a person most needs to notice is the
/// one they never see.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FileKind {
    /// Hidden or system, whatever else it may also be.
    Dim,
    Directory,
    Executable,
    Archive,
    /// Scratch — a `.tmp`, a `.bak`, an editor's leftovers.
    Temporary,
    Plain,
}

impl FileKind {
    pub fn of(e: &FileEntry) -> FileKind {
        if e.attrs & (ATTR_HIDDEN | ATTR_SYSTEM) != 0 {
            return FileKind::Dim;
        }
        if e.is_dir() {
            return FileKind::Directory;
        }
        // Lowercased by hand rather than with `to_lowercase`, which allocates
        // and has opinions about Turkish.
        let mut ext = [0u8; 8];
        let mut n = 0;
        for b in e.ext().bytes().take(8) {
            ext[n] = b.to_ascii_lowercase();
            n += 1;
        }
        match &ext[..n] {
            b"com" | b"exe" | b"bat" | b"btm" | b"cmd" => FileKind::Executable,
            b"zip" | b"rar" | b"arj" | b"lzh" | b"lha" | b"7z" | b"gz" | b"tar" => {
                FileKind::Archive
            }
            b"tmp" | b"bak" | b"old" | b"$$$" | b"syd" | b"chk" => FileKind::Temporary,
            _ => FileKind::Plain,
        }
    }
}

/// Does a name match a DOS-style mask? `*` and `?` only.
pub fn matches(name: &str, mask: &str) -> bool {
    // Characters, not bytes: a name in a core string is one char per
    // glyph, and `?` has to eat exactly one of them.
    fn go(n: &[char], m: &[char]) -> bool {
        match (n.first(), m.first()) {
            (_, None) => n.is_empty(),
            (_, Some('*')) => {
                // The star matches nothing, then one more, then one more. The
                // recursion is fine here: masks are a handful of characters,
                // not a regular expression engine.
                go(n, &m[1..]) || (!n.is_empty() && go(&n[1..], m))
            }
            (Some(_), Some('?')) => go(&n[1..], &m[1..]),
            (Some(a), Some(b)) if a.eq_ignore_ascii_case(b) => go(&n[1..], &m[1..]),
            _ => false,
        }
    }
    // DOS saw every name as `NAME.EXT`, with the dot there whether or not
    // anything followed it. That is why `*.*` meant "every file" and `*.`
    // meant "the ones with no extension" - and why a name without a dot has
    // to be tried with one added, or `README` is invisible to the default
    // mask and the dialog looks empty in a folder that is not.
    let m: Vec<char> = mask.chars().collect();
    let n: Vec<char> = name.chars().collect();
    if go(&n, &m) {
        return true;
    }
    if !name.contains('.') {
        let mut dotted = n;
        dotted.push('.');
        return go(&dotted, &m);
    }
    false
}

pub struct FileList {
    /// Everything the application handed over, untouched.
    pub entries: Vec<FileEntry>,
    pub mask: String,
    /// Indices into `entries`, filtered and sorted. Rebuilt when either
    /// changes; never sorted in place, so the application's order survives and
    /// a second view of the same directory can sort differently.
    view: Vec<usize>,
    pub current: usize,
    /// First visible column of entries.
    pub left: i16,
    /// How many rows the last layout had. Needed to move the cursor by a
    /// column, which is what Left and Right do in a list that flows downward.
    rows: i16,
    cols: i16,
    colw: i16,
    /// Set when a file was chosen or a directory entered. The application
    /// takes it and decides what that means.
    pub chosen: Option<String>,
    /// Insert marks the file under the cursor and moves on - Norton's way,
    /// and the reason a two-panel file manager can be built on this. Marks
    /// belong to entries, not to rows, so a new mask does not lose them.
    pub multi: bool,
    marked: Vec<bool>,
    /// Show the path-and-mask line above the names. An Open dialog wants
    /// it; a commander's panel does not - people who grew up on Norton read
    /// a `*.*` at the top of a panel as something gone wrong, and they
    /// navigate by Enter, not by typing. Without it the names start on the
    /// first row and the foot still says where you are.
    pub path_line: bool,

    /// The line at the top: a directory and a mask together, the way an open
    /// dialog has shown them since before any of us were typing. Editing it is
    /// how you both change directory and change the filter, with one idea
    /// instead of two — which is also why there is no menu item for the mask.
    /// A second way to do the same thing is a second place to look for it.
    pub path: crate::input::InputLine,
    pub focus: Focus,
    /// Somewhere the person asked to go. The application tries it and either
    /// hands back a new list or sets `error` — going there is I/O and I/O is
    /// not ours.
    pub pending_path: Option<String>,
    /// Shown in place of the information line, in red. A drive that went away
    /// while the dialog was open is not an error to crash on; it is a sentence
    /// to put on the screen.
    pub error: Option<String>,
    /// How many rows the foot wants, and how much of its width belongs to the
    /// buttons that are drawn over it.
    foot: i16,
    foot_taken: i16,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Focus {
    Path,
    List,
}

impl FileList {
    pub fn new(entries: Vec<FileEntry>, mask: &str) -> Self {
        let mut f = FileList {
            entries,
            mask: mask.to_string(),
            view: Vec::new(),
            current: 0,
            left: 0,
            rows: 1,
            cols: 1,
            colw: 14,
            chosen: None,
            multi: false,
            marked: Vec::new(),
            path_line: true,
            path: crate::input::InputLine::new("Path:", ""),
            focus: Focus::List,
            pending_path: None,
            error: None,
            foot: 2,
            foot_taken: 0,
        };
        f.rebuild();
        f
    }

    /// Take a fresh list, keeping the cursor on the *name* it was on rather
    /// than the position.
    ///
    /// Directories change while a dialog is open — a build finishes, a
    /// download lands, somebody deletes something. Restoring the index puts
    /// the cursor on whatever moved into that slot, and the next Enter opens
    /// a file nobody chose.
    ///
    /// Marks follow the same rule, for the same reason and a worse outcome:
    /// kept by position, the mark on a file that was moved away passed to
    /// whatever took its row, and the next F8 deleted a file nobody had
    /// marked. A mark is on a name; a name that is gone takes its mark
    /// with it.
    pub fn set_entries(&mut self, entries: Vec<FileEntry>) {
        let was = self.selected().map(|e| e.name.clone());
        let marked: Vec<String> = (0..self.entries.len())
            .filter(|&e| self.marked.get(e).copied().unwrap_or(false))
            .map(|e| self.entries[e].name.clone())
            .collect();
        self.entries = entries;
        self.marked = self.entries.iter().map(|e| marked.contains(&e.name)).collect();
        self.rebuild();
        self.current = was
            .and_then(|n| (0..self.view.len()).find(|&i| self.entries[self.view[i]].name == n))
            .unwrap_or(0);
        self.error = None;
        self.reveal();
    }

    /// More of the same listing: a folder too big to arrive in one piece
    /// arrives in several, and each piece is added to what is there. The
    /// cursor stays on the name it was on.
    pub fn add_entries(&mut self, more: Vec<FileEntry>) {
        let was = self.selected().map(|e| e.name.clone());
        self.entries.extend(more);
        self.rebuild();
        self.current = was
            .and_then(|n| (0..self.view.len()).find(|&i| self.entries[self.view[i]].name == n))
            .unwrap_or(0);
        self.reveal();
    }

    pub fn set_mask(&mut self, mask: &str) {
        self.mask = mask.to_string();
        self.current = 0;
        self.left = 0;
        self.rebuild();
    }

    /// Directories first, then names. `..` before everything, because it is
    /// the one entry people reach for without looking.
    fn rebuild(&mut self) {
        let mut v: Vec<usize> = (0..self.entries.len())
            .filter(|&i| {
                let e = &self.entries[i];
                e.is_dir() || matches(&e.name, &self.mask)
            })
            .collect();
        let key = |e: &FileEntry| {
            (
                if e.name == ".." { 0 } else { 1 },
                if e.is_dir() { 0 } else { 1 },
                e.name.to_ascii_lowercase(),
            )
        };
        v.sort_by(|&a, &b| key(&self.entries[a]).cmp(&key(&self.entries[b])));
        self.view = v;
        self.current = self.current.min(self.view.len().saturating_sub(1));
    }

    pub fn len(&self) -> usize {
        self.view.len()
    }

    pub fn is_empty(&self) -> bool {
        self.view.is_empty()
    }

    pub fn at(&self, ix: usize) -> Option<&FileEntry> {
        self.view.get(ix).map(|&i| &self.entries[i])
    }

    pub fn selected(&self) -> Option<&FileEntry> {
        self.at(self.current)
    }

    /// The panel's own geometry.
    ///
    /// Five rows are not the list's: the path field, a gap, a gap, and the
    /// two-row information pane at the foot.
    ///
    /// The gaps replace the rules we drew before. A coloured panel butted
    /// against the dialog's frame looks like a hole cut in the dialog; the
    /// same panel with a margin around it looks like something sitting on it,
    /// which is what it is. Turbo Vision's own file dialog is built this way
    /// and it is why a grey dialog holding a blue list reads as one object.
    /// `screen_w` is how wide the whole screen is, which is what decides how
    /// many columns the panel gets.
    pub fn layout(&mut self, r: Rect, screen_w: i16) -> (i16, i16) {
        // The foot is two rows, and three when a message needs the room.
        // Cutting the message instead is the worst of both: the person is told
        // that something went wrong and not what, which is the half that would
        // have helped.
        let width = (r.w - 2 - self.foot_taken).max(8);
        self.foot = match &self.error {
            Some(msg) => (wrap(msg, width).len() as i16).clamp(2, 3),
            None => 2,
        };
        self.rows = (r.h - 1 - self.top() - self.foot).max(1);
        // The column is as wide as the longest name it has to hold, within
        // reason. A fixed width is what a commander used because a fixed width
        // is what 8.3 names are; with names of any length it either wastes
        // half the panel or cuts the one file you were looking for.
        let longest = self
            .view
            .iter()
            .map(|&i| self.entries[i].name.chars().count() + 1)
            .max()
            .unwrap_or(8) as i16;
        // Four columns in a panel taking more than half the screen, two in a
        // smaller one, and every column the same width.
        //
        // Sizing them to the longest name instead - which is what this did
        // before - makes the number of columns depend on what happens to be in
        // the directory, so the same window changes shape as you walk around
        // the disk. Names that no longer fit are cut with three dots, which is
        // the trade every commander makes.
        let _ = longest;
        let inner = (r.w - 2).max(4);
        let want = if r.w * 2 > screen_w { 4 } else { 2 };
        self.cols = want.min((inner / 10).max(1));
        self.colw = inner / self.cols;

        // Resizing changes how many rows a column holds, so an entry that was
        // in the second column may now be in the first — and `left` may point
        // past the last column there is. Leave either alone and the panel
        // comes back scrolled into blank space with the names apparently gone,
        // which is exactly what a wider window did before this line existed.
        let cols_used = self.total_cols();
        self.left = self.left.clamp(0, (cols_used - self.cols).max(0));
        self.reveal();

        (self.rows, self.cols)
    }

    fn total_cols(&self) -> i16 {
        let n = self.view.len() as i16;
        (n + self.rows - 1) / self.rows.max(1)
    }

    /// True when there are more columns than fit. The window puts a horizontal
    /// scrollbar on its frame for this — horizontal, because that is the way
    /// the names actually run off the edge. The list flows downward and then
    /// across, so what is out of sight is to the right of the panel and not
    /// below it.
    pub fn overflow(&self) -> (i16, i16) {
        (self.left, self.total_cols())
    }

    pub fn column_width(&self) -> i16 {
        self.colw
    }

    pub fn rows(&self) -> i16 {
        self.rows
    }

    pub fn foot_rows(&self) -> i16 {
        self.foot
    }

    pub fn set_foot_taken(&mut self, n: i16) {
        self.foot_taken = n;
    }

    /// The message, broken to fit. Words stay whole where they can: a path cut
    /// across two lines is a path nobody can read back to you.
    pub fn error_lines(&self, width: i16) -> Vec<String> {
        match &self.error {
            Some(msg) => wrap(msg, width),
            None => Vec::new(),
        }
    }

    pub fn cols(&self) -> i16 {
        self.cols
    }

    /// How many columns actually hold something.
    ///
    /// Drawing the empty ones would be more regular and it would also put two
    /// headers and two rules over nothing at all, which reads as a panel that
    /// has lost its contents rather than one that does not need the room.
    pub fn used_cols(&self) -> i16 {
        (self.total_cols() - self.left).clamp(1, self.cols)
    }

    /// Mark or unmark the entry under the cursor and step down. Directories
    /// and `..` are not marked: a mark says "this file", and they are not.
    pub fn toggle_mark(&mut self) {
        if !self.multi {
            return;
        }
        let Some(&e) = self.view.get(self.current) else { return };
        if self.marked.len() != self.entries.len() {
            self.marked.resize(self.entries.len(), false);
        }
        if !self.entries[e].is_dir() {
            self.marked[e] = !self.marked[e];
        }
        self.step(1);
    }

    /// Whether the entry at a *row* of the view is marked.
    pub fn is_marked(&self, view_ix: usize) -> bool {
        self.view
            .get(view_ix)
            .and_then(|&e| self.marked.get(e))
            .copied()
            .unwrap_or(false)
    }

    /// No marks at all: after the marked files were copied, moved or
    /// deleted, or on going into another folder.
    pub fn clear_marks(&mut self) {
        self.marked.clear();
    }

    /// The marked names, in the order they are shown.
    pub fn marked_names(&self) -> Vec<String> {
        self.view
            .iter()
            .filter(|&&e| self.marked.get(e).copied().unwrap_or(false))
            .map(|&e| self.entries[e].name.clone())
            .collect()
    }

    /// Move by whole entries (`1`) or whole columns (`rows`).
    pub fn step(&mut self, delta: i16) {
        if self.view.is_empty() {
            return;
        }
        let n = self.view.len() as i16;
        self.current = (self.current as i16 + delta).clamp(0, n - 1) as usize;
        self.reveal();
    }

    pub fn home(&mut self) {
        self.current = 0;
        self.reveal();
    }

    pub fn end(&mut self) {
        self.current = self.view.len().saturating_sub(1);
        self.reveal();
    }

    fn reveal(&mut self) {
        let col = self.current as i16 / self.rows;
        if col < self.left {
            self.left = col;
        } else if col >= self.left + self.cols {
            self.left = col - self.cols + 1;
        }
        self.left = self.left.max(0);
    }

    /// The row the names start on: under the path line and its gap, or at
    /// the top when there is no path line.
    pub fn top(&self) -> i16 {
        if self.path_line {
            2
        } else {
            0
        }
    }

    /// Which entry is under a point inside the panel.
    pub fn at_point(&self, x: i16, y: i16) -> Option<usize> {
        // Row 0 is the path field and row 1 is the gap under it; the list
        // starts at row 2 (or at 0, without a path line) and the information
        // pane is at the foot.
        let top = self.top();
        if y < top || y >= self.rows + top {
            return None;
        }
        let y = y - top;
        let col = self.left + (x - 1).max(0) / self.column_width();
        let ix = (col * self.rows + y) as usize;
        (ix < self.view.len()).then_some(ix)
    }

    /// Move the view by whole columns, for the scrollbar on the frame. The
    /// cursor follows only if it would otherwise be off the panel — scrolling
    /// is looking, not choosing.
    pub fn scroll_columns(&mut self, col: i16) {
        let total = self.total_cols();
        self.left = col.clamp(0, (total - self.cols).max(0));
        let cur_col = self.current as i16 / self.rows.max(1);
        if cur_col < self.left {
            self.current = (self.left * self.rows) as usize;
        } else if cur_col >= self.left + self.cols {
            self.current = ((self.left + self.cols - 1) * self.rows) as usize;
        }
        self.current = self.current.min(self.view.len().saturating_sub(1));
    }

    pub fn set_path(&mut self, path: &str) {
        self.path.set_text(path);
        self.error = None;
    }

    pub fn path_text(&self) -> &str {
        &self.path.text
    }

    /// Edit the path line. Returns true if the key was used.
    pub fn edit_path(&mut self, k: crate::event::Key) -> bool {
        let used = self.path.key(k);
        if used {
            // Any edit means they are fixing whatever went wrong, so the
            // message goes. Leaving it up while the text changes underneath
            // makes it look like the program is still complaining about
            // something that is no longer on the screen.
            self.error = None;
        }
        if self.path.entered {
            self.path.entered = false;
            self.pending_path = Some(self.path.text.clone());
        }
        used
    }

    /// Jump to the first name starting with this letter, wrapping round from
    /// where the cursor is.
    ///
    /// This is what a letter means at a file list, and it is what every
    /// commander has done with one. It used to be typed into the path field
    /// instead, which put a letter somewhere nobody was looking and moved the
    /// focus out from under the next keystroke.
    pub fn jump_to(&mut self, c: char) -> bool {
        let c = c.to_ascii_lowercase();
        let n = self.view.len();
        if n == 0 {
            return false;
        }
        for k in 1..=n {
            let i = (self.current + k) % n;
            let name = &self.entries[self.view[i]].name;
            if name
                .chars()
                .next()
                .map(|f| f.to_ascii_lowercase() == c)
                .unwrap_or(false)
            {
                self.current = i;
                self.reveal();
                return true;
            }
        }
        false
    }

    pub fn choose(&mut self) {
        if let Some(e) = self.selected() {
            self.chosen = Some(e.name.clone());
        }
    }

    /// The line under the panel: size, date and attributes of whatever the
    /// cursor is on. Volkov showed this and it is the reason its file panel
    /// needs no second window.
    pub fn info(&self) -> String {
        let Some(e) = self.selected() else {
            return String::new();
        };
        let (y, mo, d, h, mi) = e.date;
        let size = if e.is_dir() {
            "   <DIR>".to_string()
        } else {
            format!("{:>8}", e.size)
        };
        // An entry with no date — `..`, which we invent rather than read —
        // gets blanks. Printing 00-00-0000 would look like a broken clock
        // rather than a missing fact.
        let when = if y == 0 {
            " ".repeat(16)
        } else {
            format!("{:02}-{:02}-{:04} {:02}:{:02}", d, mo, y, h, mi)
        };
        format!(
            "{} {} {} {}",
            trim_to(&e.name, 12),
            size,
            when,
            e.attr_string()
        )
    }
}

fn trim_to(s: &str, n: usize) -> String {
    let mut out: String = s.chars().take(n).collect();
    while out.chars().count() < n {
        out.push(' ');
    }
    out
}

/// A name cut to fit, with three dots where the rest went.
///
/// The dots are not decoration. A name silently cut at the edge of its column
/// reads as a shorter name, and two files whose names differ only past the cut
/// become one file listed twice.
pub fn fit(name: &str, width: i16) -> String {
    let w = width.max(1) as usize;
    if name.chars().count() <= w {
        return name.to_string();
    }
    if w <= 3 {
        return ".".repeat(w);
    }
    let mut out: String = name.chars().take(w - 3).collect();
    out.push_str("...");
    out
}

/// Break a line at spaces, and inside a word only when the word alone is
/// longer than the room there is.
fn wrap(text: &str, width: i16) -> Vec<String> {
    let w = width.max(4) as usize;
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        if !cur.is_empty() && cur.chars().count() + 1 + word.chars().count() > w {
            out.push(core::mem::take(&mut cur));
        }
        if word.chars().count() > w {
            // A single word too long for the line has to be cut, and the edge
            // is the least surprising place to cut it.
            let mut rest: &str = word;
            while rest.chars().count() > w {
                out.push(rest.chars().take(w).collect());
                let at = rest.char_indices().nth(w).map_or(rest.len(), |(i, _)| i);
                rest = &rest[at..];
            }
            cur = rest.to_string();
            continue;
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(word);
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}
