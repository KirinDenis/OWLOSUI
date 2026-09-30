//! DEMO - the OWLOS UI demo, in Rust, the core linked in.
//!
//!     RUN DEMO
//!
//! The same program as DEMO in `..\Asm`, `..\C` and `..\Pascal`, part for
//! part: a menu bar, a status line, a desktop, and small windows that each
//! use one part of the kit.
//!
//!   File      New: an editor window of its own. Open: the file panel,
//!             the folder read through DOS, a file shown in a viewer. Exit.
//!   Tools     Calculator  - a keypad of button rows; typing works too,
//!                           because the window carries the keys it wants
//!             Calendar    - a canvas drawn again for every month
//!             ASCII table - every glyph of the font; a click names one
//!             Puzzle      - the fifteen puzzle: click a tile, or arrows
//!   Window    Zoom, Next, Previous, Close, List, Cascade, Tile
//!   Help      About
//!
//! Those three ask the resident for every part over INT 60h; this one
//! makes the parts from the core's own types (`kit.rs`) and runs the core
//! itself (`owlosui_dos::run`). What is here is the program: what to
//! build, and what to do with each command.

#![no_std]
#![no_main]

extern crate alloc;

mod kit;

use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use kit::{alt, item, key};
use owlosui_core::{Cell, KeyCode, MenuBar, MenuItem, Rect, StatusLine, Ui, ViewId};
use owlosui_dos::{dos_date, file_attr, init, read_file, run, ticks, Program};

// The program's own command numbers; 0 means none.
const CM_NEW: u16 = 1;
const CM_EXIT: u16 = 2;
const CM_OPEN: u16 = 3;
const CM_CALC: u16 = 10;
const CM_CALENDAR: u16 = 11;
const CM_ASCII: u16 = 12;
const CM_PUZZLE: u16 = 13;
const CM_NEXT: u16 = 30;
const CM_ZOOM: u16 = 31;
const CM_CLOSE: u16 = 32;
const CM_CASCADE: u16 = 33;
const CM_TILE: u16 = 34;
const CM_PREVIOUS: u16 = 35;
const CM_LIST: u16 = 36;
const CM_ABOUT: u16 = 40;
const CM_HELP: u16 = 41;
const CM_DISMISS: u16 = 62;
const CM_FILE_OPEN: u16 = 63; // the Open dialog's buttons
const CM_OPEN_CANCEL: u16 = 64;
// the calculator's keys
const CM_DIGIT: u16 = 100; // 100..109: 0..9
const CM_ADD: u16 = 110;
const CM_SUB: u16 = 111;
const CM_MUL: u16 = 112;
const CM_DIV: u16 = 113;
const CM_EQUALS: u16 = 114;
const CM_CLEAR: u16 = 115;
const CM_NEGATE: u16 = 116;
// the calendar's
const CM_MONTH_BACK: u16 = 160;
const CM_MONTH_ON: u16 = 161;
const CM_TODAY: u16 = 162;
// the puzzle's
const CM_SCRAMBLE: u16 = 180;
const CM_UP: u16 = 181;
const CM_DOWN: u16 = 182;
const CM_LEFT: u16 = 183;
const CM_RIGHT: u16 = 184;

const MONTHS: [&str; 12] = [
    "January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November",
    "December",
];
const HEX: &[u8] = b"0123456789ABCDEF";

#[derive(Default)]
struct Calc {
    win: Option<ViewId>,
    display: Option<ViewId>,
    acc: i32,
    entry: i32,
    op: u16,
    typing: bool,
    error: bool,
}

#[derive(Default)]
struct Calendar {
    win: Option<ViewId>,
    title: Option<ViewId>,
    canvas: Option<ViewId>,
    year: i32,
    month: i32,
    today: (i32, i32, i32),
}

#[derive(Default)]
struct Ascii {
    win: Option<ViewId>,
    canvas: Option<ViewId>,
    info: Option<ViewId>,
}

