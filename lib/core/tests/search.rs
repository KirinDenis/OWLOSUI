//! Find and replace in the editor: the next match after the caret,
//! selected; the selected match replaced and the next one found; all of
//! them at once.

use owlosui_core::{glyphs, TextView};

fn text(s: &str) -> TextView {
    TextView::new(s.split('\n').map(glyphs).collect())
}

#[test]
fn find_selects_the_next_match_and_stops_at_the_end() {
    let mut t = text("the cat sat\non the mat\nThe end");
    assert!(t.find(&glyphs("the"), false, false));
    assert_eq!(t.selection().map(|(a, b)| (a.y, a.x, b.x)), Some((0, 0, 3)));
    assert!(t.find(&glyphs("the"), false, false), "the next one");
    assert_eq!(t.selection().map(|(a, b)| (a.y, a.x, b.x)), Some((1, 3, 6)));
    assert!(t.find(&glyphs("the"), false, false));
    assert_eq!(t.selection().map(|(a, _)| (a.y, a.x)), Some((2, 0)), "The, case folded");
    assert!(!t.find(&glyphs("the"), false, false), "no more: no wrapping");
    // Case sensitive: "The" is only on the last line.
    let mut t = text("the cat sat\non the mat\nThe end");
    assert!(t.find(&glyphs("The"), true, false));
    assert_eq!(t.selection().map(|(a, _)| (a.y, a.x)), Some((2, 0)));
    // Whole words: "the" inside "other" does not count.
    let mut t = text("other\nthe");
    assert!(t.find(&glyphs("the"), false, true));
    assert_eq!(t.selection().map(|(a, _)| (a.y, a.x)), Some((1, 0)));
    assert!(!text("x").find(&glyphs(""), false, false), "nothing to find");
}

#[test]
fn replace_does_one_and_finds_the_next_and_all_does_them_all() {
    let mut t = text("a cat and a cat");
    assert!(t.find(&glyphs("cat"), false, false));
    let (replaced, found) = t.replace(&glyphs("cat"), &glyphs("dog"), false, false);
    assert!(replaced && found);
    assert_eq!(t.text(), "a dog and a cat");
    let (replaced, found) = t.replace(&glyphs("cat"), &glyphs("dog"), false, false);
    assert!(replaced && !found);
    assert_eq!(t.text(), "a dog and a dog");
    // Nothing selected: nothing replaced, but the search goes on.
    let (replaced, found) = t.replace(&glyphs("dog"), &glyphs("cat"), false, false);
    assert!(!replaced && !found, "the caret is at the end");

    let mut t = text("cat catalogue\ncat");
    assert_eq!(t.replace_all(&glyphs("cat"), &glyphs("dog"), false, true), 2, "whole words: catalogue stays");
    assert_eq!(t.text(), "dog catalogue\ndog");
    assert_eq!(t.replace_all(&glyphs("o"), &glyphs("00"), false, false), 3);
    assert_eq!(t.text(), "d00g catal00gue\nd00g");
    // Undo takes the whole lot back one edit at a time; the count is honest.
    let mut ro = text("a a a");
    ro.readonly = true;
    assert_eq!(ro.replace_all(&glyphs("a"), &glyphs("b"), false, false), 0, "a viewer changes nothing");
}
