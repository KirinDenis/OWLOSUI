//! The first slice: a desktop, framed windows that move and resize, and a
//! scrolling viewer inside each one.
//!
//! Keys are bound here, in the application, not in the core. The core exposes
//! primitives — `toggle_zoom`, `cycle_windows`, `close` — and which key calls
//! which is a table. That is how Borland shipped four different keymaps for
//! one editor, and it is why we can ship an authentic one and a modern one
//! without touching a line of the toolkit.

mod compare;
mod dir;
mod help;
mod cp437;
mod scenes;
mod term;

use owlosui_core::{
    ButtonRow, Dock, Event, FileList, Html, Key, KeyCode, Kind, MenuBar, MenuItem, Rect, TextView,
    TreeNode, Ui, Window,
};

use term::Term;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if let Some(i) = args.iter().position(|a| a == "--dump") {
        return dump(args.get(i + 1).map(|s| s.as_str()));
    }

    // --match <scene> <reference.bin> [--rows a:b]
    //
    // Renders a scene we also built with the real Turbo Vision and says which
    // cells disagree. `--rows` exists because every reference screen carries a
    // menu bar on row 0 and a status line on row 24, and neither of those is a
    // view here yet; without it the report is mostly noise about work that has
    // not been done rather than work that is wrong.
    if let Some(i) = args.iter().position(|a| a == "--match") {
        let Some(scene) = args.get(i + 1) else {
            eprintln!("usage: --match <scene> <reference.bin> [--rows a:b]");
            return Ok(());
        };
        let Some(path) = args.get(i + 2) else {
            eprintln!("usage: --match <scene> <reference.bin> [--rows a:b]");
            return Ok(());
        };
        let rows = args
            .iter()
            .position(|a| a == "--rows")
            .and_then(|j| args.get(j + 1))
            .and_then(|s| s.split_once(':'))
            .and_then(|(a, b)| Some((a.parse().ok()?, b.parse().ok()?)));
        let code = compare::run(scene, path, compare::Options { rows })?;
        std::process::exit(code);
    }

    if args.iter().any(|a| a == "--keys") {
        return show_keys();
    }

    let (w, h) = Term::size()?;
    let mut term = Term::new()?;

    let (mut ui, mut buf) = build(w, h);

    loop {
        ui.draw(&mut buf);
        status_line(&mut buf);
        term.render(&buf, ui.cursor())?;

        // A menu item clicked with the mouse has just been drawn lit up. Hold
        // it there long enough to be seen, then act. The core has no clock;
        // waiting is the backend's business, and on DOS this would be the BIOS
        // tick and in a browser a timer.
        if ui.menu_pick_pending() {
            std::thread::sleep(std::time::Duration::from_millis(90));
            ui.complete_menu_pick();
            if let Some(cmd) = ui.take_command() {
                if !run_command(&mut ui, cmd) {
                    break;
                }
            }
            continue;
        }

        let Some(ev) = term.next_event()? else { continue };

        // Application-level keys get first refusal, then the tree.
        if let Event::Key(k) = ev {
            match k {
                Key {
                    code: KeyCode::Char('x'),
                    mods,
                } if mods.alt => break,
                Key {
                    code: KeyCode::F(5),
                    ..
                } => {
                    if let Some(a) = ui.active_window() {
                        ui.toggle_zoom(a);
                    }
                    continue;
                }
                Key {
                    code: KeyCode::F(1),
                    ..
                } => {
                    open_help(&mut ui);
                    continue;
                }
                Key {
                    code: KeyCode::F(4),
                    ..
                } => {
                    open_controls(&mut ui);
                    continue;
                }
                Key {
                    code: KeyCode::F(6),
                    ..
                } => {
                    ui.cycle_windows();
                    continue;
                }
                Key {
                    code: KeyCode::F(3),
                    mods,
                } if mods.alt => {
                    if let Some(a) = ui.active_window() {
                        ui.close(a);
                    }
                    continue;
                }
                _ => {}
            }
        }

        if let Event::Resize(nw, nh) = ev {
            buf.resize(nw, nh);
            ui.handle(Event::Resize(nw, nh - 1));
            continue;
        }

        ui.handle(ev);
        if let Some(cmd) = ui.take_pressed() {
            if !run_command(&mut ui, cmd) {
                break;
            }
        }
        if let Some(cmd) = ui.take_command() {
            if !run_command(&mut ui, cmd) {
                break;
            }
        }
        follow_links(&mut ui);
        follow_files(&mut ui);
        update_footers(&mut ui);
    }

    Ok(())
}

