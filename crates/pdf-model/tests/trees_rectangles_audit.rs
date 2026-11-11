//! ISO 32000-2 §7.9.5: rectangle arrays emitted by the attachment writer.

use pdf_model::attachment::filing::{file_attachment_annotation, rect_around};
use pdf_syntax::{Object, ObjectId};

/// The writer emits four numeric coordinates for two opposite corners; either diagonal order
/// denotes the same bounds.
#[test]
#[expect(
    clippy::float_cmp,
    reason = "these exact small integer coordinates are only stored and reordered"
)]
fn attachment_writer_emits_four_numeric_coordinates_with_the_requested_bounds() {
    let annotation = file_attachment_annotation(
        ObjectId::new(3, 0),
        [20.0, 30.0, 10.0, 40.0],
        ObjectId::new(4, 0),
        "PushPin",
        None,
    );
    let rect = annotation
        .get("Rect")
        .and_then(Object::as_array)
        .expect("Rect array");
    assert_eq!(rect.len(), 4);
    let coordinates: Vec<f64> = rect
        .iter()
        .map(|value| value.as_number().expect("numeric coordinate"))
        .collect();
    let bounds = [
        coordinates[0].min(coordinates[2]),
        coordinates[1].min(coordinates[3]),
        coordinates[0].max(coordinates[2]),
        coordinates[1].max(coordinates[3]),
    ];
    assert_eq!(bounds, [10.0, 30.0, 20.0, 40.0]);

    let generated = rect_around((50.0, 60.0));
    assert!(generated[0] < generated[2] && generated[1] < generated[3]);
}

/// The canonical name-tree writer uses the legal one-node form and paired, byte-sorted keys.
#[test]
fn attachment_name_tree_writer_emits_a_flat_sorted_names_root() {
    use pdf_model::attachment::filing::tree_root;

    let root = tree_root(vec![
        (b"b".to_vec(), Object::Integer(3)),
        (b"a\0".to_vec(), Object::Integer(2)),
        (b"a".to_vec(), Object::Integer(1)),
    ]);
    let root = root.as_dict().expect("the root is a dictionary");
    assert!(root.get("Kids").is_none(), "flat root has no Kids");
    assert!(root.get("Limits").is_none(), "root nodes have no Limits");
    let pairs = root.get("Names").and_then(Object::as_array).expect("Names");
    assert_eq!(pairs.len(), 6, "three key/value pairs");
    let keys: Vec<&[u8]> = pairs
        .iter()
        .step_by(2)
        .map(|key| key.as_string().expect("all emitted keys are strings"))
        .collect();
    assert_eq!(keys, [&b"a"[..], b"a\0", b"b"]);
    assert_eq!(
        pairs
            .iter()
            .skip(1)
            .step_by(2)
            .map(Object::as_integer)
            .collect::<Vec<_>>(),
        [Some(1), Some(2), Some(3)],
        "each key keeps its paired value"
    );
}