#[derive(Default)]
struct Puzzle {
    win: Option<ViewId>,
    canvas: Option<ViewId>,
    moves_text: Option<ViewId>,
    board: [u8; 16], // tiles 1..15, 0 the hole
    moves: u32,
}

#[derive(Default)]
struct Open {
    dialog: Option<ViewId>,
    panel: Option<ViewId>,
    dir: String, // the folder the panel shows
}

struct Demo {
    ui: Ui,
    box_: Option<ViewId>,
    editors: i16,
    calc: Calc,
    cal: Calendar,
    ascii: Ascii,
    puz: Puzzle,
    open: Open,
}

impl Program for Demo {
    fn ui(&mut self) -> &mut Ui {
        &mut self.ui
    }

    /// Every command a key or click caused, then what is not a command: a
    /// click on a canvas, a name entered in the file panel.
    fn after_input(&mut self) -> bool {
        while let Some(cmd) = self.ui.take_pressed() {
            if !self.command(cmd) {
                return false;
            }
        }
        while let Some(cmd) = self.ui.take_command() {
            if !self.command(cmd) {
                return false;
            }
        }
        self.ascii_poll();
        self.puzzle_poll();
        self.open_poll();
        true
    }
}

impl Demo {
    fn new() -> Demo {
        let mut d = Demo {
            ui: Ui::new(80, 25),
            box_: None,
            editors: 0,
            calc: Calc::default(),
            cal: Calendar::default(),
            ascii: Ascii::default(),
            puz: Puzzle::default(),
            open: Open::default(),
        };
        d.build_bars();
        d.about(); // say hello
        d
    }

    // ------------------------------------------------ the bars, the commands

    fn build_bars(&mut self) {
        let menu = MenuBar::new(vec![
            MenuItem::sub(
                "~F~ile",
                vec![
                    MenuItem::new("~N~ew", "", CM_NEW).hint("An editor window of its own"),
                    MenuItem::new("~O~pen...", "F3", CM_OPEN).hint("A file from the disk, in a viewer"),
                    MenuItem::line(),
                    MenuItem::new("E~x~it", "Alt+X", CM_EXIT).hint("Leave the program"),
                ],
            ),
            MenuItem::sub(
                "~T~ools",
                vec![
                    MenuItem::new("~C~alculator", "", CM_CALC).hint("Add, take, times, share: type or click"),
                    MenuItem::new("Ca~l~endar", "", CM_CALENDAR).hint("A month at a time; PgUp and PgDn page"),
                    MenuItem::new("~A~SCII table", "", CM_ASCII).hint("Every glyph of the font; click one"),
                    MenuItem::new("~P~uzzle", "", CM_PUZZLE).hint("The fifteen puzzle: click a tile, or arrows"),
                ],
            ),
            MenuItem::sub(
                "~W~indow",
                vec![
                    MenuItem::new("~Z~oom", "F5", CM_ZOOM).hint("The window fills the desktop, or goes back"),
                    MenuItem::new("~N~ext", "F6", CM_NEXT).hint("The front window goes to the back"),
                    MenuItem::new("~P~revious", "Shift+F6", CM_PREVIOUS)
                        .hint("The window at the back comes to the front"),
                    MenuItem::new("~C~lose", "Alt+F3", CM_CLOSE).hint("Close the front window"),
                    MenuItem::new("~L~ist...", "Alt+0", CM_LIST).hint("Every window by number"),
                    MenuItem::line(),
                    MenuItem::new("C~a~scade", "", CM_CASCADE).hint("The windows along the diagonal"),
                    MenuItem::new("~T~ile", "", CM_TILE).hint("The windows share the desktop"),
                ],
            ),
            MenuItem::sub(
                "~H~elp",
                vec![MenuItem::new("~A~bout", "", CM_ABOUT).hint("What this program is and what draws it")],
            ),
        ]);
        let root = self.ui.root();
        self.ui.insert(root, Rect::new(0, 0, 80, 1), owlosui_core::Kind::MenuBar(menu));
        let status = StatusLine::new(vec![
            item("~F1~ Help", CM_HELP, key(KeyCode::F(1))),
            item("~F3~ Open", CM_OPEN, key(KeyCode::F(3))),
            item("~F5~ Zoom", CM_ZOOM, key(KeyCode::F(5))),
            item("~F6~ Next", CM_NEXT, key(KeyCode::F(6))),
            item("~Alt-F3~ Close", CM_CLOSE, alt(KeyCode::F(3))),
            item("~Alt-X~ Exit", CM_EXIT, alt(KeyCode::Char('x'))),
        ]);
        self.ui.insert(root, Rect::new(0, 24, 80, 1), owlosui_core::Kind::Status(status));
    }

