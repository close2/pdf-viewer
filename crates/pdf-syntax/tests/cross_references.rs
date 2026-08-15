//! §7.5's cross-reference rules, one test per rule.
//!
//! These files are hand-built, which trap 4 in `doc/HANDOVER.md` warns against — and trap 8 is
//! why they are right here. A corpus finds what documents *contain*: the pdf.js corpus has three
//! documents that delete an object and none that deletes one still referenced, no document whose
//! `/W` array starts with a zero, and none that writes an entry type ISO 32000-2 does not define.
//! Every rule below is required of any valid PDF and reachable by no file anybody has. Each is
//! written as a comparison where one can be — the same document assembled two ways, differing
//! only in the rule under test — because an assertion about one file passes for a reader that
//! never applies the rule at all.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a fixture that does not open should fail loudly, and every fixture \
              here is a few hundred bytes, so no offset or object number can overflow"
)]

use std::fmt::Write as _;

use pdf_syntax::{Document, Object, ObjectId};

/// The three objects most fixtures here are built over: a catalogue, a page tree and one page.
const SKELETON: [&str; 3] = [
    "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
    "2 0 obj\n<< /Type /Pages /Count 1 /Kids [3 0 R] >>\nendobj\n",
    "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>\nendobj\n",
];

/// Object 4, which exists to be replaced, deleted or hidden by whichever rule is under test.
const SPARE: &str = "4 0 obj\n(the original)\nendobj\n";

/// Assembles a body from `%PDF-` onwards, returning it and the offset of each chunk.
fn body(chunks: &[&str]) -> (String, Vec<usize>) {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for chunk in chunks {
        offsets.push(out.len());
        out.push_str(chunk);
    }
    (out, offsets)
}

/// Writes one classic subsection: its header, then one entry per `(first field, keyword)` pair.
///
/// The first field is an offset for an `n` entry and the next free object's number for an `f`
/// one, which is §7.5.4's own distinction and the reason this takes a number rather than an
/// offset.
fn subsection(out: &mut String, first: u32, entries: &[(usize, char)]) {
    let _ = writeln!(out, "{first} {}", entries.len());
    for (field, kind) in entries {
        let _ = writeln!(out, "{field:010} 00000 {kind} ");
    }
}

/// The classic table over the whole of `offsets`, plus its trailer, `startxref` and `%%EOF`.
fn classic_section(out: &mut String, offsets: &[usize], extra: &str) {
    classic_section_starting_at(out, 0, offsets, extra);
}

/// As [`classic_section`], with the subsection header's first object number given.
///
/// It is `0` for every well-formed file — §7.5.4 requires it of a file that has never been
/// incrementally updated — and a parameter here only so that one test can write it wrong.
fn classic_section_starting_at(out: &mut String, first: u32, offsets: &[usize], extra: &str) {
    let at = out.len();
    out.push_str("xref\n");
    let mut entries = vec![(0, 'f')];
    entries.extend(offsets.iter().map(|offset| (*offset, 'n')));
    subsection(out, first, &entries);
    let _ = write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R {extra}>>\nstartxref\n{at}\n%%EOF\n",
        offsets.len() + 1
    );
}

/// One cross-reference stream object, `number 0 obj … endobj`, holding Table 18 rows.
///
/// `index` is `/Index`'s first object number, or `None` to leave the entry out and take the
/// clause's `[0 Size]` default. `/W` is `[1 4 2]` except where a test's whole subject is a
/// different one.
fn xref_stream_object(
    number: u32,
    index: Option<u32>,
    entries: &[[u64; 3]],
    extra: &str,
    widths: [usize; 3],
) -> Vec<u8> {
    let mut data = Vec::new();
    for row in entries {
        for (width, value) in widths.iter().zip(row) {
            let bytes = value.to_be_bytes();
            data.extend_from_slice(bytes.get(8 - width..).unwrap_or_default());
        }
    }

    let index_entry = index.map_or_else(String::new, |first| {
        format!("/Index [{first} {}] ", entries.len())
    });
    let dict = format!(
        "{number} 0 obj\n<< /Type /XRef /Size {} {index_entry}\
         /W [{} {} {}] {extra}/Length {} >>\nstream\n",
        number + 1,
        widths[0],
        widths[1],
        widths[2],
        data.len()
    );
    let mut bytes = dict.into_bytes();
    bytes.extend_from_slice(&data);
    bytes.extend_from_slice(b"\nendstream\nendobj\n");
    bytes
}

