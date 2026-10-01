//! Compare what we draw with what the reference DOS program drew.
//!
//! A reference file is 4000 bytes — 2000 cells of glyph byte then attribute
//! byte, straight out of video memory. We render the same scene and walk both
//! grids together.
//!
//! The per-cell list is not the useful output; forty cells differing in the
//! same way is one mistake, not forty. So the report leads with the *kinds* of
//! difference, ranked by how many cells each accounts for. In practice the
//! first line names the bug.

use std::collections::BTreeMap;

use owlosui_core::Buffer;

use crate::scenes;

pub struct Options {
    pub rows: Option<(i16, i16)>,
}

pub fn run(scene: &str, path: &str, opt: Options) -> std::io::Result<i32> {
    let Some(mut ui) = scenes::build(scene) else {
        eprintln!("unknown scene '{scene}'. known: {}", scenes::NAMES.join(", "));
        return Ok(2);
    };

    let bytes = std::fs::read(path)?;
    if bytes.len() != 80 * 25 * 2 {
        eprintln!(
            "{path} is {} bytes; an 80x25 screen is {}. Wrong file, or a capture \
             taken in a different text mode.",
            bytes.len(),
            80 * 25 * 2
        );
        return Ok(2);
    }

    let mut buf = Buffer::new(80, 25);
    ui.draw(&mut buf);

    let (y0, y1) = opt.rows.unwrap_or((0, 24));

    // Kind of difference -> how many cells, and the first place it happened.
    let mut glyph_diff: BTreeMap<(u8, u8), (u32, (i16, i16))> = BTreeMap::new();
    let mut attr_diff: BTreeMap<(u8, u8), (u32, (i16, i16))> = BTreeMap::new();
    let mut total = 0u32;
    let mut checked = 0u32;

    for y in y0..=y1 {
        for x in 0..80i16 {
            let i = (y as usize * 80 + x as usize) * 2;
            let want = (bytes[i], bytes[i + 1]);
            let got = buf.get(x, y).unwrap();
            checked += 1;
            if got.ch == want.0 as owlosui_core::Glyph && got.attr == want.1 {
                continue;
            }
            total += 1;
            if got.ch != want.0 as owlosui_core::Glyph {
                let e = glyph_diff.entry((want.0, got.ch as u8)).or_insert((0, (x, y)));
                e.0 += 1;
            }
            if got.attr != want.1 {
                let e = attr_diff.entry((want.1, got.attr)).or_insert((0, (x, y)));
                e.0 += 1;
            }
        }
    }

    println!("scene '{scene}'  vs  {path}   rows {y0}..{y1}");
    println!("{total} of {checked} cells differ");
    println!();

    if total == 0 {
        println!("identical.");
        return Ok(0);
    }

    report("attributes", &attr_diff, |b| format!("0x{b:02X} {}", colour_name(b)));
    report("glyphs", &glyph_diff, |b| format!("0x{b:02X} {}", glyph_name(b)));

    // Exit code so a shell loop can tell without reading anything.
    Ok(1)
}

fn report<F: Fn(u8) -> String>(what: &str, m: &BTreeMap<(u8, u8), (u32, (i16, i16))>, fmt: F) {
    if m.is_empty() {
        return;
    }
    let mut rows: Vec<_> = m.iter().collect();
    rows.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));
    println!("{what} — expected vs ours, commonest first");
    for ((want, got), (n, (x, y))) in rows.iter().take(12) {
        println!(
            "  {:>5} cells   want {:<22} got {:<22} first at {},{}",
            n,
            fmt(*want),
            fmt(*got),
            x,
            y
        );
    }
    println!();
}

fn colour_name(a: u8) -> String {
    const N: [&str; 16] = [
        "black", "blue", "green", "cyan", "red", "magenta", "brown", "lightgray", "darkgray",
        "ltblue", "ltgreen", "ltcyan", "ltred", "ltmagenta", "yellow", "white",
    ];
    format!("{} on {}", N[(a & 0x0F) as usize], N[(a >> 4) as usize])
}

fn glyph_name(b: u8) -> String {
    let c = crate::codepage::current().to_char(b as owlosui_core::Glyph);
    if b == 0x20 {
        "space".into()
    } else {
        format!("'{c}'")
    }
}
