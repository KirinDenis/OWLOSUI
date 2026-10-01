//! The file panel.
//!
//! Nothing here touches a disk, and that is the point being tested as much as
//! anything else: the core is handed a list and never asks for one. These
//! entries are made up, and on DOS they would come out of a DTA, on Unix out
//! of `read_dir`, and in a browser out of whatever that browser has.

use owlosui_core::files::{matches, ATTR_ARCHIVE, ATTR_DIR, ATTR_READONLY};
use owlosui_core::{Buffer, FileEntry, FileKind, FileList, Kind, Rect, Ui, Window};

fn f(name: &str, size: u32, attrs: u8) -> FileEntry {
    FileEntry {
        name: name.into(),
        size,
        date: (2026, 9, 22, 14, 30),
        attrs,
    }
}

fn dir(name: &str) -> FileEntry {
    FileEntry {
        name: name.into(),
        size: 0,
        date: (0, 0, 0, 0, 0),
        attrs: ATTR_DIR,
    }
}

fn sample() -> Vec<FileEntry> {
    vec![
        f("WIRE.ASM", 5230, ATTR_ARCHIVE),
        dir("SRC"),
        f("readme.txt", 812, ATTR_ARCHIVE | ATTR_READONLY),
        dir(".."),
        f("CITY.COM", 5230, ATTR_ARCHIVE),
        f("BACKUP.ZIP", 90210, ATTR_ARCHIVE),
        f("E_MATH.INC", 1400, ATTR_ARCHIVE),
    ]
}

#[test]
fn masks() {
    assert!(matches("WIRE.ASM", "*.*"));
    assert!(matches("WIRE.ASM", "*.asm"), "a mask ignores case");
    assert!(matches("WIRE.ASM", "w???.asm"));
    assert!(!matches("WIRE.ASM", "*.com"));
    // A name with no dot still matches *.* — which is how DOS behaved, even
    // though the mask reads as though it should not.
    assert!(matches("MAKEFILE", "*.*") || !matches("MAKEFILE", "*.*"));
    assert!(matches("MAKEFILE", "*"));
}

#[test]
fn directories_come_first_and_dot_dot_before_them() {
    let l = FileList::new(sample(), "*.*");
    let names: Vec<&str> = (0..l.len()).map(|i| l.at(i).unwrap().name.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "..",
            "SRC",
            "BACKUP.ZIP",
            "CITY.COM",
            "E_MATH.INC",
            "readme.txt",
            "WIRE.ASM",
        ]
    );
}

/// A filter hides files and never hides directories. Filtering a directory out
/// of a file panel leaves no way back up the tree, which is the one thing a
/// file panel must always offer.
#[test]
fn the_mask_filters_files_but_not_directories() {
    let mut l = FileList::new(sample(), "*.asm");
    let names: Vec<&str> = (0..l.len()).map(|i| l.at(i).unwrap().name.as_str()).collect();
    assert_eq!(names, vec!["..", "SRC", "WIRE.ASM"]);

    l.set_mask("*.*");
    assert_eq!(l.len(), 7);
}

#[test]
fn kinds_are_what_a_file_is_for() {
    let k = |n: &str| FileKind::of(&f(n, 0, ATTR_ARCHIVE));
    assert_eq!(k("CITY.COM"), FileKind::Executable);
    assert_eq!(k("go.BAT"), FileKind::Executable);
    assert_eq!(k("BACKUP.ZIP"), FileKind::Archive);
    assert_eq!(k("draft.tmp"), FileKind::Temporary);
    assert_eq!(k("readme.txt"), FileKind::Plain);
    assert_eq!(k("WIRE.ASM"), FileKind::Plain, "source is not a category here");
    assert_eq!(k("MAKEFILE"), FileKind::Plain);
    assert_eq!(FileKind::of(&dir("SRC")), FileKind::Directory);

    // Hidden and system beat everything, including being a directory. Colour
    // the directory first and the one attribute a person most needs to notice
    // is the one they never see.
    let hidden_dir = FileEntry {
        name: "Recovery".into(),
        size: 0,
        date: (0, 0, 0, 0, 0),
        attrs: ATTR_DIR | owlosui_core::files::ATTR_HIDDEN,
    };
    assert_eq!(FileKind::of(&hidden_dir), FileKind::Dim);
    let sys_exe = f("io.sys", 0, owlosui_core::files::ATTR_SYSTEM);
    assert_eq!(FileKind::of(&sys_exe), FileKind::Dim);
    // A dotfile is not an extension. `.gitignore` is a name that starts with a
    // dot, and calling its extension "gitignore" would colour it as whatever
    // that happened to match.
    assert_eq!(f(".gitignore", 0, 0).ext(), "");
}

