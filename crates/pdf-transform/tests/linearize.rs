//! `optimize --linearize`: ISO 32000-2 Annex F's linearised file, read back and held to the annex.
//!
//! Every expected value here is derived from Annex F and nothing else. The reader of the hint
//! tables below is written from Tables F.3 to F.9 — field widths, item order, §F.4.1's "as if the
//! primary hint stream itself were not present" — and each test asks the written file where an
//! object *is* (its cross-reference entry) and whether the parameter dictionary and the hint
//! tables said the same (trap 8: the output is asked whether it conforms, not whether it matches
//! what the writer intended).
//!
//! The fixtures are built here, object by object, because each one has to exercise a sentence of
//! the annex that the corpus may not: a first page that shares objects with later pages, an object
//! two later pages share and the first does not (§F.3.9's part 8), inherited page attributes
//! (§F.3.10's push-down), an outline with a closed item (§F.3.10's display order), an `/OpenAction`
//! naming page 2 (§F.3.7's `/P`), and `/PageMode /UseOutlines` (§F.3.7's outline in part 6). The
//! corpus walk is `tests/optimize_corpus.rs`'s linearised arm. `qpdf --check`, where installed, is
//! evidence about the reading and never its definition (principle 5).

#![expect(
    clippy::arithmetic_side_effects,
    clippy::too_many_lines,
    clippy::struct_excessive_bools,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stderr,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly, a skipped \
              comparison says so, and the fixture is one document written out object by object, \
              its shape a handful of independent switches"
)]

mod support;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::process::Command;

use pdf_model::Pages;
use pdf_syntax::linearize::{State, state};
use pdf_syntax::serialize::{ObjectStreams, Streams};
use pdf_syntax::{Document, Limits, Object, ObjectId};
use pdf_transform::optimize::OptimizePlan;
use pdf_transform::render::{ImageFormat, RenderPlan, Sizing};
use pdf_transform::{
    Budget, MemorySinks, Plan, Policy, Protect, Refusal, Secret, Source, apply, apply_protected,
};

use support::linearized::{
    PageHint, Read, faults, faults_with, generic, object_at, page_offsets, shared_objects,
};

/// The test's own shorthand over the reader: a file that is not linearised, or an object that
/// is not at an offset, is a failure here rather than a fault to report.
trait Must {
    /// Opens a file this suite wrote.
    fn of(bytes: Vec<u8>) -> Self;
    /// An object's offset.
    fn at(&self, number: u64) -> u64;
    /// Page `index`'s object number.
    fn page_number(&self, index: usize) -> u64;
}

impl Must for Read {
    fn of(bytes: Vec<u8>) -> Self {
        Read::open(bytes).unwrap_or_else(|fault| panic!("{fault}"))
    }

    fn at(&self, number: u64) -> u64 {
        self.placed(number)
            .unwrap_or_else(|| panic!("object {number} is not in the file"))
    }

    fn page_number(&self, index: usize) -> u64 {
        *self.page_numbers.get(index).expect("a page")
    }
}

/// What varies between the fixtures.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct Shape {
    /// The catalog's `/OpenAction` names page 2.
    open_at_page_two: bool,
    /// The catalog's `/OpenAction` names the first page's object itself, which Table 29 does not
    /// admit: the page is still F.3.7's to place, not F.3.5's.
    open_action_is_a_page: bool,
    /// Every construct Table F.2 attaches a table to: thumbnails, an article thread, named
    /// destinations, a form, a structure tree, a renditions tree and an embedded file.
    every_table: bool,
    /// The catalog's `/PageMode` is `/UseOutlines`.
    use_outlines: bool,
    /// Two article threads cross page one, and the producer stated neither a page's `/B` nor any
    /// bead's `/T` but the first's, which Table 163 and Table 31 permit and §F.3.7 (b) does not.
    threads: bool,
    /// Written with §7.5.7's object streams, and so with §7.5.8's cross-reference streams.
    object_streams: bool,
}

/// A classic PDF file out of numbered object bodies, with a correct §7.5.4 table.
fn file(objects: &[(u32, Vec<u8>)], root: u32, info: Option<u32>) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = BTreeMap::new();
    for (number, body) in objects {
        offsets.insert(*number, out.len());
        out.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let size = offsets.keys().max().copied().unwrap_or(0) + 1;
    let at = out.len();
    let mut table = format!("xref\n0 {size}\n0000000000 65535 f \n");
    for number in 1..size {
        match offsets.get(&number) {
            Some(offset) => {
                let _ = writeln!(table, "{offset:010} 00000 n ");
            }
            None => table.push_str("0000000000 65535 f \n"),
        }
    }
    let info = info.map_or(String::new(), |info| format!(" /Info {info} 0 R"));
    let _ = write!(
        table,
        "trailer\n<< /Size {size} /Root {root} 0 R{info} >>\nstartxref\n{at}\n%%EOF\n"
    );
    out.extend_from_slice(table.as_bytes());
    out
}

/// A stream object's body.
fn stream(dict: &str, data: &[u8]) -> Vec<u8> {
    let mut out = format!("<< {dict} /Length {} >>\nstream\n", data.len()).into_bytes();
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream");
    out
}

