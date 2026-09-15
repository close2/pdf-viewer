//! The rules no corpus document fails, pinned by the smallest document that must fail each.
//!
//! `crates/pdf-archive/examples/withdrawn.rs` counts, per requirement, how many documents each
//! predicate finds a place in. Eleven of the implemented rows find one in no document of the
//! seven read corpora, and `doc/todo/62` section 7 says what that means: a rule no file has ever
//! failed is a rule nobody has ever judged, and its predicate may be right, wrong, or never
//! reached without the corpus being able to tell those three apart.
//!
//! So each rule here gets a pair — the smallest document the clause says must fail it, and the
//! smallest that must pass — and the pair is what ranks the predicate when the corpus cannot.
//! Trap 13's calibration is what makes the failing half worth anything: each was watched to go
//! green with its row's predicate replaced by `Check::Unchecked`, so the assertion is a statement
//! about the predicate rather than about some other row firing on the same fixture.
//!
//! Every clause is cited and none is quoted: `crate`'s module comment says why.

#![expect(
    clippy::expect_used,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly"
)]

use std::fmt::Write as _;

use pdf_archive::{Check, Examination, Findings, Flavour, Level, Outcome, Target, check, table};
use pdf_syntax::Document;

// -----------------------------------------------------------------------------------------
// Building documents, byte by byte, because two of these rules are about binary stream data.
// -----------------------------------------------------------------------------------------

