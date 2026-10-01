//! The application inside the module: every part of the kit, in one
//! program, the way the terminal demo and the C# demo are - but built
//! against the core alone, because there is no file system to read and
//! no console to ask. What the page shows is what this puts on the
//! desktop; the page has no idea what a window is.

// Shared with the DOS build (`Examples/DOS/Rust`, which is `no_std`): the names
// `std` would give for free have to be asked for from `alloc`.
#[allow(unused_imports)]
use alloc::{format, string::{String, ToString}, vec, vec::Vec};
use owlosui_core::{
    ButtonRow, Cluster, Dock, Html, InputLine, Key, KeyCode, Kind, ListBox, MenuBar, MenuItem, Mods, PushButton,
    Rect, StaticText, StatusItem, StatusLine, TextView, Ui, ViewId, WinPalette, Window,
};

const CM_NEW: u16 = 1;
const CM_CONTROLS: u16 = 2;
const CM_HELP: u16 = 3;
const CM_ABOUT: u16 = 4;
const CM_EXIT: u16 = 5;
const CM_NEXT: u16 = 6;
const CM_ZOOM: u16 = 7;
const CM_CLOSE: u16 = 8;
const CM_CASCADE: u16 = 9;
const CM_TILE: u16 = 10;
const CM_LIST: u16 = 11;
const CM_OK: u16 = 20;
const CM_CANCEL: u16 = 21;
const CM_LEAVE: u16 = 22;
const CM_STAY: u16 = 23;
const CM_DISMISS: u16 = 24;

const SAMPLE: &str = "OWLOSUI in a browser.\n\
\n\
This window is an editor: type into it, select with Shift and the arrows,\n\
Ctrl+Z takes it back. Drag the title bar; the corner resizes; [\u{2191}] zooms.\n\
\n\
Every cell you see was decided by the same Rust core that draws the\n\
terminal version and sits behind the C# examples. The page only draws\n\
cells and sends keys and clicks - it does not know what a window is.\n\
\n\
F4 opens a dialog with every control in it. F1 is help. Alt+X leaves.\n";

const HELP: &str = "<h1>OWLOSUI</h1>\
<p>A text mode UI toolkit in the classic DOS style, with one portable core.</p>\
<p>The same core is meant to drive a terminal, a browser canvas, a native \
window and, eventually, DOS text memory. This page is the browser: a \
WebAssembly module holding the core and this program, and a canvas the \
page draws its cells on.</p>\
<h2>Keys</h2>\
<ul><li>F10 opens the menu, Alt and a letter opens one menu</li>\
<li>F5 zooms a window, F6 is the next one, Alt+F3 closes it</li>\
<li>Alt+1..9 bring a numbered window to the front, Alt+0 lists them</li>\
<li>Ctrl+F5 moves or resizes the window from the keyboard</li>\
<li>Alt+X leaves - and asks first if a window has been changed</li></ul>\
<p>A double click on a title bar zooms. A menu item's hint is shown on the \
status line while the cursor is on it.</p>";

pub struct App {
    pub ui: Ui,
    /// True once Exit was answered: the page stops drawing.
    pub done: bool,
    /// The question box, while it is up.
    box_: Option<ViewId>,
    /// The Controls dialog, while it is up.
    dialog: Option<ViewId>,
    windows: u16,
}

