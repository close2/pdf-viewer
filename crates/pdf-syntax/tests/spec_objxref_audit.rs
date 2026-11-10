//! Focused producer assertions for ISO 32000-2 §§7.5.7–7.5.8.3.

#![expect(
    clippy::expect_used,
    reason = "test fixtures must stop immediately when construction or a required invariant fails"
)]

use std::fmt::Write as _;

use pdf_syntax::object::{Object, ObjectId};
use pdf_syntax::serialize::{
    Access, Assembly, Entropy, Form, ObjectStreams, Options, Protection, Streams, serialize,
    serialize_encrypted,
};
use pdf_syntax::{Document, Limits, Location, Version};

fn source_with_dictionaries(count: u32) -> Vec<u8> {
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for number in 1..=count {
        offsets.push(bytes.len());
        let body = if number == 1 {
            "<< /Type /Catalog /Pages 2 0 R >>".to_owned()
        } else if number == 2 {
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned()
        } else if number == 3 {
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>".to_owned()
        } else {
            format!("<< /Payload ({number}) >>")
        };
        write!(StringWriter(&mut bytes), "{number} 0 obj\n{body}\nendobj\n")
            .expect("writing to the fixture byte vector is infallible");
    }
    let xref = bytes.len();
    let size = count.checked_add(1).expect("bounded fixture object count");
    writeln!(StringWriter(&mut bytes), "xref\n0 {size}")
        .expect("writing to the fixture byte vector is infallible");
    bytes.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets {
        writeln!(StringWriter(&mut bytes), "{offset:010} 00000 n ")
            .expect("writing to the fixture byte vector is infallible");
    }
    writeln!(
        StringWriter(&mut bytes),
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF"
    )
    .expect("writing to the fixture byte vector is infallible");
    bytes
}

struct StringWriter<'a>(&'a mut Vec<u8>);

impl std::fmt::Write for StringWriter<'_> {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        self.0.extend_from_slice(text.as_bytes());
        Ok(())
    }
}

fn pack(
    source: &Document,
    count: u32,
    object_streams: ObjectStreams,
) -> (Vec<u8>, pdf_syntax::Written) {
    let mut assembly = Assembly::new(vec![source]);
    for number in 1..=count {
        assembly
            .copy(0, ObjectId::new(number, 0))
            .expect("source object is present");
    }
    assembly.set_root(ObjectId::new(1, 0));
    let mut bytes = Vec::new();
    let written = serialize(
        &assembly,
        Version { major: 1, minor: 3 },
        Options {
            form: Form::Stream,
            object_streams,
            streams: Streams::Carry,
        },
        &mut bytes,
    )
    .expect("the source assembly writes");
    (bytes, written)
}

fn open(bytes: Vec<u8>) -> Document {
    let document = Document::open(bytes).expect("the generated fixture parses");
    assert!(
        !document.was_recovered(),
        "the fixture must use its stated xref stream"
    );
    document
}

struct Counter(u8);

impl Entropy for Counter {
    fn fill(&mut self, out: &mut [u8]) -> bool {
        for byte in out {
            *byte = self.0;
            self.0 = self.0.wrapping_add(1);
        }
        true
    }
}

