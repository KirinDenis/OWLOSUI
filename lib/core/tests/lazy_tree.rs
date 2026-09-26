//! A tree that is filled as it is opened: a lazy node is a branch whose
//! children the program gives when somebody opens it. A disk is not read
//! whole.

use owlosui_core::{Buffer, Event, Key, KeyCode, Kind, Mods, Rect, TreeNode, TreeView, Ui, Window};

fn key(ui: &mut Ui, code: KeyCode) {
    ui.handle(Event::Key(Key { code, mods: Mods::default() }));
}

#[test]
fn opening_a_lazy_node_asks_and_the_answer_opens_it() {
    let mut ui = Ui::new(60, 20);
    let root = ui.root();
    let w = ui.insert(root, Rect::new(0, 0, 40, 15), Kind::Window(Window::new("Tree")));
    let mut top = TreeNode::branch("C:", vec![TreeNode::lazy("Users"), TreeNode::lazy("Windows")]);
    top.open = true;
    let tree = ui.insert(w, Rect::default(), Kind::Tree(TreeView::new(vec![top])));
    ui.focus_first();
    let mut buf = Buffer::new(60, 20);
    ui.draw(&mut buf);
    let row: String = (0..40).map(|x| buf.get(x, 2).unwrap().to_char()).collect();
    assert!(row.contains("+ Users"), "a lazy node is drawn as a closed branch: {row:?}");

    // Down to Users, Right: nothing opens, the program is asked.
    key(&mut ui, KeyCode::Down);
    key(&mut ui, KeyCode::Right);
    assert_eq!(ui.tree_take_expand(tree), Some(vec![0, 0]));
    assert_eq!(ui.tree_take_expand(tree), None, "asked once");
    assert_eq!(ui.tree_texts(tree, &[0, 0]), vec!["C:".to_string(), "Users".to_string()]);
    assert_eq!(ui.tree_path(tree), vec!["C:".to_string(), "Users".to_string()]);
    // The answer: two folders, and the node opens on them.
    assert!(ui.tree_set_children(tree, &[0, 0], vec![TreeNode::lazy("Egor"), TreeNode::lazy("Public")]));
    ui.draw(&mut buf);
    let rows: Vec<String> = (0..8).map(|y| (0..40).map(|x| buf.get(x, y).unwrap().to_char()).collect()).collect();
    assert!(rows.iter().any(|r| r.contains("- Users")) && rows.iter().any(|r| r.contains("+ Egor")), "{rows:#?}");
    // Opened again: no question, it has its children now.
    key(&mut ui, KeyCode::Left);
    key(&mut ui, KeyCode::Right);
    assert_eq!(ui.tree_take_expand(tree), None);
    // An empty answer makes a leaf.
    key(&mut ui, KeyCode::Down);
    key(&mut ui, KeyCode::Right);
    assert_eq!(ui.tree_take_expand(tree), Some(vec![0, 0, 0]));
    assert!(ui.tree_set_children(tree, &[0, 0, 0], vec![]));
    ui.draw(&mut buf);
    let rows: Vec<String> = (0..8).map(|y| (0..40).map(|x| buf.get(x, y).unwrap().to_char()).collect()).collect();
    assert!(rows.iter().any(|r| r.contains("  Egor")) && !rows.iter().any(|r| r.contains("+ Egor")), "{rows:#?}");
    assert!(!ui.tree_set_children(tree, &[9], vec![]), "no such node");
    assert!(!ui.tree_set_children(w, &[0], vec![]), "not a tree");
}