// Commands the menu hands back. Numbers, not closures: a callback would have
// to survive an interrupt or a WebAssembly boundary one day, and neither
// carries one.
const CM_HELP: u16 = 1;
const CM_ZOOM: u16 = 2;
const CM_NEXT: u16 = 3;
const CM_CLOSE: u16 = 4;
const CM_QUIT: u16 = 5;
const CM_CLASSIC: u16 = 6;
const CM_MODERN: u16 = 7;
// There is deliberately no command for the mask. It lives in the path line
// and nowhere else: a second way to do the same thing is a second place to
// look for it, and the menu item was so well hidden that even the person who
// asked for it could not find it.
const CM_FILES: u16 = 8;
const CM_OPEN: u16 = 20;
const CM_CANCEL: u16 = 21;
const CM_DEMO: u16 = 22;
const CM_DEMO_OK: u16 = 23;
const CM_DEMO_CANCEL: u16 = 24;

/// Print every key exactly as the terminal delivers it, until Esc.
///
/// There is no other honest way to find out whether Alt with a letter even
/// reaches us: it can be eaten by the terminal emulator, swallowed by the
/// window manager, or arrive as an escape prefix with no modifier flag at all,
/// and all three look identical from inside the program.
fn show_keys() -> std::io::Result<()> {
    use crossterm::event::{self as ct, KeyEventKind};
    use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

    println!("Press keys. Esc quits.");
    println!("Try Alt+F, Alt+W, F10 — and note what arrives.
");
    enable_raw_mode()?;
    loop {
        if let ct::Event::Key(k) = ct::read()? {
            if k.kind != KeyEventKind::Press {
                continue;
            }
            print!("  crossterm: {:?} + {:?}\r\n", k.code, k.modifiers);
            if k.code == ct::KeyCode::Esc {
                break;
            }
        }
    }
    disable_raw_mode()
}

fn build(w: i16, h: i16) -> (Ui, owlosui_core::Buffer) {
    let mut ui = Ui::new(w, h - 1); // last row is the status line placeholder
    let buf = owlosui_core::Buffer::new(w, h);
    let root = ui.root();

    open_file(&mut ui, root, Rect::new(2, 2, 58, 16), "core/src/ui.rs");
    open_file(&mut ui, root, Rect::new(14, 6, 56, 13), "core/src/views.rs");

    let bar = MenuBar::new(vec![
        MenuItem::sub(
            "~F~ile",
            vec![
                MenuItem::new("~O~pen", "F3", CM_FILES),
                MenuItem::new("~H~elp", "F1", CM_HELP),
                MenuItem::new("~C~ontrols", "F4", CM_DEMO),
                MenuItem::line(),
                MenuItem::new("E~x~it", "Alt+X", CM_QUIT),
            ],
        ),
        MenuItem::sub(
            "~W~indow",
            vec![
                MenuItem::new("~Z~oom", "F5", CM_ZOOM),
                MenuItem::new("~N~ext", "F6", CM_NEXT),
                MenuItem::new("~C~lose", "Alt+F3", CM_CLOSE),
            ],
        ),
        MenuItem::sub(
            "~K~eys",
            vec![
                MenuItem::new("~B~orland", "", CM_CLASSIC),
                MenuItem::new("~M~odern", "", CM_MODERN),
            ],
        ),
    ]);
    ui.insert(root, Rect::new(0, 0, w, 1), Kind::MenuBar(bar));

    (ui, buf)
}

/// Act on whatever the menu produced. The application decides what a command
/// means; the toolkit only says which one was chosen.
fn run_command(ui: &mut Ui, cmd: u16) -> bool {
    match cmd {
        CM_HELP => open_help(ui),
        CM_FILES => open_files(ui),
        CM_DEMO => open_controls(ui),
        CM_DEMO_OK | CM_DEMO_CANCEL => {
            if let Some(a) = ui.active_window() {
                ui.close(a);
            }
        }
        // What a real program does here is return the name to whoever asked
        // for it. This one puts it in the frame and shuts the dialog, which is
        // the same shape with nobody on the other end.
        CM_OPEN => {
            // Whatever the cursor is on, whether or not Enter was pressed on
            // it: the button means "this one", and the person has been looking
            // at it the whole time.
            let picked = ui.take_chosen().or_else(|| {
                let win = ui.active_window()?;
                let first = ui.children(win).first().copied()?;
                match ui.kind(first) {
                    Kind::Files(f) => f.selected().map(|e| e.name.clone()),
                    _ => None,
                }
            });
            let here = match ui.active_window().and_then(|w| ui.children(w).first().copied()) {
                Some(first) => match ui.kind(first) {
                    Kind::Files(f) => split_path(f.path_text()).0,
                    _ => ".".into(),
                },
                None => ".".into(),
            };
            if let Some(a) = ui.active_window() {
                ui.close(a);
            }
            if let Some(name) = picked {
                open_document(ui, &here.join(name));
            }
        }
        CM_CANCEL => {
            if let Some(a) = ui.active_window() {
                ui.close(a);
            }
        }
        CM_ZOOM => {
            if let Some(a) = ui.active_window() {
                ui.toggle_zoom(a)
            }
        }
        CM_NEXT => ui.cycle_windows(),
        CM_CLOSE => {
            if let Some(a) = ui.active_window() {
                ui.close(a)
            }
        }
        CM_CLASSIC => ui.keymap = owlosui_core::Keymap::Classic,
        CM_MODERN => ui.keymap = owlosui_core::Keymap::Modern,
        CM_QUIT => return false,
        _ => {}
    }
    true
}

/// Render one frame to stdout as plain text and exit.
///
/// The output medium is a grid of characters, so it is its own screenshot.
/// A layout bug is visible here without a terminal, without a human and
/// without a picture — which is what makes this both the test harness and,
/// later, the thing an MCP renderer hands to a model so it can check its own
/// work. The same trick is not available to anyone generating HTML.
fn dump(what: Option<&str>) -> std::io::Result<()> {
    use std::io::Write;
    let (mut ui, mut buf) = build(80, 25);
    match what {
        Some("help") => open_help(&mut ui),
        Some("menu") => ui.open_menu(0),
        Some("files") => open_files(&mut ui),
        Some("controls") => open_controls(&mut ui),
        _ => {}
    }
    ui.draw(&mut buf);
    status_line(&mut buf);

    let mut out = String::new();
    for y in 0..buf.height() {
        for x in 0..buf.width() {
            out.push(cp437::to_char(buf.get(x, y).map(|c| c.ch).unwrap_or(b' ')));
        }
        out.push('\n');
    }
    // Attributes of the last row, so a colour bug is visible here too — the
    // glyph picture alone cannot show one.
    out.push_str("\nlast row attrs: ");
    for x in 0..buf.width().min(24) {
        out.push_str(&format!("{:02X} ", buf.get(x, buf.height() - 1).unwrap().attr));
    }
    out.push('\n');
    std::io::stdout().write_all(out.as_bytes())
}

fn open_file(ui: &mut Ui, parent: owlosui_core::ViewId, rect: Rect, path: &str) {
    // Resolved against the crate, not the working directory: a demo that only
    // runs from one folder is a demo that makes a bad first impression.
    let full = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(path);
    let text = std::fs::read_to_string(&full)
        .unwrap_or_else(|e| format!("{path}\n\ncould not be read:\n{e}"));

    let mut win = Window::new(path);
    win.footer = String::from(" 1:1 ");
    win.number = Some(ui.children(parent).len() as u8 + 1);
    let wid = ui.insert(parent, rect, Kind::Window(win));

    // Encode once, here at the edge. Tabs are expanded to spaces for now;
    // whether the editor keeps them as characters is a decision we have not
    // taken yet, and guessing it in the loader would prejudge it.
    let lines: Vec<Vec<u8>> = text
        .lines()
        .map(|l| cp437::encode(&l.replace('\t', "    ")))
        .collect();

    // The size given here does not matter: the measure pass makes a window's
    // content fill its client area before every frame.
    ui.insert(wid, Rect::default(), Kind::Text(TextView::new(lines)));
}

/// Every remaining control in one dialog, so the whole set can be seen at
/// once — and, more to the point, so the one thing that is hard to get right
/// can be: that Tab walks all of them in the order the eye reads them.
fn open_controls(ui: &mut Ui) {
    use owlosui_core::{Cluster, InputLine, ListBox, StaticText};

    let work = ui.work_area();
    let (w, h) = (62i16, 20i16);
    let r = Rect::new(
        work.x + (work.w - w) / 2,
        work.y + (work.h - h) / 2,
        w,
        h,
    );

    let mut win = Window::new("Controls");
    win.palette = owlosui_core::WinPalette::Gray;
    win.number = Some(6);
    win.min_w = w;
    win.min_h = h;
    let wid = ui.insert(ui.root(), r, Kind::Window(win));

    // Placed by hand, which is what every Turbo Vision dialog did and what
    // `Dock::Manual` is for. Honest while the contents are known and fixed,
    // and the wrong tool the moment one of them has to grow.
    let at = |ui: &mut Ui, x, y, w, h, kind| {
        let id = ui.insert(wid, Rect::new(x, y, w, h), kind);
        ui.set_dock(id, Dock::Manual);
        id
    };

    at(
        ui,
        2,
        0,
        56,
        2,
        Kind::Static(StaticText::new(
            "Everything a dialog is made of. Tab walks them in the order              they are read.",
        )),
    );

    at(
        ui,
        2,
        3,
        40,
        1,
        Kind::Input(InputLine::new("Name:", "OWLOSUI")),
    );

    at(
        ui,
        2,
        5,
        26,
        3,
        Kind::Cluster(Cluster::checks(&[
            "~S~ave on exit",
            "~B~ackup files",
            "~R~ead only",
        ])),
    );
    at(
        ui,
        32,
        5,
        26,
        3,
        Kind::Cluster(Cluster::radio(&["~T~ext", "~H~ex", "~A~uto"])),
    );

    at(
        ui,
        2,
        9,
        16,
        6,
        Kind::List(ListBox::new(&[
            "Blue", "Cyan", "Grey", "Classic", "Modern", "Measured", "Invented",
        ])),
    );

    // A tree, which is the one control whose shape is not a rectangle of rows
    // and has to become one anyway.
    at(
        ui,
        19,
        9,
        20,
        6,
        Kind::Tree(owlosui_core::TreeView::new(vec![
            TreeNode::branch(
                "core",
                vec![
                    TreeNode::leaf("ui.rs"),
                    TreeNode::leaf("files.rs"),
                    TreeNode::branch("views", vec![TreeNode::leaf("window.rs")]),
                ],
            ),
            TreeNode::branch("console", vec![TreeNode::leaf("term.rs")]),
            TreeNode::leaf("README.md"),
        ])),
    );

    // The editor, because a dialog that cannot hold one is a dialog that
    // cannot ask for more than a line.
    // The memo: the editor in a box. `boxed` is what makes it look like
    // something to type into rather than like part of the dialog.
    let mut memo = TextView::new(vec![
        b"A memo is the editor in a box.".to_vec(),
        b"Type here: undo, selection and".to_vec(),
        b"the clipboard all work.".to_vec(),
    ]);
    memo.boxed = true;
    at(ui, 40, 9, 18, 6, Kind::Text(memo));

    let row = ui.insert(
        wid,
        Rect::default(),
        Kind::Buttons(owlosui_core::ButtonRow::ok_cancel(
            "~O~K",
            CM_DEMO_OK,
            CM_DEMO_CANCEL,
        )),
    );
    ui.set_dock(row, Dock::BottomRight(24, 2));
    ui.advance_focus(false);
}

/// Open a file: text in an editor, anything else in a hex view.
///
/// Which one is not a question anybody should be asked. A binary opened in an
/// editor is a screenful of rubbish and a stray keystroke away from being a
/// corrupted binary; the program can tell the difference and so it should.
fn open_document(ui: &mut Ui, path: &std::path::Path) {
    let Ok(bytes) = std::fs::read(path) else {
        return;
    };
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());

    let root = ui.root();
    let work = ui.work_area();
    let r = Rect::new(
        work.x + 2,
        work.y + 1,
        (work.w - 6).min(76),
        (work.h - 3).min(22),
    );

    let binary = owlosui_core::hex::looks_binary(&bytes);
    let mut win = Window::new(if binary {
        format!("{name}  [hex]")
    } else {
        name
    });
    win.number = Some(7);
    let wid = ui.insert(root, r, Kind::Window(win));

    if binary {
        ui.insert(
            wid,
            Rect::default(),
            Kind::Hex(owlosui_core::HexView::new(bytes)),
        );
    } else {
        let text = String::from_utf8_lossy(&bytes);
        let lines: Vec<Vec<u8>> = text
            .lines()
            .map(|l| cp437::encode(&l.replace('\t', "    ")))
            .collect();
        ui.insert(wid, Rect::default(), Kind::Text(TextView::new(lines)));
    }
}