fn hybrid_update_with_stream_entry() -> Vec<u8> {
    let mut bytes = source_with_dictionaries(4);
    let previous = bytes
        .windows(b"startxref\n".len())
        .position(|window| window == b"startxref\n")
        .and_then(|start| {
            let number = start
                .checked_add(b"startxref\n".len())
                .expect("bounded fixture offset");
            let end = bytes[number..]
                .iter()
                .position(|byte| !byte.is_ascii_digit())?;
            let end = number.checked_add(end)?;
            std::str::from_utf8(&bytes[number..end])
                .ok()?
                .parse::<usize>()
                .ok()
        })
        .expect("the original section has a startxref offset");

    // The previous revision has object 4 in use. The next classic update frees it.
    bytes.push(b'\n');
    let hidden_at = bytes.len();
    writeln!(
        StringWriter(&mut bytes),
        "xref\n4 1\n0000000000 00000 f \ntrailer\n<< /Size 5 /Root 1 0 R /Prev {previous} >>\nstartxref\n{hidden_at}\n%%EOF"
    )
    .expect("writing to the fixture byte vector is infallible");

    // In the hybrid revision, a stream makes #4 live again. The newest classic section also
    // names the stream object itself, as §7.5.8.3 requires for a hybrid-reference file.
    bytes.push(b'\n');
    let stream_at = bytes.len();
    let object_four_offset = bytes
        .windows(b"4 0 obj".len())
        .position(|window| window == b"4 0 obj")
        .expect("object 4's header is present");
    let offset = u32::try_from(object_four_offset)
        .expect("fixture object offset fits the Table 18 four-byte field")
        .to_be_bytes();
    let data = [1, offset[0], offset[1], offset[2], offset[3], 0, 0];
    write!(
        StringWriter(&mut bytes),
        "5 0 obj\n<< /Type /XRef /Size 6 /Index [4 1] /W [1 4 2] /Root 1 0 R /Length 7 >>\nstream\n"
    )
    .expect("writing to the fixture byte vector is infallible");
    bytes.extend_from_slice(&data);
    bytes.extend_from_slice(b"\nendstream\nendobj\n");

    let table_at = bytes.len();
    writeln!(
        StringWriter(&mut bytes),
        "xref\n5 1\n{stream_at:010} 00000 n \ntrailer\n<< /Size 6 /Root 1 0 R /XRefStm {stream_at} /Prev {hidden_at} >>\nstartxref\n{table_at}\n%%EOF"
    )
    .expect("writing to the fixture byte vector is infallible");
    bytes
}

fn xref_record(kind: u8, field_two: u32, field_three: u16) -> [u8; 7] {
    let second = field_two.to_be_bytes();
    let third = field_three.to_be_bytes();
    [
        kind, second[0], second[1], second[2], second[3], third[0], third[1],
    ]
}

fn append_xref_stream(
    bytes: &mut Vec<u8>,
    number: u32,
    size: u32,
    index: &[u32],
    previous: Option<usize>,
    records: &[u8],
) -> usize {
    let offset = bytes.len();
    let pairs = index
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(" ");
    let prev = previous.map_or_else(String::new, |offset| format!(" /Prev {offset}"));
    let header = format!(
        "{number} 0 obj\n<< /Type /XRef /Size {size} /Index [{pairs}] /W [1 4 2] /Root 1 0 R /Length {}{prev} >>\nstream\n",
        records.len()
    );
    bytes.extend_from_slice(header.as_bytes());
    bytes.extend_from_slice(records);
    bytes.extend_from_slice(b"\nendstream\nendobj\n");
    offset
}