impl App {
    pub fn new(w: i16, h: i16) -> Self {
        let mut ui = Ui::new(w, h);
        let root = ui.root();
        ui.insert(
            root,
            Rect::new(0, 0, w, 1),
            Kind::MenuBar(MenuBar::new(vec![
                MenuItem::sub(
                    "~F~ile",
                    vec![
                        MenuItem::new("~N~ew", "", CM_NEW).hint("Another editor window, with the same words in it"),
                        MenuItem::new("~C~ontrols...", "F4", CM_CONTROLS).hint("A dialog with every control the kit has"),
                        MenuItem::line(),
                        MenuItem::new("E~x~it", "Alt+X", CM_EXIT).hint("Leave; asks first if something was changed"),
                    ],
                ),
                MenuItem::sub(
                    "~W~indow",
                    vec![
                        MenuItem::new("~Z~oom", "F5", CM_ZOOM).hint("The window fills the desktop, or goes back"),
                        MenuItem::new("~N~ext", "F6", CM_NEXT).hint("The front window goes to the back"),
                        MenuItem::new("~C~lose", "Alt+F3", CM_CLOSE).hint("Close the front window"),
                        MenuItem::new("~L~ist...", "Alt+0", CM_LIST).hint("Every window by number"),
                        MenuItem::line(),
                        MenuItem::new("C~a~scade", "", CM_CASCADE).hint("The windows along the diagonal"),
                        MenuItem::new("~T~ile", "", CM_TILE).hint("The windows share the desktop with no overlap"),
                    ],
                ),
                MenuItem::sub(
                    "~H~elp",
                    vec![
                        MenuItem::new("~I~ndex", "F1", CM_HELP).hint("What this is and which keys do what"),
                        MenuItem::new("~A~bout", "", CM_ABOUT).hint("The one-paragraph version"),
                    ],
                ),
            ])),
        );
        let f = |n: u8| Some(Key { code: KeyCode::F(n), mods: Mods::default() });
        let alt = |c: char| Some(Key { code: KeyCode::Char(c), mods: Mods { alt: true, ..Mods::default() } });
        let alt_f = |n: u8| Some(Key { code: KeyCode::F(n), mods: Mods { alt: true, ..Mods::default() } });
        ui.insert(
            root,
            Rect::new(0, h - 1, w, 1),
            Kind::Status(StatusLine::new(vec![
                StatusItem::new("~F1~ Help", f(1), CM_HELP),
                StatusItem::new("~F4~ Controls", f(4), CM_CONTROLS),
                StatusItem::new("~F5~ Zoom", f(5), CM_ZOOM),
                StatusItem::new("~F6~ Next", f(6), CM_NEXT),
                StatusItem::new("~Alt-F3~ Close", alt_f(3), CM_CLOSE),
                StatusItem::new("~Alt-X~ Exit", alt('x'), CM_EXIT),
            ])),
        );
        let mut app = App { ui, done: false, box_: None, dialog: None, windows: 0 };
        app.new_window();
        app
    }

    fn new_window(&mut self) {
        let work = self.ui.work_area();
        let n = self.windows as i16;
        self.windows += 1;
        let w = (work.w - 12).max(30);
        let h = (work.h - 6).max(8);
        let r = Rect::new(work.x + 2 + n, work.y + 1 + n, w, h);
        let root = self.ui.root();
        let mut win = Window::new(format!("SAMPLE{}.TXT", self.windows));
        win.close_cmd = CM_CLOSE;
        let wid = self.ui.insert(root, r, Kind::Window(win));
        let lines = SAMPLE.lines().map(owlosui_core::glyphs).collect();
        let text = self.ui.insert(wid, Rect::default(), Kind::Text(TextView::new(lines)));
        // Everything the editor has, on an Edit menu of its own while this
        // window is in front: the core runs it all, nothing comes back here.
        self.ui.set_editor(text, owlosui_core::edit::offer::ALL, 0);
        self.ui.focus_first();
    }

    fn open_controls(&mut self) {
        if let Some(d) = self.dialog {
            if self.ui.is_alive(d) {
                self.ui.activate(d);
                return;
            }
        }
        let work = self.ui.work_area();
        let (w, h) = (62i16, 20i16);
        let r = Rect::new(work.x + (work.w - w) / 2, work.y + (work.h - h) / 2, w, h);
        let mut win = Window::new("Controls");
        win.palette = WinPalette::Gray;
        win.modal = true;
        win.resizable = false;
        win.zoomable = false;
        win.close_cmd = CM_CANCEL;
        win.min_w = w;
        win.max_w = w;
        win.min_h = h;
        win.max_h = h;
        let root = self.ui.root();
        let wid = self.ui.insert(root, r, Kind::Window(win));
        let ui = &mut self.ui;
        let at = |ui: &mut Ui, x, y, w, h, kind| {
            let id = ui.insert(wid, Rect::new(x, y, w, h), kind);
            ui.set_dock(id, Dock::Manual);
            id
        };
        at(ui, 2, 1, 56, 1, Kind::Static(StaticText::new("Everything a dialog is made of. Tab walks them in order.")));
        let mut name = InputLine::new("Name:", "OWLOSUI");
        name.history = vec!["OWLOS".into(), "OWL FLY".into()];
        at(ui, 2, 3, 40, 1, Kind::Input(name));
        at(ui, 2, 5, 26, 3, Kind::Cluster(Cluster::checks(&["~S~ave on exit", "~B~ackup files", "~R~ead only"])));
        at(ui, 32, 5, 26, 3, Kind::Cluster(Cluster::radio(&["~T~ext", "~H~ex", "~A~uto"])));
        at(ui, 2, 9, 26, 6, Kind::List(ListBox::new(&["Terminal", "Browser", "Native window", "DOS", "Resident"])));
        at(
            ui,
            32,
            9,
            26,
            6,
            Kind::Static(StaticText::new(
                "The field above has a history: press Down in it. The list takes Insert marks. Enter is OK, Escape is Cancel.",
            )),
        );
        let row = ui.insert(
            wid,
            Rect::default(),
            Kind::Buttons(ButtonRow::new(vec![
                PushButton::new("~O~K", CM_OK).default(),
                PushButton::new("~C~ancel", CM_CANCEL).cancel(),
            ])),
        );
        ui.set_dock(row, Dock::BottomRight(w - 2, 2));
        ui.focus_first();
        self.dialog = Some(wid);
    }