/// A three-page document shaped to exercise Annex F's sentences.
///
/// Page 1 and page 2 inherit `/MediaBox` and a `/Resources` naming font 10 from the root node;
/// page 2 also names font 11 in its own resources, and so does page 3, so font 11 and its
/// `/Widths` array 14 are shared by two pages after the first (part 8), while font 10 is shared by
/// the first page and page 2 (part 6's second run). Image 13 is page 3's alone.
fn fixture(shape: Shape) -> Vec<u8> {
    let mut catalog =
        "<< /Type /Catalog /Pages 2 0 R /Outlines 20 0 R /PageLabels 31 0 R".to_owned();
    if shape.open_at_page_two {
        catalog.push_str(" /OpenAction [4 0 R /Fit]");
    }
    if shape.use_outlines {
        catalog.push_str(" /PageMode /UseOutlines");
    }
    if shape.open_action_is_a_page {
        catalog.push_str(" /OpenAction 3 0 R");
    }
    if shape.threads {
        catalog.push_str(" /Threads [100 0 R 110 0 R]");
    }
    if shape.every_table {
        catalog.push_str(
            " /Threads [50 0 R] /Dests 95 0 R /AcroForm << /Fields [60 0 R] >> \
             /StructTreeRoot 70 0 R /Names << /Renditions 80 0 R /EmbeddedFiles 90 0 R >>",
        );
    }
    catalog.push_str(" >>");
    let (page_one, page_two, page_three): (&[u8], &[u8], &[u8]) = if shape.every_table {
        (
            b"<< /Type /Page /Parent 2 0 R /Contents 6 0 R /Thumb 40 0 R /B [52 0 R] >>",
            b"<< /Type /Page /Parent 2 0 R /Contents 7 0 R /Annots [12 0 R 61 0 R] \
              /Resources << /Font << /F1 10 0 R /F2 11 0 R >> >> >>",
            b"<< /Type /Page /Parent 2 0 R /Contents 8 0 R /MediaBox [0 0 300 200] /Thumb 41 0 R \
              /Resources << /Font << /F2 11 0 R >> /XObject << /Im 13 0 R >> >> >>",
        )
    } else {
        (
            b"<< /Type /Page /Parent 2 0 R /Contents 6 0 R >>",
            b"<< /Type /Page /Parent 2 0 R /Contents 7 0 R /Annots [12 0 R] \
              /Resources << /Font << /F1 10 0 R /F2 11 0 R >> >> >>",
            b"<< /Type /Page /Parent 2 0 R /Contents 8 0 R /MediaBox [0 0 300 200] \
              /Resources << /Font << /F2 11 0 R >> /XObject << /Im 13 0 R >> >> >>",
        )
    };
    let mut objects: Vec<(u32, Vec<u8>)> =
        vec![
        (1, catalog.into_bytes()),
        (
            2,
            b"<< /Type /Pages /Kids [3 0 R 4 0 R 5 0 R] /Count 3 /MediaBox [0 0 200 200] \
              /Resources << /Font << /F1 10 0 R >> >> >>"
                .to_vec(),
        ),
        (3, page_one.to_vec()),
        (4, page_two.to_vec()),
        (5, page_three.to_vec()),
        (6, stream("", b"BT /F1 24 Tf 20 100 Td (one) Tj ET")),
        (
            7,
            stream("", b"BT /F1 24 Tf 20 100 Td (two) Tj /F2 24 Tf ( and) Tj ET"),
        ),
        (
            8,
            stream(
                "",
                b"q 80 0 0 80 10 10 cm /Im Do Q BT /F2 24 Tf 120 150 Td (three) Tj ET",
            ),
        ),
        (
            10,
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
        ),
        (
            11,
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Times-Roman /FirstChar 32 /LastChar 32 \
              /Widths 14 0 R >>"
                .to_vec(),
        ),
        (
            12,
            b"<< /Type /Annot /Subtype /Link /Rect [10 10 60 60] /Border [0 0 0] \
              /Dest [3 0 R /Fit] >>"
                .to_vec(),
        ),
        (
            13,
            stream(
                "/Type /XObject /Subtype /Image /Width 2 /Height 2 /ColorSpace /DeviceGray \
                 /BitsPerComponent 8",
                b"\x00\xff\xff\x00",
            ),
        ),
        (14, b"[250]".to_vec()),
        (
            20,
            b"<< /Type /Outlines /First 21 0 R /Last 22 0 R /Count 3 >>".to_vec(),
        ),
        (
            21,
            b"<< /Title (One) /Parent 20 0 R /Next 22 0 R /First 23 0 R /Last 23 0 R \
              /Count -1 /Dest [3 0 R /Fit] >>"
                .to_vec(),
        ),
        (
            22,
            b"<< /Title (Two) /Parent 20 0 R /Prev 21 0 R /First 24 0 R /Last 24 0 R \
              /Count 1 /Dest [4 0 R /Fit] >>"
                .to_vec(),
        ),
        (
            23,
            b"<< /Title (One.a) /Parent 21 0 R /Dest [5 0 R /Fit] >>".to_vec(),
        ),
        (
            24,
            b"<< /Title (Two.a) /Parent 22 0 R /Dest [5 0 R /Fit] >>".to_vec(),
        ),
        (30, b"<< /Title (Linearised) /Producer (fixture) >>".to_vec()),
        (31, b"<< /Nums [0 << /S /D >>] >>".to_vec()),
    ];
    if shape.every_table {
        let thumbnail = "/Width 1 /Height 1 /ColorSpace 42 0 R /BitsPerComponent 8";
        objects.extend([
            (40, stream(thumbnail, b"\x00")),
            (41, stream(thumbnail, b"\x01")),
            (42, b"[/Indexed /DeviceRGB 1 <000000FFFFFF>]".to_vec()),
            (50, b"<< /Type /Thread /I 51 0 R /F 52 0 R >>".to_vec()),
            (51, b"<< /Title (Article) >>".to_vec()),
            (
                52,
                b"<< /Type /Bead /T 50 0 R /N 52 0 R /V 52 0 R /P 3 0 R /R [0 0 100 100] >>"
                    .to_vec(),
            ),
            (60, b"<< /FT /Tx /T (field) /Kids [61 0 R] >>".to_vec()),
            (
                61,
                b"<< /Type /Annot /Subtype /Widget /Parent 60 0 R /Rect [100 100 150 120] >>"
                    .to_vec(),
            ),
            (70, b"<< /Type /StructTreeRoot /K 71 0 R >>".to_vec()),
            (
                71,
                b"<< /Type /StructElem /S /P /P 70 0 R /Pg 3 0 R /K 0 >>".to_vec(),
            ),
            (80, b"<< /Names [(r) 81 0 R] >>".to_vec()),
            (81, b"<< /Type /Rendition /S /MR /N (movie) >>".to_vec()),
            (90, b"<< /Names [(a.txt) 91 0 R] >>".to_vec()),
            (
                91,
                b"<< /Type /Filespec /F (a.txt) /EF << /F 92 0 R >> >>".to_vec(),
            ),
            (92, stream("/Type /EmbeddedFile", b"hello")),
            (95, b"<< /d1 [3 0 R /Fit] >>".to_vec()),
        ]);
        objects.sort_by_key(|(number, _)| *number);
    }
    if shape.threads {
        // Thread A runs from page one to page two, thread B from page three back to page one; only
        // each thread's first bead states `/T`, as Table 163 permits.
        objects.extend([
            (100, b"<< /Type /Thread /F 101 0 R >>".to_vec()),
            (
                101,
                b"<< /Type /Bead /T 100 0 R /N 102 0 R /V 102 0 R /P 3 0 R /R [0 0 90 90] >>"
                    .to_vec(),
            ),
            (
                102,
                b"<< /Type /Bead /N 101 0 R /V 101 0 R /P 4 0 R /R [0 0 90 90] >>".to_vec(),
            ),
            (110, b"<< /Type /Thread /F 111 0 R >>".to_vec()),
            (
                111,
                b"<< /Type /Bead /T 110 0 R /N 112 0 R /V 112 0 R /P 5 0 R /R [0 0 90 90] >>"
                    .to_vec(),
            ),
            (
                112,
                b"<< /Type /Bead /N 111 0 R /V 111 0 R /P 3 0 R /R [100 100 190 190] >>".to_vec(),
            ),
        ]);
    }
    file(&objects, 1, Some(30))
}