/// One member per carrier makes `/N`, `/First`, each header pair, and `/Extends` observable.
#[test]
fn generated_object_streams_state_their_members_and_form_an_extends_dag() {
    let count = 6;
    let source = open(source_with_dictionaries(count));
    let (bytes, written) = pack(
        &source,
        count,
        ObjectStreams::Generate {
            max_objects: 1,
            max_bytes: usize::MAX,
        },
    );
    assert!(
        bytes.starts_with(b"%PDF-1.5\n"),
        "compressed objects need the 1.5 floor"
    );
    assert_eq!(written.compressed, count);
    assert_eq!(written.object_streams, count);

    let document = open(bytes);
    let members: Vec<_> = (1..=count)
        .map(|number| {
            let Some(Location::InStream { stream, index }) = document.xref().location(number)
            else {
                panic!("eligible object {number} must have a type-2 location");
            };
            assert_eq!(index, 0, "each carrier has exactly one member");
            (number, stream)
        })
        .collect();
    let carriers: Vec<_> = members.iter().map(|(_, stream)| *stream).collect();
    let mut extends_edges = Vec::new();
    for (member, carrier) in &members {
        assert!(matches!(
            document.xref().location(*carrier),
            Some(Location::Offset(_))
        ));
        let object = document.get(ObjectId::new(*carrier, 0));
        let stream = object
            .as_stream()
            .expect("carrier is an indirect stream object");
        let dict = &stream.dict;
        assert_eq!(
            dict.get("Type")
                .and_then(Object::as_name)
                .map(pdf_syntax::Name::as_bytes),
            Some(&b"ObjStm"[..])
        );
        assert_eq!(dict.get("N").and_then(Object::as_integer), Some(1));
        let first = usize::try_from(
            dict.get("First")
                .and_then(Object::as_integer)
                .expect("/First is direct integer"),
        )
        .expect("/First is a nonnegative offset");
        let decoded = document
            .decoded_stream_data(stream)
            .expect("FlateDecode payload decodes");
        let mut header = pdf_syntax::Lexer::new(&decoded[..first]);
        assert_eq!(
            header.next_token(),
            Some(pdf_syntax::Token::Integer(i64::from(*member)))
        );
        assert_eq!(header.next_token(), Some(pdf_syntax::Token::Integer(0)));
        assert!(
            header.next_token().is_none(),
            "the header has exactly one pair"
        );
        assert!(first < decoded.len(), "the first object starts at /First");
        assert!(
            decoded[first..].starts_with(b"<<"),
            "only the object value follows its header"
        );
        match dict.get("Extends") {
            None => {}
            Some(Object::Reference(previous)) => {
                assert!(
                    carriers.contains(&previous.number),
                    "/Extends names a carrier"
                );
                assert_ne!(previous.number, *carrier, "/Extends does not self-link");
                assert_eq!(previous.generation, 0);
                extends_edges.push((*carrier, previous.number));
            }
            _ => panic!("/Extends is a reference to another object stream"),
        }
    }
    assert!(!extends_edges.is_empty(), "the fixture exercises /Extends");
    for (start, _) in &extends_edges {
        let mut seen = std::collections::BTreeSet::new();
        let mut current = *start;
        while let Some((_, next)) = extends_edges.iter().find(|(from, _)| *from == current) {
            assert!(
                seen.insert(current),
                "/Extends links form a directed acyclic graph"
            );
            current = *next;
        }
    }
}

/// A pure stream-form output carries Table 17's required values directly and /Root by reference.
#[test]
fn generated_cross_reference_stream_has_direct_table17_values_and_no_trailer_keyword() {
    let count = 4;
    let source = open(source_with_dictionaries(count));
    let (bytes, _) = pack(&source, count, ObjectStreams::Disable);
    assert!(!bytes.windows(7).any(|window| window == b"trailer"));
    let document = open(bytes);
    let trailer = document.trailer();

    assert_eq!(
        trailer
            .get("Type")
            .and_then(Object::as_name)
            .map(pdf_syntax::Name::as_bytes),
        Some(&b"XRef"[..])
    );
    assert_eq!(
        trailer.get("Size").and_then(Object::as_integer),
        Some(i64::from(
            count.checked_add(2).expect("bounded fixture size")
        ))
    );
    assert_eq!(
        trailer
            .get("W")
            .and_then(Object::as_array)
            .map(<[Object]>::len),
        Some(3)
    );
    assert!(
        trailer
            .get("W")
            .and_then(Object::as_array)
            .is_some_and(|values| values.iter().all(|value| value.as_integer().is_some()))
    );
    assert!(
        matches!(trailer.get("Index"), Some(Object::Array(values)) if values.len() == 2 && values.iter().all(|value| value.as_integer().is_some()))
    );
    assert_eq!(
        trailer
            .get("Index")
            .and_then(Object::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Object::as_integer)
                    .collect::<Vec<_>>()
            }),
        Some(vec![
            0,
            i64::from(count.checked_add(2).expect("bounded fixture size")),
        ])
    );
    assert!(matches!(
        document
            .xref()
            .location(count.checked_add(1).expect("bounded fixture size")),
        Some(Location::Offset(_))
    ));
    assert!(
        matches!(trailer.get("Root"), Some(Object::Reference(id)) if id.number == 1 && id.generation == 0)
    );
    assert!(
        trailer.get("F").is_none(),
        "Table 5 /F is not used for xref streams"
    );
}

