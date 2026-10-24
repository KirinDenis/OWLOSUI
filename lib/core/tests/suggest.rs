//! Words offered while one is typed: two letters of a word in a text
//! coloured as a language, and a list under the caret of what it may
//! become - the language's words and the program's own, the most used first.

use owlosui_core::complete::candidates;
use owlosui_core::syntax::{parse, BUILTIN};
use owlosui_core::{glyphs, Buffer, Button, Event, Key, KeyCode, Kind, Mods, Mouse, MouseKind, Rect, TextView, Ui, ViewId, Window};

fn asm() -> owlosui_core::syntax::Syntax {
    parse(BUILTIN).into_iter().find(|s| s.name == "Assembler").expect("an Assembler in syntax.ini")
}

fn words(v: Vec<Vec<owlosui_core::Glyph>>) -> Vec<String> {
    v.iter().map(|w| w.iter().map(|&g| g as u8 as char).collect()).collect()
}

#[test]
fn the_language_s_words_come_in_the_case_typed_and_the_program_s_most_used_first() {
    let s = asm();
    let lines: Vec<_> = ["start:", "        mov ax,cs", "        loop_again: dec cx", "        jnz loop_again", "        jmp loop_again", "        lo"]
        .iter().map(|l| glyphs(l)).collect();
    // "lo" on the last line, from column 8: the program's label first -
    // used three times - then the language's, the shorter before the longer.
    let got = words(candidates(&s, &lines, (5, 8), &glyphs("lo")));
    assert_eq!(got[0], "loop_again", "{got:?}");
    assert!(got.contains(&"loop".to_string()) && got.contains(&"lodsb".to_string()), "{got:?}");
    // In capitals, the language's words in capitals; the program's as written.
    let got = words(candidates(&s, &lines, (5, 8), &glyphs("MO")));
    assert!(got.contains(&"MOV".to_string()) && !got.contains(&"mov".to_string()), "{got:?}");
    // One letter offers nothing; a word that is already whole is not offered to itself.
    assert!(candidates(&s, &lines, (5, 8), &glyphs("m")).is_empty());
    assert!(!words(candidates(&s, &lines, (5, 8), &glyphs("mov"))).contains(&"mov".to_string()));
}

fn key(ui: &mut Ui, code: KeyCode) {
    ui.handle(Event::Key(Key { code, mods: Mods::default() }));
}

fn typed(ui: &mut Ui, s: &str) {
    for c in s.chars() {
        key(ui, KeyCode::Char(c));
    }
}

fn frame(ui: &mut Ui) -> Buffer {
    let mut b = Buffer::new(80, 25);
    ui.draw(&mut b);
    b
}

fn screen(b: &Buffer) -> String {
    (0..25).map(|y| (0..80).map(|x| b.get(x, y).unwrap().to_char()).collect::<String>() + "\n").collect()
}

fn text(ui: &Ui, t: ViewId) -> String {
    match ui.kind(t) {
        Kind::Text(tv) => tv.text(),
        _ => unreachable!(),
    }
}

/// An editor on an assembler source, focused, caret at the end.
fn editor(ui: &mut Ui, src: &str) -> ViewId {
    let root = ui.root();
    let w = ui.insert(root, Rect::new(0, 1, 60, 15), Kind::Window(Window::new("HELLO.ASM")));
    let t = ui.insert(w, Rect::default(), Kind::Text(TextView::new(src.split('\n').map(glyphs).collect())));
    ui.activate(w);
    ui.focus_on(w, t);
    assert!(ui.set_syntax(t, "HELLO.ASM").is_some());
    frame(ui);
    ui.handle(Event::Key(Key { code: KeyCode::End, mods: Mods { ctrl: true, ..Mods::default() } }));
    t
}

#[test]
fn tab_takes_the_word_enter_is_a_new_line_until_a_word_is_chosen() {
    let mut ui = Ui::new(80, 25);
    let t = editor(&mut ui, "start:\n");
    typed(&mut ui, "mo");
    let s = screen(&frame(&mut ui));
    assert!(s.contains("mov"), "nothing offered:\n{s}");
    key(&mut ui, KeyCode::Tab);
    assert_eq!(text(&ui, t), "start:\nmov");
    assert!(!screen(&frame(&mut ui)).contains("movsb"), "the list stayed up");

    // Enter with nothing chosen: a new line, and the list gone.
    typed(&mut ui, " ax\nlo");
    key(&mut ui, KeyCode::Enter);
    assert_eq!(text(&ui, t), "start:\nmov ax\nlo\n");

    // Down chooses; Enter then takes it.
    typed(&mut ui, "jn");
    key(&mut ui, KeyCode::Down);
    key(&mut ui, KeyCode::Enter);
    let last = text(&ui, t).lines().last().unwrap().to_string();
    assert!(last.starts_with("jn") && last.len() > 2, "Enter did not take the chosen word: {last:?}");

    // Escape puts it away, and the word stays as typed.
    key(&mut ui, KeyCode::Enter);
    typed(&mut ui, "pu");
    key(&mut ui, KeyCode::Esc);
    assert!(!screen(&frame(&mut ui)).contains("pusha"), "Escape left the list");
    assert!(text(&ui, t).ends_with("\npu"));
}

#[test]
fn a_click_on_the_list_takes_the_word_under_it() {
    let mut ui = Ui::new(80, 25);
    let t = editor(&mut ui, "");
    typed(&mut ui, "MO");
    let b = frame(&mut ui);
    let s = screen(&b);
    let y = s.lines().position(|l| l.contains(" MOV ")).expect("MOV offered") as i16;
    let x = s.lines().nth(y as usize).unwrap().find(" MOV ").unwrap() as i16 + 1;
    ui.handle(Event::Mouse(Mouse { x, y, kind: MouseKind::Down(Button::Left) }));
    ui.handle(Event::Mouse(Mouse { x, y, kind: MouseKind::Up(Button::Left) }));
    assert_eq!(text(&ui, t), "MOV");
}

#[test]
fn edit_suggest_words_turns_it_off() {
    let mut ui = Ui::new(80, 25);
    let t = editor(&mut ui, "");
    let (offers, state, _, _) = ui.editor_state(t).unwrap();
    ui.set_editor(t, offers, state | owlosui_core::edit::state::QUIET);
    typed(&mut ui, "mo");
    assert!(!screen(&frame(&mut ui)).contains("mov"), "offered while quiet");
}