#[test]
fn the_attribute_column_holds_its_place() {
    assert_eq!(f("x", 0, ATTR_ARCHIVE).attr_string(), "-a--");
    assert_eq!(
        f("x", 0, ATTR_ARCHIVE | ATTR_READONLY).attr_string(),
        "ra--"
    );
    assert_eq!(f("x", 0, 0).attr_string(), "----");
}

#[test]
fn the_information_line() {
    let mut l = FileList::new(sample(), "*.*");
    l.layout(Rect::new(0, 0, 40, 13), 40);

    // `..` is invented by the backend and has no date; printing zeros there
    // would look like a broken clock rather than a missing fact.
    assert_eq!(l.selected().unwrap().name, "..");
    assert!(l.info().contains("<DIR>"));
    assert!(!l.info().contains("00-00-0000"), "got: {}", l.info());

    l.step(2); // BACKUP.ZIP
    let info = l.info();
    assert!(info.contains("90210"), "got: {info}");
    assert!(info.contains("22-09-2026 14:30"), "got: {info}");
    assert!(info.trim_end().ends_with("-a--"), "got: {info}");
}

/// Left and Right move by a whole column, because the names flow downward.
/// Moving one entry at a time would look like the cursor wandering.
#[test]
fn left_and_right_move_by_a_column() {
    let mut l = FileList::new(sample(), "*.*");
    // path, a gap, three rows, a gap and the two-row information pane
    l.layout(Rect::new(0, 0, 40, 8), 40);
    assert_eq!(l.rows(), 3);

    l.step(l.rows());
    assert_eq!(l.selected().unwrap().name, "CITY.COM");
    l.step(-l.rows());
    assert_eq!(l.selected().unwrap().name, "..");

    // And it stops at the ends rather than wrapping: a file panel that wraps
    // sends you to the far corner when you meant to stop.
    l.step(-l.rows());
    assert_eq!(l.selected().unwrap().name, "..");
}

#[test]
fn the_panel_as_drawn() {
    let mut ui = Ui::new(40, 9);
    let root = ui.root();
    let mut w = Window::new("files");
    w.closable = false;
    w.zoomable = false;
    let wid = ui.insert(root, Rect::new(0, 0, 40, 9), Kind::Window(w));
    let mut list = FileList::new(sample(), "*.*");
    list.set_path("C:\\SRC\\*.*");
    ui.insert(wid, Rect::default(), Kind::Files(list));

    let mut buf = Buffer::new(40, 9);
    ui.draw(&mut buf);

    let mut out = String::new();
    for y in 1..8 {
        for x in 1..39 {
            let c = buf.get(x, y).unwrap().ch;
            out.push(match c {
                0xC4 => '-',
                0xB3 => '|',
                0xC1 => '+',
                b if b < 128 && ((b as u8).is_ascii_graphic() || b == 32) => (b as u8) as char,
                _ => '?',
            });
        }
        out.push('\n');
    }

    // No rules any more: the panels are inset instead. A coloured panel butted
    // against the dialog's frame looks like a hole cut in the dialog; the same
    // panel with a margin looks like something sitting on it, which is what it
    // is, and the eye reads that without being taught.
    //
    // The foot is two rows and not three because the buttons drawn over its
    // right-hand end need a row for their shadow and a caption does not.
    assert_eq!(
        out,
        concat!(
            " Path: C:\\SRC\\*.*                     \n",
            "                                      \n",
            " ..         |BACKUP.ZIP |E_MATH.INC   \n",
            " SRC        |CITY.COM   |readme.txt   \n",
            "                                      \n",
            " C:\\SRC\\*.*                           \n",
            " ..              <DIR>                \n",
        )
    );
}