/// The xref stream is parsed before authentication and therefore has to remain in the clear.
#[test]
fn encrypted_stream_form_keeps_the_cross_reference_stream_and_its_id_readable() {
    let count = 4;
    let source = open(source_with_dictionaries(count));
    let mut assembly = Assembly::new(vec![&source]);
    for number in 1..=count {
        assembly
            .copy(0, ObjectId::new(number, 0))
            .expect("source object is present");
    }
    assembly.set_root(ObjectId::new(1, 0));
    let mut bytes = Vec::new();
    serialize_encrypted(
        &assembly,
        Version { major: 1, minor: 7 },
        Options {
            form: Form::Stream,
            object_streams: ObjectStreams::Disable,
            streams: Streams::Carry,
        },
        &Protection {
            user_password: "",
            owner_password: "keeper",
            access: Access::ALL,
            encrypt_metadata: true,
        },
        &mut Counter(0),
        &mut bytes,
    )
    .expect("the encrypted source assembly writes");

    let document = Document::open_with_password(bytes, Limits::DEFAULT, "")
        .expect("the default user password opens the file");
    assert!(
        !document.was_recovered(),
        "the xref stream was parsed directly"
    );
    let trailer = document.trailer();
    assert_eq!(
        trailer
            .get("Type")
            .and_then(Object::as_name)
            .map(pdf_syntax::Name::as_bytes),
        Some(&b"XRef"[..])
    );
    assert!(matches!(trailer.get("Root"), Some(Object::Reference(_))));
    assert!(
        matches!(trailer.get("ID"), Some(Object::Array(ids)) if ids.len() == 2 && ids.iter().all(|id| id.as_string().is_some()))
    );
    assert!(
        trailer
            .get("Length")
            .is_some_and(|length| length.as_integer().is_some())
    );
    assert!(trailer.get("Filter").is_none());
    assert!(trailer.get("F").is_none());
}

/// A valid hybrid update puts the stream object's own entry in the companion classic table.
#[test]
fn hybrid_xref_stream_entry_precedes_an_older_free_entry() {
    let document = open(hybrid_update_with_stream_entry());
    assert!(matches!(
        document.xref().location(5),
        Some(Location::Offset(_))
    ));
    assert_eq!(
        document
            .get(ObjectId::new(4, 0))
            .as_dict()
            .and_then(|dict| dict.get("Payload"))
            .and_then(Object::as_string)
            .map(<[u8]>::to_vec),
        Some(b"4".to_vec()),
        "the hybrid stream's live entry outranks the older free entry"
    );
}