/// Open the help browser, or bring it to the front if it is already up.
fn open_help(ui: &mut Ui) {
    let root = ui.root();
    for w in ui.children(root).to_vec() {
        if let Some(first) = ui.children(w).first().copied() {
            if matches!(ui.kind(first), Kind::Html(_)) {
                ui.activate(w);
                return;
            }
        }
    }
    let r = Rect::new(8, 2, 60, 18);
    let mut win = Window::new("Help");
    win.number = Some(9);
    // Help is cyan, frame included — that is a whole window family in Turbo
    // Vision, not a colour for the body.
    win.palette = owlosui_core::WinPalette::Cyan;
    let wid = ui.insert(root, r, Kind::Window(win));
    ui.insert(wid, Rect::default(), Kind::Html(Html::new(help::INDEX)));
}

/// The file panel, on the directory the program was started in.
fn open_files(ui: &mut Ui) {
    let root = ui.root();
    for w in ui.children(root).to_vec() {
        if let Some(first) = ui.children(w).first().copied() {
            if matches!(ui.kind(first), Kind::Files(_)) {
                ui.activate(w);
                return;
            }
        }
    }
    let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
    let entries = dir::read(&cwd).unwrap_or_default();
    let mut win = Window::new("Open");
    win.number = Some(8);
    // A dialog is the one kind of window that knows how small is too small:
    // below this its own contents stop making sense, and the toolkit has no
    // way to find that out for itself.
    win.min_w = 34;
    win.min_h = 12;
    // Grey, because it is a dialog. The blue field and the cyan list sitting
    // on it are what say "these are not part of the dialog" — and that is how
    // somebody knows at a glance that this is not another editor window.
    win.palette = owlosui_core::WinPalette::Gray;
    // Big and centred. A file dialog that opens small makes its first job -
    // showing you what is there - the first thing you have to fix.
    let screen = ui.rect(root);
    let w = (screen.w - 6).min(84).max(win.min_w);
    let h = (screen.h - 4).min(30).max(win.min_h);
    let r = Rect::new((screen.w - w) / 2, (screen.h - h) / 2, w, h);
    let wid = ui.insert(root, r, Kind::Window(win));
    let mut list = FileList::new(entries, "*.*");
    list.set_path(&join(&cwd, "*.*"));
    list.path.focused = true;
    list.focus = owlosui_core::files::Focus::Path;
    ui.insert(wid, Rect::default(), Kind::Files(list));

    // Enter on a file presses Open; Escape presses Cancel. The buttons are not
    // a second way to do it - they are the thing the keys are doing, which is
    // why they also say what will happen.
    let row = ui.insert(
        wid,
        Rect::default(),
        Kind::Buttons(ButtonRow::ok_cancel("~O~pen", CM_OPEN, CM_CANCEL)),
    );
    // Two rows, not one: a button casts a shadow one row down, and it has to
    // land on the dialog rather than on its bottom frame.
    // Positioned rather than docked: the buttons sit beside the information
    // pane, not below it, and a strip would have taken the whole width.
    ui.set_dock(row, Dock::BottomRight(24, 2));
}