/// Opens a document, insisting the cross-reference table was the thing that was read.
///
/// Every fixture here is intact, so a rebuild means the table was rejected — and a rebuilt table
/// answers most of these questions by scanning the body, which is the reader whose behaviour
/// these tests exist to distinguish from the specification's.
fn open(bytes: Vec<u8>) -> Document {
    let document = Document::open(bytes).expect("the fixture's objects are all intact");
    assert!(
        !document.was_recovered(),
        "the table is well formed and must be the thing that was read"
    );
    document
}

/// The object numbered `number`, resolved.
fn object(document: &Document, number: u32) -> Object {
    document.get(ObjectId {
        number,
        generation: 0,
    })
}

/// ISO 32000-2 §7.5.1: which copy of an object is read is the table's answer, not the body's.
///
/// > Reading a non-linearized file in a serial manner is not reliable because of the way objects
/// > are to be processed after an incremental update.
///
/// The file below writes object 4 twice and points its one cross-reference section at the
/// *first* copy. A reader that takes the body's order — the last definition wins, which is what
/// `scan_for_objects` does and what a serial reader would do — returns the second.
#[test]
fn the_table_decides_which_copy_of_an_object_is_read_rather_than_the_bodys_order() {
    let (mut out, offsets) = body(&[
        SKELETON[0],
        SKELETON[1],
        SKELETON[2],
        SPARE,
        "4 0 obj\n(the later copy)\nendobj\n",
    ]);
    // The first four offsets: the later copy of object 4 sits in the body unreferenced.
    classic_section(&mut out, &offsets[..4], "");

    let document = open(out.into_bytes());
    assert_eq!(
        object(&document, 4).as_string().map(<[u8]>::to_vec),
        Some(b"the original".to_vec()),
        "the table names the first copy, so that is the object"
    );
}

/// ISO 32000-2 §7.5.3: the body is reached through the table and never sequentially.
///
/// > The body of a PDF file shall consist of a sequence of indirect objects
///
/// An object written into the body but absent from the table is not an object of this document.
/// The pair below differs only in whether the table lists it.
#[test]
fn a_body_object_the_table_does_not_list_is_not_reachable() {
    let build = |listed: bool| {
        let (mut out, offsets) = body(&[SKELETON[0], SKELETON[1], SKELETON[2], SPARE]);
        let listed_count = if listed { 4 } else { 3 };
        classic_section(&mut out, &offsets[..listed_count], "");
        out.into_bytes()
    };

    assert!(
        object(&open(build(true)), 4).as_string().is_some(),
        "listed, object 4 is the string the body holds"
    );
    assert!(
        object(&open(build(false)), 4).is_null(),
        "unlisted, the same bytes in the same place are not an object of this document"
    );
}

/// A file states which object a cross-reference entry describes twice. §7.5.4:
///
/// > The two integers denote (respectively) the object number of the first object in this
/// > subsection and the number of entries in the subsection.
///
/// and §7.3.10, of the object those bytes turn out to hold:
///
/// > The definition of an indirect object in a PDF file shall consist of its object number and
/// > generation number (separated by white-space), followed by the value of the object bracketed
/// > between the keywords obj and endobj
///
/// The pair below is one document written two ways, differing in nothing but the subsection
/// header: `0 5` in one and `1 5` in the other, over the same five entries. Under the second the
/// catalogue's entry is filed under object 2, the page tree's under 3, and every reference in
/// the file misses by one — so a reader that believes the header alone finds a page tree where
/// it expects a catalogue and has no document at all.
///
/// `issue7229.pdf` is that file. Its first section declares `1 7` and lists seven entries
/// beginning with object 0's free entry, which §7.5.4 says is "[t]he first entry in the table
/// (object number 0)"; the reader drew the *second* page as the first for the project's whole
/// life, and page two not at all.
#[test]
fn a_subsection_displaced_by_one_is_realigned_by_the_objects_own_headers() {
    let build = |first: u32| {
        let (mut out, offsets) = body(&[SKELETON[0], SKELETON[1], SKELETON[2], SPARE]);
        classic_section_starting_at(&mut out, first, &offsets, "");
        out.into_bytes()
    };

    for first in [0, 1] {
        let document = open(build(first));
        assert_eq!(
            object(&document, 4).as_string().map(<[u8]>::to_vec),
            Some(b"the original".to_vec()),
            "subsection header {first}: object 4 is the string the body files under 4"
        );
        assert!(
            document.catalog().is_ok(),
            "subsection header {first}: the catalogue is object 1, whatever the header says"
        );
    }
}