/// A member's object number need not follow its place in the carrier, and its value ends at
/// the next offset even when the producer writes no separator between values.
#[test]
fn object_stream_header_order_and_offsets_locate_adjacent_values() {
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let catalog_at = bytes.len();
    bytes.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let tree_at = bytes.len();
    bytes.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [4 0 R] /Count 1 >>\nendobj\n");

    let page = b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>";
    let payload = b"<< /Payload (member three) >>";
    let header = format!("4 0 3 {} ", page.len());
    let first = header.len();
    let mut member_bytes = header.into_bytes();
    member_bytes.extend_from_slice(page);
    member_bytes.extend_from_slice(payload);
    let member_offset = bytes.len();
    let member_header = format!(
        "5 0 obj\n<< /Type /ObjStm /N 2 /First {first} /Length {} >>\nstream\n",
        member_bytes.len()
    );
    bytes.extend_from_slice(member_header.as_bytes());
    bytes.extend_from_slice(&member_bytes);
    bytes.extend_from_slice(b"\nendstream\nendobj\n");

    let xref_at = bytes.len();
    let mut records = Vec::new();
    records.extend_from_slice(&xref_record(0, 0, u16::MAX));
    records.extend_from_slice(&xref_record(
        1,
        u32::try_from(catalog_at).expect("small fixture"),
        0,
    ));
    records.extend_from_slice(&xref_record(
        1,
        u32::try_from(tree_at).expect("small fixture"),
        0,
    ));
    records.extend_from_slice(&xref_record(2, 5, 1));
    records.extend_from_slice(&xref_record(2, 5, 0));
    records.extend_from_slice(&xref_record(
        1,
        u32::try_from(member_offset).expect("small fixture"),
        0,
    ));
    records.extend_from_slice(&xref_record(
        1,
        u32::try_from(xref_at).expect("small fixture"),
        0,
    ));
    append_xref_stream(&mut bytes, 6, 7, &[0, 7], None, &records);
    writeln!(StringWriter(&mut bytes), "startxref\n{xref_at}\n%%EOF")
        .expect("writing to fixture vector is infallible");

    let document = open(bytes);
    assert_eq!(
        document
            .get(ObjectId::new(3, 0))
            .as_dict()
            .and_then(|dict| dict.get("Payload"))
            .and_then(Object::as_string),
        Some(&b"member three"[..]),
        "object 3 is the second header member despite preceding object 4 in number order"
    );
    let catalog = document.catalog().expect("catalog resolves");
    let pages_object = document.get_key(&catalog, "Pages");
    let pages = pages_object.as_dict().expect("catalog page tree resolves");
    assert_eq!(
        document.get_key(pages, "Count").as_integer(),
        Some(1),
        "the first page value ends exactly at object 3's relative offset"
    );
}

#[test]
fn a_previous_pure_cross_reference_stream_is_followed_and_updated_entry_wins() {
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let catalog_at = bytes.len();
    bytes.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let tree_at = bytes.len();
    bytes.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    let page_at = bytes.len();
    bytes.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>\nendobj\n",
    );
    let old_at = bytes.len();
    bytes.extend_from_slice(b"5 0 obj\n(old value)\nendobj\n");

    let previous = bytes.len();
    let mut initial_records = Vec::new();
    initial_records.extend_from_slice(&xref_record(0, 0, u16::MAX));
    initial_records.extend_from_slice(&xref_record(
        1,
        u32::try_from(catalog_at).expect("small fixture"),
        0,
    ));
    initial_records.extend_from_slice(&xref_record(
        1,
        u32::try_from(tree_at).expect("small fixture"),
        0,
    ));
    initial_records.extend_from_slice(&xref_record(
        1,
        u32::try_from(page_at).expect("small fixture"),
        0,
    ));
    initial_records.extend_from_slice(&xref_record(
        1,
        u32::try_from(previous).expect("small fixture"),
        0,
    ));
    initial_records.extend_from_slice(&xref_record(
        1,
        u32::try_from(old_at).expect("small fixture"),
        0,
    ));
    append_xref_stream(&mut bytes, 4, 6, &[0, 6], None, &initial_records);
    writeln!(StringWriter(&mut bytes), "startxref\n{previous}\n%%EOF")
        .expect("writing to fixture vector is infallible");

    bytes.push(b'\n');
    let new_at = bytes.len();
    bytes.extend_from_slice(b"5 0 obj\n(new value)\nendobj\n");
    let latest = bytes.len();
    let mut update_records = Vec::new();
    update_records.extend_from_slice(&xref_record(
        1,
        u32::try_from(new_at).expect("small fixture"),
        0,
    ));
    update_records.extend_from_slice(&xref_record(
        1,
        u32::try_from(latest).expect("small fixture"),
        0,
    ));
    append_xref_stream(&mut bytes, 6, 7, &[5, 2], Some(previous), &update_records);
    writeln!(StringWriter(&mut bytes), "startxref\n{latest}\n%%EOF")
        .expect("writing to fixture vector is infallible");

    let document = open(bytes);
    assert_eq!(
        document.get(ObjectId::new(5, 0)).as_string(),
        Some(&b"new value"[..]),
        "the latest stream section overrides the previous stream entry"
    );
}