fn join(dir: &std::path::Path, mask: &str) -> String {
    let d = dir.display().to_string();
    if d.ends_with(['\\', '/']) {
        format!("{d}{mask}")
    } else {
        format!("{d}{}{mask}", std::path::MAIN_SEPARATOR)
    }
}

/// Split `C:\Pascal\*.pas` into where to look and what to look for.
///
/// The last element is a mask if it has a wildcard in it, and a directory
/// otherwise. That is the rule every open dialog has used since DOS, and it is
/// why one line can do the work of two fields.
fn split_path(text: &str) -> (std::path::PathBuf, String) {
    let t = text.trim();
    let cut = t.rfind(['\\', '/']);
    match cut {
        Some(i) => {
            let (d, last) = (&t[..=i], &t[i + 1..]);
            if last.contains('*') || last.contains('?') {
                (std::path::PathBuf::from(d), last.to_string())
            } else {
                (std::path::PathBuf::from(t), "*.*".to_string())
            }
        }
        None => (std::path::PathBuf::from("."), t.to_string()),
    }
}

/// Everything the file panel asked the application to do.
///
/// Reading a directory is I/O, and I/O is never the view's. This is also where
/// the awkward cases live, and they are not edge cases: a directory changes
/// while a dialog is open more often than it does not, and a removable drive
/// that was there when the dialog opened may not be there when somebody
/// presses Enter.
fn follow_files(ui: &mut Ui) {
    let Some(win) = ui.active_window() else { return };
    let Some(first) = ui.children(win).first().copied() else {
        return;
    };

    // 1. The path line: somewhere the person typed.
    let typed = match ui.kind_mut(first) {
        Kind::Files(f) => f.pending_path.take(),
        _ => None,
    };
    if let Some(text) = typed {
        let (d, mask) = split_path(&text);
        match dir::read(&d) {
            Ok(entries) => {
                if let Kind::Files(f) = ui.kind_mut(first) {
                    f.set_mask(&mask);
                    f.set_entries(entries);
                    f.set_path(&join(&d, &mask));
                }
            }
            Err(e) => {
                // The old listing stays on screen. Emptying it would throw
                // away the one thing the person could still navigate from,
                // and they have a typo to fix, not a program to restart.
                if let Kind::Files(f) = ui.kind_mut(first) {
                    f.error = Some(match e.kind() {
                        std::io::ErrorKind::NotFound => {
                            format!("Folder not found: {}", d.display())
                        }
                        std::io::ErrorKind::PermissionDenied => {
                            format!("No access to {}", d.display())
                        }
                        _ => format!("Cannot read {}: {e}", d.display()),
                    });
                }
            }
        }
        return;
    }

    // 2. The list: a name somebody pressed Enter on.
    let Some(name) = ui.take_chosen() else { return };
    let (here, mask) = match ui.kind(first) {
        Kind::Files(f) => (split_path(f.path_text()).0, f.mask.clone()),
        _ => return,
    };

    let target = if name == ".." {
        here.parent().map(|p| p.to_path_buf())
    } else {
        let p = here.join(&name);
        p.is_dir().then_some(p)
    };

    let Some(target) = target else {
        // Not a directory. A real program opens the file here; this one says
        // which one it would have opened.
        if let Kind::Window(w) = ui.kind_mut(win) {
            w.footer = format!(" {name} ");
        }
        return;
    };

    match dir::read(&target) {
        Ok(entries) => {
            if let Kind::Files(f) = ui.kind_mut(first) {
                f.set_entries(entries);
                f.set_path(&join(&target, &mask));
            }
        }
        Err(e) => {
            // The directory was listed a moment ago and is not readable now —
            // ejected, unplugged, deleted by something else. Say so and stay
            // where we are.
            if let Kind::Files(f) = ui.kind_mut(first) {
                f.error = Some(format!("Cannot open {}: {e}", target.display()));
            }
        }
    }
}