/// `optimize --linearize`'s plan, with §7.5.7's object streams as the shape asks.
fn linear_plan(shape: Shape, streams: Streams) -> OptimizePlan {
    OptimizePlan {
        source: 0,
        names: "out.pdf".parse().expect("a pattern"),
        prune: true,
        object_streams: if shape.object_streams {
            ObjectStreams::DEFAULT
        } else {
            ObjectStreams::Disable
        },
        streams,
        linearize: true,
    }
}

/// Runs `optimize` under a plan, answering the one output or the refusal.
fn run(bytes: &[u8], plan: OptimizePlan) -> Result<Vec<u8>, Refusal> {
    let sinks = MemorySinks::new();
    apply(
        &Plan::Optimize(plan),
        &[Source::new(bytes.to_vec())],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )?;
    let mut outputs = sinks.into_outputs();
    assert_eq!(outputs.len(), 1, "one input, one output");
    Ok(outputs.remove(0).1)
}

/// The fixture, linearised.
fn linearised(shape: Shape) -> Vec<u8> {
    run(&fixture(shape), linear_plan(shape, Streams::Carry)).expect("the fixture linearises")
}

/// Runs `optimize` with passwords, answering the one output.
fn run_protected(bytes: &[u8], plan: OptimizePlan, protect: &Protect) -> Vec<u8> {
    let sinks = MemorySinks::new();
    let source = Source::new(bytes.to_vec());
    apply_protected(
        &Plan::Optimize(plan),
        &[&source],
        &sinks,
        &Policy::default(),
        &Budget::default(),
        Some(protect),
    )
    .expect("the fixture linearises encrypted");
    let mut outputs = sinks.into_outputs();
    assert_eq!(outputs.len(), 1, "one input, one output");
    outputs.remove(0).1
}

/// Every fixture, once with classic tables and once with object streams.
fn fixtures() -> Vec<(String, Shape)> {
    let mut out = Vec::new();
    for (name, shape) in base_fixtures() {
        out.push((name.to_owned(), shape));
        out.push((
            format!("{name}, in object streams"),
            Shape {
                object_streams: true,
                ..shape
            },
        ));
    }
    out
}

/// The document's own shapes.
fn base_fixtures() -> Vec<(&'static str, Shape)> {
    vec![
        ("plain", Shape::default()),
        (
            "opening at page 2",
            Shape {
                open_at_page_two: true,
                ..Shape::default()
            },
        ),
        (
            "outline shown on opening",
            Shape {
                use_outlines: true,
                ..Shape::default()
            },
        ),
        (
            "an /OpenAction naming a page object",
            Shape {
                open_action_is_a_page: true,
                ..Shape::default()
            },
        ),
        (
            "every table",
            Shape {
                every_table: true,
                ..Shape::default()
            },
        ),
        (
            "two threads crossing page one",
            Shape {
                threads: true,
                ..Shape::default()
            },
        ),
    ]
}

/// Table F.1's entries, each checked against the file it describes.
#[test]
fn every_value_of_the_parameter_dictionary_is_where_the_file_says() {
    for (name, shape) in fixtures() {
        let read = Read::of(linearised(shape));
        let p = read.parameters;
        let bytes = &read.bytes;
        // /L: "It shall be exactly equal to the actual length of the PDF file."
        assert_eq!(p.length, bytes.len() as u64, "{name}: /L");
        // §F.3.3: the dictionary is the first object and inside the first 1024 bytes.
        let header_end = bytes
            .iter()
            .skip(1)
            .position(|b| *b == b'\n')
            .expect("a line")
            + 2;
        let second_line = bytes[header_end..]
            .iter()
            .position(|b| *b == b'\n')
            .expect("a line");
        let first_object = header_end + second_line + 1;
        let parameters_number = object_at(bytes, first_object as u64).expect("part 2");
        let end = bytes[first_object..]
            .windows(6)
            .position(|w| w == b"endobj")
            .expect("its endobj")
            + first_object
            + 6;
        assert!(end <= 1024, "{name}: F.3.3's first 1024 bytes");
        assert_eq!(
            read.at(u64::from(parameters_number)),
            first_object as u64,
            "{name}: F.3.4: \"this cross-reference table shall contain entries for the \
             linearization parameter dictionary (at the beginning)\""
        );
        // /H: "offset 1 shall be the offset of the primary hint stream … length 1 shall be the
        // length of this stream, including stream object overhead."
        let (offset, length) = p.primary_hints;
        let hint_number = object_at(bytes, offset).expect("part 5");
        assert_eq!(read.at(u64::from(hint_number)), offset, "{name}: /H offset");
        let last = usize::try_from(offset + length).expect("an end");
        assert!(
            bytes[..last].ends_with(b"endobj\n"),
            "{name}: /H length ends at endobj"
        );
        // §F.3.6: "The hint streams shall be assigned the last object numbers in the PDF file".
        let size = read
            .document
            .trailer()
            .get("Size")
            .and_then(Object::as_integer)
            .expect("a /Size");
        assert_eq!(
            i64::from(hint_number),
            size - 1,
            "{name}: the hint stream is numbered last"
        );
        // /O: "The object number of the first page's page object."
        let first = usize::try_from(p.first_page).expect("an index");
        assert_eq!(
            u64::from(p.first_page_object),
            read.page_number(first),
            "{name}: /O"
        );
        // /N.
        assert_eq!(p.pages, 3, "{name}: /N");
        let t = usize::try_from(p.main_cross_reference).expect("an offset");
        let prev = read
            .document
            .trailer()
            .get("Prev")
            .and_then(Object::as_integer)
            .expect("a /Prev");
        let prev = usize::try_from(prev).expect("an offset");
        // §F.3.11: "The startxref line shall give the offset of the first-page cross-reference
        // table in the PDF file."
        let startxref = support::linearized::startxref(bytes).expect("a startxref");
        assert!(
            startxref < 1024 && startxref > first_object,
            "{name}: part 3 follows part 2"
        );
        assert_eq!(
            read.streams_form(),
            shape.object_streams,
            "{name}: Table 18's type 2 entry needs a cross-reference stream, and only then is one written"
        );
        if shape.object_streams {
            // Table F.1's `/T` for "[d]ocuments that use cross-reference streams exclusively":
            // "the offset of the main cross-reference stream object in the PDF file"; and §F.3.4's
            // `/Prev`, "with the appropriate syntactic changes", names the same object.
            let main = object_at(bytes, t as u64).expect("an object at /T");
            let main = read.document.get(ObjectId::new(main, 0));
            let main = main.as_stream().expect("a stream");
            assert_eq!(
                main.dict
                    .get("Type")
                    .and_then(Object::as_name)
                    .map(|n| n.as_bytes().to_vec()),
                Some(b"XRef".to_vec()),
                "{name}: /T is the main cross-reference stream"
            );
            // "The main trailer has no Prev entry".
            assert!(
                main.dict.get("Prev").is_none(),
                "{name}: no /Prev in the main stream"
            );
            assert_eq!(prev, t, "{name}: /Prev");
            // §F.3.4's "single cross-reference subsection that has no free entries", from the
            // parameter dictionary to the hint stream: `/Index [k+1 n]` and `/Size` k + 1 + n.
            let first = object_at(bytes, startxref as u64).expect("an object at startxref");
            assert_eq!(
                first,
                parameters_number + 1,
                "{name}: part 3 follows part 2 in number"
            );
            let first = read.document.get(ObjectId::new(first, 0));
            let index = first
                .as_stream()
                .and_then(|stream| stream.dict.get("Index"))
                .and_then(Object::as_array)
                .expect("an /Index")
                .iter()
                .filter_map(Object::as_integer)
                .collect::<Vec<i64>>();
            assert_eq!(
                index,
                vec![
                    i64::from(parameters_number),
                    size - i64::from(parameters_number)
                ],
                "{name}: the first-page section runs from the parameter dictionary to the end"
            );
        } else {
            // /T: "the offset of the white-space character preceding the first entry of the main
            // cross-reference table (the entry for object number 0)".
            assert!(bytes[t].is_ascii_whitespace(), "{name}: /T is white space");
            assert_eq!(
                &bytes[t + 1..t + 20],
                b"0000000000 65535 f ",
                "{name}: /T precedes entry 0"
            );
            // §F.3.4: "The trailer's Prev entry shall give the offset of the main cross-reference
            // table", and the main table is the one /T is inside.
            assert_eq!(&bytes[prev..prev + 4], b"xref", "{name}: /Prev");
            assert!(prev < t, "{name}: /T is inside the table /Prev names");
            assert_eq!(
                &bytes[startxref..startxref + 4],
                b"xref",
                "{name}: startxref"
            );
        }
        // /P: stated where the first page is not page 0, and absent where it is.
        let expected_first = u64::from(shape.open_at_page_two);
        assert_eq!(p.first_page, expected_first, "{name}: /P");
    }
}

