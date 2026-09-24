//! The small controls, and the tree.

use owlosui_core::tree::TreeView;
use owlosui_core::{Choice, Cluster, ListBox, StaticText, TreeNode};

/// Check boxes and radio buttons are one view with two modes, because the
/// difference between them is one rule and everything else is the same.
#[test]
fn a_cluster_keeps_its_own_rule() {
    let mut c = Cluster::checks(&["~A~", "~B~", "~C~"]);
    assert_eq!(c.mode, Choice::Many);
    assert_eq!(c.chosen(), Vec::<usize>::new(), "checks start empty");

    c.toggle();
    c.step(1);
    c.toggle();
    assert_eq!(c.chosen(), vec![0, 1], "any number may be on at once");
    c.toggle();
    assert_eq!(c.chosen(), vec![0], "and off again");

    let mut r = Cluster::radio(&["~T~ext", "~H~ex", "~A~uto"]);
    assert_eq!(r.chosen(), vec![0], "one is on from the start");

    r.step(2);
    r.toggle();
    assert_eq!(r.chosen(), vec![2]);

    // Exactly one, always. Turning the chosen one off would leave a question
    // with no answer and no way to give one.
    r.toggle();
    assert_eq!(r.chosen(), vec![2]);
}

#[test]
fn the_bracket_says_which_kind_it_is() {
    let c = Cluster::checks(&["x"]);
    assert_eq!(&c.marker(0)[..], b"[ ]");
    let mut c = Cluster::checks(&["x"]);
    c.toggle();
    assert_eq!(&c.marker(0)[..], b"[X]");

    // Round brackets for the one-of-many kind. A square means "and" and a
    // round one means "or", and people read that without being told once.
    let r = Cluster::radio(&["x", "y"]);
    assert_eq!(r.marker(0)[0], b'(');
    assert_eq!(r.marker(0)[2], b')');
}

#[test]
fn hotkeys_pick_and_toggle() {
    let mut c = Cluster::checks(&["~S~ave", "~B~ackup", "~R~ead only"]);
    assert_eq!(c.by_hotkey('b'), Some(1));
    assert_eq!(c.by_hotkey('B'), Some(1), "case does not matter");
    assert_eq!(c.by_hotkey('z'), None);
    c.current = c.by_hotkey('r').unwrap();
    c.toggle();
    assert_eq!(c.chosen(), vec![2]);
}

#[test]
fn a_list_scrolls_only_as_far_as_it_must() {
    let mut l = ListBox::new(&["a", "b", "c", "d", "e", "f", "g"]);
    l.set_rows(3);
    assert_eq!(l.top, 0);

    l.step(2);
    assert_eq!(l.top, 0, "still in view");
    l.step(1);
    assert_eq!(l.top, 1, "one row, not a jump");

    l.step(10);
    assert_eq!(l.current, 6, "the end is an end");
    assert_eq!(l.top, 4);
}

#[test]
fn static_text_wraps_at_whole_words() {
    let t = StaticText::new("the quick brown fox jumps over the lazy dog");
    let lines = t.lines(20);
    assert_eq!(lines, vec!["the quick brown fox", "jumps over the lazy", "dog"]);

    // A newline in the text is a newline on the screen.
    let t = StaticText::new("one\ntwo");
    assert_eq!(t.lines(40), vec!["one", "two"]);
}

// ---------------------------------------------------------------------- tree

fn sample_tree() -> TreeView {
    TreeView::new(vec![
        TreeNode::branch(
            "core",
            vec![TreeNode::leaf("ui.rs"), TreeNode::leaf("files.rs")],
        ),
        TreeNode::branch("console", vec![TreeNode::leaf("term.rs")]),
        TreeNode::leaf("README.md"),
    ])
}

/// A screen is rows, so a tree has to become rows before it can be drawn — and
/// every position the keyboard and the mouse talk about is a position in that
/// flattening rather than in the tree.
#[test]
fn flattening_is_what_is_on_the_screen() {
    let t = sample_tree();
    let names: Vec<String> = t.flatten().iter().map(|r| r.text.clone()).collect();
    assert_eq!(
        names,
        vec!["core", "ui.rs", "files.rs", "console", "term.rs", "README.md"]
    );

    let rows = t.flatten();
    assert_eq!(rows[1].depth, 1);
    assert_eq!(rows[5].depth, 0);
    assert!(rows[2].last, "files.rs is the last of its parent");
    assert!(!rows[1].last);
}

#[test]
fn closing_a_branch_takes_its_children_off_the_screen() {
    let mut t = sample_tree();
    t.set_rows(10);

    t.toggle(); // close "core"
    let names: Vec<String> = t.flatten().iter().map(|r| r.text.clone()).collect();
    assert_eq!(names, vec!["core", "console", "term.rs", "README.md"]);

    t.toggle();
    assert_eq!(t.flatten().len(), 6);
}

/// Right opens a branch and steps into one already open. Left closes it and,
/// when it is closed already, goes up to the parent — which is what makes Left
/// usable, because in a deep tree the thing wanted after closing a branch is
/// nearly always the branch above it.
#[test]
fn left_and_right_walk_the_shape() {
    let mut t = sample_tree();
    t.set_rows(10);

    t.step(1); // ui.rs, a leaf one level down
    assert_eq!(t.flatten()[t.current].text, "ui.rs");

    t.collapse();
    assert_eq!(
        t.flatten()[t.current].text,
        "core",
        "a closed leaf sends you to the parent"
    );

    t.collapse();
    assert_eq!(t.flatten().len(), 4, "and again closes it");

    t.expand();
    assert_eq!(t.flatten().len(), 6);
    t.expand();
    assert_eq!(
        t.flatten()[t.current].text,
        "ui.rs",
        "an open branch steps into itself"
    );
}

#[test]
fn the_prefix_shows_the_shape() {
    let t = sample_tree();
    let rows = t.flatten();

    // `├─-` for an open branch, `└─ ` for the last leaf of one.
    let core = TreeView::prefix(&rows[0]);
    assert_eq!(core[core.len() - 2], b'-' as owlosui_core::Glyph, "an open branch says so");

    let files = TreeView::prefix(&rows[2]);
    assert_eq!(files[0], b' ' as owlosui_core::Glyph, "indented one level");
    assert_eq!(files[2], 0xC0, "the last child gets the elbow");

    let ui = TreeView::prefix(&rows[1]);
    assert_eq!(ui[2], 0xC3, "and the others a tee");
}
