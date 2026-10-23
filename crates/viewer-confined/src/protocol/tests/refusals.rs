//! What the decoders refuse, by name, and that no corruption makes one panic.

use super::*;

/// A §7.11.6 value Table 47 does not describe is refused, and the refusal names the answer.
#[test]
fn a_collection_value_outside_table_47_is_refused_by_name() {
    use pdf_model::collection::{Collection, Folder, Item, Value};

    let mut values = std::collections::BTreeMap::new();
    values.insert(
        "odd".to_owned(),
        Value {
            // An array is not the text string, date string or number Table 47 describes.
            data: pdf_syntax::Object::Array(vec![pdf_syntax::Object::Integer(1)]),
            prefix: None,
        },
    );
    let collection = Collection {
        folders: Some(Folder {
            id: 0,
            name: "root".to_owned(),
            description: None,
            item: Item { values },
            has_thumbnail: false,
            children: Vec::new(),
        }),
        ..Collection::default()
    };
    let refused = encode_answer(
        &Answer::Collection {
            collection,
            initial: pdf_model::collection::Initial::Container,
            order: Vec::new(),
        },
        &Marks::default(),
    )
    .unwrap_err();
    assert_eq!(refused.message, "Answer::Collection");
}

/// A tree that nests past what this reader follows is a refusal, not a deep recursion.
///
/// Written by hand rather than by encoding a deep outline, because `pdf_model::outline` will
/// not build one — which is the point: these bytes cannot have come from that reader, and the
/// bound is on the *message*.
#[test]
fn a_tree_nested_past_the_bound_is_refused_rather_than_followed() {
    let mut bytes = vec![answer_kind::LAYERS];
    // One layer collection inside another, over and over: a count of one, the collection
    // discriminant, and no label.
    bytes.extend_from_slice(&1u64.to_be_bytes());
    for _ in 0..(panels::MAX_TREE_DEPTH + 2) {
        bytes.push(1);
        bytes.push(0);
        bytes.extend_from_slice(&1u64.to_be_bytes());
    }
    let error = decode_answer(&bytes).unwrap_err();
    assert!(
        matches!(error, ProtocolError::TooDeep { .. }),
        "a tree past the bound is refused: {error}"
    );
}

/// A structure tree whose parent links do not point backwards is refused.
///
/// A host walking the answer follows `parent` upwards; a node naming itself, or naming one it
/// has not read yet, is a loop rather than a tree — and the confined side produces the answer
/// parent-first, so a single comparison is the whole check.
#[test]
fn a_structure_node_whose_parent_is_not_behind_it_is_refused() {
    let mut bytes = vec![answer_kind::ACCESSIBILITY];
    // One page on the screen, which is page one: the answer has been a list of pages since
    // Table 29's arrangements were obeyed, and the check below is inside one of its entries.
    bytes.extend_from_slice(&1u64.to_be_bytes());
    bytes.extend_from_slice(&0u64.to_be_bytes());
    bytes.extend_from_slice(&1u64.to_be_bytes());
    bytes.push(1);
    bytes.extend_from_slice(&0u64.to_be_bytes());
    let error = decode_answer(&bytes).unwrap_err();
    assert!(
        matches!(error, ProtocolError::Unrecognised { .. }),
        "a node that is its own parent is refused: {error}"
    );
}

/// What each of the eleven costs to cross, on a document that has one of each.
///
/// A measurement rather than a threshold, and it prints rather than asserts for a reason: the
/// numbers are a property of the *document*, so a bound written here would be a bound on
/// whichever file this test happens to open. What it does assert is the thing a number cannot
/// say — that each of them round trips on real content.
///
/// Run it with `cargo test -p viewer-confined --lib -- --nocapture panels`.
#[test]
fn what_each_panel_costs_to_cross() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for name in [
        "doc/PDF20_AN002-AF.pdf",
        "doc/PDF-Declarations.pdf",
        "doc/ISO_32000-2_sponsored_EC3.pdf",
    ] {
        let Ok(bytes) = std::fs::read(root.join(name)) else {
            panic!("{name} is committed");
        };
        let mut viewer = viewer_core::Viewer::new(900, 1200, 1.0);
        for _ in viewer.handle(Command::Open {
            id: DocumentId(1),
            bytes: bytes.into(),
            password: None,
            fragment: None,
        }) {}

        println!("{name}");
        for (label, query) in [
            ("outline", Query::Outline),
            ("layers", Query::Layers),
            ("attachments", Query::Attachments),
            ("collection", Query::Collection),
            ("articles", Query::Articles),
            ("thumbnail", Query::Thumbnail(0)),
            ("properties", Query::Properties),
            ("opening", Query::Opening),
            ("preferences", Query::Preferences),
            ("popups", Query::Popups),
            ("fields", Query::Fields),
            ("structure", Query::AccessibilityTree),
        ] {
            let answer = viewer.query(query);
            let at = std::time::Instant::now();
            let encoded = encode_answer(&answer, &Marks::default()).expect("this answer crosses");
            let encoded_in = at.elapsed();
            let at = std::time::Instant::now();
            let read = decode_answer(&encoded).expect("what was written reads back");
            println!(
                "  {label:12} {:>9} bytes  encode {:>7.3} ms  decode {:>7.3} ms  {}",
                encoded.len(),
                encoded_in.as_secs_f64() * 1e3,
                at.elapsed().as_secs_f64() * 1e3,
                if matches!(read, Reply::None) {
                    "(nothing to answer with)"
                } else {
                    ""
                }
            );
        }
    }
}