/// §F.4.2's page offset hint table: every page is where its entry says, and holds what it says.
#[test]
fn every_page_offset_hint_names_the_page_s_own_objects_where_they_are() {
    for (name, shape) in fixtures() {
        let read = Read::of(linearised(shape));
        let table = page_offsets(&read.data, 3);
        let order = read.page_order();
        // Item 1: "The first object of the first page shall have an object number that is the
        // value of the O entry … The first object of the second page shall have an object number
        // of 1. Object numbers for subsequent pages shall be determined by accumulating the
        // number of objects in all previous pages."
        // Item 2: "The locations of subsequent pages shall be determined by accumulating the
        // lengths of all previous pages."
        let mut number = u64::from(read.parameters.first_page_object);
        let mut location = table.first_page_location;
        assert_eq!(
            read.actual(location),
            read.at(number),
            "{name}: header item 2 is the first page's page object"
        );
        for (at, (page, hint)) in order.iter().zip(&table.pages).enumerate() {
            if at == 1 {
                number = 1;
            }
            let start = read.actual(location);
            assert_eq!(
                start,
                read.at(number),
                "{name}: page {page} begins at its entry"
            );
            assert_eq!(
                number,
                read.page_number(*page),
                "{name}: page {page}'s first object is its page"
            );
            // Every object of the page lies inside its byte range, one after another.
            for member in number..number + hint.objects {
                let offset = read.at(member);
                assert!(
                    offset >= start && offset < start + hint.length,
                    "{name}: object {member} of page {page} lies within the page"
                );
            }
            // Items 6 and 7: the content stream "relative to the beginning of the page" and "its
            // length … including object overhead".
            let contents = read.document.get(ObjectId::new(
                u32::try_from(read.page_number(*page)).expect("n"),
                0,
            ));
            let contents = contents
                .as_dict()
                .and_then(|dict| dict.get("Contents"))
                .and_then(Object::as_reference)
                .expect("a content stream");
            if hint.content_length > 0 {
                assert_eq!(
                    start + hint.content_offset,
                    read.at(u64::from(contents.number)),
                    "{name}: page {page}'s content stream is where item 6 says"
                );
                let end = usize::try_from(start + hint.content_offset + hint.content_length)
                    .expect("an end");
                assert!(
                    read.bytes[..end].ends_with(b"endobj\n"),
                    "{name}: item 7's length"
                );
            }
            number += hint.objects;
            location += hint.length;
        }
        // Item 3 and 4: the second page onward name shared objects the shared table holds.
        let shared = shared_objects(&read.data, read.table("S").expect("F.3.6: /S is required"));
        for hint in &table.pages {
            for id in &hint.shared {
                assert!(
                    usize::try_from(*id).is_ok_and(|id| id < shared.groups.len()),
                    "{name}: a shared object identifier indexes the shared object hint table"
                );
            }
        }
        // Font 11 is used by pages 2 and 3 and not by page 1, so both of those pages name it.
        let later: Vec<&PageHint> = table.pages.iter().skip(1).collect();
        assert!(
            later.iter().all(|hint| !hint.shared.is_empty()),
            "{name}: every page after the first shares something"
        );
    }
}

/// §F.4.3's shared object hint table: every group is where the table says.
#[test]
fn every_shared_object_group_is_where_the_table_says() {
    for (name, shape) in fixtures() {
        let read = Read::of(linearised(shape));
        let pages = page_offsets(&read.data, 3);
        let table = shared_objects(&read.data, read.table("S").expect("/S"));
        assert!(
            table.first_page_entries >= 1,
            "{name}: entry 0 spans the first page's start"
        );
        // Item 1: "The location of the first object of the first page shall be given in the
        // page offset hint table … The locations of subsequent object groups can be determined by
        // accumulating the lengths of all previous object groups until all shared objects in the
        // first page have been enumerated. Following that, the location of the first object in
        // the shared objects section can be obtained from the header section".
        // Item 4: "The first object of the first page shall be the one whose object number is
        // given by the O entry".
        let mut location = pages.first_page_location;
        let mut number = u64::from(read.parameters.first_page_object);
        let first_page_end = read.actual(pages.first_page_location) + pages.pages[0].length;
        for (at, group) in table.groups.iter().enumerate() {
            if at as u64 == table.first_page_entries {
                location = table.first_location;
                number = table.first_object;
            }
            let start = read.actual(location);
            assert_eq!(
                start,
                read.at(number),
                "{name}: shared group {at} begins at its entry"
            );
            for member in number..number + group.objects {
                let offset = read.at(member);
                assert!(
                    offset >= start && offset < start + group.length,
                    "{name}: object {member} of group {at} lies within it"
                );
            }
            if (at as u64) < table.first_page_entries {
                assert!(
                    start + group.length <= first_page_end,
                    "{name}: F.4.3 — \"objects that are referenced from the first page shall be \
                     located with the first-page objects (part 6)\""
                );
            }
            location += group.length;
            number += group.objects;
        }
        if !shape.open_at_page_two {
            assert!(
                table.groups.len() as u64 > table.first_page_entries,
                "{name}: part 8 holds font 11, shared by two pages after the first"
            );
        }
    }
}

