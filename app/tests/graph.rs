//! The graph: the name that keeps the nodes it matches, and a node dragged
//! where this device keeps it.

mod common;

use std::{collections::HashMap, fs, sync::Arc, time::Duration};

use common::{desktop, vault};
use nsh_core::{
    prelude::{point, px},
    storage::StorageBackend,
};
use nsh_testing::{DragOptions, Headless, MemoryStorage};

/// The places the device keeps for the vault at `root`, by node.
fn kept(stored: &MemoryStorage, root: &std::path::Path) -> HashMap<String, (f32, f32)> {
    let entries = stored.entries();
    let json = entries
        .get(&format!("graph/{}", root.display()))
        .expect("the graph's places are kept");
    serde_json::from_str(json).expect("a map of node to (x, y)")
}

/// The name filter keeps the nodes it matches, with the links between them;
/// emptied, every node comes back. The other filters stay as they were.
#[test]
fn desktop_filters_the_nodes_by_name() {
    let root = vault("graph-names");
    fs::write(root.join("A.md"), "[[Bee]] and [[Later]]\n").unwrap();
    fs::write(root.join("Bee.md"), "# The bee\n").unwrap();
    let mut app = Headless::start(desktop(&root)).unwrap();
    app.wait_for("Bee").unwrap();

    // Linked notes only, at first: A and Bee.
    app.press_key("mod+g").unwrap();
    app.wait_for("id:Bee.md").unwrap();
    assert!(app.screen().matches("id:A.md").unwrap());

    // The other filters still add what they did.
    app.press("Uncreated notes").unwrap();
    app.wait_for("id:Later").unwrap();

    // A name keeps the node it matches: A and Later, whose names it is
    // not, go.
    app.type_text("label:Filter by name", "be").unwrap();
    app.wait_for_gone("id:A.md", Duration::from_secs(2))
        .unwrap();
    assert!(!app.screen().matches("id:Later").unwrap());
    assert!(app.screen().matches("id:Bee.md").unwrap());

    // A name no node holds keeps none of them.
    app.clear("label:Filter by name").unwrap();
    app.type_text("label:Filter by name", "zed").unwrap();
    app.wait_for_gone("id:Bee.md", Duration::from_secs(2))
        .unwrap();

    // Emptied, the graph is what the filters alone say.
    app.clear("label:Filter by name").unwrap();
    app.wait_for("id:Bee.md").unwrap();
    assert!(app.screen().matches("id:A.md").unwrap());
    assert!(app.screen().matches("id:Later").unwrap());
    let _ = fs::remove_dir_all(&root);
}

/// A drag that starts on a node moves that node, and only it, with the
/// pointer: the place goes to this device's storage, the note's file never
/// hears of it, and the graph around it stays where it was.
#[test]
fn desktop_drags_a_node_and_keeps_it_on_the_device() {
    let root = vault("graph-drag");
    fs::write(root.join("A.md"), "[[Bee]]\n").unwrap();
    fs::write(root.join("Bee.md"), "# The bee\n").unwrap();
    let stored = MemoryStorage::new([("vault".to_owned(), root.display().to_string())]);
    let mut config = desktop(&root);
    config.storage = Some(stored.clone() as Arc<dyn StorageBackend>);
    let mut app = Headless::start(config).unwrap();
    app.wait_for("Bee").unwrap();
    app.press_key("mod+g").unwrap();
    app.wait_for("id:Bee.md").unwrap();

    // 60 × 40 points down and right: the bee is kept there, and only it.
    let by = point(px(60.0), px(40.0));
    app.drag("id:Bee.md", by, DragOptions::default()).unwrap();
    app.wait_for("id:Bee.md").unwrap();
    let places = kept(&stored, &root);
    let (bee, &(x, y)) = places.iter().next().expect("one node moved");
    assert_eq!(bee, "Bee.md");
    assert_eq!(places.len(), 1, "A stayed where the layout put it");
    assert!(x.is_finite() && y.is_finite());

    // Back the same way: the place it started from, not further on.
    let back = point(px(-60.0), px(-40.0));
    app.drag("id:Bee.md", back, DragOptions::default()).unwrap();
    let places = kept(&stored, &root);
    let (_, &(x2, y2)) = places.iter().next().unwrap();
    assert!(
        x2 < x - 30.0 && y2 < y - 20.0,
        "back: ({x}, {y}) → ({x2}, {y2})"
    );

    // The note keeps its text: a drag writes nothing into the vault.
    assert_eq!(
        fs::read_to_string(root.join("Bee.md")).unwrap(),
        "# The bee\n"
    );

    // And the node is still the bee's: a press opens its note.
    app.press("id:Bee.md").unwrap();
    app.wait_for_gone("label:Filter by name", Duration::from_secs(2))
        .unwrap();
    let _ = fs::remove_dir_all(&root);
}
