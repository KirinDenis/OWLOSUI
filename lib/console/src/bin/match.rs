//! owlosui-match <scene> <reference.bin> [--rows a:b]
//!
//! Renders a scene we also built with the real Turbo Vision and says which
//! cells disagree. `--rows` exists because every reference screen carries a
//! menu bar on row 0 and a status line on row 24, and a scene may not model
//! either; without it the report is mostly noise about work that has not been
//! done rather than work that is wrong.
//!
//! The reference dumps are not in the repository — they are the output of
//! commercial units. What is committed is what they taught us, as constants
//! in `core/src/palette.rs` with the measurement noted beside each one.

use owlosui_console::compare;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(scene), Some(path)) = (args.first(), args.get(1)) else {
        eprintln!("usage: owlosui-match <scene> <reference.bin> [--rows a:b]");
        eprintln!("scenes: {}", owlosui_console::scenes::NAMES.join(", "));
        std::process::exit(2);
    };
    let rows = args
        .iter()
        .position(|a| a == "--rows")
        .and_then(|j| args.get(j + 1))
        .and_then(|s| s.split_once(':'))
        .and_then(|(a, b)| Some((a.parse().ok()?, b.parse().ok()?)));
    let code = compare::run(scene, path, compare::Options { rows })?;
    std::process::exit(code);
}
