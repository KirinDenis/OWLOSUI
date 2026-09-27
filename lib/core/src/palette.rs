//! Colour by role, never by value.
//!
//! A view says *what it is* — an inactive frame, a selected row, a status key
//! — and the palette decides the colour. This is the one rule that keeps a
//! whole application looking like one application, and it is the first rule
//! people will want to break. Do not give them a way to set a colour directly
//! on an element; give them a different palette.
//!
//! Windows come in three families, as Turbo Vision's did: blue for documents,
//! cyan for help, grey for dialogs. A window says which family it belongs to
//! and everything in it — frame, body, scrollbar, text — follows. That is why
//! a help window in Borland Pascal is cyan *including its frame*, and why
//! colouring only the body leaves you with the wrong window holding the right
//! contents.
//!
//! The numbers were not chosen by eye. The blue set was read out of video
//! memory while the real Turbo Vision was drawing; the cyan set is written
//! down in `HELPFILE.PAS` as `CHelpColor`.

// `no_std` needs these named; with `std` they are the prelude's.
#[allow(unused_imports)]
use alloc::{boxed::Box, string::{String, ToString}, vec::Vec};
use crate::cell::{attr, attr_bg, Color};

/// One family of window colours.
#[derive(Clone, Copy, Debug)]
pub struct WinColors {
    pub frame_passive: u8,
    pub frame_active: u8,
    /// The frame while the window is being dragged or resized.
    pub frame_dragging: u8,
    /// The close box, the zoom box and the resize corner.
    pub handle: u8,

    /// The window's own background, before anything is put in it.
    pub body: u8,
    pub body_passive: u8,

    /// The whole scrollbar. Track, marker and arrows share one attribute and
    /// differ only by glyph — one colour, three shapes, which is why the
    /// control reads as a single object.
    pub scroll: u8,

    /// Ordinary content.
    pub text: u8,
    pub text_selected: u8,
    /// A word the reader can act on, and the one they are standing on.
    pub link: u8,
    pub link_focus: u8,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WinPalette {
    Blue,
    Cyan,
    Gray,
}

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub desktop: u8,

    /// What a cell becomes when a window's shadow falls on it. The glyph is
    /// kept; only the colour changes, which is why a shadow over text still
    /// shows the text, dimmed.
    pub shadow: u8,

    pub blue: WinColors,
    pub cyan: WinColors,
    pub gray: WinColors,

    /// Menus. Measured: the panel is one colour throughout, frame included,
    /// and the item under the cursor turns green rather than inverting.
    pub menu: u8,
    pub menu_key: u8,
    pub menu_disabled: u8,
    pub menu_selected: u8,
    pub menu_selected_key: u8,

    /// A file list, coloured by what the extension says the file is for.
    ///
    /// These are ours, not measured. Volkov Commander's own palette we have
    /// not read out of a running copy the way we did Turbo Vision's, and until
    /// we do these are a choice and are marked as one.
    pub file_dir: u8,
    pub file_exe: u8,
    pub file_archive: u8,
    pub file_temp: u8,
    pub file_dim: u8,
    pub file_plain: u8,
    pub file_selected: u8,
    pub file_selected_passive: u8,
    pub file_info: u8,
    /// The lines that divide the columns and cap the information pane.
    pub file_frame: u8,
    /// The editable path line, and the same line while it has the focus.
    pub file_path: u8,
    /// The label in front of a field — plain, so the field is what the eye
    /// lands on.
    pub file_path_label: u8,
    pub file_path_focus: u8,
    /// Something went wrong and the person needs to read it.
    pub file_error: u8,

    pub button: u8,
    pub button_key: u8,
    pub button_focus: u8,
    pub button_focus_key: u8,
    pub button_disabled: u8,
    /// The other two kinds of button: set apart (cyan) and destructive (red).
    pub button_accent: u8,
    pub button_accent_key: u8,
    pub button_accent_focus: u8,
    pub button_danger: u8,
    pub button_danger_key: u8,
    pub button_danger_focus: u8,
    /// Half blocks in dark grey over the dialog's own colour.

    /// The hex view: the offset column, the bytes, and the text beside them.
    /// Three roles because the eye uses them differently — the offsets to find
    /// a place, the bytes to read a value, the text to recognise a file.
    pub hex_offset: u8,
    pub hex_byte: u8,
    pub hex_text: u8,
    pub hex_cursor: u8,

