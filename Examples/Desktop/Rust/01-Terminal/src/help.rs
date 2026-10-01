//! A few pages of help, so the viewer has something to show.
//!
//! They live in the program rather than on disk on purpose: the core cannot
//! open a file and should not learn how. When a link is followed the view puts
//! the href in `pending` and stops; resolving that into content is the
//! application's job, and here the application is a lookup table. In a real
//! program it would be a file, a resource in the executable, or a fetch.

pub const INDEX: &str = "\
<h1>OWLOSUI</h1>
<p>A text mode toolkit in the classic DOS style, with one portable core. The same core is
meant to drive a terminal, a browser canvas, a native window and, in the end,
DOS text memory.</p>
<hr>
<p>Read on:</p>
<ul>
<li><a href=\"keys.htm\">Keys</a> &mdash; what the editor answers to
<li><a href=\"colours.htm\">Colours</a> &mdash; why you cannot choose one
<li><a href=\"#bottom\">Jump to the bottom of this page</a>
</ul>
<p>Press <b>Tab</b> to walk the links and <b>Enter</b> to follow one. The
mouse works too: a single click both picks a link and follows it, because a
page read with one hand should not need two.</p>
<p>&nbsp;</p><p>&nbsp;</p><p>&nbsp;</p>
<a name=\"bottom\"></a>
<p><b>The bottom.</b> You arrived here without the viewer asking anyone to
fetch anything: a link to a fragment is the one kind this view can follow by
itself.</p>
<p><a href=\"index.htm\">Back to the top</a></p>";

pub const KEYS: &str = "\
<h1>Keys</h1>
<p>The editor contains no key codes at all. It knows commands with names, and
a table turns keys into them &mdash; which is how the old DOS editors shipped four
arrangements for one editor, and how this one ships two.</p>
<h2>Modern</h2>
<pre>
  Ctrl+C  Ctrl+X  Ctrl+V     copy, cut, paste
  Ctrl+Z  Ctrl+Y             undo, redo
  Ctrl+A                     select all
  Shift + any movement       extend the selection
</pre>
<h2>Classic (WordStar)</h2>
<pre>
  Ctrl+E  Ctrl+S  Ctrl+D  Ctrl+X    up, left, right, down
  Ctrl+A  Ctrl+F                    word left, word right
  Ctrl+Y                            delete the line
  Ctrl+G  Ctrl+H                    delete, backspace
</pre>
<h2>Both</h2>
<p>Ctrl+Ins, Shift+Ins and Shift+Del belong to both. They are older than
Ctrl+C and Ctrl+V, they still work in Windows today, and in a terminal they
are the only ones that can work at all &mdash; Ctrl+C is taken by the
signal.</p>
<p><a href=\"index.htm\">Back</a></p>";

pub const COLOURS: &str = "\
<h1>Colours</h1>
<p>An element says <b>what it is</b> &mdash; an inactive frame, a selected
row, a status key &mdash; and the palette decides what colour that is. There
is no way to set a colour on an element, and there will not be one.</p>
<p>This is the first rule anyone will want to break, and it is the one that
keeps a whole application looking like one application. Give somebody
<b>style=\"color:red\"</b> and within a year there are eight different reds on a
screen that only has sixteen colours to begin with.</p>
<hr>
<p>The values themselves were not chosen by eye. They were read out of video
memory while a classic DOS program was drawing:</p>
<pre>
  0x71   blue on light grey    the desktop
  0x1F   white on blue         an active window
  0x17   light grey on blue    an inactive one
  0x1A   light green on blue   the close box and the resize corner
  0x31   blue on cyan          the whole scrollbar
  0x08   dark grey on black    the shadow
</pre>
<p>Two of those we had wrong and could not have seen by looking at a
screenshot.</p>
<p><a href=\"index.htm\">Back</a></p>";

/// What the application does with a link the view handed back.
pub fn page(href: &str) -> Option<&'static str> {
    Some(match href {
        "index.htm" => INDEX,
        "keys.htm" => KEYS,
        "colours.htm" => COLOURS,
        _ => return None,
    })
}