/// §F.3.7's first-page section: every object the first page refers to lies before `/E`.
#[test]
fn every_object_the_first_page_needs_lies_before_the_end_of_the_first_page() {
    for (name, shape) in fixtures() {
        let read = Read::of(linearised(shape));
        let end = read.parameters.end_of_first_page;
        // "All objects that the page object refers to, to an arbitrary depth, except page tree
        // nodes, other page objects and DPart tree nodes." Three more are placed by the clauses
        // that name them rather than by this one, and the walk does not enter them: a page's
        // `/Thumb`, which §F.3.10 orders among the thumbnails; a thread dictionary, which §F.3.5
        // puts in part 4 "along with all thread dictionaries" and whose information dictionary
        // §F.3.10 puts in part 9; and an embedded file stream, §F.3.10's own category (ADR 1293).
        let pages = Pages::new(&read.document);
        let page_numbers: Vec<u32> = (0..pages.len())
            .filter_map(|index| {
                pages
                    .get(index)
                    .and_then(|page| page.id)
                    .map(|id| id.number)
            })
            .collect();
        let first = read.parameters.first_page_object;
        let mut stack = vec![first];
        let mut seen = std::collections::BTreeSet::new();
        while let Some(number) = stack.pop() {
            if !seen.insert(number) {
                continue;
            }
            let object = read.document.get(ObjectId::new(number, 0));
            let kind = object
                .as_dict()
                .and_then(|dict| dict.get("Type"))
                .and_then(Object::as_name)
                .map(|name| name.as_bytes().to_vec());
            let placed_elsewhere = matches!(
                kind.as_deref(),
                Some(b"Pages" | b"Thread" | b"EmbeddedFile" | b"DPart")
            );
            if placed_elsewhere || (number != first && page_numbers.contains(&number)) {
                continue;
            }
            let offset = read.at(u64::from(number));
            assert!(
                offset < end,
                "{name}: object {number}, which the first page refers to, is at {offset}, not \
                 before /E {end}"
            );
            let mut references = Vec::new();
            match (number == first, object.as_dict()) {
                (true, Some(page)) => {
                    for (key, value) in page.iter() {
                        if key.as_bytes() != b"Thumb" {
                            gather(value, &mut references, 0);
                        }
                    }
                }
                _ => gather(&object, &mut references, 0),
            }
            stack.extend(references);
        }
        // "The offset of the end of the first page (the end of Example 6 …)": the page offset
        // table's first entry ends exactly there.
        let table = page_offsets(&read.data, 3);
        assert_eq!(
            read.actual(table.first_page_location) + table.pages[0].length,
            end,
            "{name}: /E is where the first page's section ends"
        );
        if shape.use_outlines {
            // §F.3.7: "The entire outline hierarchy, if the value of the PageMode entry in the
            // catalog dictionary is UseOutlines."
            for number in [20, 21, 22, 23, 24] {
                let found = outline_number(&read.document, number);
                assert!(
                    read.at(u64::from(found)) < end,
                    "{name}: outline item in part 6"
                );
            }
        }
    }
}

/// Every reference inside a value, as object numbers; a stream's `/Length` is not followed.
fn gather(value: &Object, out: &mut Vec<u32>, depth: usize) {
    if depth > 64 {
        return;
    }
    match value {
        Object::Reference(id) => out.push(id.number),
        Object::Array(items) => items.iter().for_each(|item| gather(item, out, depth + 1)),
        Object::Dictionary(dict) => {
            for (_, item) in dict.iter() {
                gather(item, out, depth + 1);
            }
        }
        Object::Stream(stream) => {
            for (key, item) in stream.dict.iter() {
                if key.as_bytes() != b"Length" {
                    gather(item, out, depth + 1);
                }
            }
        }
        _ => {}
    }
}

/// The output's number for the fixture's outline object `source` (20 the root, 21 to 24 items),
/// found by walking `/First` and `/Next` from the catalog.
fn outline_number(document: &Document, source: u32) -> u32 {
    let catalog = document.catalog().expect("a catalog");
    let root = catalog
        .get("Outlines")
        .and_then(Object::as_reference)
        .expect("an outline");
    if source == 20 {
        return root.number;
    }
    let link = |id: ObjectId, key: &str| {
        document
            .get(id)
            .as_dict()
            .and_then(|dict| dict.get(key))
            .and_then(Object::as_reference)
            .expect("a link")
    };
    let one = link(root, "First");
    let two = link(one, "Next");
    match source {
        21 => one.number,
        22 => two.number,
        23 => link(one, "First").number,
        _ => link(two, "First").number,
    }
}

/// §F.3.10's outline order, and §F.4.5's generic tables for the outline, the information
/// dictionary and the page labels.
#[test]
fn the_outline_is_written_in_display_order_and_every_generic_table_finds_its_group() {
    let read = Read::of(linearised(Shape::default()));
    // "This is a preorder traversal of the outline tree, skipping over any subtree that is closed
    // … Following that shall be the subtrees that were skipped over": One is closed, so its child
    // One.a comes after Two and Two.a.
    let order: Vec<u64> = [20, 21, 22, 24, 23]
        .iter()
        .map(|source| read.at(u64::from(outline_number(&read.document, *source))))
        .collect();
    assert!(
        order.windows(2).all(|pair| pair[0] < pair[1]),
        "display order: {order:?}"
    );

    for key in ["O", "I", "L"] {
        // Table F.2: `/O` "( Required only if a document outline exists )", `/I` "( Required only
        // if a document information dictionary exists )", and `/L` for the page labels.
        let at = read
            .table(key)
            .unwrap_or_else(|| panic!("the hint stream states /{key}"));
        let (first, location, count, length) = generic(&read.data, at);
        let start = read.actual(location);
        assert_eq!(start, read.at(first), "/{key}: item 2 is item 1's location");
        assert!(count >= 1, "/{key}: the group holds its objects");
        for member in first..first + count {
            let offset = read.at(member);
            assert!(
                offset >= start && offset < start + length,
                "/{key}: object {member}"
            );
        }
    }
    // The page offset table is first and starts at 0 (§F.3.6), and no table the document does not
    // call for is stated.
    for absent in ["T", "A", "E", "V", "C", "R", "B"] {
        assert!(
            read.table(absent).is_none(),
            "/{absent} is stated for nothing"
        );
    }
}