    fn open_help(&mut self) {
        let root = self.ui.root();
        for w in self.ui.children(root).to_vec() {
            if let Some(first) = self.ui.children(w).first().copied() {
                if matches!(self.ui.kind(first), Kind::Html(_)) {
                    self.ui.activate(w);
                    return;
                }
            }
        }
        let work = self.ui.work_area();
        let (w, h) = ((work.w - 16).max(30), (work.h - 4).max(8));
        let r = Rect::new(work.x + 8, work.y + 2, w, h);
        let mut win = Window::new("Help");
        win.palette = WinPalette::Cyan;
        win.close_cmd = CM_CLOSE;
        let wid = self.ui.insert(root, r, Kind::Window(win));
        self.ui.insert(wid, Rect::default(), Kind::Html(Html::new(HELP)));
    }

    fn any_modified(&self) -> bool {
        let root = self.ui.root();
        self.ui.children(root).iter().any(|w| {
            self.ui.children(*w).iter().any(|c| matches!(self.ui.kind(*c), Kind::Text(t) if t.modified))
        })
    }

    fn close_box(&mut self) {
        if let Some(b) = self.box_.take() {
            if self.ui.is_alive(b) {
                self.ui.close(b);
            }
        }
    }

    fn close_dialog(&mut self) {
        if let Some(d) = self.dialog.take() {
            if self.ui.is_alive(d) {
                self.ui.close(d);
            }
        }
    }

    fn command(&mut self, cmd: u16) {
        match cmd {
            CM_NEW => self.new_window(),
            CM_CONTROLS => self.open_controls(),
            CM_HELP => self.open_help(),
            CM_ABOUT => {
                self.close_box();
                self.box_ = Some(self.ui.message_box(
                    "About",
                    "OWLOSUI: one portable core, drawn here by a page that only knows cells. \
                     The same core runs on a terminal and behind a pipe for C#.",
                    ButtonRow::new(vec![PushButton::new("~O~K", CM_DISMISS).default()]),
                ));
            }
            CM_EXIT => {
                if !self.any_modified() {
                    self.done = true;
                    return;
                }
                self.close_box();
                self.box_ = Some(self.ui.message_box(
                    "Confirm",
                    "A window has been changed. Leave anyway?",
                    ButtonRow::new(vec![
                        PushButton::new("~L~eave", CM_LEAVE),
                        PushButton::new("~S~tay", CM_STAY).default().cancel(),
                    ]),
                ));
            }
            CM_LEAVE => {
                self.close_box();
                self.done = true;
            }
            CM_STAY | CM_DISMISS => self.close_box(),
            CM_OK | CM_CANCEL => self.close_dialog(),
            CM_NEXT => self.ui.cycle_windows(),
            CM_ZOOM => {
                if let Some(a) = self.ui.active_window() {
                    self.ui.toggle_zoom(a);
                }
            }
            CM_CLOSE => {
                if let Some(a) = self.ui.active_window() {
                    if Some(a) == self.dialog {
                        self.close_dialog();
                    } else if Some(a) == self.box_ {
                        self.close_box();
                    } else {
                        self.ui.close(a);
                    }
                }
            }
            CM_CASCADE => self.ui.cascade(),
            CM_TILE => self.ui.tile(),
            CM_LIST => self.ui.window_list(),
            _ => {}
        }
    }

    /// After every event: whatever was pressed or chosen, done.
    pub fn after_input(&mut self) {
        while let Some(cmd) = self.ui.take_pressed() {
            self.command(cmd);
        }
        while let Some(cmd) = self.ui.take_command() {
            self.command(cmd);
        }
    }
}