/// One disproved entry is not a displaced subsection.
///
/// The realignment above shifts a whole subsection, so it must not fire on the evidence of a
/// single entry — a stale offset in an otherwise sound table would then take every object with
/// it. This file's second subsection has exactly one entry, filed under object 5 and pointing at
/// object 4; the body holds both. A reader that shifted on one witness would answer object 4's
/// string for object 5, and one that stopped at the mismatch would answer null. What the entry's
/// own object says wins, so object 5 is object 5.
#[test]
fn a_single_entry_pointing_at_another_object_is_resolved_by_its_own_header() {
    const FIFTH: &str = "5 0 obj\n(the fifth)\nendobj\n";
    let (mut out, offsets) = body(&[SKELETON[0], SKELETON[1], SKELETON[2], SPARE, FIFTH]);
    let at = out.len();
    out.push_str("xref\n");
    let mut entries = vec![(0, 'f')];
    entries.extend(offsets.iter().take(4).map(|offset| (*offset, 'n')));
    subsection(&mut out, 0, &entries);
    // Object 5's entry, pointing one object short of it.
    subsection(&mut out, 5, &[(offsets[3], 'n')]);
    let _ = write!(
        out,
        "trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{at}\n%%EOF\n"
    );

    let document = open(out.into_bytes());
    assert_eq!(
        object(&document, 5).as_string().map(<[u8]>::to_vec),
        Some(b"the fifth".to_vec()),
        "object 5's own header is what says where object 5 is"
    );
    assert_eq!(
        object(&document, 4).as_string().map(<[u8]>::to_vec),
        Some(b"the original".to_vec()),
        "and object 4, whose entry was right, is untouched"
    );
}

/// ISO 32000-2 §7.5.6: an update that *deletes* an object is a copy like any other.
///
/// > A cross-reference section for an incremental update shall contain entries only for objects
/// > that have been changed, replaced, or deleted. Deleted objects shall be left unchanged in
/// > the PDF file, but shall be marked as deleted by means of their cross-reference entries.
///
/// > the most recent copy of each object shall be the one accessed from the PDF file
///
/// A deletion is written as an `f` entry over an in-use one, and the object's *bytes are still
/// there* — which is why dropping free entries rather than recording them lets the older
/// section's offset win and resurrects what the file deleted. Three corpus documents do this
/// (`prefilled_f1040.pdf` deletes three form-field appearance streams, `issue13520.pdf` an
/// image), and in all three nothing references what was deleted, so no page changed: the rule is
/// here on the clause's evidence rather than on a picture's.
///
/// The three cases are one test because only their contrast is meaningful — replaced, deleted
/// and untouched are the three things an update section can say about an object.
#[test]
fn an_incremental_update_that_deletes_an_object_is_not_undone_by_the_older_section() {
    let build = |update: &dyn Fn(&mut String, usize)| {
        let (mut out, offsets) = body(&[SKELETON[0], SKELETON[1], SKELETON[2], SPARE]);
        let first_at = out.len();
        classic_section(&mut out, &offsets, "");
        update(&mut out, first_at);
        out.into_bytes()
    };

    let untouched = build(&|_, _| ());
    assert_eq!(
        object(&open(untouched), 4).as_string().map(<[u8]>::to_vec),
        Some(b"the original".to_vec()),
        "with one section object 4 is what the body holds"
    );

    let replaced = build(&|out, previous| {
        let at = out.len();
        out.push_str("4 0 obj\n(the replacement)\nendobj\n");
        let second_at = out.len();
        out.push_str("xref\n");
        subsection(out, 4, &[(at, 'n')]);
        let _ = write!(
            out,
            "trailer\n<< /Size 5 /Root 1 0 R /Prev {previous} >>\nstartxref\n{second_at}\n%%EOF\n"
        );
    });
    assert_eq!(
        object(&open(replaced), 4).as_string().map(<[u8]>::to_vec),
        Some(b"the replacement".to_vec()),
        "an update that replaces an object is read from the newest section"
    );

    let deleted = build(&|out, previous| {
        let second_at = out.len();
        out.push_str("xref\n");
        subsection(out, 4, &[(0, 'f')]);
        let _ = write!(
            out,
            "trailer\n<< /Size 5 /Root 1 0 R /Prev {previous} >>\nstartxref\n{second_at}\n%%EOF\n"
        );
    });
    let document = open(deleted);
    assert!(
        object(&document, 4).is_null(),
        "an update that deletes an object must not be undone by the section it updates"
    );
    assert!(
        document.catalog().is_ok(),
        "and everything the update did not mention is untouched"
    );
}