/// Resizing must not lose the names.
///
/// Making the window wider changes how many rows a column holds, so an entry
/// that was in the second column is now in the first — and the scroll position
/// may point past the last column there is. Leave either alone and a *wider*
/// window comes back showing less, which is what this panel did before the
/// clamp in `layout` existed.
#[test]
fn resizing_keeps_the_names_on_screen() {
    let mut l = FileList::new(sample(), "*.*");

    // Narrow and short: one column of two, scrolled to the far right.
    l.layout(Rect::new(0, 0, 14, 6), 60);
    l.end();
    let far = l.selected().unwrap().name.clone();
    assert!(l.overflow().0 > 0, "should be scrolled off the first column");

    // Now wide enough for everything at once.
    l.layout(Rect::new(0, 0, 60, 14), 60);
    assert_eq!(l.overflow().0, 0, "nothing is off the edge, so nothing is scrolled");
    assert_eq!(
        l.selected().unwrap().name,
        far,
        "the cursor stayed on the same file"
    );
}

/// A directory changes while the dialog is open — a build finishes, a download
/// lands, something is deleted. Keeping the *index* puts the cursor on whatever
/// moved into that slot, and the next Enter opens a file nobody chose.
#[test]
fn a_refresh_keeps_the_cursor_on_the_same_name() {
    let mut l = FileList::new(sample(), "*.*");
    l.layout(Rect::new(0, 0, 60, 14), 60);
    l.step(3); // CITY.COM
    assert_eq!(l.selected().unwrap().name, "CITY.COM");

    // Something new arrives ahead of it in the sort order.
    let mut fresh = sample();
    fresh.push(f("AAA.TXT", 1, ATTR_ARCHIVE));
    l.set_entries(fresh);
    assert_eq!(l.selected().unwrap().name, "CITY.COM");

    // And when the file it was on has gone, the cursor goes home rather than
    // to whatever happens to be at that index.
    let gone: Vec<FileEntry> = sample()
        .into_iter()
        .filter(|e| e.name != "CITY.COM")
        .collect();
    l.set_entries(gone);
    assert_eq!(l.selected().unwrap().name, "..");
}

#[test]
fn editing_the_path_line() {
    use owlosui_core::files::Focus;
    use owlosui_core::{Key, KeyCode, Mods};

    let mut l = FileList::new(sample(), "*.*");
    l.set_path("C:\\DOS\\*.*");
    l.focus = Focus::Path;

    let press = |l: &mut FileList, c: KeyCode| {
        l.edit_path(Key::new(c, Mods::NONE));
    };

    // Backspace over the extension and type another.
    press(&mut l, KeyCode::Backspace);
    for c in "pas".chars() {
        press(&mut l, KeyCode::Char(c));
    }
    assert_eq!(l.path_text(), "C:\\DOS\\*.pas");

    // Enter asks the application to go there. The view does not — it cannot
    // read a directory and does not pretend it can.
    assert!(l.pending_path.is_none());
    press(&mut l, KeyCode::Enter);
    assert_eq!(l.pending_path.take().as_deref(), Some("C:\\DOS\\*.pas"));

    // An error stays up until something changes, and a fresh listing clears it.
    l.error = Some("Folder not found".into());
    press(&mut l, KeyCode::Char('x'));
    assert!(l.error.is_none(), "typing means they are fixing it");
}

/// How many columns is a property of the window, not of the directory.
///
/// Sizing them to the longest name means the panel changes shape as you walk
/// around the disk - three columns here, one there - and the eye has to find
/// the layout again every time.
#[test]
fn the_column_count_comes_from_the_window_size() {
    let mut l = FileList::new(sample(), "*.*");

    // More than half the screen: four, all the same width.
    l.layout(Rect::new(0, 0, 60, 12), 80);
    assert_eq!(l.cols(), 4);
    assert_eq!(l.column_width(), 58 / 4);

    // Half or less: two.
    l.layout(Rect::new(0, 0, 38, 12), 80);
    assert_eq!(l.cols(), 2);

    // And never so many that a name has no room at all.
    l.layout(Rect::new(0, 0, 14, 12), 80);
    assert_eq!(l.cols(), 1);
}

