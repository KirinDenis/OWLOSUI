//! What the help viewer makes of a document.
//!
//! The tests are pictures of the finished page, because that is what the
//! parser produces — not a tree, not a stream of events, a laid-out page. If
//! the picture is right the parser is right, and if it is wrong the diff shows
//! exactly where in one glance.

use owlosui_core::Html;

/// The page as ASCII.
///
/// `Html::text` gives back code page bytes, because that is what the core
/// deals in — a glyph index, not a character. Turning 0xC4 into a box-drawing
/// character is the *backend's* job and the core has no table for it, so the
/// test brings its own stand-ins rather than the core growing a font.
fn page(src: &str, width: i16) -> String {
    let mut h = Html::new(src);
    h.layout(width);
    h.text()
        .chars()
        .map(|c| match c as u32 {
            0xC4 => '-', // ─ the rule
            0xF9 => '*', // ∙ the bullet
            n if n < 128 => c,
            _ => '?',
        })
        .collect()
}

#[test]
fn a_paragraph_wraps_at_the_width_it_is_given() {
    let out = page("<p>the quick brown fox jumps over the lazy dog</p>", 20);
    assert_eq!(out, "the quick brown fox\njumps over the lazy\ndog");

    // The same document at another width is a different page. Nothing is
    // cached from the first layout.
    let out = page("<p>the quick brown fox jumps over the lazy dog</p>", 40);
    assert_eq!(out, "the quick brown fox jumps over the lazy\ndog");
}

#[test]
fn headings_rules_and_lists() {
    let out = page(
        "<h1>Title</h1><p>Intro.</p><hr><ul><li>one<li>two</ul><p>End.</p>",
        16,
    );
    assert_eq!(
        out,
        concat!(
            "Title\n",
            "\n",
            "Intro.\n",
            "----------------\n",
            "\n",
            "* one\n",
            "* two\n",
            "\n",
            "End."
        )
    );
}

/// `<pre>` exists to switch the wrapping and the whitespace collapsing off.
/// If it does not, a help page cannot show a line of code, which is most of
/// what a help page is for.
#[test]
fn pre_keeps_its_spaces_and_its_line_breaks() {
    let out = page("<pre>  mov  ax, 13h\n  int  10h</pre>", 12);
    // No blank line in front — a document does not open with one — and the
    // twelve-column width is ignored entirely, which is the point of <pre>.
    assert_eq!(out, "  mov  ax, 13h\n  int  10h\n");
}

#[test]
fn entities() {
    assert_eq!(page("<p>a &lt; b &amp;&amp; c &gt; d</p>", 40), "a < b && c > d");
    // An ampersand that is not an entity stays an ampersand rather than
    // swallowing the text after it.
    assert_eq!(page("<p>R&D</p>", 40), "R&D");
}

#[test]
fn links_are_found_and_placed() {
    let mut h = Html::new("<p>see <a href=\"two.htm\">the next page</a> for more</p>");
    h.layout(20);

    assert_eq!(h.links().len(), 1);
    assert_eq!(h.links()[0].href, "two.htm");

    // The link knows where it starts, which is what Tab needs to scroll to it
    // and what a click needs to hit it.
    let at = h.links()[0].at;
    assert_eq!(h.link_at(at.y, at.x), Some(0));
    assert_eq!(h.link_at(at.y, at.x - 1), None);
}

#[test]
fn following_a_link_hands_it_out_but_an_anchor_is_handled_here() {
    // The core cannot open a file and must not pretend to. An ordinary link
    // is put in `pending` for the application to resolve.
    let mut h = Html::new("<p><a href=\"index.htm\">home</a></p>");
    h.layout(20);
    h.follow(10);
    assert_eq!(h.pending.as_deref(), Some("index.htm"));

    // A fragment needs nobody: the target is on this page.
    let mut h = Html::new(
        "<p><a href=\"#end\">skip</a></p><p>a</p><p>b</p><p>c</p><a name=\"end\"></a><p>here</p>",
    );
    h.layout(20);
    h.follow(2);
    assert!(h.pending.is_none(), "an anchor should not escape the view");
    assert!(h.top > 0, "the view should have moved to the anchor");
}

/// Tab walks the links in the order they appear and wraps round at the end.
#[test]
fn tab_walks_the_links() {
    let mut h = Html::new("<p><a href=\"a\">one</a> <a href=\"b\">two</a></p>");
    h.layout(30);
    assert_eq!(h.focus, 0);
    h.next_link(10);
    assert_eq!(h.focus, 1);
    h.next_link(10);
    assert_eq!(h.focus, 0);
    h.prev_link(10);
    assert_eq!(h.focus, 1);
}

/// A tag we do not implement is skipped, not shown. Showing it looks honest
/// and ruins the page for a reader who cannot edit the document.
#[test]
fn unknown_tags_disappear() {
    assert_eq!(page("<p><div class=\"x\">text</div></p>", 40), "text");
    assert_eq!(page("<p>a<i>b</i>c</p>", 40), "abc");
}