/// ISO 32000-2 §7.5.7: an object stream's objects are at `/First` plus their own offset.
///
/// > A PDF processor shall rely on the First entry in the stream dictionary to locate the first
/// > object.
///
/// The pair below packs a page tree into an object stream twice over, differing only in a decoy
/// object between the header's last pair and `/First`. A reader that takes the first object as
/// beginning where the pairs end reads the decoy and has no page tree — and it reads it
/// *silently*, because the decoy is a perfectly good PDF object.
#[test]
fn an_object_stream_locates_its_objects_at_first_plus_their_own_offset() {
    let build = |padding: &str| {
        let pages = "<< /Type /Pages /Count 1 /Kids [4 0 R] >>";
        let page = "<< /Type /Page /Parent 3 0 R /MediaBox [0 0 10 10] >>";
        let header = format!("3 0 4 {} ", pages.len() + 1);
        let data = format!("{header}{padding}{pages} {page}");
        let first = header.len() + padding.len();

        let (out, offsets) = body(&[
            "1 0 obj\n<< /Type /Catalog /Pages 3 0 R >>\nendobj\n",
            &format!(
                "2 0 obj\n<< /Type /ObjStm /N 2 /First {first} /Length {} >>\nstream\n{data}\n\
                 endstream\nendobj\n",
                data.len()
            ),
        ]);
        let mut bytes = out.into_bytes();

        // Objects 3 and 4 are at indices 0 and 1 of object 2, which only a cross-reference
        // stream can say — a classic table has no way to name a location inside another object.
        let stream_at = bytes.len();
        let entries: [[u64; 3]; 6] = [
            [0, 0, 65535],
            [1, offset_of(offsets[0]), 0],
            [1, offset_of(offsets[1]), 0],
            [2, 2, 0],
            [2, 2, 1],
            [1, offset_of(stream_at), 0],
        ];
        bytes.extend_from_slice(&xref_stream_object(
            5,
            Some(0),
            &entries,
            "/Root 1 0 R ",
            [1, 4, 2],
        ));
        bytes.extend_from_slice(format!("startxref\n{stream_at}\n%%EOF\n").as_bytes());
        bytes
    };

    for padding in ["", "(a decoy) "] {
        let document = open(build(padding));
        let catalog = document.catalog().expect("/Root");
        let pages = document.get_key(&catalog, "Pages").as_dict().cloned();
        let pages = pages.unwrap_or_else(|| {
            panic!("the page tree is in the object stream, padding {padding:?}")
        });
        assert_eq!(
            document.get_key(&pages, "Count").as_integer(),
            Some(1),
            "and /First is what says where it starts, padding {padding:?}"
        );
    }
}

/// ISO 32000-2 §7.5.8: a cross-reference stream is the same information in another shape.
///
/// > Each cross-reference stream contains the information equivalent to the cross-reference
/// > table … and trailer … for one cross-reference section
///
/// So the same document written both ways must be the same document, which is the strongest
/// form this claim has: one function reads both, chosen by whether the `startxref` offset
/// begins with the `xref` keyword.
#[test]
fn a_cross_reference_stream_and_a_classic_table_describe_the_same_document() {
    let from_stream = open(skeleton_with_xref_stream([1, 4, 2], true));
    let from_table = open({
        let (mut out, offsets) = body(&SKELETON);
        classic_section(&mut out, &offsets, "");
        out.into_bytes()
    });

    for document in [&from_stream, &from_table] {
        let catalog = document.catalog().expect("/Root resolves");
        let pages = document
            .get_key(&catalog, "Pages")
            .as_dict()
            .cloned()
            .expect("the page tree is reachable");
        assert_eq!(
            document.get_key(&pages, "Count").as_integer(),
            Some(1),
            "and it holds one page"
        );
    }
}

