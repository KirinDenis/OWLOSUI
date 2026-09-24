//! Reading a directory, on this platform.
//!
//! This is the whole of the portability layer for files: one function, per
//! backend, producing what the core already knows how to show. There is no
//! trait, because nothing in the core would call one — the core is handed the
//! list and never asks for it.
//!
//! The other backends have the same shape and nothing else in common:
//!
//! * **DOS** — INT 21h 4Eh to find the first match and 4Fh for each one after,
//!   reading the attribute byte, the packed date and the size straight out of
//!   the DTA. The attributes there *are* our attributes, unconverted.
//! * **Browser** — there is no standard way to list a directory, and pretending
//!   otherwise would be the one lie in this file. It comes from a virtual file
//!   system (js-dos has one), the File System Access API where it exists, or a
//!   listing fetched from a server. All three are "the application supplies a
//!   list", which is the shape above.

use owlosui_core::files::{ATTR_ARCHIVE, ATTR_DIR, ATTR_HIDDEN, ATTR_READONLY};
use owlosui_core::FileEntry;

/// List a directory, with `..` in front unless this is a root.
pub fn read(path: &std::path::Path) -> std::io::Result<Vec<FileEntry>> {
    let mut out = Vec::new();

    if path.parent().is_some() {
        out.push(FileEntry {
            name: "..".into(),
            size: 0,
            date: (0, 0, 0, 0, 0),
            attrs: ATTR_DIR,
        });
    }

    for e in std::fs::read_dir(path)? {
        let Ok(e) = e else { continue };
        let name = e.file_name().to_string_lossy().into_owned();
        let Ok(m) = e.metadata() else { continue };

        let mut attrs = 0u8;
        if m.is_dir() {
            attrs |= ATTR_DIR;
        } else {
            // Unix has no archive bit. Rather than leave the column always
            // blank, an ordinary file carries it — which is what the bit
            // actually meant: written since the last backup, and we have no
            // backup to compare against.
            attrs |= ATTR_ARCHIVE;
        }
        if m.permissions().readonly() {
            attrs |= ATTR_READONLY;
        }
        // Unix hides by convention rather than by flag, and Windows has a real
        // bit we are not reading here; the dot rule is right on one and
        // harmless on the other.
        if name.starts_with('.') && name != ".." {
            attrs |= ATTR_HIDDEN;
        }

        out.push(FileEntry {
            name,
            size: m.len().min(u32::MAX as u64) as u32,
            date: modified(&m),
            attrs,
        });
    }

    Ok(out)
}

/// Year, month, day, hour, minute — worked out from the seconds since the
/// epoch rather than pulled in with a date library.
///
/// A calendar crate would be the sensible thing in a program that did anything
/// else with dates. This one shows a file's timestamp in a column, and the
/// arithmetic below is the whole of what that needs.
fn modified(m: &std::fs::Metadata) -> (u16, u8, u8, u8, u8) {
    let Ok(t) = m.modified() else {
        return (0, 0, 0, 0, 0);
    };
    let Ok(d) = t.duration_since(std::time::UNIX_EPOCH) else {
        return (0, 0, 0, 0, 0);
    };
    let secs = d.as_secs();
    let days = (secs / 86400) as i64;
    let sod = secs % 86400;

    // Civil date from a day count — Howard Hinnant's algorithm, which is a
    // page of shifts rather than a table of month lengths and leap years.
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u8;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
    let year = if month <= 2 { y + 1 } else { y };

    (
        year as u16,
        month,
        day,
        (sod / 3600) as u8,
        ((sod % 3600) / 60) as u8,
    )
}