/// Table F.2: every table the document's constructs call for is stated, and none other.
#[test]
fn every_table_table_f_2_requires_of_the_document_is_stated() {
    let read = Read::of(linearised(Shape {
        every_table: true,
        ..Shape::default()
    }));
    // `/S` "( Required )", and each of the others "( Required only if … exists )" — thumbnails,
    // an outline, article threads, named destinations, a form, an information dictionary, a
    // structure tree, a renditions tree, embedded files — and `/L` for the page labels.
    for key in ["S", "T", "O", "A", "E", "V", "I", "C", "L", "R", "B"] {
        assert!(read.table(key).is_some(), "Table F.2: /{key} is stated");
    }
    // §F.3.10: "Thumbnail images. These objects shall simply be ordered by page number", and the
    // colour space both thumbnails use is a thumbnail shared object after them.
    let pages = Pages::new(&read.document);
    let thumb = |index: usize| {
        let id = pages.get(index).and_then(|page| page.id).expect("a page");
        read.document
            .get(id)
            .as_dict()
            .and_then(|dict| dict.get("Thumb"))
            .and_then(Object::as_reference)
            .expect("a thumbnail")
    };
    let first = read.at(u64::from(thumb(0).number));
    let third = read.at(u64::from(thumb(2).number));
    assert!(first < third, "page 1's thumbnail precedes page 3's");
    let colour_space = read
        .document
        .get(thumb(0))
        .as_stream()
        .and_then(|stream| stream.dict.get("ColorSpace"))
        .and_then(Object::as_reference)
        .expect("an indirect colour space");
    assert!(
        read.at(u64::from(colour_space.number)) > third,
        "the thumbnail shared objects follow the thumbnails"
    );
    // §F.3.5: the thread dictionary is part 4, before the first page.
    let catalog = read.document.catalog().expect("a catalog");
    let thread = catalog
        .get("Threads")
        .and_then(Object::as_array)
        .and_then(|threads| threads.first())
        .and_then(Object::as_reference)
        .expect("a thread");
    assert!(
        read.at(u64::from(thread.number)) < read.at(u64::from(read.parameters.first_page_object)),
        "F.3.5: \"The Threads entry in the document catalog dictionary, along with all thread \
         dictionaries it refers to\" are in part 4"
    );
}

/// §F.3.7 and §F.3.10: every page states its inheritable attributes itself.
#[test]
fn every_page_object_states_its_media_box_and_resources_itself() {
    let read = Read::of(linearised(Shape::default()));
    let pages = Pages::new(&read.document);
    for index in 0..pages.len() {
        let id = pages
            .get(index)
            .and_then(|page| page.id)
            .expect("an indirect page");
        let object = read.document.get(id);
        let dict = object.as_dict().expect("a page");
        // "This page object shall explicitly specify all required attributes, such as Resources
        // and MediaBox ; the attributes may not be inherited from ancestor page tree nodes."
        assert!(
            dict.get("MediaBox").is_some(),
            "page {index} states /MediaBox"
        );
        assert!(
            dict.get("Resources").is_some(),
            "page {index} states /Resources"
        );
    }
    // The page that stated its own box keeps it rather than the root node's.
    let third = pages.get(2).expect("page 3");
    assert!(
        (third.width() - 300.0).abs() < f32::EPSILON,
        "page 3 keeps its own /MediaBox"
    );
}

/// Page `index` of `bytes` as a PPM.
fn draw(bytes: &[u8], index: usize) -> Vec<u8> {
    draw_source(Source::new(bytes.to_vec()), index)
}