    /// One command. False means leave.
    fn command(&mut self, cmd: u16) -> bool {
        match cmd {
            CM_DIGIT..=CM_NEGATE => self.calc_key(cmd),
            CM_MONTH_BACK..=CM_TODAY => self.calendar_key(cmd),
            CM_SCRAMBLE..=CM_RIGHT => self.puzzle_key(cmd),
            CM_EXIT => return false,
            CM_NEW => self.editor_new(),
            CM_OPEN => self.open_show(),
            CM_FILE_OPEN => self.open_button(),
            CM_OPEN_CANCEL => self.open_close(),
            CM_CALC => self.calc_show(),
            CM_CALENDAR => self.calendar_show(),
            CM_ASCII => self.ascii_show(),
            CM_PUZZLE => self.puzzle_show(),
            CM_ABOUT | CM_HELP => self.about(),
            CM_DISMISS => self.box_close(),
            CM_NEXT => self.ui.cycle_windows(),
            CM_PREVIOUS => self.ui.cycle_windows_back(),
            CM_ZOOM => {
                if let Some(a) = self.ui.active_window() {
                    self.ui.toggle_zoom(a);
                }
            }
            CM_CLOSE => self.window_close(),
            CM_CASCADE => self.ui.cascade(),
            CM_TILE => self.ui.tile(),
            CM_LIST => self.ui.window_list(),
            _ => {}
        }
        true
    }

    /// The front window is being closed: whoever owns it forgets it.
    fn window_close(&mut self) {
        let Some(a) = self.ui.active_window() else { return };
        for w in [
            &mut self.calc.win,
            &mut self.cal.win,
            &mut self.ascii.win,
            &mut self.puz.win,
            &mut self.box_,
            &mut self.open.dialog,
        ] {
            if *w == Some(a) {
                *w = None;
            }
        }
        self.ui.close(a);
    }

    // ------------------------------------------ about, and the one message box

    fn box_close(&mut self) {
        if let Some(b) = self.box_.take() {
            self.ui.close(b);
        }
    }

    fn about(&mut self) {
        self.box_close();
        self.box_ = Some(kit::message_box(
            &mut self.ui,
            "OWLOS UI Demo",
            "A Rust program on DOS. The OWLOSUI core is linked into it and draws every window - the \
             same core that answers INT 60h for the assembler, C and Pascal demos.",
            &[("~O~K", CM_DISMISS, kit::DEFAULT)],
        ));
    }

    // ---------------------------------------- File > New: an editor window

    fn editor_new(&mut self) {
        self.editors += 1;
        let n = self.editors & 7;
        let title = format!("UNTITLED{}.TXT", self.editors);
        let w = kit::window(&mut self.ui, &title, 2 + n, 1 + n, 56, 14, kit::BLUE, CM_CLOSE);
        let text = kit::lines_of(b"Type here. Shift and the arrows select, Ctrl+Z takes it back.");
        kit::editor(&mut self.ui, w, text, false);
    }

    // --------------------------------- File > Open: the folder read through DOS

    fn open_show(&mut self) {
        if let Some(d) = self.open.dialog {
            self.ui.activate(d);
            return;
        }
        if self.open.dir.is_empty() {
            self.open.dir = owlosui_dos::current_dir();
        }
        let d = kit::window(&mut self.ui, "Open", -1, -1, 70, 20, kit::DIALOG | kit::MODAL, CM_OPEN_CANCEL);
        let dir = self.open.dir.clone();
        self.open.panel = Some(kit::files(&mut self.ui, d, 1, &dir)); // a row of air above it
        kit::buttons(&mut self.ui, d, &[("~O~pen", CM_FILE_OPEN, kit::DEFAULT), ("~C~ancel", CM_OPEN_CANCEL, kit::CANCEL)]);
        self.open.dialog = Some(d);
    }