/// ISO 32000-2 §7.5.8.1: the stream's own dictionary is the trailer.
///
/// > with the exception of the startxref address, %%EOF segment and comments, a PDF file may be
/// > entirely a sequence of objects
///
/// There is no `trailer` keyword anywhere in this file, so `/Root` can only have come from the
/// stream's own dictionary. A reader that looks for the keyword finds nothing and has no
/// catalogue.
#[test]
fn a_cross_reference_streams_own_dictionary_is_the_trailer() {
    let bytes = skeleton_with_xref_stream([1, 4, 2], true);
    assert!(
        !bytes.windows(7).any(|window| window == b"trailer"),
        "the fixture must not contain the keyword whose absence is the point"
    );
    let document = open(bytes);
    assert!(
        document.catalog().is_ok(),
        "/Root comes from the stream's dictionary"
    );
}

/// ISO 32000-2 §7.5.8.2: `/Index` is optional and defaults to the whole file.
///
/// > Default value: [0 Size ].
///
/// The pair below is the same table written with the default left implicit and then stated.
#[test]
fn an_absent_index_covers_object_zero_to_size() {
    for stated in [false, true] {
        let document = open(skeleton_with_xref_stream([1, 4, 2], stated));
        assert!(
            document.catalog().is_ok(),
            "with /Index {} the same four entries are read",
            if stated { "stated" } else { "left out" }
        );
    }
}

/// ISO 32000-2 §7.5.8.2, Table 17: a zero width means the field is absent and takes its default.
///
/// > A value of zero for an element in the W array indicates that the corresponding field shall
/// > not be present in the stream, and the default value shall be used, if there is one.
///
/// > If the first element is zero, the type field shall not be present, and shall default to
/// > Type 1.
///
/// A default only ever exercised by a file that also writes the value is a default nobody
/// implements, so this fixture writes no type field at all: every entry is type 1 because the
/// clause says so. Object 0's free entry becomes an in-use one pointing at byte zero, which is
/// what the clause requires of a file writing `/W [0 …]` and is part of why no producer does.
#[test]
fn a_zero_first_field_width_defaults_every_entry_to_type_one() {
    let document = open(skeleton_with_xref_stream([0, 2, 1], true));
    let catalog = document
        .catalog()
        .expect("every entry is type 1, so the catalogue is at an offset");
    assert!(document.get_key(&catalog, "Pages").as_dict().is_some());
}

/// ISO 32000-2 §7.5.8.3: an entry type the standard does not define resolves to null.
///
/// > In PDF 1.5 through PDF 2.0, only types 0, 1, and 2 are allowed. Any other value shall be
/// > interpreted as a reference to the null object, thus permitting new entry types to be
/// > defined in the future.
///
/// "Interpreted as" rather than "ignored": the entry is a statement the section makes, so an
/// older section must not answer instead. The pair below writes object 4 in use in the first
/// section and then, in an update, as an entry of a type this version of PDF has no meaning for.
#[test]
fn an_entry_type_the_standard_does_not_define_is_the_null_object() {
    for (kind, names_the_object) in [(1u64, true), (7, false)] {
        let (mut out, offsets) = body(&[SKELETON[0], SKELETON[1], SKELETON[2], SPARE]);
        let first_at = out.len();
        classic_section(&mut out, &offsets, "");

        // The update says only what object 4 now is.
        let mut bytes = out.into_bytes();
        let stream_at = bytes.len();
        let entries = [[kind, offset_of(offsets[3]), 0]];
        bytes.extend_from_slice(&xref_stream_object(
            5,
            Some(4),
            &entries,
            &format!("/Root 1 0 R /Prev {first_at} "),
            [1, 4, 2],
        ));
        bytes.extend_from_slice(format!("startxref\n{stream_at}\n%%EOF\n").as_bytes());

        let document = open(bytes);
        assert_eq!(
            object(&document, 4).as_string().is_some(),
            names_the_object,
            "an entry of type {kind} should {} the object",
            if names_the_object { "name" } else { "not name" }
        );
    }
}

