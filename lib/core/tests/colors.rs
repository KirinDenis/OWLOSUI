//! The palette by name: what a colour dialog lists and sets.

use owlosui_core::{attr, Buffer, Color, Palette, Rect, Ui};

#[test]
fn every_colour_has_a_name_and_can_be_set_by_it() {
    let p = Palette::classic();
    let n = Palette::NAMES.len();
    assert!(n > 60, "{n} entries");
    // Names are unique within their group.
    let mut seen = std::collections::HashSet::new();
    for (g, name) in Palette::NAMES {
        assert!(seen.insert((g, name)), "{g}: {name} twice");
    }
    for ix in 0..n {
        assert!(p.get(ix).is_some(), "entry {ix} readable");
    }
    assert_eq!(p.get(n), None);
    let desktop = Palette::NAMES.iter().position(|(g, nm)| *g == "Desktop" && *nm == "desktop").unwrap();
    assert_eq!(p.get(desktop), Some(p.desktop));

    // Set one, and the next frame wears it.
    let mut ui = Ui::new(20, 5);
    let mut buf = Buffer::new(20, 5);
    ui.draw(&mut buf);
    let was = buf.get(3, 3).unwrap().attr;
    assert!(ui.palette.set(desktop, attr(Color::White, Color::Red)));
    ui.draw(&mut buf);
    assert_eq!(buf.get(3, 3).unwrap().attr, attr(Color::White, Color::Red));
    assert_ne!(was, attr(Color::White, Color::Red));
    assert!(!ui.palette.set(999, 0));
    let _ = Rect::default();
}