    fn open_close(&mut self) {
        if let Some(d) = self.open.dialog.take() {
            self.ui.close(d);
        }
        self.open.panel = None;
    }

    /// A name made into a whole path: a drive's path as it is, anything
    /// else from the folder the panel shows.
    fn join(&self, name: &str) -> String {
        let b = name.as_bytes();
        if b.len() >= 2 && b[1] == b':' {
            return String::from(name);
        }
        if b.first() == Some(&b'\\') {
            return format!("{}{}", &self.open.dir[..2], name);
        }
        if self.open.dir.ends_with('\\') {
            format!("{}{}", self.open.dir, name)
        } else {
            format!("{}\\{}", self.open.dir, name)
        }
    }

    /// Everything before the last backslash: C:\A\B is C:\A, C:\A is C:\.
    fn folder_of(path: &str) -> String {
        match path.rfind('\\') {
            Some(2) => String::from(&path[..3]),
            Some(i) => String::from(&path[..i]),
            None => String::from(path),
        }
    }

    fn has_mask(s: &str) -> bool {
        s.contains('*') || s.contains('?')
    }

    /// The panel shows another folder - or says why it cannot. A path with
    /// a mask on the end, as the path line shows it, means its folder.
    fn go_to(&mut self, path: &str) {
        let Some(panel) = self.open.panel else { return };
        let dir = if Self::has_mask(path) { Self::folder_of(path) } else { String::from(path) };
        if kit::set_files(&mut self.ui, panel, &dir) {
            self.open.dir = dir;
        } else {
            kit::files_error(&mut self.ui, panel, &format!("Folder not found: {}", dir));
        }
    }

    /// A file into a viewer window of its own: its first 8 KB, read by DOS.
    fn view_file(&mut self, path: &str) -> bool {
        let Some((bytes, size)) = read_file(path, 8192) else { return false };
        let name = &path[path.rfind('\\').map_or(0, |i| i + 1)..];
        let title = if size as usize > bytes.len() {
            format!("{} - the first {} bytes", name, bytes.len())
        } else {
            String::from(name)
        };
        let w = kit::window(&mut self.ui, &title, 2, 2, 76, 20, kit::BLUE, CM_CLOSE);
        kit::editor(&mut self.ui, w, kit::lines_of(&bytes), true); // read-only: a viewer
        true
    }

    /// A name entered in the panel, or its path typed: a folder is walked
    /// into, a file is opened and the dialog closes.
    fn chosen(&mut self, name: &str) {
        let Some(panel) = self.open.panel else { return };
        if name == ".." {
            let up = Self::folder_of(&self.open.dir);
            self.go_to(&up);
            return;
        }
        let full = self.join(name);
        match file_attr(&full) {
            None => kit::files_error(&mut self.ui, panel, &format!("Not found: {}", full)),
            Some(a) if a & 0x10 != 0 => self.go_to(&full),
            Some(_) => {
                if self.view_file(&full) {
                    self.open_close();
                } else {
                    kit::files_error(&mut self.ui, panel, &format!("Cannot read: {}", full));
                }
            }
        }
    }

    fn open_poll(&mut self) {
        let Some(panel) = self.open.panel else { return };
        match kit::take_files(&mut self.ui, panel) {
            (1, name) => self.chosen(&name),
            (2, typed) => {
                let full = self.join(&typed);
                if Self::has_mask(&full) {
                    self.go_to(&full)
                } else {
                    self.chosen(&full)
                }
            }
            _ => {}
        }
    }

    /// The Open button: whatever is under the cursor.
    fn open_button(&mut self) {
        let Some(panel) = self.open.panel else { return };
        if let Some(name) = kit::current_name(&mut self.ui, panel) {
            self.chosen(&name);
        }
    }