#[test]
fn a_truncated_message_is_a_refusal_that_says_where() {
    let encoded = encode_command(&Command::Resize {
        width: 800,
        height: 1000,
        scale: 1.0,
    })
    .unwrap();
    let short = &encoded[..encoded.len() - 1];
    let error = decode_command(short).unwrap_err();
    assert!(
        matches!(error, ProtocolError::Truncated { what } if what == "a display scale"),
        "{error}"
    );
}

#[test]
fn a_discriminant_this_build_does_not_define_is_refused() {
    let error = decode_command(&[200]).unwrap_err();
    assert!(
        matches!(
            error,
            ProtocolError::Unrecognised {
                what: "a command",
                value: 200
            }
        ),
        "{error}"
    );
}

/// A length is a claim, and a claim from the other side of the pipe is checked.
///
/// The shape of a message from a worker that has been subverted: it says the string is four
/// gigabytes long and sends none of it. Allocating from that number is the failure.
#[test]
fn a_length_larger_than_the_message_is_refused_before_anything_is_allocated() {
    let mut bytes = vec![command_kind::EXTRACT];
    bytes.extend_from_slice(&(1u64 << 32).to_be_bytes());
    let error = decode_command(&bytes).unwrap_err();
    assert!(matches!(error, ProtocolError::Overlong { .. }), "{error}");
}

#[test]
fn bytes_left_over_are_a_refusal_rather_than_a_silence() {
    let mut encoded = encode_command(&Command::Save).unwrap();
    encoded.push(0);
    let error = decode_command(&encoded).unwrap_err();
    assert!(
        matches!(error, ProtocolError::Trailing { left: 1, .. }),
        "{error}"
    );
}

/// A frame's length field is three bytes shorter than the message it heads on purpose.
/// Every truncation and every single-byte change of a valid message, decoded.
///
/// **The confined side is the untrusted side of this boundary**, so its messages are a parser
/// over hostile input in the same sense a content stream is — and this project's rule is that
/// a parser is fuzzed. This is the deterministic half of that: every prefix of a valid
/// message and every one-byte change to it, which is where a length that is a claim, a
/// discriminant this build does not define and a `usize` that does not fit all live. The
/// requirement is not that any particular one is refused; it is that **none of them panics**,
/// because a panic here is the confined process reaching into the host.
#[test]
fn no_truncation_or_flip_of_a_message_makes_a_decoder_panic() {
    let messages = [
        encode_command(&Command::Open {
            id: DocumentId(3),
            bytes: b"%PDF-2.0\n1 0 obj".to_vec().into(),
            password: Some("p".to_owned().into()),
            fragment: None,
        })
        .unwrap(),
        encode_command(&Command::Edit(Edit::Markup {
            kind: Markup::Highlight,
            colour: [1.0, 1.0, 0.0],
        }))
        .unwrap(),
        encode_events(&[
            Event::Opened {
                document: DocumentId(3),
                pages: 5,
            },
            Event::Reported {
                document: DocumentId(3),
                page: Some(0),
                notes: vec!["a note".to_owned()],
            },
        ])
        .unwrap(),
        encode_answer(
            &Answer::Selected(viewer_core::Selected {
                text: std::borrow::Cow::Borrowed("selected"),
                quads: vec![[0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]],
            }),
            &Marks::default(),
        )
        .unwrap(),
        encode_query(Query::Find("needle")).unwrap(),
    ];

    for message in &messages {
        for cut in 0..=message.len() {
            let Some(prefix) = message.get(..cut) else {
                continue;
            };
            let _ = decode_command(prefix);
            let _ = decode_events(prefix);
            let _ = decode_answer(prefix);
            let _ = decode_query(prefix);
        }
        for at in 0..message.len() {
            for value in [0x00u8, 0x01, 0x7F, 0x80, 0xFF] {
                let mut changed = message.clone();
                if let Some(byte) = changed.get_mut(at) {
                    *byte = value;
                }
                let _ = decode_command(&changed);
                let _ = decode_events(&changed);
                let _ = decode_answer(&changed);
                let _ = decode_query(&changed);
            }
        }
    }
}

#[test]
fn a_frame_header_names_its_kind_and_length() {
    let framed = frame(FRAME_COMMAND, b"payload");
    let header: [u8; FRAME_HEADER_LEN] = framed[..FRAME_HEADER_LEN].try_into().unwrap();
    assert_eq!(parse_frame_header(header), Some((FRAME_COMMAND, 7)));
    assert_eq!(parse_frame_header([200, 0, 0, 0, 0, 0, 0, 0, 0]), None);
    // A length past `MAX_MESSAGE` is refused at the header, before a buffer is sized from it.
    let mut absurd = [0u8; FRAME_HEADER_LEN];
    absurd[0] = FRAME_EVENTS;
    absurd[1..].copy_from_slice(&(confined_transport::frame::MAX_MESSAGE + 1).to_be_bytes());
    assert_eq!(parse_frame_header(absurd), None);
}
