//! Buttons, and the row they live in.
//!
//! A row and not a button, because a dialog's buttons are laid out together —
//! centred, evenly spaced, and moved as a group when the dialog resizes. One
//! view that knows about all of them does that in a line; separate views need
//! a container to arrange them, and we have no containers yet.
//!
//! When there are containers this becomes one, and the buttons inside it stay
//! exactly as they are. That is the test of whether a shortcut was the right
//! shape: the thing it stands in for can replace it without the parts
//! changing.

pub struct Button {
    /// With the hotkey between tildes: `~O~pen`.
    pub text: String,
    pub cmd: u16,
    /// The one Enter presses when nothing else has claimed the key. Turbo
    /// Vision called this `bfDefault` and it is the whole of the bargain
    /// between "Enter confirms" and "Enter does whatever I am standing on".
    pub default: bool,
    pub enabled: bool,
}

impl Button {
    pub fn new(text: &str, cmd: u16) -> Self {
        Button {
            text: text.into(),
            cmd,
            default: false,
            enabled: true,
        }
    }

    pub fn default(mut self) -> Self {
        self.default = true;
        self
    }

    pub fn label(&self) -> String {
        self.text.replace('~', "")
    }

    pub fn hotkey(&self) -> Option<char> {
        let mut it = self.text.split('~');
        it.next()?;
        it.next()?.chars().next().map(|c| c.to_ascii_lowercase())
    }

    pub fn hotkey_at(&self) -> Option<usize> {
        self.text.find('~').map(|i| self.text[..i].chars().count())
    }

    /// Two spaces either side of the label, the way Turbo Vision drew them.
    pub fn width(&self) -> i16 {
        self.label().chars().count() as i16 + 4
    }
}

/// Where the row sits in the space it was given.
///
/// Bottom right by default, which is where thirty years of desktop dialogs
/// have put them and therefore where the hand goes without being told. Turbo
/// Vision centred its own; we are not copying that one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Align {
    Right,
    Centre,
}

pub struct ButtonRow {
    pub align: Align,
    pub buttons: Vec<Button>,
    pub current: usize,
    pub focused: bool,
    /// Set when one was pressed.
    pub pressed: Option<u16>,
    /// The one being held down right now.
    ///
    /// A button that fires the instant it is touched cannot be changed your
    /// mind about. Turbo Vision's went down on the press, stayed down while
    /// the button was held, and did the thing on release — so sliding off it
    /// first was a way out. That is worth keeping and costs one field.
    pub down: Option<usize>,
    /// Pressed by a key, shown down, its command not yet delivered.
    ///
    /// A mouse press is seen: the button goes down under the pointer and
    /// fires on release. A key press used to fire at once, and the button
    /// never moved — Alt+S saved the file and nothing on the screen said
    /// so. Now the key puts the button down, the backend shows that frame
    /// for a moment, and only then `complete` delivers the command. The
    /// pause belongs to the backend because the core has no clock.
    pub pending: Option<usize>,
}

impl ButtonRow {
    pub fn new(buttons: Vec<Button>) -> Self {
        // The cursor starts on the default button, not on the first. Enter on
        // a focused row presses the button under the cursor - Turbo Vision
        // made the focused button the default for as long as it was focused
        // - so a row whose cursor started elsewhere would press the wrong
        // one on the first Enter: "Yes" in a box built to answer "No".
        let current = buttons.iter().position(|b| b.default).unwrap_or(0);
        ButtonRow {
            align: Align::Right,
            buttons,
            current,
            focused: false,
            pressed: None,
            down: None,
            pending: None,
        }
    }

    /// Open and Cancel, which is most dialogs.
    pub fn ok_cancel(ok: &str, ok_cmd: u16, cancel_cmd: u16) -> Self {
        ButtonRow::new(vec![
            Button::new(ok, ok_cmd).default(),
            Button::new("~C~ancel", cancel_cmd),
        ])
    }

    /// Total width including the gaps between.
    pub fn width(&self) -> i16 {
        let w: i16 = self.buttons.iter().map(|b| b.width()).sum();
        w + 2 * (self.buttons.len().max(1) as i16 - 1)
    }

    /// Where each button starts, given the row's width.
    ///
    /// The right margin is three and not one: a button casts a shadow two
    /// columns wide, and a shadow that falls outside the dialog is clipped
    /// away, leaving the last button looking flatter than its neighbours.
    pub fn x_of(&self, ix: usize, total: i16) -> i16 {
        let mut x = match self.align {
            Align::Right => (total - self.width() - 3).max(0),
            Align::Centre => ((total - self.width()) / 2).max(0),
        };
        for b in &self.buttons[..ix] {
            x += b.width() + 2;
        }
        x
    }

    pub fn at(&self, x: i16, total: i16) -> Option<usize> {
        (0..self.buttons.len()).find(|&i| {
            let s = self.x_of(i, total);
            x >= s && x < s + self.buttons[i].width()
        })
    }

    pub fn step(&mut self, d: i16) {
        if self.buttons.is_empty() {
            return;
        }
        let n = self.buttons.len() as i16;
        self.current = ((self.current as i16 + d).rem_euclid(n)) as usize;
    }

    /// Fire now. What a mouse release does: the press was already seen.
    pub fn press(&mut self, ix: usize) {
        if let Some(b) = self.buttons.get(ix) {
            if b.enabled {
                self.pressed = Some(b.cmd);
            }
        }
    }

    /// Put the button down and fire on `complete`. What every key does —
    /// Enter, Escape, a hotkey, Space — so the press is seen first.
    pub fn press_by_key(&mut self, ix: usize) {
        if let Some(b) = self.buttons.get(ix) {
            if b.enabled {
                self.current = ix;
                self.down = Some(ix);
                self.pending = Some(ix);
            }
        }
    }

    /// The other half of `press_by_key`. Returns whether there was one.
    pub fn complete(&mut self) -> bool {
        let Some(ix) = self.pending.take() else {
            return false;
        };
        self.down = None;
        self.press(ix);
        true
    }

    /// Where the default button is, if there is one.
    pub fn default_ix(&self) -> Option<usize> {
        self.buttons.iter().position(|b| b.default && b.enabled)
    }

    /// The command Enter should run when nothing else wanted the key.
    pub fn default_cmd(&self) -> Option<u16> {
        self.buttons
            .iter()
            .find(|b| b.default && b.enabled)
            .map(|b| b.cmd)
    }

    pub fn by_hotkey(&self, c: char) -> Option<usize> {
        let c = c.to_ascii_lowercase();
        self.buttons
            .iter()
            .position(|b| b.enabled && b.hotkey() == Some(c))
    }
}