    // ------------------- the calculator: a pocket calculator's arithmetic

    fn calc_show(&mut self) {
        if let Some(w) = self.calc.win {
            self.ui.activate(w);
            return;
        }
        let ui = &mut self.ui;
        let w = kit::window(ui, "Calculator", -1, -1, 46, 13, kit::DIALOG, CM_CLOSE);
        // The display is a canvas one line high, so the number can stand at
        // its right-hand end - a static line would fold the spaces away.
        self.calc.display = Some(kit::canvas(ui, w, 2, 1, 40, 1));
        // The keypad: four placed rows, labels padded to three so the
        // columns line up; a keypad is for the mouse and for typing.
        let d = |n: u16| CM_DIGIT + n;
        kit::button_row(ui, w, 2, 3, &[(" 7 ", d(7), 0), (" 8 ", d(8), 0), (" 9 ", d(9), 0), (" / ", CM_DIV, kit::ACCENT), (" C ", CM_CLEAR, kit::DANGER)]);
        kit::button_row(ui, w, 2, 5, &[(" 4 ", d(4), 0), (" 5 ", d(5), 0), (" 6 ", d(6), 0), (" * ", CM_MUL, kit::ACCENT)]);
        kit::button_row(ui, w, 2, 7, &[(" 1 ", d(1), 0), (" 2 ", d(2), 0), (" 3 ", d(3), 0), (" - ", CM_SUB, kit::ACCENT)]);
        kit::button_row(ui, w, 2, 9, &[(" 0 ", d(0), 0), ("+/-", CM_NEGATE, kit::ACCENT), (" = ", CM_EQUALS, kit::ACCENT | kit::DEFAULT), (" + ", CM_ADD, kit::ACCENT)]);
        // The keys the window carries: while it is in front, a digit or an
        // operator typed is its command. No labels: the status line keeps
        // its own words. Enter is the default button, =.
        let mut keys: Vec<_> = (0..10u16).map(|i| item("", d(i), key(KeyCode::Char((b'0' + i as u8) as char)))).collect();
        for (c, cmd) in [('+', CM_ADD), ('-', CM_SUB), ('*', CM_MUL), ('/', CM_DIV), ('=', CM_EQUALS), ('c', CM_CLEAR)] {
            keys.push(item("", cmd, key(KeyCode::Char(c))));
        }
        keys.push(item("", CM_CLEAR, key(KeyCode::Esc)));
        ui.set_window_status(w, keys);
        self.calc.win = Some(w);
        self.calc_key(CM_CLEAR);
    }

    fn calc_print(&mut self, value: i32) {
        let Some(display) = self.calc.display else { return };
        let text = if self.calc.error { String::from("Error") } else { format!("{}", value) };
        let pad = 40 - text.len();
        let cells: Vec<Cell> = (0..40).map(|i| Cell::new(if i < pad { b' ' } else { text.as_bytes()[i - pad] }, 0x1F)).collect();
        kit::blit(&mut self.ui, display, 40, 1, &cells);
    }

    /// acc = acc (op) entry; an overflow or a division by nought is an error.
    fn calc_apply(&mut self) {
        let c = &mut self.calc;
        let r = if c.op == 0 {
            Some(c.entry) // nothing waiting: the number is the total
        } else if !c.typing {
            Some(c.acc) // two operators in a row: the second wins
        } else {
            match c.op {
                CM_ADD => c.acc.checked_add(c.entry),
                CM_SUB => c.acc.checked_sub(c.entry),
                CM_MUL => c.acc.checked_mul(c.entry),
                _ => c.acc.checked_div(c.entry),
            }
        };
        match r {
            Some(v) => c.acc = v,
            None => c.error = true,
        }
        c.typing = false;
    }

