//! A tree.
//!
//! The one control in the set whose shape is not a rectangle of rows, and the
//! whole difficulty is that it has to become one anyway: a screen is rows, so
//! a tree has to be flattened before it can be drawn, on every frame, and
//! every position the keyboard and the mouse talk about is a position in that
//! flattening rather than in the tree.
//!
//! Which is why the flattening is done once, here, and handed out — rather
//! than each of drawing, the arrow keys and the mouse walking the tree in its
//! own way and disagreeing about which row is which.

// `no_std` needs these named; with `std` they are the prelude's.
#[allow(unused_imports)]
use alloc::{boxed::Box, string::{String, ToString}, vec::Vec};
pub struct TreeNode {
    pub text: String,
    pub children: Vec<TreeNode>,
    /// Whether the children are showing. A node with none is never open,
    /// because there would be nothing to see.
    pub open: bool,
    /// Children exist but have not been given yet: a folder on a disk
    /// nobody has looked into. Drawn as a branch; opening it asks the
    /// program for the children (`TreeView::pending`) instead of opening.
    pub lazy: bool,
}

impl TreeNode {
    pub fn leaf(text: &str) -> Self {
        TreeNode {
            text: text.into(),
            children: Vec::new(),
            open: false,
            lazy: false,
        }
    }

    pub fn branch(text: &str, children: Vec<TreeNode>) -> Self {
        TreeNode {
            text: text.into(),
            children,
            open: true,
            lazy: false,
        }
    }

    /// A branch whose children will be asked for when it is opened.
    pub fn lazy(text: &str) -> Self {
        TreeNode {
            text: text.into(),
            children: Vec::new(),
            open: false,
            lazy: true,
        }
    }
}

/// One visible row.
#[derive(Clone, Debug)]
pub struct Row {
    /// The path from the roots down to this node, as child indices.
    pub path: Vec<usize>,
    pub depth: i16,
    pub text: String,
    pub has_children: bool,
    pub open: bool,
    /// Whether this is the last child of its parent, which is what decides
    /// between `├` and `└`.
    pub last: bool,
}

pub struct TreeView {
    pub roots: Vec<TreeNode>,
    pub current: usize,
    pub top: i16,
    pub focused: bool,
    rows: i16,
    /// A lazy node somebody tried to open: its path, until the program
    /// gives its children with `set_children`.
    pub pending: Option<Vec<usize>>,
    /// Scrolled by the wheel or the bar while the cursor was on this row:
    /// the view stays where it was put until the cursor moves.
    scrolled_at: Option<usize>,
}

impl TreeView {
    pub fn new(roots: Vec<TreeNode>) -> Self {
        TreeView {
            roots,
            current: 0,
            top: 0,
            focused: false,
            rows: 1,
            pending: None,
            scrolled_at: None,
        }
    }

    /// The view by `delta` rows, the cursor left where it is.
    pub fn scroll(&mut self, delta: i16) {
        let max = (self.flatten().len() as i16 - self.rows).max(0);
        self.top = (self.top.saturating_add(delta)).clamp(0, max);
        self.scrolled_at = Some(self.current);
    }

    /// The children of a node, given by the program - usually in answer to
    /// `pending`. The node opens; with nothing given it becomes a leaf.
    pub fn set_children(&mut self, path: &[usize], children: Vec<TreeNode>) -> bool {
        match self.node_mut(path) {
            Some(n) => {
                n.open = !children.is_empty();
                n.children = children;
                n.lazy = false;
                true
            }
            None => false,
        }
    }

    /// The path of the current row, as child indices.
    pub fn current_path(&self) -> Vec<usize> {
        self.flatten().get(self.current).map(|r| r.path.clone()).unwrap_or_default()
    }

    /// The texts along a path, root first.
    pub fn texts(&self, path: &[usize]) -> Vec<String> {
        let mut out = Vec::new();
        let mut nodes = &self.roots;
        for &i in path {
            let Some(n) = nodes.get(i) else { break };
            out.push(n.text.clone());
            nodes = &n.children;
        }
        out
    }

    /// Opening a node that has yet to be filled: remember it for the
    /// program and do not open. True if that is what happened.
    fn ask_for(&mut self, path: &[usize]) -> bool {
        let unfilled = matches!(self.node_mut(path), Some(n) if n.lazy && n.children.is_empty());
        if unfilled {
            self.pending = Some(path.to_vec());
        }
        unfilled
    }

    pub fn set_rows(&mut self, rows: i16) {
        self.rows = rows.max(1);
        if self.scrolled_at == Some(self.current) {
            // Scrolled, and the cursor has not moved since: stay put.
            let max = (self.flatten().len() as i16 - self.rows).max(0);
            self.top = self.top.clamp(0, max);
        } else {
            self.scrolled_at = None;
            self.follow();
        }
    }