/// ISO 32000-2 §7.5.8.4: a hybrid-reference file's `/XRefStm` outranks the previous section.
///
/// > When the PDF reader searches for an object, if an entry is not found in any given standard
/// > cross-reference section, the search shall proceed to a cross-reference stream specified by
/// > the XRefStm entry before looking in the previous cross-reference section (the Prev entry in
/// > the trailer).
///
/// > A PDF reader shall look in the cross-reference stream first, find the object there, and
/// > shall ignore the free entry in the previous section.
///
/// The clause's own example in miniature: the newest table says nothing about object 4, the
/// section before it marks the object free, and the stream says where it is. Both halves of the
/// rule are under test — the ordering, and that the free entry loses to the stream. This is the
/// one place where §7.5.6's rule about deletions has to *not* apply, so it is also the test that
/// says the two orderings compose rather than fight.
#[test]
fn a_hybrid_reference_file_finds_through_xrefstm_what_a_previous_section_marks_free() {
    let (mut out, offsets) = body(&[SKELETON[0], SKELETON[1], SKELETON[2], SPARE]);

    // The main section: everything in use.
    let main_at = out.len();
    classic_section(&mut out, &offsets, "");

    // An update that hides object 4 from a reader predating PDF 1.5.
    let hidden_at = out.len();
    out.push_str("xref\n");
    subsection(&mut out, 4, &[(0, 'f')]);
    let _ = write!(
        out,
        "trailer\n<< /Size 5 /Root 1 0 R /Prev {main_at} >>\nstartxref\n{hidden_at}\n%%EOF\n"
    );

    // …and the stream that says where it really is, named by the newest table's `/XRefStm`.
    let mut bytes = out.into_bytes();
    let stream_at = bytes.len();
    let entries = [[1u64, offset_of(offsets[3]), 0]];
    bytes.extend_from_slice(&xref_stream_object(5, Some(4), &entries, "", [1, 4, 2]));

    let table_at = bytes.len();
    let mut tail = String::from("xref\n");
    subsection(&mut tail, 0, &[(0, 'f')]);
    let _ = write!(
        tail,
        "trailer\n<< /Size 6 /Root 1 0 R /XRefStm {stream_at} /Prev {hidden_at} >>\n\
         startxref\n{table_at}\n%%EOF\n"
    );
    bytes.extend_from_slice(tail.as_bytes());

    let document = open(bytes);
    assert_eq!(
        object(&document, 4).as_string().map(<[u8]>::to_vec),
        Some(b"the original".to_vec()),
        "the hybrid stream is searched before the section that marks the object free"
    );
}

/// The skeleton written with a cross-reference stream rather than a classic table.
///
/// `widths` is `/W`, and `stated` decides whether `/Index` is written or left to its default.
/// The stream carries an entry for itself, which §7.5.8.3 requires: "an entry for it shall exist
/// in either a cross-reference stream (usually itself) or in a cross-reference table".
fn skeleton_with_xref_stream(widths: [usize; 3], stated: bool) -> Vec<u8> {
    let (out, offsets) = body(&SKELETON);
    let mut bytes = out.into_bytes();
    let stream_at = bytes.len();

    let entries: [[u64; 3]; 5] = [
        [0, 0, 65535],
        [1, offset_of(offsets[0]), 0],
        [1, offset_of(offsets[1]), 0],
        [1, offset_of(offsets[2]), 0],
        [1, offset_of(stream_at), 0],
    ];
    let index = if stated { Some(0) } else { None };
    bytes.extend_from_slice(&xref_stream_object(
        4,
        index,
        &entries,
        "/Root 1 0 R ",
        widths,
    ));
    bytes.extend_from_slice(format!("startxref\n{stream_at}\n%%EOF\n").as_bytes());
    bytes
}

/// A body offset as a Table 18 field.
fn offset_of(offset: usize) -> u64 {
    u64::try_from(offset).expect("every fixture here is a few hundred bytes")
}