    /// The small controls, on a dialog's grey.
    pub ctl: u8,
    pub ctl_key: u8,
    pub ctl_focus: u8,
    pub ctl_focus_key: u8,
    pub ctl_disabled: u8,
    /// A list's own box, which is sunk into the dialog rather than sitting on
    /// it — that is what says you may put something into it.
    /// An editable box sunk into a dialog: a memo, and the same colours the
    /// input line uses. Sitting on the dialog's own grey it would look like
    /// part of the dialog, and nobody types into the dialog.
    pub memo: u8,
    pub memo_focus: u8,
    pub list: u8,
    pub list_selected: u8,
    pub list_selected_passive: u8,

    pub status: u8,
    pub status_key: u8,
    pub status_disabled: u8,

    /// A label beside a control: its words, its hotkey letter, and the
    /// whole of it when its control has the focus.
    pub label: u8,
    pub label_key: u8,
    pub label_active: u8,
    /// A marked item in a list or a file panel - yellow, as Norton had it.
    pub list_marked: u8,
}

impl Palette {
    /// The colour of a shadow cast on a surface.
    ///
    /// A shadow is not a colour of its own; it is the surface, darker. On the
    /// grey of a dialog that is dark grey, on the blue of a document window
    /// it is black, and on black - where a darker black does not exist - it
    /// is dark grey so that it is there at all. The background is the
    /// surface's, always: a shadow that brought its own background would be
    /// a grey smear on a blue window, which is what this replaced.
    /// A progress bar on a surface: blue where the surface is not, cyan
    /// where it is, so the bar is never the colour of what it sits on.
    pub fn progress_on(surface: u8) -> u8 {
        let bg = attr_bg(surface);
        let fg = if bg == Color::Blue { Color::Cyan } else { Color::Blue };
        attr(fg, bg)
    }

    pub fn shadow_on(surface: u8) -> u8 {
        let bg = attr_bg(surface);
        let fg = match bg {
            Color::LightGray | Color::White => Color::DarkGray,
            Color::Black => Color::DarkGray,
            c if c.index() >= 8 => Color::from_index(c.index() - 8),
            _ => Color::Black,
        };
        attr(fg, bg)
    }

    pub fn window(&self, which: WinPalette) -> &WinColors {
        match which {
            WinPalette::Blue => &self.blue,
            WinPalette::Cyan => &self.cyan,
            WinPalette::Gray => &self.gray,
        }
    }