/// The view cannot open anything; it only says where it was asked to go. This
/// is the half of the bargain that belongs to the application — here a lookup
/// table, elsewhere a file or a fetch.
fn follow_links(ui: &mut Ui) {
    let root = ui.root();
    for w in ui.children(root).to_vec() {
        let Some(first) = ui.children(w).first().copied() else {
            continue;
        };
        let Kind::Html(h) = ui.kind_mut(first) else {
            continue;
        };
        let Some(href) = h.pending.take() else { continue };
        match help::page(&href) {
            Some(src) => h.load(src),
            None => h.load(&format!(
                "<h1>Not found</h1><p>There is no page called <b>{href}</b> in this                  demonstration.</p><p><a href=\"index.htm\">Back</a></p>"
            )),
        }
    }
}

/// Keep the bottom-left frame slot showing the cursor position, the way every
/// Borland editor did.
fn update_footers(ui: &mut Ui) {
    let windows: Vec<_> = ui.children(ui.root()).to_vec();
    for wid in windows {
        let Some(first) = ui.children(wid).first().copied() else {
            continue;
        };
        let pos = match ui.kind(first) {
            Kind::Text(t) => format!(" {}:{} ", t.cur.y + 1, t.cur.x + 1),
            _ => continue,
        };
        if let Kind::Window(w) = ui.kind_mut(wid) {
            w.footer = pos;
        }
    }
}

/// Placeholder until `StatusLine` exists as a real view. It is drawn straight
/// into the buffer so the screen looks finished; it is not part of the tree
/// and does not receive events.
fn status_line(buf: &mut owlosui_core::Buffer) {
    use owlosui_core::{attr, Color};
    let y = buf.height() - 1;
    let clip = buf.rect();
    let normal = attr(Color::Black, Color::LightGray);
    let key = attr(Color::Red, Color::LightGray);

    buf.fill(Rect::new(0, y, buf.width(), 1), b' ', normal, clip);

    let mut x = 1i16;
    for (k, label) in [
        ("Alt-X", "Exit"),
        ("F1", "Help"),
        ("F5", "Zoom"),
        ("F6", "Next"),
        ("Alt-F3", "Close"),
    ] {
        x += buf.text(x, y, k, key, clip);
        x += buf.text(x, y, " ", normal, clip);
        x += buf.text(x, y, label, normal, clip);
        x += 2;
    }
}