/// §7.5.7's compressed objects, when the stream carrying them decodes only in part.
///
/// The clause states where each object *ends* as well as where it begins — "[t]he byte offsets
/// shall be in increasing order", and NOTE 7 (2020):
///
/// > processing of each object in an object stream starts at the specified byte offset in the
/// > decompressed stream and ends prior to the byte offset of the next object or when the end of
/// > stream is encountered.
///
/// So an object whose stated end the decoded prefix carries is whole and is the producer's own,
/// and the *last* object's end is the end of the stream — which a damaged decode has not reached.
/// Reading it anyway is what ADR 0366 refuses: a truncated token still parses, so the number
/// would name a value the producer never wrote.
///
/// **The pair differs in one bit**, RFC 1951's BFINAL on the single stored block, so the two
/// files carry the same bytes and only one of them says it is finished. No document on this disk
/// makes the comparison — the crawl's damaged object streams lose their *header* rather than
/// their last object — which is trap 8's own case for building it.
#[test]
fn an_object_a_damaged_object_stream_does_not_wholly_carry_is_not_read() {
    let build = |complete: bool| {
        let bodies = ["<< /A 5 >>\n", "(six)\n"];
        let mut header = String::new();
        let mut at = 0usize;
        for (number, part) in [(5u32, bodies[0]), (6, bodies[1])] {
            let _ = write!(header, "{number} {at} ");
            at += part.len();
        }
        let first = header.len();
        let payload = format!("{header}{}{}", bodies[0], bodies[1]).into_bytes();
        let data = zlib_stored(&payload, complete);

        let (out, offsets) = body(&SKELETON);
        let mut bytes = out.into_bytes();
        let object_stream_at = bytes.len();
        bytes.extend_from_slice(
            format!(
                "4 0 obj\n<< /Type /ObjStm /N 2 /First {first} /Filter /FlateDecode \
                 /Length {} >>\nstream\n",
                data.len()
            )
            .as_bytes(),
        );
        bytes.extend_from_slice(&data);
        bytes.extend_from_slice(b"\nendstream\nendobj\n");

        let stream_at = bytes.len();
        let entries: [[u64; 3]; 7] = [
            [0, 0, 65535],
            [1, offset_of(offsets[0]), 0],
            [1, offset_of(offsets[1]), 0],
            [1, offset_of(offsets[2]), 0],
            [1, offset_of(object_stream_at), 0],
            [2, 4, 0],
            [2, 4, 1],
        ];
        bytes.extend_from_slice(&xref_stream_object(
            7,
            Some(0),
            &entries,
            "/Root 1 0 R ",
            [1, 4, 2],
        ));
        bytes.extend_from_slice(format!("startxref\n{stream_at}\n%%EOF\n").as_bytes());
        bytes
    };

    let whole = open(build(true));
    assert!(
        object(&whole, 5).as_dict().is_some(),
        "the first compressed object is a dictionary"
    );
    assert_eq!(
        object(&whole, 6).as_string().map(<[u8]>::to_vec),
        Some(b"six".to_vec()),
        "and the second is the string the stream carries"
    );
    assert!(
        whole.objects_lost_to_damage().is_empty(),
        "a complete stream loses nothing"
    );

    let damaged = open(build(false));
    // Asked for before the record is read, because nothing expands an object stream until an
    // object inside it is wanted — which is `CLAUDE.md`'s startup rule and the reason the
    // sentence a host says about this cannot be said when the file opens.
    assert!(
        object(&damaged, 6).is_null(),
        "the last object's end is the end of the stream, which this decode never reached, so it \
         is not read: a value taken from bytes that stop early is not the producer's"
    );
    assert!(
        object(&damaged, 5).as_dict().is_some(),
        "and the object the prefix wholly carries is still read — the same bytes in the same \
         place, with the next offset as their end"
    );
    assert!(
        damaged.objects_lost_to_damage().objects.contains(&6),
        "and the reader can say what it did not read"
    );
}