    fn calc_key(&mut self, cmd: u16) {
        if self.calc.win.is_none() {
            return;
        }
        if cmd == CM_CLEAR {
            self.calc = Calc { win: self.calc.win, display: self.calc.display, ..Calc::default() };
            self.calc_print(0);
            return;
        }
        if self.calc.error {
            return; // after an error only C does anything
        }
        let c = &mut self.calc;
        let shown = if cmd < CM_ADD {
            let digit = (cmd - CM_DIGIT) as i32;
            if !c.typing {
                c.entry = 0;
                c.typing = true;
            }
            if (-99_999_999..=99_999_999).contains(&c.entry) {
                c.entry = if c.entry < 0 { c.entry * 10 - digit } else { c.entry * 10 + digit };
            }
            c.entry
        } else if cmd == CM_EQUALS {
            self.calc_apply();
            let c = &mut self.calc;
            c.op = 0;
            c.entry = c.acc;
            c.acc
        } else if cmd == CM_NEGATE {
            if c.typing {
                c.entry = c.entry.wrapping_neg();
                c.entry
            } else {
                c.acc = c.acc.wrapping_neg();
                c.entry = c.acc;
                c.acc
            }
        } else {
            // + - * /: what waited is done, and this one waits
            self.calc_apply();
            self.calc.op = cmd;
            self.calc.acc
        };
        self.calc_print(shown);
    }

    // ------------------------------------------------------------ the calendar

    fn calendar_show(&mut self) {
        if let Some(w) = self.cal.win {
            self.ui.activate(w);
            return;
        }
        let (y, m, d) = dos_date();
        self.cal.today = (y as i32, m as i32, d as i32);
        self.cal.year = y as i32;
        self.cal.month = m as i32;
        let ui = &mut self.ui;
        let w = kit::window(ui, "Calendar", -1, -1, 40, 15, kit::DIALOG, CM_CLOSE);
        self.cal.title = Some(kit::words(ui, w, 9, 1, 22, 1, " "));
        self.cal.canvas = Some(kit::canvas(ui, w, 8, 3, 21, 7));
        kit::buttons(ui, w, &[("< ~B~ack", CM_MONTH_BACK, 0), ("~T~oday", CM_TODAY, kit::DEFAULT), ("~O~n >", CM_MONTH_ON, 0)]);
        ui.set_window_status(
            w,
            vec![
                item("", CM_MONTH_BACK, key(KeyCode::PageUp)),
                item("", CM_MONTH_ON, key(KeyCode::PageDown)),
                item("", CM_TODAY, key(KeyCode::Home)),
            ],
        );
        self.cal.win = Some(w);
        self.calendar_draw();
    }

    /// 0 Sunday .. 6 Saturday. Sakamoto's: January and February counted in
    /// the year before.
    fn day_of_week(mut y: i32, m: i32, d: i32) -> i32 {
        const T: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
        if m < 3 {
            y -= 1;
        }
        (y + y / 4 - y / 100 + y / 400 + T[(m - 1) as usize] + d) % 7
    }

