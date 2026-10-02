//! The console: what arrives is written at the end, the ANSI colours in it
//! become attributes, and the window follows the newest line until it is
//! scrolled back - which is what "show me what is in the console" needs.

use owlosui_core::console::CONSOLE_ATTR;
use owlosui_core::{Buffer, Console, Dock, Event, Glyph, Key, KeyCode, Kind, Mods, Rect, Ui, Window};

fn ascii(c: char) -> Glyph {
    if (c as u32) < 128 { c as Glyph } else { b'?' as Glyph }
}

fn written(text: &str) -> Console {
    let mut c = Console::new(100);
    c.write(text, &mut ascii);
    c
}

fn line_text(c: &Console, i: usize) -> String {
    c.lines()[i].iter().map(|cell| cell.to_char()).collect()
}

#[test]
fn colours_become_attributes_and_the_sequences_vanish() {
    let c = written("plain \x1b[31mred\x1b[0m \x1b[1;33;44mbold yellow on blue\x1b[m end");
    assert_eq!(line_text(&c, 0), "plain red bold yellow on blue end");
    let at = |i: usize| c.lines()[0][i].attr;
    assert_eq!(at(0), CONSOLE_ATTR, "before any colour: grey on black");
    assert_eq!(at(6), 0x04, "ANSI red is IBM red, 4 - not ANSI's 1, which is blue there");
    assert_eq!(at(9), CONSOLE_ATTR, "reset");
    assert_eq!(at(10), 0x1E, "bold yellow (14) on blue (1)");
    assert_eq!(at(31), CONSOLE_ATTR);
}

#[test]
fn bright_reverse_and_the_colours_this_screen_does_not_have() {
    let c = written("\x1b[92mA\x1b[0;7mB\x1b[0;38;5;196mC\x1b[0;38;2;255;255;85mD\x1b[0;48;5;4mE");
    let at = |i: usize| c.lines()[0][i].attr;
    assert_eq!(at(0), 0x0A, "90-97 are the bright eight: light green");
    assert_eq!(at(1), 0x70, "reverse: black on grey");
    assert_eq!(at(2), 0x0C, "256-colour 196, pure red, is the nearest bright red");
    assert_eq!(at(3), 0x0E, "24-bit yellow is yellow");
    assert_eq!(at(4) >> 4, 1, "256-colour 4 is ANSI blue, IBM 1");
}

#[test]
fn newline_return_tab_backspace_and_erase() {
    let c = written("one\ntwo\rT\nx\ty\nabc\x08\x08Z\nkeep this\x1b[5D\x1b[K!");
    assert_eq!(line_text(&c, 0), "one");
    assert_eq!(line_text(&c, 1), "Two", "return goes back to column 0 and writes over");
    assert_eq!(line_text(&c, 2), "x       y", "tab to the next eighth column");
    assert_eq!(line_text(&c, 3), "aZc");
    assert_eq!(line_text(&c, 4), "keep!", "five back, erase to the end, then the rest");
}

#[test]
fn a_sequence_cut_between_two_writes_is_still_one_sequence() {
    let mut c = Console::new(100);
    c.write("a\x1b[3", &mut ascii);
    c.write("2mb", &mut ascii);
    assert_eq!(line_text(&c, 0), "ab");
    assert_eq!(c.lines()[0][1].attr, 0x02, "ANSI green, 32, is IBM green, 2");
}

#[test]
fn what_it_does_not_understand_leaves_nothing_on_the_screen() {
    let c = written("\x1b[?25l\x1b[2A\x1b]0;a title\x07ok\x1b[s\x1b[u");
    assert_eq!(line_text(&c, 0), "ok");
}

#[test]
fn clear_screen_starts_over() {
    let c = written("old\nlines\x1b[2J\x1b[Hnew");
    assert_eq!(c.lines().len(), 1);
    assert_eq!(line_text(&c, 0), "new");
}

#[test]
fn the_oldest_lines_go_first() {
    let mut c = Console::new(10);
    for i in 0..30 {
        c.write(&format!("line {i}\n"), &mut ascii);
    }
    assert!(c.lines().len() <= 10 + 10 / 8 + 1, "kept {} lines", c.lines().len());
    let last = c.lines().len() - 2;
    assert_eq!(line_text(&c, last), "line 29");
}

#[test]
fn long_lines_fold_to_the_width_and_fold_again_when_it_changes() {
    let mut c = written(&"x".repeat(25));
    c.layout(10, 5);
    assert_eq!(c.row_count(), 3);
    c.layout(20, 5);
    assert_eq!(c.row_count(), 2);
}

#[test]
fn it_follows_the_newest_line_until_scrolled_back() {
    let mut c = Console::new(1000);
    for i in 0..50 {
        c.write(&format!("{i}\n"), &mut ascii);
    }
    c.layout(20, 10);
    assert_eq!(c.top, 40, "the last ten rows are in view");
    c.scroll(-5);
    assert!(!c.follow);
    c.write("more\n", &mut ascii);
    c.layout(20, 10);
    assert_eq!(c.top, 35, "scrolled back, it stays where it was put");
    c.end();
    c.write("again\n", &mut ascii);
    c.layout(20, 10);
    assert_eq!(c.top, c.row_count() - 10, "End follows again");
}

#[test]
fn in_a_window_it_is_drawn_scrolls_with_the_keys_and_has_a_bar() {
    let mut ui = Ui::new(80, 25);
    let root = ui.root();
    let w = ui.insert(root, Rect::new(5, 2, 40, 10), Kind::Window(Window::new("Console")));
    let mut con = Console::new(500);
    for i in 0..30 {
        con.write(&format!("\x1b[32mline\x1b[0m {i}\n"), &mut ascii);
    }
    let id = ui.insert(w, Rect::default(), Kind::Console(con));
    ui.set_dock(id, Dock::Fill);
    ui.activate(w);
    ui.focus_on(w, id);
    let mut buf = Buffer::new(80, 25);
    ui.draw(&mut buf);
    let row = |buf: &Buffer, y: i16| -> String { (6..44).map(|x| buf.get(x, y).unwrap().to_char()).collect() };
    // Eight rows inside the frame; the last is line 29.
    assert!(row(&buf, 10).starts_with("line 29"), "bottom row reads {:?}", row(&buf, 10));
    assert_eq!(buf.get(6, 10).unwrap().attr, 0x02, "green as it was written");
    // The bar is there, on the right edge of the frame.
    assert_ne!(buf.get(44, 6).unwrap().to_char(), '\u{2551}', "no scrollbar on the right edge");

    ui.handle(Event::Key(Key { code: KeyCode::Home, mods: Mods::default() }));
    ui.draw(&mut buf);
    assert!(row(&buf, 3).starts_with("line 0"), "Home: top row reads {:?}", row(&buf, 3));
    ui.handle(Event::Key(Key { code: KeyCode::End, mods: Mods::default() }));
    ui.draw(&mut buf);
    assert!(row(&buf, 10).starts_with("line 29"));
}