/// §7.5.8's own arithmetic for how long a cross-reference stream is, and what a short one loses.
///
/// Table 17 states it of `/W`: "[t]he sum of the items shall be the total length of each entry;
/// it can be used with the Index array to determine the starting position of each subsection."
/// With `/Index`'s counts that gives the stream's whole extent, which §7.3.8.2 then requires the
/// data to agree with — "[a]ll of these constraints shall be consistent". A stream carrying
/// fewer records than that is short whether a filter failed or a producer miscounted, and the
/// records it does carry are whole: each field's width comes from `/W` and each row's object
/// number from `/Index` and its position, so nothing in a row depends on a row after it.
///
/// What the pair pins is the *statement* that comes with the shortfall. Everywhere else in this
/// reader an object number with no entry has been deleted — §7.5.6's most recent copy, ADR 0100 —
/// and here it has not: the file meant to say something about it and the bytes are gone. Nothing
/// on this disk exercises it: the crawl's damaged cross-reference streams lose no *stated* row,
/// because a truncated `FlateDecode` usually loses only RFC 1951's final block.
#[test]
fn a_cross_reference_stream_shorter_than_its_own_index_says_what_it_lost() {
    let build = |carried: usize| {
        let (out, offsets) = body(&[SKELETON[0], SKELETON[1], SKELETON[2], SPARE]);
        let mut bytes = out.into_bytes();
        let stream_at = bytes.len();
        let entries: [[u64; 3]; 6] = [
            [0, 0, 65535],
            [1, offset_of(offsets[0]), 0],
            [1, offset_of(offsets[1]), 0],
            [1, offset_of(offsets[2]), 0],
            [1, offset_of(offsets[3]), 0],
            [1, offset_of(stream_at), 0],
        ];
        // `/Index [0 6]` is written whatever `carried` is: the dictionary's claim is the
        // constant, and the data under it is what moves.
        let widths = [1usize, 4, 2];
        let mut data = Vec::new();
        for row in entries.iter().take(carried) {
            for (width, value) in widths.iter().zip(row) {
                let field = value.to_be_bytes();
                data.extend_from_slice(field.get(8 - width..).unwrap_or_default());
            }
        }
        bytes.extend_from_slice(
            format!(
                "5 0 obj\n<< /Type /XRef /Size 6 /Index [0 6] /W [1 4 2] /Root 1 0 R \
                 /Length {} >>\nstream\n",
                data.len()
            )
            .as_bytes(),
        );
        bytes.extend_from_slice(&data);
        bytes.extend_from_slice(b"\nendstream\nendobj\n");
        bytes.extend_from_slice(format!("startxref\n{stream_at}\n%%EOF\n").as_bytes());
        bytes
    };

    let whole = open(build(6));
    assert_eq!(
        whole.cross_reference_entries_lost(),
        0,
        "a stream as long as its own arithmetic loses nothing"
    );
    assert!(
        object(&whole, 4).as_string().is_some(),
        "and every object it names is reachable"
    );

    let short = open(build(4));
    assert_eq!(
        short.cross_reference_entries_lost(),
        2,
        "two of the six records /Index states are not in the data"
    );
    assert!(
        object(&short, 3).as_dict().is_some(),
        "the records the data carries are whole and are read"
    );
    assert!(
        object(&short, 4).is_null(),
        "and the object whose record went missing is unreachable — the same answer a deletion \
         gives, which is why the count above is what tells them apart"
    );
}

/// One zlib stream holding `payload` in a single stored block, finished or not.
///
/// RFC 1951 section 3.2.3's BFINAL is the only difference between the two, which is what makes the pair
/// above a comparison rather than two files: the decoder receives every byte either way and only
/// one of them says the stream is over. §7.4.4.1 makes the format normative for `FlateDecode`,
/// and RFC 1950's Adler-32 is written only where the stream claims to be complete.
fn zlib_stored(payload: &[u8], complete: bool) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    out.push(u8::from(complete));
    let length = u16::try_from(payload.len()).expect("every fixture here is a few hundred bytes");
    out.extend_from_slice(&length.to_le_bytes());
    out.extend_from_slice(&(!length).to_le_bytes());
    out.extend_from_slice(payload);
    if complete {
        let (mut low, mut high) = (1u32, 0u32);
        for byte in payload {
            low = (low + u32::from(*byte)) % 65521;
            high = (high + low) % 65521;
        }
        out.extend_from_slice(&((high << 16) | low).to_be_bytes());
    }
    out
}