/// Wraps numbered object bodies in a header, a cross-reference table and a trailer.
///
/// Objects are given whole — `N 0 obj` through `endobj\n` — and numbered from one in the order
/// they appear, so a fixture reads as the file it is.
fn assemble(objects: &[Vec<u8>], trailer: &str) -> Document {
    let mut out: Vec<u8> = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for object in objects {
        offsets.push(out.len());
        out.extend_from_slice(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let mut tail = String::new();
    let _ = writeln!(tail, "xref\n0 {size}");
    tail.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(tail, "{offset:010} 00000 n ");
    }
    let _ = write!(
        tail,
        "trailer\n<< /Size {size} /Root 1 0 R {trailer} >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.extend_from_slice(tail.as_bytes());
    Document::open(out).expect("the fixture is a valid PDF")
}

/// One object whose body is the given text.
fn object(number: usize, body: &str) -> Vec<u8> {
    format!("{number} 0 obj\n{body}\nendobj\n").into_bytes()
}

/// One stream object, whose data may be any bytes at all.
fn stream(number: usize, entries: &str, data: &[u8]) -> Vec<u8> {
    let mut out = format!(
        "{number} 0 obj\n<< {entries} /Length {} >>\nstream\n",
        data.len()
    )
    .into_bytes();
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream\nendobj\n");
    out
}

/// The catalog, page tree node and page every fixture starts from.
///
/// Objects 1 to 4 are the catalog, the page tree node, the page and its content stream, so a
/// fixture's own objects begin at 5 — the numbering `tests/reach.rs` already uses.
fn page(catalog: &str, page_entries: &str, content: &str, rest: Vec<Vec<u8>>) -> Document {
    let mut objects = vec![
        object(1, &format!("<< /Type /Catalog /Pages 2 0 R {catalog} >>")),
        object(2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        object(
            3,
            &format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 4 0 R \
                 {page_entries} >>"
            ),
        ),
        stream(4, "", content.as_bytes()),
    ];
    objects.extend(rest);
    assemble(&objects, "")
}

/// The identifier of every requirement a report says was not met.
fn failed(document: &Document, target: Target) -> Vec<&'static str> {
    check(document, target)
        .judgements
        .into_iter()
        .filter(|judgement| matches!(judgement.outcome, Outcome::Failed { .. }))
        .map(|judgement| judgement.id)
        .collect()
}

/// Asserts that `document` fails `id` under `target`, naming what it did fail if it did not.
#[track_caller]
fn fails(document: &Document, target: Target, id: &str) {
    let failed = failed(document, target);
    assert!(
        failed.contains(&id),
        "expected {id} to fail under {target:?}; what failed was {failed:?}"
    );
}

/// Asserts that `document` does not fail `id` under `target`.
#[track_caller]
fn passes(document: &Document, target: Target, id: &str) {
    let failed = failed(document, target);
    assert!(
        !failed.contains(&id),
        "expected {id} to be met under {target:?}; what failed was {failed:?}"
    );
}

/// The two targets these fixtures are judged against, one per part.
const PART_TWO: Target = Target::Two(Level::B);
/// The part 4 target, whose flavour adds no requirement any fixture here turns on.
const PART_FOUR: Target = Target::Four(Flavour::Plain);

// -----------------------------------------------------------------------------------------
// An ICC profile, built from its header outwards.
// -----------------------------------------------------------------------------------------

/// A profile with the given header version, device class, data colour space and tag signatures.
///
/// ICC.1 section 7.2 gives the header its 128 bytes and puts the tag table immediately after it:
/// a count, then twelve bytes per tag of signature, offset and size. Nothing here reads a tag's
/// *content*, so each entry points at zero bytes past the end of the table — enough for a
/// requirement asking which tags a profile carries, and not a profile a colour engine could use.
fn icc(version: (u8, u8, u8), class: [u8; 4], space: [u8; 4], tags: &[[u8; 4]]) -> Vec<u8> {
    let mut out = vec![0u8; 128];
    out[8] = version.0;
    out[9] = (version.1 << 4) | (version.2 & 0x0f);
    out[12..16].copy_from_slice(&class);
    out[16..20].copy_from_slice(&space);
    out[20..24].copy_from_slice(b"XYZ ");
    out[36..40].copy_from_slice(b"acsp");
    let count = u32::try_from(tags.len()).expect("a fixture states a handful of tags");
    out.extend_from_slice(&count.to_be_bytes());
    let after = u32::try_from(132usize.saturating_add(tags.len().saturating_mul(12)))
        .expect("a fixture is small");
    for tag in tags {
        out.extend_from_slice(tag);
        out.extend_from_slice(&after.to_be_bytes());
        out.extend_from_slice(&0u32.to_be_bytes());
    }
    let size = u32::try_from(out.len()).expect("a fixture is small");
    out[0..4].copy_from_slice(&size.to_be_bytes());
    out
}

/// The tags ICC.1:1998-09 clause 6.3 asks of a monochrome-form display profile.
///
/// The profile description, the copyright, the media white point and one transform — here the
/// grey tone reproduction curve, which is the shortest of the three forms that clause admits.
const MONOCHROME_DISPLAY_TAGS: [[u8; 4]; 4] = [*b"desc", *b"cprt", *b"wtpt", *b"kTRC"];

/// A destination profile the output-intent rows accept: a monitor profile in RGB.
fn destination_profile() -> Vec<u8> {
    icc((2, 2, 0), *b"mntr", *b"RGB ", &MONOCHROME_DISPLAY_TAGS)
}

// -----------------------------------------------------------------------------------------
// ISO 19005-4 section 6.2.9 — the blend mode an annotation dictionary sets.
// -----------------------------------------------------------------------------------------

/// The clause puts an annotation dictionary's `BM` under the same rule as a graphics state's:
/// only a blend mode ISO 32000 defines. `Frobnicate` is in neither of that standard's two
/// tables of them — the clause number is left off for the reason `table::graphics`'s own list
/// leaves it off, part 2 reading its base standard in the 2008 edition — and the
/// corpus's 6.2.9 files put every undefined mode it holds in a graphics state instead.
#[test]
fn an_annotation_blend_mode_the_base_standard_does_not_define_fails() {
    let document = page(
        "",
        "/Annots [<< /Type /Annot /Subtype /Square /Rect [0 0 10 10] /F 4 /BM /Frobnicate >>]",
        "",
        Vec::new(),
    );
    fails(
        &document,
        PART_FOUR,
        "graphics/annotation-blend-modes-are-defined",
    );
}

#[test]
fn an_annotation_blend_mode_the_base_standard_defines_passes() {
    let document = page(
        "",
        "/Annots [<< /Type /Annot /Subtype /Square /Rect [0 0 10 10] /F 4 /BM /Multiply >>]",
        "",
        Vec::new(),
    );
    passes(
        &document,
        PART_FOUR,
        "graphics/annotation-blend-modes-are-defined",
    );
}

// -----------------------------------------------------------------------------------------
// ISO 19005-2 section 6.2.3 — `DestOutputProfileRef` in a PDF/X output intent.
// -----------------------------------------------------------------------------------------

/// Part 2 forbids the key in a PDF/X output intent, which is what makes a file that would
/// otherwise be PDF/X-4p unable to be PDF/A-2 as well; the part 4 row beside this one forbids it
/// in every output intent, and that difference is what the pass fixture below holds.
#[test]
fn a_pdfx_output_intent_naming_a_profile_outside_the_file_fails() {
    let document = page(
        "/OutputIntents [<< /Type /OutputIntent /S /GTS_PDFX \
         /DestOutputProfileRef << /FN (profile.icc) >> >>]",
        "",
        "",
        Vec::new(),
    );
    fails(
        &document,
        PART_TWO,
        "graphics/no-destination-profile-reference-in-a-pdfx-output-intent",
    );
}

#[test]
fn a_pdfx_output_intent_with_no_such_key_passes() {
    let document = page(
        "/OutputIntents [<< /Type /OutputIntent /S /GTS_PDFX >>]",
        "",
        "",
        Vec::new(),
    );
    passes(
        &document,
        PART_TWO,
        "graphics/no-destination-profile-reference-in-a-pdfx-output-intent",
    );
}

/// The same key in a PDF/A output intent is outside this row, and the clause is why: part 2
/// states the prohibition of a PDF/X output intent and part 4 states it of any. A predicate that
/// read the key without reading `S` would fail this document, and it must not.
#[test]
fn the_same_key_in_a_pdfa_output_intent_is_not_this_rows_finding() {
    let document = page(
        "/OutputIntents [<< /Type /OutputIntent /S /GTS_PDFA1 \
         /DestOutputProfileRef << /FN (profile.icc) >> >>]",
        "",
        "",
        Vec::new(),
    );
    passes(
        &document,
        PART_TWO,
        "graphics/no-destination-profile-reference-in-a-pdfx-output-intent",
    );
}

// -----------------------------------------------------------------------------------------
// ISO 19005-4 section 6.2.3 — a page's own output intents array.
// -----------------------------------------------------------------------------------------

/// Part 4 admits a PDF/A output intent in a page dictionary's array as well as the catalog's,
/// and asks the same of it: the `S` key naming the PDF/A intent and a valid ICC profile stream
/// under `DestOutputProfile`. A page-level entry with no profile at all is the smallest failure.
#[test]
fn a_page_level_pdfa_output_intent_without_a_profile_fails() {
    let document = page(
        "",
        "/OutputIntents [<< /Type /OutputIntent /S /GTS_PDFA1 >>]",
        "",
        Vec::new(),
    );
    fails(
        &document,
        PART_FOUR,
        "graphics/page-output-intents-have-the-same-shape",
    );
}

#[test]
fn a_page_level_pdfa_output_intent_with_a_profile_passes() {
    let document = page(
        "",
        "/OutputIntents [<< /Type /OutputIntent /S /GTS_PDFA1 /DestOutputProfile 5 0 R >>]",
        "",
        vec![stream(5, "/N 3", &destination_profile())],
    );
    passes(
        &document,
        PART_FOUR,
        "graphics/page-output-intents-have-the-same-shape",
    );
}

/// The array's other two rules are this row's too, and a second entry is what reaches them: two
/// entries naming different profile objects is the shape the clause forbids.
#[test]
fn two_page_level_entries_naming_different_profiles_fail() {
    let document = page(
        "",
        "/OutputIntents [<< /Type /OutputIntent /S /GTS_PDFA1 /DestOutputProfile 5 0 R >> \
         << /Type /OutputIntent /S /GTS_PDFX /DestOutputProfile 6 0 R >>]",
        "",
        vec![
            stream(5, "/N 3", &destination_profile()),
            stream(6, "/N 3", &destination_profile()),
        ],
    );
    fails(
        &document,
        PART_FOUR,
        "graphics/page-output-intents-have-the-same-shape",
    );
}

// -----------------------------------------------------------------------------------------
// ISO 19005-2 section 6.2.4.2 — which ICC specifications a profile may claim.
// -----------------------------------------------------------------------------------------

/// A page whose content selects an `ICCBased` space built on the given profile.
///
/// The content stream names the resource, so ISO 19005-2 section 6.2.2's last sentence does not
/// put the colour space outside the population — a fixture that left the name unstated would
/// test the exemption rather than the row.
fn iccbased_page(profile: &[u8]) -> Document {
    page(
        "",
        "/Resources << /ColorSpace << /CS0 [/ICCBased 5 0 R] >> >>",
        "/CS0 cs 0 0 0 sc 0 0 200 200 re f",
        vec![stream(5, "/N 3", profile)],
    )
}

/// Part 2 names four ICC specifications a profile may conform to, all of them editions of ICC.1
/// or the ISO 15076-1 that carries one. A header claiming version 5 names ICC.2's iccMAX, which
/// is a different specification rather than a later edition of those.
#[test]
fn an_iccbased_profile_claiming_iccmax_fails() {
    let document = iccbased_page(&icc(
        (5, 0, 0),
        *b"mntr",
        *b"RGB ",
        &MONOCHROME_DISPLAY_TAGS,
    ));
    fails(
        &document,
        PART_TWO,
        "graphics/icc-profiles-claim-a-permitted-edition",
    );
}

#[test]
fn an_iccbased_profile_claiming_an_admitted_edition_passes() {
    let document = iccbased_page(&icc(
        (2, 2, 0),
        *b"mntr",
        *b"RGB ",
        &MONOCHROME_DISPLAY_TAGS,
    ));
    passes(
        &document,
        PART_TWO,
        "graphics/icc-profiles-claim-a-permitted-edition",
    );
}

// -----------------------------------------------------------------------------------------
// ISO 19005-4 section 6.2.4.2 — the tags the edition a profile names requires of it.
// -----------------------------------------------------------------------------------------

/// Part 4 sends an `ICCBased` profile to ISO 32000-2 §8.6.5.5, which requires it to conform to
/// the specification version its own header states. A header naming ICC.1:1998-09 with an empty
/// tag table carries none of the four tags that edition's clause 6.3 asks of a display profile.
#[test]
fn an_iccbased_profile_missing_the_tags_its_edition_requires_fails() {
    let document = iccbased_page(&icc((2, 2, 0), *b"mntr", *b"RGB ", &[]));
    fails(
        &document,
        PART_FOUR,
        "graphics/icc-profiles-carry-the-tags-their-version-requires",
    );
}

#[test]
fn an_iccbased_profile_carrying_them_passes() {
    let document = iccbased_page(&icc(
        (2, 2, 0),
        *b"mntr",
        *b"RGB ",
        &MONOCHROME_DISPLAY_TAGS,
    ));
    passes(
        &document,
        PART_FOUR,
        "graphics/icc-profiles-carry-the-tags-their-version-requires",
    );
}

// -----------------------------------------------------------------------------------------
// ISO 19005-2 section 6.1.13 — a DeviceN colour space's colourants.
// -----------------------------------------------------------------------------------------

/// A `DeviceN` space naming `count` colourants, as a page resource the content stream selects.
fn devicen_page(count: usize) -> Document {
    let names: Vec<String> = (0..count).map(|index| format!("/Ink{index}")).collect();
    let space = format!("[/DeviceN [{}] /DeviceGray 5 0 R]", names.join(" "));
    page(
        "",
        &format!("/Resources << /ColorSpace << /CS0 {space} >> >>"),
        "/CS0 cs",
        vec![stream(
            5,
            "/FunctionType 4 /Domain [0 1] /Range [0 1]",
            b"{}",
        )],
    )
}

/// One of the implementation limits the clause lists: no more than 32 colourants.
#[test]
fn a_devicen_space_of_thirty_three_colourants_fails() {
    fails(
        &devicen_page(33),
        PART_TWO,
        "implementation-limits/devicen-colourants",
    );
}

/// The same clause's limit on indirect objects, on an ordinary file.
///
/// The failing half is [`a_cross_reference_stream_naming_more_objects_than_the_limit_fails`],
/// which this comment used to say a test could not build. ADR 1026 section 5.1 attributed this
/// row's silence to the one document `examples/withdrawn.rs` skips by name; that document states
/// 40 015 indirect objects, so it never exercised the row at all.
#[test]
fn a_file_of_a_handful_of_indirect_objects_passes() {
    passes(
        &page("", "", "", Vec::new()),
        PART_TWO,
        "implementation-limits/indirect-object-count",
    );
}

// -----------------------------------------------------------------------------------------
// ISO 19005-2 section 6.1.13's indirect-object limit, and the one fixture here too large to
// judge through `check`.
// -----------------------------------------------------------------------------------------

/// A file whose cross-reference *stream* names `in_use` object numbers as in use.
///
/// §7.5.8 lets a cross-reference section be a stream of fixed-width records, so a file can state
/// its whole table in `in_use + 1` records — object 0 free, as §7.5.4 requires, and the rest of
/// type 1. The records here are written undeflated, which makes the fixture six bytes an object
/// and no compressor a dependency.
///
/// **Records 5 and above name the offset of object 3**, and the row does not read what is there:
/// ISO 19005-2 section 6.1.13 counts the indirect objects a file contains, and ISO 19005-2
/// section 6.1.4 is what makes the cross-reference table the statement of that — an indirect
/// object no cross-reference section names is exempt, so the numbers a section *does* name are
/// the file's own account of what it holds. A `/Size` is not that account and the predicate does
/// not read one.
fn cross_reference_stream_naming(in_use: u32) -> Document {
    use std::fmt::Write as _;

    let mut out: Vec<u8> = b"%PDF-1.5\n".to_vec();
    let mut offsets = Vec::new();
    for (number, body) in [
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >>",
    ]
    .iter()
    .enumerate()
    {
        offsets.push(out.len());
        out.extend_from_slice(object(number.saturating_add(1), body).as_slice());
    }
    let table_at = out.len();

    // §7.5.8.2's `/W`: one byte of type, four of offset, two of generation.
    let record = |kind: u8, field: u32, generation: u16| {
        let mut row = vec![kind];
        row.extend_from_slice(&field.to_be_bytes());
        row.extend_from_slice(&generation.to_be_bytes());
        row
    };
    let mut records: Vec<u8> = Vec::with_capacity(
        usize::try_from(in_use)
            .unwrap_or(0)
            .saturating_add(1)
            .saturating_mul(7),
    );
    records.extend_from_slice(&record(0, 0, 65535));
    for offset in &offsets {
        records.extend_from_slice(&record(1, u32::try_from(*offset).unwrap_or(0), 0));
    }
    records.extend_from_slice(&record(1, u32::try_from(table_at).unwrap_or(0), 0));
    let filler = record(1, u32::try_from(offsets[2]).unwrap_or(0), 0);
    for _ in 5..=in_use {
        records.extend_from_slice(&filler);
    }

    let mut head = String::new();
    let _ = write!(
        head,
        "4 0 obj\n<< /Type /XRef /Size {} /W [1 4 2] /Root 1 0 R /Length {} >>\nstream\n",
        in_use.saturating_add(1),
        records.len()
    );
    out.extend_from_slice(head.as_bytes());
    out.extend_from_slice(&records);
    out.extend_from_slice(b"\nendstream\nendobj\n");
    let mut tail = String::new();
    let _ = write!(tail, "startxref\n{table_at}\n%%EOF\n");
    out.extend_from_slice(tail.as_bytes());
    Document::open(out).expect("the fixture is a valid PDF")
}

/// The one requirement of this file judged by its own predicate rather than through `check`.
///
/// **A whole report on this fixture costs 6.9 s and about a gibibyte**, and none of it is this
/// row's: `Examination::objects` fetches every number the table names, and a dozen rows ask for
/// that population (`examples/cost.rs` on the failing fixture — opening 345 ms, the population
/// 6.6 s, the predicates 9.4 s in release). So the assertion is made against the predicate
/// itself, which is also what makes it calibrated in the sense trap 13 asks for without a scratch
/// build: no other row can produce this finding, because no other row is run.
///
/// What the fixture then costs is 0.58 s and 640 MiB of `VmHWM` for the two documents together,
/// nearly all of it the 58 MB of records each file carries and the cross-reference table each
/// builds from them.
#[track_caller]
fn judged_alone(document: &Document, target: Target, id: &str) -> bool {
    let requirement = table::binding(target)
        .find(|requirement| requirement.id == id)
        .expect("the requirement is one this target binds");
    // `expect` rather than an `else` arm that panics, which this file's lint level refuses.
    let predicate = match requirement.check {
        Check::Implemented(predicate) => Some(predicate),
        _ => None,
    }
    .expect("the requirement is a predicate a document can fail");
    let mut findings = Findings::default();
    predicate(&Examination::new(document, target), &mut findings);
    !findings.met()
}

/// ISO 19005-2 section 6.1.13: no more than 8 388 607 indirect objects.
///
/// The smallest document that must fail it names 8 388 608, and the boundary is asserted from
/// both sides because an off-by-one here would be invisible in every other test: the corpus holds
/// no witness at all, and the first fixture built for this row named exactly 8 388 607 objects
/// and was met.
#[test]
fn a_cross_reference_stream_naming_more_objects_than_the_limit_fails() {
    let id = "implementation-limits/indirect-object-count";
    assert!(
        judged_alone(&cross_reference_stream_naming(8_388_608), PART_TWO, id),
        "a file naming 8 388 608 indirect objects is one more than the clause admits"
    );
    assert!(
        !judged_alone(&cross_reference_stream_naming(8_388_607), PART_TWO, id),
        "the limit itself conforms"
    );
}

/// The count the row reads and the population every other row reads are one number.
///
/// `indirect_object_count` takes `XrefTable::len` where it used to take `Examination::objects`,
/// which is what lets the fixture above exist at all. The two are the same by construction —
/// `objects` maps `XrefTable::object_numbers` one for one — and this is what stops them drifting
/// apart in silence, on a document small enough for both.
#[test]
fn the_two_ways_of_counting_the_objects_agree() {
    let document = page("", "", "", vec![object(5, "<< /Length 0 >>")]);
    let examination = Examination::new(&document, PART_TWO);
    assert_eq!(document.xref().len(), examination.objects().len());
    assert_eq!(document.xref().len(), 5);
}

#[test]
fn a_devicen_space_of_thirty_two_colourants_passes() {
    passes(
        &devicen_page(32),
        PART_TWO,
        "implementation-limits/devicen-colourants",
    );
}

// -----------------------------------------------------------------------------------------
// ISO 19005-2 section 6.6.2.1 and ISO 19005-4 section 6.7.2.1 — how a packet is serialised.
// -----------------------------------------------------------------------------------------

/// A document whose catalog carries the given bytes as its metadata stream.
fn metadata_page(packet: &str) -> Document {
    page(
        "/Metadata 5 0 R",
        "",
        "",
        vec![stream(
            5,
            "/Type /Metadata /Subtype /XML",
            packet.as_bytes(),
        )],
    )
}

/// The opening of an XMP packet, up to and including its first `rdf:RDF` start tag.
const RDF_OPEN: &str = "<?xpacket begin=\"\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\
     <x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\
     <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\" \
     xmlns:pdf=\"http://ns.adobe.com/pdf/1.3/\">";

/// Its close.
const RDF_CLOSE: &str = "</rdf:RDF></x:xmpmeta><?xpacket end=\"w\"?>";

/// One description, carrying one simple property.
const DESCRIPTION: &str =
    "<rdf:Description rdf:about=\"\"><pdf:Producer>quorra</pdf:Producer></rdf:Description>";

/// Both parts require an XMP packet to be well formed as ISO 16684-1 defines it, and that
/// standard serialises one packet as one `rdf:RDF` element. Two of them in one stream is the
/// smallest failure there is.
#[test]
fn a_metadata_stream_stating_two_rdf_elements_fails() {
    // The second element declares the two prefixes again: the first element's declarations are
    // scoped to it, and a packet whose second root borrowed them would not be XML at all.
    let second = "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\" \
                  xmlns:pdf=\"http://ns.adobe.com/pdf/1.3/\">";
    let packet = format!("{RDF_OPEN}{DESCRIPTION}</rdf:RDF>{second}{DESCRIPTION}{RDF_CLOSE}");
    let document = metadata_page(&packet);
    for target in [PART_TWO, PART_FOUR] {
        fails(
            &document,
            target,
            "metadata/xmp-packets-state-one-rdf-element",
        );
    }
}

#[test]
fn a_metadata_stream_stating_one_rdf_element_passes() {
    let document = metadata_page(&format!("{RDF_OPEN}{DESCRIPTION}{RDF_CLOSE}"));
    for target in [PART_TWO, PART_FOUR] {
        passes(
            &document,
            target,
            "metadata/xmp-packets-state-one-rdf-element",
        );
    }
}

/// ISO 16684-1 section 7.2 confines non-white character data to the leaf elements standing for
/// simple values; a description is not one of those, and text directly inside it is the smallest
/// document that must fail.
#[test]
fn character_data_in_a_description_fails() {
    let stray = "<rdf:Description rdf:about=\"\">loose text\
                 <pdf:Producer>quorra</pdf:Producer></rdf:Description>";
    let document = metadata_page(&format!("{RDF_OPEN}{stray}{RDF_CLOSE}"));
    for target in [PART_TWO, PART_FOUR] {
        fails(
            &document,
            target,
            "metadata/xmp-character-data-only-in-simple-values",
        );
    }
}

#[test]
fn character_data_in_a_simple_value_passes() {
    let document = metadata_page(&format!("{RDF_OPEN}{DESCRIPTION}{RDF_CLOSE}"));
    for target in [PART_TWO, PART_FOUR] {
        passes(
            &document,
            target,
            "metadata/xmp-character-data-only-in-simple-values",
        );
    }
}

// -----------------------------------------------------------------------------------------
// A TrueType program, built table by table, for the two rules that read one.
// -----------------------------------------------------------------------------------------

/// Wraps tables in an sfnt table directory.
///
/// The directory's binary-search hints — `searchRange`, `entrySelector`, `rangeShift` — are left
/// zero: no reader in this tree consults them, and a fixture that computed them would be saying
/// something about the builder rather than about the font.
fn sfnt(tables: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
    let count = u16::try_from(tables.len()).expect("a fixture states a handful of tables");
    let mut out = Vec::new();
    out.extend_from_slice(&0x0001_0000u32.to_be_bytes());
    out.extend_from_slice(&count.to_be_bytes());
    out.extend_from_slice(&[0; 6]);
    let mut at = u32::try_from(12usize.saturating_add(tables.len().saturating_mul(16)))
        .expect("a fixture is small");
    let mut body = Vec::new();
    for (tag, data) in tables {
        let length = u32::try_from(data.len()).expect("a fixture is small");
        out.extend_from_slice(tag);
        out.extend_from_slice(&0u32.to_be_bytes());
        out.extend_from_slice(&at.to_be_bytes());
        out.extend_from_slice(&length.to_be_bytes());
        body.extend_from_slice(data);
        // Every table begins on a four-byte boundary, which the offset has to account for.
        let padding = 4usize.saturating_sub(data.len() % 4) % 4;
        body.extend(std::iter::repeat_n(0u8, padding));
        at = at
            .checked_add(length)
            .and_then(|at| at.checked_add(u32::try_from(padding).expect("at most three")))
            .expect("a fixture is small");
    }
    out.extend_from_slice(&body);
    out
}

/// A `cmap` table whose encoding records name the given platform and encoding identifiers.
///
/// Every record points at one shared format 6 subtable mapping [`MAPPED_CODE`] to the program's
/// one real glyph — enough, under either the Macintosh Roman or the Microsoft Unicode rule of
/// ISO 32000-2 §9.6.5.4, for the code the fixture shows to reach it. A subtable covering no code
/// at all would be a different fixture than intended: this tree's font loader refuses a program
/// through which nothing can be drawn, so the rule under test would never be reached and the
/// assertion would be about the loader.
fn cmap(subtables: &[(u16, u16)]) -> Vec<u8> {
    let count = u16::try_from(subtables.len()).expect("a fixture states a handful of subtables");
    let mut out = vec![0, 0];
    out.extend_from_slice(&count.to_be_bytes());
    let after = u32::try_from(4usize.saturating_add(subtables.len().saturating_mul(8)))
        .expect("a fixture is small");
    for (platform, encoding) in subtables {
        out.extend_from_slice(&platform.to_be_bytes());
        out.extend_from_slice(&encoding.to_be_bytes());
        out.extend_from_slice(&after.to_be_bytes());
    }
    // Format 6, twelve bytes: one code, mapped to glyph one.
    for field in [6u16, 12, 0, MAPPED_CODE, 1, 1] {
        out.extend_from_slice(&field.to_be_bytes());
    }
    out
}

/// The one character code every program built here can draw.
///
/// Sixty-five is `A`, whose Macintosh Roman code and Unicode code point are the same number, so
/// one subtable serves whichever of §9.6.5.4's rules a fixture's font dictionary sends the code
/// through.
const MAPPED_CODE: u16 = 65;

/// How many font units one em is, in every program built here.
const UNITS_PER_EM: u16 = 1000;

/// How many glyphs one of these programs defines: `.notdef` and one more.
const GLYPHS: u16 = 2;

/// A program carrying the given `cmap` subtables, and vertical metrics where asked for.
///
/// `vertical` is the advance height `vmtx` states for every glyph, in font units; `None` leaves
/// both vertical tables out, which is the shape ISO 19005-4 section 6.2.10.5's third paragraph
/// makes its own condition.
fn truetype(subtables: &[(u16, u16)], vertical: Option<u16>) -> Vec<u8> {
    let mut head = vec![0u8; 54];
    head[0..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    head[12..16].copy_from_slice(&0x5F0F_3CF5u32.to_be_bytes());
    head[18..20].copy_from_slice(&UNITS_PER_EM.to_be_bytes());
    // `indexToLocFormat` zero: a `loca` of two-byte halved offsets.
    head[50..52].copy_from_slice(&0u16.to_be_bytes());

    let mut hhea = vec![0u8; 36];
    hhea[0..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    hhea[34..36].copy_from_slice(&1u16.to_be_bytes());

    let mut maxp = vec![0u8; 32];
    maxp[0..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    maxp[4..6].copy_from_slice(&GLYPHS.to_be_bytes());

    // One long metric of advance 500, then a left side bearing for the remaining glyph.
    let mut hmtx = Vec::new();
    hmtx.extend_from_slice(&500u16.to_be_bytes());
    hmtx.extend_from_slice(&0i16.to_be_bytes());
    hmtx.extend_from_slice(&0i16.to_be_bytes());

    // Every glyph empty: three halved offsets, all zero, and no outline data.
    let loca = vec![0u8; usize::from(GLYPHS + 1) * 2];

    let mut tables = vec![
        (*b"cmap", cmap(subtables)),
        (*b"glyf", vec![0; 4]),
        (*b"head", head),
        (*b"hhea", hhea),
        (*b"hmtx", hmtx),
        (*b"loca", loca),
        (*b"maxp", maxp),
    ];
    if let Some(advance) = vertical {
        let mut vhea = vec![0u8; 36];
        vhea[0..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        vhea[34..36].copy_from_slice(&1u16.to_be_bytes());
        let mut vmtx = Vec::new();
        vmtx.extend_from_slice(&advance.to_be_bytes());
        vmtx.extend_from_slice(&0i16.to_be_bytes());
        vmtx.extend_from_slice(&0i16.to_be_bytes());
        tables.push((*b"vhea", vhea));
        tables.push((*b"vmtx", vmtx));
    }
    tables.sort_by_key(|(tag, _)| *tag);
    sfnt(&tables)
}

// -----------------------------------------------------------------------------------------
// ISO 19005-2 section 6.2.11.6 and ISO 19005-4 section 6.2.10.6 — a Differences array in a
// non-symbolic TrueType font.
// -----------------------------------------------------------------------------------------

/// A page showing one glyph of a non-symbolic TrueType font with the given encoding and program.
fn truetype_page(encoding: &str, program: &[u8]) -> Document {
    page(
        "",
        "/Resources << /Font << /F0 5 0 R >> >>",
        "BT /F0 12 Tf (A) Tj ET",
        vec![
            object(
                5,
                &format!(
                    "<< /Type /Font /Subtype /TrueType /BaseFont /Fixture /FirstChar 65 \
                     /LastChar 65 /Widths [500] /FontDescriptor 6 0 R {encoding} >>"
                ),
            ),
            object(
                6,
                "<< /Type /FontDescriptor /FontName /Fixture /Flags 32 /ItalicAngle 0 \
                 /Ascent 800 /Descent -200 /CapHeight 700 /StemV 80 /FontBBox [0 -200 500 800] \
                 /FontFile2 7 0 R >>",
            ),
            stream(7, &format!("/Length1 {}", program.len()), program),
        ],
    )
}

/// Both parts allow a `Differences` array in a non-symbolic TrueType font only where the embedded
/// program carries the Microsoft Unicode (3, 1) subtable. A program carrying only Macintosh Roman
/// (1, 0) satisfies the paragraph above it — which admits either — and fails this one.
#[test]
fn a_differences_array_without_the_unicode_cmap_fails() {
    let document = truetype_page(
        "/Encoding << /Type /Encoding /BaseEncoding /WinAnsiEncoding /Differences [65 /A] >>",
        &truetype(&[(1, 0)], None),
    );
    for target in [PART_TWO, PART_FOUR] {
        fails(
            &document,
            target,
            "fonts/non-symbolic-truetype-differences-need-the-unicode-cmap",
        );
    }
}

#[test]
fn a_differences_array_with_the_unicode_cmap_passes() {
    let document = truetype_page(
        "/Encoding << /Type /Encoding /BaseEncoding /WinAnsiEncoding /Differences [65 /A] >>",
        &truetype(&[(1, 0), (3, 1)], None),
    );
    for target in [PART_TWO, PART_FOUR] {
        passes(
            &document,
            target,
            "fonts/non-symbolic-truetype-differences-need-the-unicode-cmap",
        );
    }
}

/// The same program with no `Differences` array at all: the clause's condition is the array, so
/// a font without one is outside the rule however its `cmap` is built.
#[test]
fn no_differences_array_without_the_unicode_cmap_passes() {
    let document = truetype_page("/Encoding /WinAnsiEncoding", &truetype(&[(1, 0)], None));
    for target in [PART_TWO, PART_FOUR] {
        passes(
            &document,
            target,
            "fonts/non-symbolic-truetype-differences-need-the-unicode-cmap",
        );
    }
}

// -----------------------------------------------------------------------------------------
// ISO 19005-4 section 6.2.10.5 — vertical metrics against the program's.
// -----------------------------------------------------------------------------------------

/// A page showing one glyph of a vertical composite font whose program states `advance` as the
/// advance height of every glyph, against the `DW2` the font dictionary states. `None` leaves the
/// program's vertical tables out altogether.
fn vertical_page(default_vertical: &str, advance: Option<u16>) -> Document {
    let program = truetype(&[(3, 1)], advance);
    page(
        "",
        "/Resources << /Font << /F0 5 0 R >> >>",
        "BT /F0 12 Tf <0001> Tj ET",
        vec![
            object(
                5,
                "<< /Type /Font /Subtype /Type0 /BaseFont /Fixture /Encoding /Identity-V \
                 /DescendantFonts [6 0 R] >>",
            ),
            object(
                6,
                &format!(
                    "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Fixture \
                     /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> \
                     /FontDescriptor 7 0 R /DW 1000 {default_vertical} /CIDToGIDMap /Identity >>"
                ),
            ),
            object(
                7,
                "<< /Type /FontDescriptor /FontName /Fixture /Flags 4 /ItalicAngle 0 \
                 /Ascent 800 /Descent -200 /CapHeight 700 /StemV 80 /FontBBox [0 -200 500 800] \
                 /FontFile2 8 0 R >>",
            ),
            stream(8, &format!("/Length1 {}", program.len()), &program),
        ],
    )
}

/// The clause asks a composite font rendered in vertical writing mode, whose program carries
/// vertical metrics, to state `DW2` and `W2` consistent with them — consistent being a difference
/// of no more than one thousandth of a text space unit. A `vmtx` advance of one em against a
/// `DW2` displacement of half of one is a difference of five hundred times that.
#[test]
fn a_dw2_disagreeing_with_the_programs_vmtx_fails() {
    let document = vertical_page("/DW2 [880 -500]", Some(UNITS_PER_EM));
    fails(
        &document,
        PART_FOUR,
        "fonts/vertical-metrics-agree-with-the-program",
    );
}

#[test]
fn a_dw2_agreeing_with_the_programs_vmtx_passes() {
    let document = vertical_page("/DW2 [880 -1000]", Some(UNITS_PER_EM));
    passes(
        &document,
        PART_FOUR,
        "fonts/vertical-metrics-agree-with-the-program",
    );
}

/// The same disagreement in a program carrying no vertical metrics: the clause's condition is
/// that the program contains them, so this document is outside the rule.
#[test]
fn a_dw2_in_a_program_without_vertical_metrics_passes() {
    let document = vertical_page("/DW2 [880 -500]", None);
    passes(
        &document,
        PART_FOUR,
        "fonts/vertical-metrics-agree-with-the-program",
    );
}