/// Page `index` of a source as a PPM.
fn draw_source(source: Source, index: usize) -> Vec<u8> {
    let sinks = MemorySinks::new();
    apply(
        &Plan::Render(RenderPlan {
            source: 0,
            pages: format!("{}", index + 1).parse().expect("a selection"),
            size: Sizing::Dpi(72.0),
            format: ImageFormat::Ppm,
            page_box: None,
            annotations: true,
            names: "page.ppm".parse().expect("a pattern"),
            strips: None,
        }),
        &[source],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("the page draws");
    sinks.into_outputs().remove(0).1
}

/// The producer's content, byte for byte, and every page drawn as the source draws it.
#[test]
fn a_linearised_file_reopened_draws_every_page_as_its_source_does() {
    for (name, shape) in fixtures() {
        let source = fixture(shape);
        for streams in [Streams::Carry, Streams::DEFAULT] {
            let out = run(&source, linear_plan(shape, streams)).expect("linearises");
            let before =
                Document::open_with_limits(source.clone(), Limits::DEFAULT).expect("opens");
            let after = Document::open_with_limits(out.clone(), Limits::DEFAULT).expect("opens");
            assert_eq!(Pages::new(&after).len(), 3, "{name}: every page is there");
            for index in 0..3 {
                assert_eq!(
                    draw(&source, index),
                    draw(&out, index),
                    "{name}: page {} draws",
                    index + 1
                );
                let content = |document: &Document| {
                    let pages = Pages::new(document);
                    let page = pages.get(index).expect("a page");
                    page.content(document)
                };
                assert_eq!(
                    content(&before),
                    content(&after),
                    "{name}: page {}'s content",
                    index + 1
                );
            }
        }
    }
}

/// §F.1: "Incremental update shall still be permitted, but the resulting PDF is no longer
/// linearized and subsequently shall be treated as ordinary PDF."
#[test]
fn an_incremental_update_leaves_a_file_that_opens_and_is_no_longer_linearised() {
    let out = linearised(Shape::default());
    let document = Document::open_with_limits(out.clone(), Limits::DEFAULT).expect("opens");
    let info = document
        .trailer()
        .get("Info")
        .and_then(Object::as_reference)
        .expect("an information dictionary");
    let mut title = pdf_syntax::Dictionary::new();
    title.insert(
        pdf_syntax::Name::new(&b"Title"[..]),
        Object::String(b"Updated".to_vec().into()),
    );
    let mut replacements = BTreeMap::new();
    replacements.insert(info, Object::Dictionary(title));
    let updated = pdf_syntax::write::incremental_update(&document, &replacements)
        .expect("§7.5.6's update applies");
    assert!(
        updated.starts_with(&out),
        "§7.5.6: the original bytes are unchanged"
    );

    let reopened = Document::open_with_limits(updated.clone(), Limits::DEFAULT).expect("reopens");
    match state(&reopened) {
        State::NoLongerLinearized { parameters, length } => {
            assert_eq!(
                parameters.length,
                out.len() as u64,
                "/L is the linearised length"
            );
            assert_eq!(length, updated.len() as u64, "and the file is longer");
        }
        other => {
            panic!("Table F.1's /L mismatch: this file is no longer linearised, not {other:?}")
        }
    }
    // "treated as ordinary PDF": the update is what the file now says, and every page is there.
    let title = reopened
        .get(info)
        .as_dict()
        .and_then(|dict| dict.get("Title"))
        .and_then(Object::as_string)
        .map(<[u8]>::to_vec);
    assert_eq!(
        title.as_deref(),
        Some(&b"Updated"[..]),
        "the update is read"
    );
    assert_eq!(Pages::new(&reopened).len(), 3);
    assert_eq!(draw(&out, 0), draw(&updated, 0), "page one draws as it did");
    // And the file a non-linearised writer produced is not mistaken for one.
    let plain =
        Document::open_with_limits(fixture(Shape::default()), Limits::DEFAULT).expect("opens");
    assert_eq!(state(&plain), State::Ordinary);
}

/// RFC 0002 section 9's idempotence, under the fifth pass: a linearised file linearised again.
#[test]
fn linearising_a_linearised_file_changes_nothing() {
    for (name, shape) in fixtures() {
        let once = linearised(shape);
        let twice = run(&once, linear_plan(shape, Streams::Carry)).expect("linearises again");
        assert!(
            once == twice,
            "{name}: the second pass wrote {} bytes over {}",
            twice.len(),
            once.len()
        );
    }
}

/// §F.3.1's conditions on a linearised file that holds object streams, each asked of the file.
#[test]
fn object_streams_in_a_linearised_file_meet_every_condition_f_3_1_states() {
    for (name, shape) in fixtures()
        .into_iter()
        .filter(|(_, shape)| shape.object_streams)
    {
        let read = Read::of(linearised(shape));
        let size = read
            .document
            .trailer()
            .get("Size")
            .and_then(Object::as_integer)
            .and_then(|size| u64::try_from(size).ok())
            .expect("a /Size");
        let parameters = support::linearized::first_object(&read.bytes)
            .and_then(|at| object_at(&read.bytes, at as u64))
            .map(u64::from)
            .expect("part 2");
        // Both sections hold compressed objects: the first page's own dictionaries in the first,
        // the later pages', the shared objects' and part 9's in the main.
        let first: Vec<u64> = (parameters..size - 1)
            .filter(|n| read.compressed(*n))
            .collect();
        let main: Vec<u64> = (1..parameters).filter(|n| read.compressed(*n)).collect();
        assert!(!first.is_empty(), "{name}: part 4 and part 6 are packed");
        assert!(!main.is_empty(), "{name}: parts 7 to 9 are packed");
        // "Objects stored within object streams shall be given the highest range of object
        // numbers within the main and first-page cross-reference sections."
        let highest_uncompressed_main = (1..parameters)
            .filter(|n| !read.compressed(*n))
            .max()
            .expect("an uncompressed object");
        assert!(
            main.iter().all(|n| *n > highest_uncompressed_main),
            "{name}: the main section's compressed objects are numbered last"
        );
        let highest_uncompressed_first = (parameters..size - 1)
            .filter(|n| !read.compressed(*n))
            .max()
            .expect("an uncompressed object");
        assert!(
            first.iter().all(|n| *n > highest_uncompressed_first),
            "{name}: the first-page section's compressed objects are numbered last"
        );
        // "These additional objects may not be contained in an object stream: the linearization
        // dictionary, the document catalog dictionary, and page objects."
        let root = read
            .document
            .trailer()
            .get("Root")
            .and_then(Object::as_reference)
            .expect("a catalog");
        assert!(
            !read.compressed(u64::from(root.number)),
            "{name}: the catalog"
        );
        assert!(
            !read.compressed(parameters),
            "{name}: the parameter dictionary"
        );
        for page in &read.page_numbers {
            assert!(!read.compressed(*page), "{name}: page object {page}");
        }
        // "For PDF files containing object streams, hint data may specify the location and size
        // of the object streams only (or uncompressed objects), not the individual compressed
        // objects": every object the page offset and shared object tables count is at an offset
        // of its own, which `faults` asks of every entry.
        let found = faults(&read.bytes);
        assert!(found.is_empty(), "{name}: {found:?}");
        // "Similarly, shared object references shall be made to the object stream containing a
        // compressed object": font 11 is compressed and shared by pages 2 and 3, so each names
        // the group that is its carrier — and that group is one object, the carrier.
        let pages = page_offsets(&read.data, 3);
        let table = shared_objects(&read.data, read.table("S").expect("/S"));
        for hint in pages.pages.iter().skip(1) {
            for id in &hint.shared {
                let group = table.groups[usize::try_from(*id).expect("an index")];
                assert!(group.objects >= 1, "{name}: shared group {id}");
            }
        }
    }
}

/// §F.3.5 and §7.6: a linearised file encrypted on the way out, opened with its password, every
/// offset read back.
#[test]
fn a_linearised_file_is_encrypted_and_every_offset_reads_back_with_the_password() {
    let encrypted = |shape: &Shape| {
        let tables = Shape {
            object_streams: false,
            ..*shape
        };
        tables == Shape::default()
            || tables
                == Shape {
                    every_table: true,
                    ..Shape::default()
                }
    };
    for (name, shape) in fixtures().into_iter().filter(|(_, shape)| encrypted(shape)) {
        let protect = Protect {
            user_password: Secret::from("reader".to_owned()),
            ..Protect::owner_only(Secret::from("keeper".to_owned()))
        };
        let source = fixture(shape);
        let out = run_protected(&source, linear_plan(shape, Streams::Carry), &protect);
        // §7.6.4.1's user password is not the empty one, so the file does not open without it.
        assert!(
            Document::open_with_limits(out.clone(), Limits::DEFAULT).is_err(),
            "{name}: the file asks for its password"
        );
        let found = faults_with(&out, "reader");
        assert!(found.is_empty(), "{name}: {found:?}");
        let read = Read::open_with(out.clone(), "reader").expect("opens with the password");
        // F.3.5: "The Encrypt entry in the first-page trailer dictionary. All values in the
        // encryption dictionary shall also be located here" — part 4, before the first page.
        let encrypt = read
            .document
            .trailer()
            .get("Encrypt")
            .and_then(Object::as_reference)
            .expect("F.3.4: the first-page trailer names the encryption dictionary");
        let at = read
            .offset(u64::from(encrypt.number))
            .expect("§7.5.7: an encryption dictionary is not in an object stream");
        assert!(
            at < read.at(u64::from(read.parameters.first_page_object)),
            "{name}: the encryption dictionary is in part 4"
        );
        let dictionary = read.document.get(encrypt);
        let dictionary = dictionary.as_dict().expect("a dictionary");
        assert!(
            dictionary
                .iter()
                .all(|(_, value)| !matches!(value, Object::Reference(_))),
            "{name}: every value of the encryption dictionary is where it is"
        );
        // Table 15: "If there is an Encrypt entry, this array and the two byte-strings shall be
        // direct objects and shall be unencrypted" — read in the first-page section as written.
        let startxref = support::linearized::startxref(&read.bytes).expect("a startxref");
        let section = &read.bytes
            [startxref..usize::try_from(read.parameters.primary_hints.0).expect("an offset")];
        let identifier = read
            .document
            .trailer()
            .get("ID")
            .and_then(Object::as_array)
            .and_then(|pair| pair.first())
            .and_then(Object::as_string)
            .map(<[u8]>::to_vec)
            .expect("an /ID");
        let hex = identifier.iter().fold(String::new(), |mut hex, byte| {
            let _ = write!(hex, "{byte:02X}");
            hex
        });
        assert!(
            String::from_utf8_lossy(section).contains(&format!("<{hex}>")),
            "{name}: the /ID is stated in the clear in the first-page section"
        );
        // The hint stream is a stream like any other, and §7.6.2 encrypts it: what the file holds
        // is not the tables the password recovers.
        let (offset, length) = read.parameters.primary_hints;
        let hint = &read.bytes
            [usize::try_from(offset).expect("o")..usize::try_from(offset + length).expect("e")];
        assert!(
            !hint
                .windows(read.data.len().min(16))
                .any(|window| window == &read.data[..read.data.len().min(16)]),
            "{name}: the hint stream's data is ciphertext on disk"
        );
        // And the document is the source's: every page draws alike under the password.
        for index in 0..3 {
            assert_eq!(
                draw(&source, index),
                draw_source(
                    Source::with_password(out.clone(), Secret::from("reader".to_owned())),
                    index
                ),
                "{name}: page {} draws",
                index + 1
            );
        }
    }
}

/// §F.3.7 (b): every bead names its thread, and every page with beads lists them.
#[test]
fn every_bead_names_its_thread_and_every_page_with_beads_lists_them_in_part_six() {
    for object_streams in [false, true] {
        let shape = Shape {
            threads: true,
            object_streams,
            ..Shape::default()
        };
        let read = Read::of(linearised(shape));
        let document = &read.document;
        let reference = |id: ObjectId, key: &str| {
            document
                .get(id)
                .as_dict()
                .and_then(|dict| dict.get(key))
                .and_then(Object::as_reference)
        };
        let threads: Vec<ObjectId> = document
            .catalog()
            .expect("a catalog")
            .get("Threads")
            .and_then(Object::as_array)
            .expect("/Threads")
            .iter()
            .filter_map(Object::as_reference)
            .collect();
        assert_eq!(threads.len(), 2);
        // §12.4.3's chain from each thread's `/F` through `/N`, the definition of which beads a
        // thread holds; "each bead in the thread (not just the first bead) shall contain a T entry
        // referring to the associated thread dictionary".
        let mut beads_on: BTreeMap<u32, Vec<ObjectId>> = BTreeMap::new();
        for thread in &threads {
            let first = reference(*thread, "F").expect("/F");
            let mut bead = first;
            loop {
                assert_eq!(
                    reference(bead, "T"),
                    Some(*thread),
                    "bead {} names its thread",
                    bead.number
                );
                let page = reference(bead, "P").expect("/P");
                beads_on.entry(page.number).or_default().push(bead);
                bead = reference(bead, "N").expect("/N");
                if bead == first {
                    break;
                }
            }
        }
        // "If any beads exist for this page, the B array shall be present in the page dictionary":
        // the beads on it, thread by thread and along each chain (ADR 1309's order).
        let pages = Pages::new(document);
        let first_page = pages.get(0).and_then(|page| page.id).expect("page one");
        for index in 0..pages.len() {
            let id = pages.get(index).and_then(|page| page.id).expect("a page");
            let stated: Vec<ObjectId> = document
                .get(id)
                .as_dict()
                .and_then(|dict| dict.get("B"))
                .and_then(Object::as_array)
                .map(|beads| beads.iter().filter_map(Object::as_reference).collect())
                .unwrap_or_default();
            assert_eq!(
                stated,
                beads_on.get(&id.number).cloned().unwrap_or_default(),
                "page {index}'s /B"
            );
        }
        let on_page_one = beads_on
            .get(&first_page.number)
            .expect("page one has beads");
        assert_eq!(on_page_one.len(), 2, "both threads cross page one");
        // (b) is in part 6's list: page one's beads lie before `/E`.
        for bead in on_page_one {
            assert!(
                read.at(u64::from(bead.number)) < read.parameters.end_of_first_page,
                "bead {} is in the first-page section",
                bead.number
            );
        }
        let found = faults(&read.bytes);
        assert!(found.is_empty(), "{found:?}");
    }
}

/// [`faults`], the corpus walk's instrument, finds nothing wrong with any fixture — and finds
/// what is planted (trap 13): one byte of the page offset table's header item 2 changed.
#[test]
fn every_statement_the_annex_lets_a_reader_check_holds_and_a_planted_fault_is_named() {
    for (name, shape) in fixtures() {
        for streams in [Streams::Carry, Streams::DEFAULT] {
            let out = run(&fixture(shape), linear_plan(shape, streams)).expect("linearises");
            let found = faults(&out);
            assert!(found.is_empty(), "{name}: {found:?}");
        }
    }
    let mut out = linearised(Shape::default());
    let read = Read::of(out.clone());
    let (offset, _) = read.parameters.primary_hints;
    let at = usize::try_from(offset).expect("an offset");
    let data = out[at..]
        .windows(7)
        .position(|window| window == b"stream\n")
        .expect("the hint stream's data")
        + at
        + 7;
    // Header item 2 is bytes 4 to 7 of the stream: the first page's location, low byte.
    out[data + 7] ^= 0x10;
    let found = faults(&out);
    assert!(
        found.iter().any(|fault| fault.contains("page 0")),
        "a moved first-page location is named: {found:?}"
    );
}

/// Foreign evidence, principle 5's register: what qpdf makes of a linearised file this program
/// wrote. Agreement raises confidence in the reading; disagreement is a question for the annex.
#[test]
fn qpdf_checks_the_linearisation_this_writer_wrote() {
    for (name, shape) in fixtures() {
        let out = linearised(shape);
        let directory = std::env::temp_dir().join(format!("pdfv-linearize-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("a scratch directory");
        let path = directory.join("out.pdf");
        std::fs::write(&path, &out).expect("written");
        let Ok(output) = Command::new("qpdf").arg("--check").arg(&path).output() else {
            eprintln!("skipped: qpdf is not installed");
            return;
        };
        let _ = std::fs::remove_dir_all(&directory);
        let text = String::from_utf8_lossy(&output.stdout).into_owned()
            + &String::from_utf8_lossy(&output.stderr);
        eprintln!("{name}: qpdf --check: {}", text.trim());
        // qpdf's exit status 2 is an error in the file and 3 is warnings; its linearisation
        // warnings are where its reading of §F.4.1 and §F.3.7 and this writer's part (ADR 1293).
        assert_ne!(
            output.status.code(),
            Some(2),
            "{name}: qpdf reads the file as broken"
        );
    }
}
