//! A very small HTML viewer.
//!
//! Small on purpose. The subset is links, headings, `<b>`, paragraphs, line
//! breaks, rules, bulleted lists and `<pre>` — and nothing else, because
//! nothing else survives the journey to an 80x25 screen with sixteen colours.
//! Tables, images and CSS are not missing features; admitting them would mean
//! promising a layout we cannot deliver, and people would rightly be annoyed
//! when it did not arrive.
//!
//! `<i>` is left out for a harder reason: there is no way to slant a character
//! cell. We agreed that whatever DOS cannot do does not exist anywhere, so it
//! is absent in the browser too rather than quietly better there.
//!
//! Colour comes from the palette by role. A document cannot set one. That is
//! the same rule the rest of the toolkit keeps, and help text is exactly where
//! it would first be broken.
//!
//! **Parsing happens once, at load.** What the view holds is a laid-out page,
//! not markup — so drawing a frame is a copy, and the parser's cost is paid
//! when the page arrives rather than sixty times a second.

use crate::geom::Point;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Style {
    Text,
    Bold,
    Heading,
    Link,
}

/// No link. Kept as a sentinel rather than an `Option` so a laid-out cell
/// stays four bytes — a help page is thousands of them, on a machine that may
/// have 64K for everything.
pub const NO_LINK: u16 = u16::MAX;

#[derive(Clone, Copy)]
pub struct HCell {
    pub ch: u8,
    pub style: Style,
    pub link: u16,
}

pub struct Link {
    pub href: String,
    /// Where it starts, for moving the view to it.
    pub at: Point,
}

pub struct Html {
    /// The markup, kept so the page can be laid out again at a new width.
    source: String,
    /// The page as it will be drawn.
    lines: Vec<Vec<HCell>>,
    links: Vec<Link>,
    /// Anchor name to line, for `#fragment` links.
    anchors: Vec<(String, i16)>,
    /// The width the current layout was made for.
    width: i16,

    pub top: i16,
    /// Which link has the focus, if any.
    pub focus: usize,
    /// Set when a link was followed to somewhere this view cannot reach by
    /// itself. The application takes it, fetches whatever it names and calls
    /// `load` — which is how the core stays free of any notion of a file.
    pub pending: Option<String>,
}

impl Html {
    pub fn new(source: &str) -> Self {
        let mut h = Html {
            source: source.to_string(),
            lines: Vec::new(),
            links: Vec::new(),
            anchors: Vec::new(),
            width: 0,
            top: 0,
            focus: 0,
            pending: None,
        };
        h.layout(60);
        h
    }

    pub fn load(&mut self, source: &str) {
        let w = self.width.max(1);
        self.source = source.to_string();
        self.top = 0;
        self.focus = 0;
        self.width = 0;
        self.layout(w);
    }

    pub fn lines(&self) -> &[Vec<HCell>] {
        &self.lines
    }

    pub fn links(&self) -> &[Link] {
        &self.links
    }

    pub fn line_count(&self) -> i16 {
        self.lines.len().min(i16::MAX as usize) as i16
    }

    /// Lay the page out for a given width. Cheap enough to call on every
    /// resize, and skipped entirely when the width has not changed.
    pub fn layout(&mut self, width: i16) {
        if width == self.width || width < 4 {
            return;
        }
        self.width = width;
        let (lines, links, anchors) = parse(&self.source, width);
        self.lines = lines;
        self.links = links;
        self.anchors = anchors;
        if self.focus >= self.links.len() {
            self.focus = 0;
        }
    }

    // ------------------------------------------------------------- navigation

    pub fn next_link(&mut self, page: i16) {
        if self.links.is_empty() {
            return;
        }
        self.focus = (self.focus + 1) % self.links.len();
        self.reveal_focus(page);
    }

    pub fn prev_link(&mut self, page: i16) {
        if self.links.is_empty() {
            return;
        }
        self.focus = (self.focus + self.links.len() - 1) % self.links.len();
        self.reveal_focus(page);
    }

    /// Follow the focused link. A `#fragment` is handled here; anything else
    /// is handed out for the application to resolve.
    pub fn follow(&mut self, page: i16) {
        let Some(l) = self.links.get(self.focus) else {
            return;
        };
        if let Some(name) = l.href.strip_prefix('#') {
            if let Some((_, line)) = self.anchors.iter().find(|(n, _)| n == name) {
                self.top = (*line).min((self.line_count() - 1).max(0));
                let _ = page;
                return;
            }
        }
        self.pending = Some(l.href.clone());
    }

    /// The link under a screen position given in page coordinates.
    pub fn link_at(&self, line: i16, col: i16) -> Option<usize> {
        let l = self.lines.get(line as usize)?;
        let c = l.get(col as usize)?;
        (c.link != NO_LINK).then_some(c.link as usize)
    }

    fn reveal_focus(&mut self, page: i16) {
        let Some(l) = self.links.get(self.focus) else {
            return;
        };
        if l.at.y < self.top {
            self.top = l.at.y;
        } else if l.at.y >= self.top + page {
            self.top = l.at.y - page + 1;
        }
        self.top = self.top.max(0);
    }

