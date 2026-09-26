//! Menus.
//!
//! Two views, as Turbo Vision had: the bar across the top, and the panel that
//! drops out of it. The panel is not part of the bar — it is an ordinary view
//! that can be put on the desktop by itself, which is exactly what a local or
//! context menu is. Borland's own IDE opened one with Alt+F10 and it was the
//! same object that dropped from the File menu.
//!
//! Everything here was measured off a real Turbo Vision screen rather than
//! guessed:
//!
//! ```text
//!   0x70  the panel, frame and text alike
//!   0x74  the hotkey letter
//!   0x78  an item that cannot be chosen
//!   0x20  the item under the cursor      (black on green)
//!   0x24  its hotkey                     (red on green)
//! ```

/// A command number. Zero means "no command" — an item that opens a submenu,
/// or a separator.
pub type Cmd = u16;

pub struct MenuItem {
    /// The label, with the hotkey letter between tildes: `~F~ile`.
    pub text: String,
    /// Shown right-aligned. Purely a reminder — the key itself is bound in the
    /// application's keymap, not here, so the two can disagree and the menu is
    /// not the place that decides.
    pub shortcut: String,
    pub cmd: Cmd,
    pub enabled: bool,
    /// Non-empty makes this a submenu rather than a command.
    pub items: Vec<MenuItem>,
    pub separator: bool,
    /// Shown with a tick in front: an option that is on. The item still
    /// sends its command; turning the tick off is the program's answer to
    /// it, not the menu's.
    pub checked: bool,
}

impl MenuItem {
    pub fn new(text: &str, shortcut: &str, cmd: Cmd) -> Self {
        MenuItem {
            text: text.into(),
            shortcut: shortcut.into(),
            cmd,
            enabled: true,
            items: Vec::new(),
            separator: false,
            checked: false,
        }
    }

    pub fn checked(mut self, on: bool) -> Self {
        self.checked = on;
        self
    }

    /// Set or clear the tick on the item - here or in any submenu - that
    /// sends `cmd`. Returns whether one was found.
    pub fn set_checked(items: &mut [MenuItem], cmd: Cmd, on: bool) -> bool {
        let mut found = false;
        for it in items {
            if it.cmd == cmd && cmd != 0 {
                it.checked = on;
                found = true;
            }
            if MenuItem::set_checked(&mut it.items, cmd, on) {
                found = true;
            }
        }
        found
    }

    pub fn sub(text: &str, items: Vec<MenuItem>) -> Self {
        MenuItem {
            text: text.into(),
            shortcut: String::new(),
            cmd: 0,
            enabled: true,
            items,
            separator: false,
            checked: false,
        }
    }

    pub fn line() -> Self {
        MenuItem {
            text: String::new(),
            shortcut: String::new(),
            cmd: 0,
            enabled: false,
            items: Vec::new(),
            separator: true,
            checked: false,
        }
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// The label without the tildes, which is what is actually drawn.
    pub fn label(&self) -> String {
        self.text.replace('~', "")
    }

    /// The hotkey letter, lowercased.
    pub fn hotkey(&self) -> Option<char> {
        let mut it = self.text.split('~');
        it.next()?;
        it.next()?.chars().next().map(|c| c.to_ascii_lowercase())
    }

    /// Where the hotkey sits in the drawn label.
    pub fn hotkey_at(&self) -> Option<usize> {
        self.text.find('~').map(|i| self.text[..i].chars().count())
    }

    pub fn selectable(&self) -> bool {
        !self.separator && self.enabled
    }
}

/// The bar across the top.
pub struct MenuBar {
    pub items: Vec<MenuItem>,
    /// Which top-level item is showing its panel.
    pub open: Option<usize>,
}

impl MenuBar {
    pub fn new(items: Vec<MenuItem>) -> Self {
        MenuBar { items, open: None }
    }

    /// Where each item's label starts. Measured from a real screen: two spaces
    /// in front of the first, two between each pair.
    pub fn item_x(&self, ix: usize) -> i16 {
        let mut x = 2i16;
        for it in &self.items[..ix] {
            x += it.label().chars().count() as i16 + 2;
        }
        x
    }

    pub fn item_at(&self, x: i16) -> Option<usize> {
        for i in 0..self.items.len() {
            let start = self.item_x(i);
            let len = self.items[i].label().chars().count() as i16;
            if x >= start && x < start + len {
                return Some(i);
            }
        }
        None
    }
}

/// A panel of items, dropped from the bar or standing on its own.
pub struct MenuBox {
    pub items: Vec<MenuItem>,
    pub current: usize,
    /// The bar item this panel belongs to, if it dropped out of one. A panel
    /// opened as a local menu has none, and Left and Right then do nothing
    /// instead of walking a bar that is not there.
    pub parent: Option<usize>,
}

impl MenuBox {
    pub fn new(items: Vec<MenuItem>) -> Self {
        let current = items.iter().position(|i| i.selectable()).unwrap_or(0);
        MenuBox {
            items,
            current,
            parent: None,
        }
    }

    /// The width of the panel including its frame and the one column of
    /// padding on each side. From the reference: label, at least two spaces,
    /// shortcut — and the widest item sets it for all of them.
    pub fn width(&self) -> i16 {
        let inner = self
            .items
            .iter()
            .map(|i| {
                1 + i.label().chars().count()
                    + if i.shortcut.is_empty() {
                        0
                    } else {
                        2 + i.shortcut.chars().count()
                    }
                    + 1
            })
            .max()
            .unwrap_or(4) as i16;
        inner.max(6) + 4
    }

    pub fn height(&self) -> i16 {
        self.items.len() as i16 + 2
    }

    /// Which item a row inside the panel refers to.
    pub fn item_at(&self, row: i16) -> Option<usize> {
        let ix = (row - 1) as usize;
        (row >= 1 && ix < self.items.len() && self.items[ix].selectable()).then_some(ix)
    }

    pub fn step(&mut self, delta: i16) {
        if self.items.is_empty() {
            return;
        }
        let n = self.items.len() as i16;
        let mut i = self.current as i16;
        for _ in 0..n {
            i = (i + delta).rem_euclid(n);
            if self.items[i as usize].selectable() {
                self.current = i as usize;
                return;
            }
        }
    }

    /// The item a letter belongs to.
    pub fn by_hotkey(&self, c: char) -> Option<usize> {
        let c = c.to_ascii_lowercase();
        self.items
            .iter()
            .position(|i| i.selectable() && i.hotkey() == Some(c))
    }
}

/// A deep copy of a set of items.
///
/// A panel gets its own copy rather than a borrow of the bar's. The borrow
/// would be the efficient thing and it would also mean the tree holding a
/// reference into one of its own nodes, which is exactly the shape this whole
/// design exists to avoid.
pub fn clone_items(items: &[MenuItem]) -> Vec<MenuItem> {
    items
        .iter()
        .map(|i| MenuItem {
            text: i.text.clone(),
            shortcut: i.shortcut.clone(),
            cmd: i.cmd,
            enabled: i.enabled,
            items: clone_items(&i.items),
            separator: i.separator,
            checked: i.checked,
        })
        .collect()
}
