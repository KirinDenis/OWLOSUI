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
        }
    }
}

impl Default for Palette {
    fn default() -> Self {
        Palette::classic()
    }
}