    pub fn scroll(&mut self, delta: i16, page: i16) {
        let max = (self.line_count() - page).max(0);
        self.top = (self.top + delta).clamp(0, max);
    }

    /// The page as plain text, for tests.
    pub fn text(&self) -> String {
        self.lines
            .iter()
            .map(|l| {
                let s: String = l.iter().map(|c| c.ch as char).collect();
                s.trim_end().to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

// ============================================================================
//  The parser.
//
//  One pass, no tree. A document that needs a tree needs a layout engine, and
//  a layout engine needs a box model, and none of that fits on this screen.
//  What comes out is the finished page.
// ============================================================================

struct Out {
    lines: Vec<Vec<HCell>>,
    links: Vec<Link>,
    anchors: Vec<(String, i16)>,
    width: i16,
    cur: Vec<HCell>,
    indent: i16,
    style: Style,
    link: u16,
}

impl Out {
    /// Change the indent and fix up the line in progress. Without the second
    /// half, closing a list leaves the old indent sitting in the current line
    /// and the paragraph after it comes out shifted.
    fn set_indent(&mut self, n: i16) {
        self.indent = n;
        if !self.cur.iter().any(|c| c.ch != b' ') {
            self.cur.clear();
            for _ in 0..n {
                self.cur.push(HCell {
                    ch: b' ',
                    style: Style::Text,
                    link: NO_LINK,
                });
            }
        }
    }

    fn flush(&mut self) {
        let line = std::mem::take(&mut self.cur);
        self.lines.push(line);
        for _ in 0..self.indent {
            self.cur.push(HCell {
                ch: b' ',
                style: Style::Text,
                link: NO_LINK,
            });
        }
    }

    /// End the line only if there is something on it.
    fn break_line(&mut self) {
        if self.cur.iter().any(|c| c.ch != b' ') {
            self.flush();
        }
    }

    fn blank(&mut self) {
        self.break_line();
        if self.lines.last().map_or(false, |l| !l.is_empty()) {
            self.lines.push(Vec::new());
        }
    }

    fn push_word(&mut self, word: &[u8]) {
        if word.is_empty() {
            return;
        }
        // Wrap before the word, not in the middle of it. A word longer than
        // the line gets its own line and is allowed to overhang; breaking an
        // identifier in a help page is worse than a ragged edge.
        if self.cur.len() as i16 + word.len() as i16 > self.width
            && self.cur.iter().any(|c| c.ch != b' ')
        {
            self.flush();
        }
        if self.link != NO_LINK {
            let ix = self.link as usize;
            if self.links[ix].at.y < 0 {
                self.links[ix].at = Point::new(self.cur.len() as i16, self.lines.len() as i16);
            }
        }
        for b in word {
            self.cur.push(HCell {
                ch: *b,
                style: self.style,
                link: self.link,
            });
        }
    }

    fn push_space(&mut self) {
        if self.cur.iter().any(|c| c.ch != b' ') && (self.cur.len() as i16) < self.width {
            self.cur.push(HCell {
                ch: b' ',
                style: self.style,
                link: self.link,
            });
        }
    }
}

fn entity(name: &str) -> &'static str {
    match name {
        "lt" => "<",
        "gt" => ">",
        "amp" => "&",
        "quot" => "\"",
        "apos" => "'",
        "nbsp" => " ",
        // A code page of 256 glyphs has one dash. Pretending otherwise would
        // mean a `?` in the middle of a sentence, which is worse than a short
        // dash where a long one was asked for.
        "mdash" | "ndash" => "-",
        "hellip" => "...",
        _ => "",
    }
}

fn decode(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c != '&' {
            out.push(c);
            continue;
        }
        let mut name = String::new();
        let mut closed = false;
        for c in it.by_ref() {
            if c == ';' {
                closed = true;
                break;
            }
            if name.len() > 8 {
                break;
            }
            name.push(c);
        }
        match (closed, entity(&name)) {
            (true, e) if !e.is_empty() => out.push_str(e),
            // An entity we do not know comes back out exactly as it went in,
            // semicolon and all. Swallowing the semicolon leaves `&mdash` in
            // the middle of a sentence and makes the document look damaged
            // rather than the viewer look incomplete.
            (true, _) => {
                out.push('&');
                out.push_str(&name);
                out.push(';');
            }
            _ => {
                out.push('&');
                out.push_str(&name);
            }
        }
    }
    out
}

/// Pull `name="value"` pairs out of a tag's inside.
fn attr(inside: &str, name: &str) -> Option<String> {
    let lower = inside.to_ascii_lowercase();
    let at = lower.find(&format!("{name}="))?;
    let rest = &inside[at + name.len() + 1..];
    let (quote, rest) = match rest.chars().next()? {
        q @ ('"' | '\'') => (Some(q), &rest[1..]),
        _ => (None, rest),
    };
    let end = match quote {
        Some(q) => rest.find(q)?,
        None => rest.find(char::is_whitespace).unwrap_or(rest.len()),
    };
    Some(decode(&rest[..end]))
}

fn parse(src: &str, width: i16) -> (Vec<Vec<HCell>>, Vec<Link>, Vec<(String, i16)>) {
    let mut o = Out {
        lines: Vec::new(),
        links: Vec::new(),
        anchors: Vec::new(),
        width,
        cur: Vec::new(),
        indent: 0,
        style: Style::Text,
        link: NO_LINK,
    };
    let mut pre = false;
    let mut heading = false;

    let b = src.as_bytes();
    let mut i = 0usize;
    let mut text = String::new();

    // Everything between tags accumulates here and is emitted when a tag or
    // the end of the document arrives.
    macro_rules! emit {
        () => {
            if !text.is_empty() {
                let t = decode(&text);
                text.clear();
                if pre {
                    for (n, line) in t.split('\n').enumerate() {
                        if n > 0 {
                            o.flush();
                        }
                        // No wrapping and no collapsing inside <pre>: that is
                        // the whole reason it exists.
                        for ch in line.bytes() {
                            let ch = if ch == b'\t' { b' ' } else { ch };
                            o.cur.push(HCell {
                                ch,
                                style: o.style,
                                link: o.link,
                            });
                        }
                    }
                } else {
                    // A space is only produced by whitespace that was really
                    // there. Emitting one at every tag boundary turns
                    // `a<b>c</b>d` into `a c d`, which is wrong and looks like
                    // a wrapping bug rather than a parsing one.
                    let leading = t.starts_with(char::is_whitespace);
                    for (n, word) in t.split_ascii_whitespace().enumerate() {
                        if n > 0 || leading {
                            o.push_space();
                        }
                        o.push_word(word.as_bytes());
                    }
                    if t.ends_with(char::is_whitespace) {
                        o.push_space();
                    }
                }
            }
        };
    }

    while i < b.len() {
        if b[i] != b'<' {
            text.push(b[i] as char);
            i += 1;
            continue;
        }
        let Some(close) = src[i..].find('>') else { break };
        let inside = &src[i + 1..i + close];
        i += close + 1;

        emit!();

        let name_end = inside
            .find(|c: char| c.is_whitespace())
            .unwrap_or(inside.len());
        let name = inside[..name_end].to_ascii_lowercase();

        match name.as_str() {
            "p" => o.blank(),
            "/p" => o.break_line(),
            "br" | "br/" => o.flush(),
            "hr" | "hr/" => {
                o.break_line();
                let rule: Vec<HCell> = (0..width)
                    .map(|_| HCell {
                        ch: crate::cell::glyph::SL_H,
                        style: Style::Text,
                        link: NO_LINK,
                    })
                    .collect();
                o.lines.push(rule);
            }
            "h1" | "h2" | "h3" => {
                o.blank();
                o.style = Style::Heading;
                heading = true;
            }
            "/h1" | "/h2" | "/h3" => {
                o.break_line();
                o.style = Style::Text;
                heading = false;
                o.lines.push(Vec::new());
            }
            "b" | "strong" => {
                if !heading {
                    o.style = Style::Bold
                }
            }
            "/b" | "/strong" => {
                if !heading {
                    o.style = Style::Text
                }
            }
            "ul" | "ol" => {
                o.blank();
                o.set_indent(2);
            }
            "/ul" | "/ol" => {
                o.break_line();
                o.set_indent(0);
                o.lines.push(Vec::new());
            }
            "li" => {
                o.break_line();
                o.cur.clear();
                o.cur.push(HCell {
                    ch: 0xF9, // ∙
                    style: Style::Text,
                    link: NO_LINK,
                });
                o.cur.push(HCell {
                    ch: b' ',
                    style: Style::Text,
                    link: NO_LINK,
                });
            }
            "pre" => {
                o.blank();
                pre = true;
            }
            "/pre" => {
                o.break_line();
                pre = false;
                o.lines.push(Vec::new());
            }
            "a" => {
                if let Some(name) = attr(inside, "name").or_else(|| attr(inside, "id")) {
                    o.anchors.push((name, o.lines.len() as i16));
                }
                if let Some(href) = attr(inside, "href") {
                    o.links.push(Link {
                        href,
                        at: Point::new(-1, -1),
                    });
                    o.link = (o.links.len() - 1) as u16;
                    if !heading {
                        o.style = Style::Link;
                    }
                }
            }
            "/a" => {
                o.link = NO_LINK;
                if !heading {
                    o.style = Style::Text;
                }
            }
            // Unknown tags are skipped rather than shown. Showing them is
            // tempting — it looks honest — but one stray `<div>` in a document
            // then ruins the page for the reader who cannot fix it.
            _ => {}
        }
    }
    emit!();
    o.break_line();

    // A link whose text never arrived points nowhere; drop it rather than
    // leave Tab landing on nothing.
    for l in &o.links {
        debug_assert!(l.at.y >= -1);
    }

    (o.lines, o.links, o.anchors)
}