    /// The colours everyone remembers.
    pub const fn classic() -> Palette {
        Palette {
            // Blue dots on light grey, not the other way round. With `░` at a
            // quarter coverage this averages to the pale lavender everyone
            // remembers; inverting the nibbles gives a dark navy instead, and
            // that one swapped nibble is the whole difference between "looks
            // like Turbo Vision" and "looks like something else".
            desktop: attr(Color::Blue, Color::LightGray),
            shadow: attr(Color::DarkGray, Color::Black),

            // A document window. Measured off a real screen.
            blue: WinColors {
                frame_passive: attr(Color::LightGray, Color::Blue), // 0x17
                frame_active: attr(Color::White, Color::Blue),      // 0x1F
                frame_dragging: attr(Color::LightGreen, Color::Blue),
                handle: attr(Color::LightGreen, Color::Blue), // 0x1A

                // An empty window is 0x1F and an inactive one 0x17. The yellow
                // everybody remembers is 0x1E, and it is the *editor's* text
                // rather than the window's background — two roles that look
                // like one until the bytes are compared.
                body: attr(Color::White, Color::Blue),
                body_passive: attr(Color::LightGray, Color::Blue),

                scroll: attr(Color::Blue, Color::Cyan), // 0x31

                text: attr(Color::Yellow, Color::Blue), // 0x1E
                text_selected: attr(Color::Black, Color::LightGray),
                link: attr(Color::LightCyan, Color::Blue),
                link_focus: attr(Color::White, Color::Blue),
            },

            // Help. This is `CHelpColor` from HELPFILE.PAS, unchanged:
            //   37 3F 3A 13 13 30 3E 1E
            // Note the scrollbar — cyan on blue, the other way round from a
            // document window's. Nothing here is arbitrary: on a cyan body the
            // document colouring would disappear.
            cyan: WinColors {
                frame_passive: attr(Color::LightGray, Color::Cyan), // 0x37
                frame_active: attr(Color::White, Color::Cyan),      // 0x3F
                frame_dragging: attr(Color::LightGreen, Color::Cyan),
                handle: attr(Color::LightGreen, Color::Cyan), // 0x3A

                body: attr(Color::Black, Color::Cyan),
                body_passive: attr(Color::Black, Color::Cyan),

                scroll: attr(Color::Cyan, Color::Blue), // 0x13

                text: attr(Color::Black, Color::Cyan), // 0x30
                text_selected: attr(Color::White, Color::Cyan),
                link: attr(Color::Yellow, Color::Cyan), // 0x3E
                link_focus: attr(Color::Yellow, Color::Blue), // 0x1E
            },

            // Dialogs. Not used yet; written down now so the set is complete
            // rather than invented in a hurry the day the first dialog needs
            // it.
            gray: WinColors {
                frame_passive: attr(Color::Black, Color::LightGray),
                frame_active: attr(Color::White, Color::LightGray),
                frame_dragging: attr(Color::LightGreen, Color::LightGray),
                handle: attr(Color::LightGreen, Color::LightGray),

                body: attr(Color::Black, Color::LightGray),
                body_passive: attr(Color::Black, Color::LightGray),

                // Grey and black, not the blue window's cyan. A scrollbar is
                // part of the frame it sits on, and a cyan bar on a grey
                // dialog reads as something that fell in from another window.
                scroll: attr(Color::Black, Color::LightGray),

                text: attr(Color::Black, Color::LightGray),
                text_selected: attr(Color::White, Color::Green),
                link: attr(Color::Blue, Color::LightGray),
                link_focus: attr(Color::White, Color::Green),
            },

            // 0x70 and 0x74, measured rather than guessed from a picture. The
            // key colour is plain red, not bright red: the difference is
            // invisible in a screenshot and obvious in the attribute byte.
            menu: attr(Color::Black, Color::LightGray),          // 0x70
            menu_key: attr(Color::Red, Color::LightGray),        // 0x74
            menu_disabled: attr(Color::DarkGray, Color::LightGray), // 0x78
            menu_selected: attr(Color::Black, Color::Green),     // 0x20
            menu_selected_key: attr(Color::Red, Color::Green),   // 0x24

            // A directory is plain white and sits above the files, which is
            // what everyone who has used a commander already expects. It is
            // not given a colour of its own: its place in the list is the
            // marker, and a colour would only compete with the ones that carry
            // real news.
            file_dir: attr(Color::White, Color::Blue),
            file_exe: attr(Color::LightGreen, Color::Blue),
            file_archive: attr(Color::LightMagenta, Color::Blue),
            file_temp: attr(Color::Brown, Color::Blue),
            file_dim: attr(Color::DarkGray, Color::Blue),
            file_plain: attr(Color::LightCyan, Color::Blue),
            // Two bars, not one. A list that has the focus shows a cyan bar
            // and a list that does not shows a grey one — because the same bar
            // in both states says "Enter will open this" when Enter is going
            // to do something else entirely.
            file_selected: attr(Color::Black, Color::Cyan),
            file_selected_passive: attr(Color::Black, Color::LightGray),
            // The foot is the dialog's own grey, not a panel. Everything that
            // is a panel is coloured; everything that is the dialog talking
            // about what is in the panels is not.
            file_info: attr(Color::Black, Color::LightGray),
            file_frame: attr(Color::Cyan, Color::Blue),
            file_path: attr(Color::Yellow, Color::Blue),
            // A blue field and a cyan list, both on the dialog's grey. The
            // colours say "these are not the dialog", which is the whole
            // reason a file dialog does not look like an editor.
            file_path_label: attr(Color::Black, Color::LightGray),
            file_path_focus: attr(Color::White, Color::Blue),
            // Bright red on the window's own blue. Red on its own is not
            // enough — a message that only differs by colour is a message half
            // the readers do not get — so the text says what happened too.
            // Plain red, not bright. Bright red on light grey is the colour
            // of something on fire; this is a sentence about a folder, and it
            // should be read rather than recoiled from.
            file_error: attr(Color::Red, Color::LightGray),

            // Green, as Turbo Vision's were, and brighter under the focus.
            // Not measured: we have no reference screen with a focused button
            // on it, and these are marked a choice until we take one.
            button: attr(Color::Black, Color::Green),
            button_key: attr(Color::LightRed, Color::Green),
            button_focus: attr(Color::White, Color::Green),
            button_focus_key: attr(Color::LightRed, Color::Green),
            button_disabled: attr(Color::DarkGray, Color::LightGray),
            button_accent: attr(Color::Black, Color::Cyan),
            button_accent_key: attr(Color::LightRed, Color::Cyan),
            button_accent_focus: attr(Color::White, Color::Cyan),
            button_danger: attr(Color::White, Color::Red),
            button_danger_key: attr(Color::Yellow, Color::Red),
            button_danger_focus: attr(Color::Yellow, Color::Red),

            hex_offset: attr(Color::Cyan, Color::Blue),
            hex_byte: attr(Color::LightGray, Color::Blue),
            hex_text: attr(Color::Yellow, Color::Blue),
            hex_cursor: attr(Color::Black, Color::Cyan),

            ctl: attr(Color::Black, Color::LightGray),
            ctl_key: attr(Color::Red, Color::LightGray),
            ctl_focus: attr(Color::White, Color::Green),
            ctl_focus_key: attr(Color::Yellow, Color::Green),
            ctl_disabled: attr(Color::DarkGray, Color::LightGray),
            memo: attr(Color::Yellow, Color::Blue),
            memo_focus: attr(Color::White, Color::Blue),
            list: attr(Color::White, Color::Blue),
            list_selected: attr(Color::Black, Color::Cyan),
            list_selected_passive: attr(Color::Black, Color::LightGray),

            status: attr(Color::Black, Color::LightGray),
            status_key: attr(Color::Red, Color::LightGray),
            status_disabled: attr(Color::DarkGray, Color::LightGray),

            // CDialog's label entries: black, yellow for the letter, white
            // when its control is current. From the palette table rather
            // than measured; there is no reference scene with a label yet.
            label: attr(Color::Black, Color::LightGray),
            label_key: attr(Color::Yellow, Color::LightGray),
            label_active: attr(Color::White, Color::LightGray),
            list_marked: attr(Color::Yellow, Color::Blue),
        }
    }
}

