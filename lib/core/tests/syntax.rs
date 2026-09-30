//! Syntax colours: the format, each language's line, what a line carries
//! to the next, and the editor painting it.

use owlosui_core::edit::{offer, state};
use owlosui_core::syntax::{parse, Role, BUILTIN};
use owlosui_core::{glyphs, Buffer, Event, Key, KeyCode, Kind, MenuBar, MenuItem, Mods, Rect, TextView, Ui, ViewId, Window};

/// The roles of a line as letters: . text, K keyword, T type, C comment,
/// S string, N number, D directive.
fn roles(lang: &str, line: &str, st: u16) -> (String, u16) {
    let all = parse(BUILTIN);
    let s = all.iter().find(|s| s.name == lang).expect(lang);
    let mut out = Vec::new();
    let next = s.paint(&glyphs(line), st, Some(&mut out));
    let letters = out
        .iter()
        .map(|r| match r {
            Role::Text => '.',
            Role::Keyword => 'K',
            Role::Type => 'T',
            Role::Comment => 'C',
            Role::String => 'S',
            Role::Number => 'N',
            Role::Directive => 'D',
        })
        .collect();
    (letters, next)
}

fn line(lang: &str, text: &str) -> String {
    roles(lang, text, 0).0
}

#[test]
fn the_core_knows_the_main_languages() {
    let names: Vec<String> = parse(BUILTIN).into_iter().map(|s| s.name).collect();
    for n in ["C", "C#", "JavaScript", "JSON", "Rust", "Pascal", "Assembler", "Batch", "INI"] {
        assert!(names.iter().any(|x| x == n), "{n} in {names:?}");
    }
}

#[test]
fn c_keywords_types_numbers_strings_and_comments() {
    assert_eq!(line("C", "int x = 42; // hi"), "TTT.....NN..CCCCC");
    assert_eq!(line("C", "return \"a\\\"b\";"), "KKKKKK.SSSSSS.");
    assert_eq!(line("C", "c = 'x' + '\\n';"), "....SSS...SSSS.");
    assert_eq!(line("C", "  #include <stdio.h>"), "..DDDDDDDDDDDDDDDDDD");
    assert_eq!(line("C", "a = 0x1F;"), "....NNNN.");
    // A keyword inside a longer word is not one.
    assert_eq!(line("C", "format"), "......");
}

#[test]
fn a_block_comment_carries_to_the_next_line() {
    let (a, st) = roles("C", "x /* one", 0);
    assert_eq!(a, "..CCCCCC");
    assert_ne!(st, 0);
    let (b, st) = roles("C", "two */ int", st);
    assert_eq!(b, "CCCCCC.TTT");
    assert_eq!(st, 0);
}

#[test]
fn pascal_ignores_case_and_tells_a_directive_from_a_comment() {
    assert_eq!(line("Pascal", "BEGIN end;"), "KKKKK.KKK.");
    assert_eq!(line("Pascal", "{$I-} { note }"), "DDDDD.CCCCCCCC");
    assert_eq!(line("Pascal", "(* old *) x := $FF + #13;"), "CCCCCCCCC......NNN...NNN.");
    assert_eq!(line("Pascal", "s := 'it''s';"), ".....SSSSSSS.");
    assert_eq!(line("Pascal", "i: Integer;"), "...TTTTTTT.");
}

#[test]
fn assembler_instructions_registers_directives_and_labels() {
    assert_eq!(line("Assembler", "mov ax,0FFh ; set"), "KKK.TT.NNNN.CCCCC");
    assert_eq!(line("Assembler", "MOV AX,BX"), "KKK.TT.TT");
    assert_eq!(line("Assembler", "msg db 'Hi',0"), "....DD.SSSS.N");
    // A local label is one word, and not the instruction inside it.
    assert_eq!(line("Assembler", ".loop: loop .loop"), ".......KKKK......");
}

#[test]
fn rust_chars_are_strings_but_a_lifetime_is_not() {
    assert_eq!(line("Rust", "let c = 'x';"), "KKK.....SSS.");
    assert_eq!(line("Rust", "fn f<'a>(s: &'a str)"), "KK..............TTT.");
    assert_eq!(line("Rust", "#[derive(Debug)]"), "DDDDDDDDDDDDDDDD");
    // A string may go on past the end of its line.
    let (_, st) = roles("Rust", "let s = \"one", 0);
    assert_ne!(st, 0);
    assert_eq!(roles("Rust", "two\";", st).0, "SSSS.");
}

#[test]
fn batch_rem_is_a_word_and_a_label_is_a_directive() {
    assert_eq!(line("Batch", "rem a note"), "CCCCCCCCCC");
    assert_eq!(line("Batch", "REM a note"), "CCCCCCCCCC");
    assert_eq!(line("Batch", "remember"), "........");
    assert_eq!(line("Batch", ":: a note"), "CCCCCCCCC");
    assert_eq!(line("Batch", ":loop"), "DDDDD");
    assert_eq!(line("Batch", "@echo off"), ".KKKK.KKK");
}

#[test]
fn a_language_is_found_by_name_extension_or_file_name() {
    let all = parse(BUILTIN);
    let find = |w: &str| all.iter().find(|s| s.answers_to(w)).map(|s| s.name.clone());
    assert_eq!(find("pascal").as_deref(), Some("Pascal"));
    assert_eq!(find("PAS").as_deref(), Some("Pascal"));
    assert_eq!(find("demo.c").as_deref(), Some("C"));
    assert_eq!(find("RUN.BAT").as_deref(), Some("Batch"));
    assert_eq!(find("README.XYZ"), None);
}