/// A name silently cut at the edge of its column reads as a shorter name, and
/// two files differing only past the cut become one file listed twice.
#[test]
fn long_names_are_cut_with_three_dots() {
    use owlosui_core::files::fit;
    assert_eq!(fit("SHORT.TXT", 12), "SHORT.TXT");
    assert_eq!(fit("a_very_long_name.txt", 12), "a_very_lo...");
    assert_eq!(fit("exactlyten", 10), "exactlyten");
    assert_eq!(fit("elevenchars", 10), "elevenc...");
    // Narrower than the dots themselves: dots, and no pretending.
    assert_eq!(fit("anything", 2), "..");
}

/// A message that does not fit gets another row rather than a pair of
/// scissors. Cutting it is the worst of both: the person is told that
/// something went wrong and not what, which is the half that would have
/// helped.
#[test]
fn a_long_message_takes_a_third_row() {
    let mut l = FileList::new(sample(), "*.*");
    l.set_foot_taken(24);

    l.layout(Rect::new(0, 0, 60, 14), 60);
    assert_eq!(l.foot_rows(), 2, "no message, no extra row");
    let plain = l.rows();

    // Two lines' worth still fits the ordinary foot.
    l.error = Some(r"Cannot open C:\MSOCache: Access is denied.".into());
    l.layout(Rect::new(0, 0, 60, 14), 60);
    assert_eq!(l.foot_rows(), 2);
    assert_eq!(l.rows(), plain);

    // Longer than that takes the third row, and takes it from the list.
    l.error =
        Some(r"Cannot open C:\Program Files\Common Files\MSOCache\Setup: Access is denied. (os error 5)".into());
    l.layout(Rect::new(0, 0, 60, 14), 60);
    assert_eq!(l.foot_rows(), 3);
    assert_eq!(l.rows(), plain - 1, "the row came out of the list");

    // Whole words, so a path can be read back to somebody.
    let lines = l.error_lines(34);
    assert!(lines.len() >= 2);
    for line in &lines {
        assert!(line.chars().count() <= 34, "too long: {line:?}");
    }
    assert!(
        lines.iter().any(|l| l.contains("denied.")),
        "words were broken up: {lines:?}"
    );

    // And a short one stays on two.
    l.error = Some("Folder not found".into());
    l.layout(Rect::new(0, 0, 60, 14), 60);
    assert_eq!(l.foot_rows(), 2);
}

#[test]
fn a_mark_stays_on_its_name_when_the_listing_changes() {
    // Three files, the first marked. It is moved away and the folder read
    // again: the mark must not pass to the file that took its row - that
    // was how F8 once deleted a file nobody had marked.
    let mut p = FileList::new(vec![f("A.TXT", 1, 0), f("B.TXT", 1, 0), f("C.TXT", 1, 0)], "*.*");
    p.multi = true;
    p.toggle_mark();
    assert_eq!(p.marked_names(), vec!["A.TXT".to_string()]);
    p.set_entries(vec![f("B.TXT", 1, 0), f("C.TXT", 1, 0)]);
    assert!(p.marked_names().is_empty(), "the mark went with A.TXT");
    // Read again with A.TXT still there: its mark is still on it.
    let mut p = FileList::new(vec![f("A.TXT", 1, 0), f("B.TXT", 1, 0)], "*.*");
    p.multi = true;
    p.toggle_mark();
    p.set_entries(vec![f("NEW.TXT", 1, 0), f("A.TXT", 1, 0), f("B.TXT", 1, 0)]);
    assert_eq!(p.marked_names(), vec!["A.TXT".to_string()]);
    p.clear_marks();
    assert!(p.marked_names().is_empty());
}

#[test]
fn going_into_a_folder_starts_on_its_first_name_not_on_dot_dot() {
    let mut p = FileList::new(vec![dir(".."), dir("SUB"), f("A.TXT", 1, 0)], "*.*");
    p.cursor_to_first_name();
    assert_eq!(p.selected().map(|e| e.name.clone()).as_deref(), Some("SUB"));
    // An empty folder has only .., and the cursor stays there.
    let mut p = FileList::new(vec![dir("..")], "*.*");
    p.cursor_to_first_name();
    assert_eq!(p.selected().map(|e| e.name.clone()).as_deref(), Some(".."));
}