impl Default for Palette {
    fn default() -> Self {
        Palette::classic()
    }
}

impl Palette {
    /// Every colour the palette has, by group and name, in a fixed order:
    /// what a colour dialog lists and what `get`/`set` index. Generated
    /// from the fields above; a new field is a new entry here.
    pub const NAMES: &'static [(&'static str, &'static str)] = &[
        ("Desktop", "desktop"),
        ("Desktop", "shadow"),
        ("Menus", "menu"),
        ("Menus", "menu key"),
        ("Menus", "menu disabled"),
        ("Menus", "menu selected"),
        ("Menus", "menu selected key"),
        ("File panel", "file dir"),
        ("File panel", "file exe"),
        ("File panel", "file archive"),
        ("File panel", "file temp"),
        ("File panel", "file dim"),
        ("File panel", "file plain"),
        ("File panel", "file selected"),
        ("File panel", "file selected passive"),
        ("File panel", "file info"),
        ("File panel", "file frame"),
        ("File panel", "file path"),
        ("File panel", "file path label"),
        ("File panel", "file path focus"),
        ("File panel", "file error"),
        ("Buttons", "button"),
        ("Buttons", "button key"),
        ("Buttons", "button focus"),
        ("Buttons", "button focus key"),
        ("Buttons", "button disabled"),
        ("Buttons", "button accent"),
        ("Buttons", "button accent key"),
        ("Buttons", "button accent focus"),
        ("Buttons", "button danger"),
        ("Buttons", "button danger key"),
        ("Buttons", "button danger focus"),
        ("Hex viewer", "hex offset"),
        ("Hex viewer", "hex byte"),
        ("Hex viewer", "hex text"),
        ("Hex viewer", "hex cursor"),
        ("Controls", "ctl"),
        ("Controls", "ctl key"),
        ("Controls", "ctl focus"),
        ("Controls", "ctl focus key"),
        ("Controls", "ctl disabled"),
        ("Memo", "memo"),
        ("Memo", "memo focus"),
        ("Lists", "list"),
        ("Lists", "list selected"),
        ("Lists", "list selected passive"),
        ("Status line", "status"),
        ("Status line", "status key"),
        ("Status line", "status disabled"),
        ("Labels", "label"),
        ("Labels", "label key"),
        ("Labels", "label active"),
        ("Lists", "list marked"),
        ("Blue window", "frame passive"),
        ("Blue window", "frame active"),
        ("Blue window", "frame dragging"),
        ("Blue window", "handle"),
        ("Blue window", "body"),
        ("Blue window", "body passive"),
        ("Blue window", "scroll"),
        ("Blue window", "text"),
        ("Blue window", "text selected"),
        ("Blue window", "link"),
        ("Blue window", "link focus"),
        ("Cyan window", "frame passive"),
        ("Cyan window", "frame active"),
        ("Cyan window", "frame dragging"),
        ("Cyan window", "handle"),
        ("Cyan window", "body"),
        ("Cyan window", "body passive"),
        ("Cyan window", "scroll"),
        ("Cyan window", "text"),
        ("Cyan window", "text selected"),
        ("Cyan window", "link"),
        ("Cyan window", "link focus"),
        ("Grey dialog", "frame passive"),
        ("Grey dialog", "frame active"),
        ("Grey dialog", "frame dragging"),
        ("Grey dialog", "handle"),
        ("Grey dialog", "body"),
        ("Grey dialog", "body passive"),
        ("Grey dialog", "scroll"),
        ("Grey dialog", "text"),
        ("Grey dialog", "text selected"),
        ("Grey dialog", "link"),
        ("Grey dialog", "link focus"),
    ];

    /// The attribute of entry `ix` of `NAMES`.
    pub fn get(&self, ix: usize) -> Option<u8> {
        match ix {
            0 => Some(self.desktop),
            1 => Some(self.shadow),
            2 => Some(self.menu),
            3 => Some(self.menu_key),
            4 => Some(self.menu_disabled),
            5 => Some(self.menu_selected),
            6 => Some(self.menu_selected_key),
            7 => Some(self.file_dir),
            8 => Some(self.file_exe),
            9 => Some(self.file_archive),
            10 => Some(self.file_temp),
            11 => Some(self.file_dim),
            12 => Some(self.file_plain),
            13 => Some(self.file_selected),
            14 => Some(self.file_selected_passive),
            15 => Some(self.file_info),
            16 => Some(self.file_frame),
            17 => Some(self.file_path),
            18 => Some(self.file_path_label),
            19 => Some(self.file_path_focus),
            20 => Some(self.file_error),
            21 => Some(self.button),
            22 => Some(self.button_key),
            23 => Some(self.button_focus),
            24 => Some(self.button_focus_key),
            25 => Some(self.button_disabled),
            26 => Some(self.button_accent),
            27 => Some(self.button_accent_key),
            28 => Some(self.button_accent_focus),
            29 => Some(self.button_danger),
            30 => Some(self.button_danger_key),
            31 => Some(self.button_danger_focus),
            32 => Some(self.hex_offset),
            33 => Some(self.hex_byte),
            34 => Some(self.hex_text),
            35 => Some(self.hex_cursor),
            36 => Some(self.ctl),
            37 => Some(self.ctl_key),
            38 => Some(self.ctl_focus),
            39 => Some(self.ctl_focus_key),
            40 => Some(self.ctl_disabled),
            41 => Some(self.memo),
            42 => Some(self.memo_focus),
            43 => Some(self.list),
            44 => Some(self.list_selected),
            45 => Some(self.list_selected_passive),
            46 => Some(self.status),
            47 => Some(self.status_key),
            48 => Some(self.status_disabled),
            49 => Some(self.label),
            50 => Some(self.label_key),
            51 => Some(self.label_active),
            52 => Some(self.list_marked),
            53 => Some(self.blue.frame_passive),
            54 => Some(self.blue.frame_active),
            55 => Some(self.blue.frame_dragging),
            56 => Some(self.blue.handle),
            57 => Some(self.blue.body),
            58 => Some(self.blue.body_passive),
            59 => Some(self.blue.scroll),
            60 => Some(self.blue.text),
            61 => Some(self.blue.text_selected),
            62 => Some(self.blue.link),
            63 => Some(self.blue.link_focus),
            64 => Some(self.cyan.frame_passive),
            65 => Some(self.cyan.frame_active),
            66 => Some(self.cyan.frame_dragging),
            67 => Some(self.cyan.handle),
            68 => Some(self.cyan.body),
            69 => Some(self.cyan.body_passive),
            70 => Some(self.cyan.scroll),
            71 => Some(self.cyan.text),
            72 => Some(self.cyan.text_selected),
            73 => Some(self.cyan.link),
            74 => Some(self.cyan.link_focus),
            75 => Some(self.gray.frame_passive),
            76 => Some(self.gray.frame_active),
            77 => Some(self.gray.frame_dragging),
            78 => Some(self.gray.handle),
            79 => Some(self.gray.body),
            80 => Some(self.gray.body_passive),
            81 => Some(self.gray.scroll),
            82 => Some(self.gray.text),
            83 => Some(self.gray.text_selected),
            84 => Some(self.gray.link),
            85 => Some(self.gray.link_focus),
            _ => None,
        }
    }

    /// Set entry `ix` of `NAMES`. False for no such entry.
    pub fn set(&mut self, ix: usize, attr: u8) -> bool {
        match ix {
            0 => self.desktop = attr,
            1 => self.shadow = attr,
            2 => self.menu = attr,
            3 => self.menu_key = attr,
            4 => self.menu_disabled = attr,
            5 => self.menu_selected = attr,
            6 => self.menu_selected_key = attr,
            7 => self.file_dir = attr,
            8 => self.file_exe = attr,
            9 => self.file_archive = attr,
            10 => self.file_temp = attr,
            11 => self.file_dim = attr,
            12 => self.file_plain = attr,
            13 => self.file_selected = attr,
            14 => self.file_selected_passive = attr,
            15 => self.file_info = attr,
            16 => self.file_frame = attr,
            17 => self.file_path = attr,
            18 => self.file_path_label = attr,
            19 => self.file_path_focus = attr,
            20 => self.file_error = attr,
            21 => self.button = attr,
            22 => self.button_key = attr,
            23 => self.button_focus = attr,
            24 => self.button_focus_key = attr,
            25 => self.button_disabled = attr,
            26 => self.button_accent = attr,
            27 => self.button_accent_key = attr,
            28 => self.button_accent_focus = attr,
            29 => self.button_danger = attr,
            30 => self.button_danger_key = attr,
            31 => self.button_danger_focus = attr,
            32 => self.hex_offset = attr,
            33 => self.hex_byte = attr,
            34 => self.hex_text = attr,
            35 => self.hex_cursor = attr,
            36 => self.ctl = attr,
            37 => self.ctl_key = attr,
            38 => self.ctl_focus = attr,
            39 => self.ctl_focus_key = attr,
            40 => self.ctl_disabled = attr,
            41 => self.memo = attr,
            42 => self.memo_focus = attr,
            43 => self.list = attr,
            44 => self.list_selected = attr,
            45 => self.list_selected_passive = attr,
            46 => self.status = attr,
            47 => self.status_key = attr,
            48 => self.status_disabled = attr,
            49 => self.label = attr,
            50 => self.label_key = attr,
            51 => self.label_active = attr,
            52 => self.list_marked = attr,
            53 => self.blue.frame_passive = attr,
            54 => self.blue.frame_active = attr,
            55 => self.blue.frame_dragging = attr,
            56 => self.blue.handle = attr,
            57 => self.blue.body = attr,
            58 => self.blue.body_passive = attr,
            59 => self.blue.scroll = attr,
            60 => self.blue.text = attr,
            61 => self.blue.text_selected = attr,
            62 => self.blue.link = attr,
            63 => self.blue.link_focus = attr,
            64 => self.cyan.frame_passive = attr,
            65 => self.cyan.frame_active = attr,
            66 => self.cyan.frame_dragging = attr,
            67 => self.cyan.handle = attr,
            68 => self.cyan.body = attr,
            69 => self.cyan.body_passive = attr,
            70 => self.cyan.scroll = attr,
            71 => self.cyan.text = attr,
            72 => self.cyan.text_selected = attr,
            73 => self.cyan.link = attr,
            74 => self.cyan.link_focus = attr,
            75 => self.gray.frame_passive = attr,
            76 => self.gray.frame_active = attr,
            77 => self.gray.frame_dragging = attr,
            78 => self.gray.handle = attr,
            79 => self.gray.body = attr,
            80 => self.gray.body_passive = attr,
            81 => self.gray.scroll = attr,
            82 => self.gray.text = attr,
            83 => self.gray.text_selected = attr,
            84 => self.gray.link = attr,
            85 => self.gray.link_focus = attr,
            _ => return false,
        }
        true
    }
}