    fn days_in_month(y: i32, m: i32) -> i32 {
        const DAYS: [i32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
        DAYS[(m - 1) as usize] + (m == 2 && leap) as i32
    }

    /// The month into the canvas: a row of day names, then up to six weeks.
    fn calendar_draw(&mut self) {
        let c = &self.cal;
        let (Some(title), Some(canvas)) = (c.title, c.canvas) else { return };
        let mut cells = vec![Cell::new(b' ', 0x70); 21 * 7]; // black on grey
        for (i, &b) in b"Su Mo Tu We Th Fr Sa ".iter().enumerate() {
            cells[i] = Cell::new(b, 0x71);
        }
        let first = Self::day_of_week(c.year, c.month, 1);
        for d in 1..=Self::days_in_month(c.year, c.month) {
            let row = 1 + (first + d - 1) / 7;
            let col = ((first + d - 1) % 7) * 3;
            let mut colour = if col == 0 { 0x74 } else { 0x70 }; // Sundays red
            if (c.year, c.month, d) == c.today {
                colour = 0x2F; // today white on green
            }
            let at = (row * 21 + col) as usize;
            cells[at] = Cell::new(if d < 10 { b' ' } else { b'0' + (d / 10) as u8 }, colour);
            cells[at + 1] = Cell::new(b'0' + (d % 10) as u8, colour);
        }
        let text = format!("{} {}", MONTHS[(c.month - 1) as usize], c.year);
        kit::set_words(&mut self.ui, title, &text);
        kit::blit(&mut self.ui, canvas, 21, 7, &cells);
    }

    fn calendar_key(&mut self, cmd: u16) {
        if self.cal.win.is_none() {
            return;
        }
        let c = &mut self.cal;
        match cmd {
            CM_TODAY => (c.year, c.month) = (c.today.0, c.today.1),
            CM_MONTH_BACK => {
                c.month -= 1;
                if c.month == 0 {
                    c.month = 12;
                    c.year -= 1;
                }
            }
            _ => {
                c.month += 1;
                if c.month == 13 {
                    c.month = 1;
                    c.year += 1;
                }
            }
        }
        self.calendar_draw();
    }

    // --------------------------------------------------------- the ASCII table

    fn ascii_show(&mut self) {
        if let Some(w) = self.ascii.win {
            self.ui.activate(w);
            return;
        }
        let ui = &mut self.ui;
        let w = kit::window(ui, "ASCII table", -1, -1, 39, 21, kit::DIALOG, CM_CLOSE);
        let canvas = kit::canvas(ui, w, 1, 1, 35, 17);
        self.ascii.info = Some(kit::words(ui, w, 2, 18, 33, 1, "Click a glyph"));
        // 35 by 17: the column numbers across the top, the row numbers down
        // the side, and every glyph two columns apart. On DOS a cell holds
        // the glyph's number itself: the card draws it.
        let mut cells = vec![Cell::new(b' ', 0x70); 35 * 17];
        for i in 0..16 {
            cells[3 + 2 * i] = Cell::new(HEX[i], 0x74);
        }
        for r in 0..16 {
            let row = (r + 1) * 35;
            cells[row] = Cell::new(HEX[r], 0x74);
            cells[row + 1] = Cell::new(b'0', 0x74);
            cells[row + 2] = Cell::new(b' ', 0x74);
            for i in 0..16 {
                cells[row + 3 + 2 * i] = Cell::new((r * 16 + i) as u8, 0x1E);
                cells[row + 4 + 2 * i] = Cell::new(b' ', 0x1E);
            }
        }
        kit::blit(ui, canvas, 35, 17, &cells);
        self.ascii.canvas = Some(canvas);
        self.ascii.win = Some(w);
    }

    /// A click on the table: which glyph, in every base.
    fn ascii_poll(&mut self) {
        let (Some(canvas), Some(info)) = (self.ascii.canvas, self.ascii.info) else { return };
        if self.ascii.win.is_none() {
            return;
        }
        let Some((x, y)) = kit::clicked(&mut self.ui, canvas) else { return };
        if x < 3 || (x - 3) % 2 != 0 || !(1..=16).contains(&y) {
            return;
        }
        let code = ((y - 1) * 16 + (x - 3) / 2) as u8;
        // The glyph itself goes in the line as its own number, as every
        // character of a core string is.
        let text = format!("dec {}  hex {:02X}  {}", code, code, code as char);
        kit::set_words(&mut self.ui, info, &text);
    }

    // ----------------------------------------------------- the fifteen puzzle

    fn puzzle_show(&mut self) {
        if let Some(w) = self.puz.win {
            self.ui.activate(w);
            return;
        }
        let ui = &mut self.ui;
        let w = kit::window(ui, "Puzzle", -1, -1, 30, 18, kit::DIALOG, CM_CLOSE);
        self.puz.canvas = Some(kit::canvas(ui, w, 3, 1, 24, 12));
        self.puz.moves_text = Some(kit::words(ui, w, 3, 13, 24, 1, " "));
        kit::buttons(ui, w, &[("~S~cramble", CM_SCRAMBLE, kit::DEFAULT)]);
        ui.set_window_status(
            w,
            vec![
                item("", CM_UP, key(KeyCode::Up)),
                item("", CM_DOWN, key(KeyCode::Down)),
                item("", CM_LEFT, key(KeyCode::Left)),
                item("", CM_RIGHT, key(KeyCode::Right)),
            ],
        );
        self.puz.win = Some(w);
        self.puzzle_key(CM_SCRAMBLE);
    }

    fn hole(&self) -> usize {
        self.puz.board.iter().position(|&t| t == 0).unwrap_or(15)
    }

    /// A tile next to the hole slides into it.
    fn slide(&mut self, t: usize) -> bool {
        let h = self.hole();
        let beside = (t as i32 - h as i32).abs() == 1 && t / 4 == h / 4;
        let above_below = (t as i32 - h as i32).abs() == 4;
        if t > 15 || !(beside || above_below) {
            return false;
        }
        self.puz.board.swap(h, t);
        self.puz.moves += 1;
        true
    }

    /// An arrow: the tile beside the hole on the far side moves into it.
    fn arrow(&mut self, cmd: u16) {
        let h = self.hole();
        match cmd {
            CM_UP if h < 12 => self.slide(h + 4),
            CM_DOWN if h >= 4 => self.slide(h - 4),
            CM_LEFT if h % 4 != 3 => self.slide(h + 1),
            CM_RIGHT if h % 4 != 0 => self.slide(h - 1),
            _ => false,
        };
    }

    /// Solved, then three hundred random slides: every scramble can be undone.
    fn scramble(&mut self) {
        for i in 0..15 {
            self.puz.board[i] = i as u8 + 1;
        }
        self.puz.board[15] = 0;
        let mut seed = ticks() as u16; // the BIOS clock
        for _ in 0..300 {
            seed = seed.wrapping_mul(25173).wrapping_add(13849);
            self.arrow(CM_UP + ((seed >> 8) & 3));
        }
        self.puz.moves = 0;
    }

    /// A tile is five by two in blue, with a column and a row of the
    /// window's grey between tiles.
    fn puzzle_draw(&mut self) {
        let (Some(canvas), Some(text)) = (self.puz.canvas, self.puz.moves_text) else { return };
        let mut cells = Vec::with_capacity(24 * 12);
        for y in 0..12 {
            for x in 0..24 {
                let t = self.puz.board[(y / 3) * 4 + x / 6];
                let (dx, dy) = (x % 6, y % 3);
                cells.push(if dx == 5 || dy == 2 || t == 0 {
                    Cell::new(b' ', 0x70)
                } else if dy == 1 && dx == 2 && t >= 10 {
                    Cell::new(b'1', 0x1F)
                } else if dy == 1 && dx == 3 {
                    Cell::new(b'0' + t % 10, 0x1F)
                } else {
                    Cell::new(b' ', 0x1F)
                });
            }
        }
        kit::blit(&mut self.ui, canvas, 24, 12, &cells);
        let solved = (0..15).all(|i| self.puz.board[i] == i as u8 + 1);
        let words = if solved { format!("Solved! Moves: {}", self.puz.moves) } else { format!("Moves: {}", self.puz.moves) };
        kit::set_words(&mut self.ui, text, &words);
    }

    fn puzzle_key(&mut self, cmd: u16) {
        if self.puz.win.is_none() {
            return;
        }
        if cmd == CM_SCRAMBLE {
            self.scramble();
        } else {
            self.arrow(cmd);
        }
        self.puzzle_draw();
    }

    /// A click on the board: the tile under it, slid if it can go.
    fn puzzle_poll(&mut self) {
        let Some(canvas) = self.puz.canvas else { return };
        if self.puz.win.is_none() {
            return;
        }
        if let Some((x, y)) = kit::clicked(&mut self.ui, canvas) {
            if self.slide((y as usize / 3) * 4 + x as usize / 6) {
                self.puzzle_draw();
            }
        }
    }
}

/// Where the loader jumps: `base` is where it put this program.
#[no_mangle]
#[link_section = ".text.start"]
pub extern "C" fn _start(base: u32) -> ! {
    init(base);
    run(&mut Demo::new())
}