// ------------------------------------------------------------ in the editor

fn editor(ui: &mut Ui, text: &str) -> ViewId {
    let root = ui.root();
    ui.insert(
        root,
        Rect::new(0, 0, 80, 1),
        Kind::MenuBar(MenuBar::new(vec![MenuItem::sub("~F~ile", vec![MenuItem::new("~N~ew", "", 1)])])),
    );
    let win = ui.insert(root, Rect::new(0, 1, 40, 10), Kind::Window(Window::new("Doc")));
    let t = ui.insert(win, Rect::default(), Kind::Text(TextView::new(text.split('\n').map(glyphs).collect())));
    ui.activate(win);
    t
}

fn attr_at(ui: &mut Ui, x: i16, y: i16) -> u8 {
    let mut buf = Buffer::new(80, 25);
    ui.draw(&mut buf);
    buf.get(x, y).unwrap().attr
}

fn press(ui: &mut Ui, code: KeyCode, mods: Mods) {
    ui.handle(Event::Key(Key { code, mods }));
    for _ in 0..4 {
        while ui.pick_pending() {
            ui.complete_pick();
        }
        if ui.take_pressed().is_none() && ui.take_command().is_none() {
            break;
        }
    }
}

#[test]
fn the_editor_paints_a_language_in_the_palettes_colours() {
    let mut ui = Ui::new(80, 25);
    let t = editor(&mut ui, "begin { note }\nend.");
    assert_eq!(ui.set_syntax(t, "DEMO.PAS").as_deref(), Some("Pascal"));
    let p = ui.palette;
    // Inside the window: row 2, from column 1.
    assert_eq!(attr_at(&mut ui, 1, 2), p.syntax_keyword, "begin");
    assert_eq!(attr_at(&mut ui, 7, 2), p.syntax_comment, "the comment");
    assert_eq!(attr_at(&mut ui, 1, 3), p.syntax_keyword, "end");
    assert_eq!(attr_at(&mut ui, 4, 3), p.blue.text, "the full stop is plain text");
    // Nobody answers to it: plain again.
    assert_eq!(ui.set_syntax(t, "NOTES.XYZ"), None);
    assert_eq!(attr_at(&mut ui, 1, 2), p.blue.text);
}

#[test]
fn opening_a_comment_on_one_line_recolours_the_lines_below() {
    let mut ui = Ui::new(80, 25);
    let t = editor(&mut ui, "int a;\nint b;\nint c;");
    ui.set_syntax(t, "C");
    let p = ui.palette;
    assert_eq!(attr_at(&mut ui, 1, 4), p.syntax_type, "int on the third line");
    for c in "/*".chars() {
        press(&mut ui, KeyCode::Char(c), Mods::default());
    }
    assert_eq!(attr_at(&mut ui, 1, 4), p.syntax_comment, "now inside the comment");
    // And closing it again gives the colours back.
    press(&mut ui, KeyCode::Backspace, Mods::default());
    assert_eq!(attr_at(&mut ui, 1, 4), p.syntax_type);
}

#[test]
fn edit_syntax_lists_the_languages_and_a_pick_colours_the_text() {
    let mut ui = Ui::new(80, 25);
    let t = editor(&mut ui, "int x;");
    ui.set_editor(t, offer::SYNTAX, 0);
    assert_eq!(ui.editor_state(t).map(|s| s.1 & state::SYNTAX), Some(0));
    // Alt+E, Y (Syntax), then down to C: None, C, ...
    press(&mut ui, KeyCode::Char('e'), Mods { alt: true, ..Mods::default() });
    press(&mut ui, KeyCode::Char('y'), Mods::default());
    press(&mut ui, KeyCode::Down, Mods::default());
    press(&mut ui, KeyCode::Enter, Mods::default());
    assert_eq!(ui.syntax_of(t).as_deref(), Some("C"));
    assert_eq!(ui.editor_state(t).map(|s| s.1 & state::SYNTAX), Some(state::SYNTAX));
    let p = ui.palette;
    assert_eq!(attr_at(&mut ui, 1, 2), p.syntax_type);
}

#[test]
fn a_program_can_add_a_language_or_replace_one() {
    let mut ui = Ui::new(80, 25);
    let t = editor(&mut ui, "SAY hello -- note");
    let n = ui.add_syntax("[Talk]\nfiles = TLK\ncase = ignore\ncomment = --\nkeywords = say\n");
    assert_eq!(n, 1);
    assert_eq!(ui.set_syntax(t, "chat.tlk").as_deref(), Some("Talk"));
    let p = ui.palette;
    assert_eq!(attr_at(&mut ui, 1, 2), p.syntax_keyword);
    assert_eq!(attr_at(&mut ui, 11, 2), p.syntax_comment);
    // A section named like one of the core's takes its place.
    ui.add_syntax("[C]\nfiles = C\nkeywords = hello\n");
    ui.set_syntax(t, "C");
    assert_eq!(attr_at(&mut ui, 5, 2), p.syntax_keyword, "hello is a C keyword now");
    assert_eq!(ui.syntax_names().iter().filter(|n| n.as_str() == "C").count(), 1, "listed once");
}