    pub fn rows(&self) -> i16 {
        self.rows
    }

    /// Everything currently visible, in the order it is drawn.
    pub fn flatten(&self) -> Vec<Row> {
        fn walk(nodes: &[TreeNode], depth: i16, path: &mut Vec<usize>, out: &mut Vec<Row>) {
            for (i, n) in nodes.iter().enumerate() {
                path.push(i);
                out.push(Row {
                    path: path.clone(),
                    depth,
                    text: n.text.clone(),
                    has_children: !n.children.is_empty() || n.lazy,
                    open: n.open && !n.children.is_empty(),
                    last: i + 1 == nodes.len(),
                });
                if n.open {
                    walk(&n.children, depth + 1, path, out);
                }
                path.pop();
            }
        }
        let mut out = Vec::new();
        walk(&self.roots, 0, &mut Vec::new(), &mut out);
        out
    }

    pub fn len(&self) -> usize {
        self.flatten().len()
    }

    pub fn is_empty(&self) -> bool {
        self.roots.is_empty()
    }

    fn node_mut(&mut self, path: &[usize]) -> Option<&mut TreeNode> {
        let (&first, rest) = path.split_first()?;
        let mut node = self.roots.get_mut(first)?;
        for &i in rest {
            node = node.children.get_mut(i)?;
        }
        Some(node)
    }

    pub fn step(&mut self, d: i16) {
        let n = self.flatten().len();
        if n == 0 {
            return;
        }
        self.current = (self.current as i16 + d).clamp(0, n as i16 - 1) as usize;
        self.scrolled_at = None;
        self.follow();
    }

    /// Open the current node, or step into it if it is open already.
    pub fn expand(&mut self) {
        let rows = self.flatten();
        let Some(row) = rows.get(self.current) else {
            return;
        };
        if !row.has_children {
            return;
        }
        if row.open {
            self.step(1);
            return;
        }
        let path = row.path.clone();
        if self.ask_for(&path) {
            return;
        }
        if let Some(n) = self.node_mut(&path) {
            n.open = true;
        }
    }

    /// Close the current node, or go up to its parent when it is closed.
    ///
    /// The second half is what makes Left usable: in a deep tree the thing you
    /// want after closing a branch is nearly always the branch above it.
    pub fn collapse(&mut self) {
        let rows = self.flatten();
        let Some(row) = rows.get(self.current) else {
            return;
        };
        if row.open {
            let path = row.path.clone();
            if let Some(n) = self.node_mut(&path) {
                n.open = false;
            }
            return;
        }
        if row.depth > 0 {
            let want = row.depth - 1;
            for i in (0..self.current).rev() {
                if rows[i].depth == want {
                    self.current = i;
                    self.follow();
                    return;
                }
            }
        }
    }

    pub fn toggle(&mut self) {
        let rows = self.flatten();
        let Some(row) = rows.get(self.current) else {
            return;
        };
        if !row.has_children {
            return;
        }
        let (path, open) = (row.path.clone(), row.open);
        if !open && self.ask_for(&path) {
            return;
        }
        if let Some(n) = self.node_mut(&path) {
            n.open = !open;
        }
    }

    pub fn at_row(&self, row: i16) -> Option<usize> {
        let ix = (self.top + row) as usize;
        (row >= 0 && row < self.rows && ix < self.flatten().len()).then_some(ix)
    }

    fn follow(&mut self) {
        let n = self.flatten().len() as i16;
        let cur = self.current as i16;
        if cur < self.top {
            self.top = cur;
        } else if cur >= self.top + self.rows {
            self.top = cur - self.rows + 1;
        }
        self.top = self.top.clamp(0, (n - self.rows).max(0));
    }

    /// What a row looks like before its own text: the indent, the elbow and
    /// the sign saying whether there is more underneath.
    /// Code page bytes, not characters: the core deals in glyph indices
    /// everywhere else and a tree's elbows are no exception.
    pub fn prefix(row: &Row) -> Vec<crate::cell::Glyph> {
        use crate::cell::{glyph, Glyph};
        let mut s = vec![glyph::SPACE; (row.depth.max(0) * 2) as usize];
        s.push(if row.last {
            glyph::SL_BL
        } else {
            0xC3 as Glyph // ├
        });
        s.push(glyph::SL_H);
        s.push(if !row.has_children {
            glyph::SPACE
        } else if row.open {
            b'-' as Glyph
        } else {
            b'+' as Glyph
        });
        s.push(glyph::SPACE);
        s
    }
}
