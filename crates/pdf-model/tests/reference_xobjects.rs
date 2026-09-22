//! Reference `XObject`s: ISO 32000-2 §8.10.4, and the two processors the clause addresses.
//!
//! # Why this file exists
//!
//! §8.10.4.1 writes a `shall` to each of two classes, and which one this program is depends on
//! what a **host** supplied rather than on anything in the code or in the file:
//!
//! > PDF processors that do not recognise the Ref entry shall simply display or print the proxy
//! > as an ordinary form XObject. Those PDF processors that do implement reference XObjects shall
//! > use the proxy in place of the imported content if the latter is unavailable.
//!
//! `CLAUDE.md` principle 3 gives the renderer no filesystem, so with no target document in hand
//! this reader draws the proxy — which is the second sentence, "if the latter is unavailable", and
//! was the whole of what this tree did until the one-thousand-and-eighty-seventh session. What
//! changed is that a host can now put the file in front of it (`pdf_model::reference`, ADR 1101),
//! and then the *first* half of that sentence binds: the referenced page is drawn.
//!
//! So this file holds both halves, against one pair of documents, differing in what is supplied
//! and in nothing else. The half that draws the proxy is the older claim and is a claim about an
//! *absence*, which is the shape that rots quietly; the half that imports is new and every one of
//! its assertions is written so that the answer it excludes is the other document's own colour.
//!
//! # Why the fixtures are hand-built
//!
//! `doc/traps/parsers-and-streams.md`'s trap 8 prefers real documents, and there are none.
//! `examples/absence_audit` asks §8.10.4.1's own condition — a form `XObject` whose form
//! dictionary holds a `/Ref` dictionary, the subtype included, which is what tells Table 93's
//! entry from Table 355's array on a `/TOCI` structure element — and finds **no witness anywhere
//! on this disk**: none in the curated population, none in the `SafeDocs` `CC-MAIN-2021-31`
//! crawl, and none in `corpus-cache/openpreserve` and `corpus-cache/tika-issue-tracker`, which
//! that example had no scope for until this session and now has (§8.10.4's ledger row carries the
//! counts). The second half of the census is empty by construction rather than by measurement:
//! no document states a `/Ref`, so no `/Ref` states an `/ID`, so no target file named by one can
//! be on this disk to be matched. The block was calibrated by pointing it at `/Group`, which
//! names documents in the same corpora, so the zero is a measurement and not a blind spot.
//!
//! The same reasoning §8.10.3's row records for `/Group << /S /Softness >>` applies — a
//! requirement no file exercises is held by a fixture or by nothing.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture should fail loudly, and these pages are 100 units \
              square where no arithmetic can overflow"
)]

use std::fmt::Write as _;

use pdf_model::reference::Supply;
use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;

/// The four colours the pair states, one per thing an assertion has to tell apart.
///
/// Read off the fixtures below rather than off a render: every assertion here is an exact
/// equality against one of these, so a page drawn out of the wrong document fails by naming the
/// document it came from.
const PROXY_RED: [u8; 4] = [255, 0, 0, 255];
const TARGET_BLUE: [u8; 4] = [0, 0, 255, 255];
const ANNOTATION_YELLOW: [u8; 4] = [255, 255, 0, 255];
const CONTAINING_MAGENTA: [u8; 4] = [255, 0, 255, 255];
const DECOY_GREEN: [u8; 4] = [0, 255, 0, 255];
const BLANK: [u8; 4] = [255, 255, 255, 255];

/// §14.4's identifier the target document states in its own trailer.
const TARGET_ID: (&str, &str) = ("0102", "0304");

/// The containing document: one page that draws a shading of its own and then a proxy.
///
/// `reference` is the proxy form's `/Ref` entry, written whole so that a test can state a
/// different one — or none — and change nothing else. The proxy fills 100 units square while its
/// `/BBox` is half that, so §8.10.4.1's "clipped to the boundaries of its bounding box" is
/// visible whichever of the two things is drawn.
///
/// **Object 6 is a `/Shading` resource table here and a `/Shading` resource table in the target
/// document too, and they name different shadings.** That is not decoration, and it is what the
/// [`DECOY_GREEN`] object in the target is for: `Interpreter::resource_tables` memoises §7.8.3's
/// category tables by the [`pdf_syntax::ObjectId`] the resource dictionary states, and every other
/// memo the interpreter holds is keyed by an object of *one* file in the same way. Two files hand
/// out the same object numbers, so a reader that imported a page without swapping those memos
/// would look the target page's `/Sh0` up in this document's table and find the wrong object.
fn containing(reference: &str) -> Vec<u8> {
    // The page's own shading, in the top-right corner and clear of the proxy's `/BBox`, so that
    // what it draws is a fact about this document whichever answer the `Do` gives.
    let page = "q 60 0 30 30 re W n /Sh0 sh Q /Fm Do";
    let proxy = "1 0 0 rg 0 0 100 100 re f";

    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /XObject << /Fm 5 0 R >> /Shading 6 0 R >> \
         /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{page}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 50 50] {reference} \
         /Length {} >>\nstream\n{proxy}\nendstream\nendobj\n\
         6 0 obj\n<< /Sh0 7 0 R >>\nendobj\n\
         7 0 obj\n{}\nendobj\n",
        page.len() + 1,
        proxy.len() + 1,
        shading("1 0 1")
    );
    assemble(&body, Some(("1111", "2222")))
}

/// The target document: one page filled by a shading, carrying one annotation.
///
/// Its `/ID` is [`TARGET_ID`], which is the only thing a `/Ref` can name it by.
///
/// **Object 7 is a shading this page never paints**, and it is [`DECOY_GREEN`] for the reason
/// [`containing`] gives: it is what object 7 would be if the containing document's `/Shading`
/// table — its own object 6 — were still the one in force when this page's `/Sh0` is looked up.
fn target() -> Vec<u8> {
    let page = "q 0 0 100 100 re W n /Sh0 sh Q";
    let appearance = "1 1 0 rg 0 0 40 40 re f";

    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /Shading 6 0 R >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{page}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Square /Rect [30 30 70 70] /F 4 \
         /AP << /N 9 0 R >> >>\nendobj\n\
         6 0 obj\n<< /Sh0 8 0 R >>\nendobj\n\
         7 0 obj\n{}\nendobj\n\
         8 0 obj\n{}\nendobj\n\
         9 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 40 40] /Length {} >>\n\
         stream\n{appearance}\nendstream\nendobj\n",
        page.len() + 1,
        shading("0 1 0"),
        shading("0 0 1"),
        appearance.len() + 1
    );
    assemble(&body, Some(TARGET_ID))
}

/// One flat §8.7.4.4 axial shading, as a direct dictionary, in the colour given.
fn shading(colour: &str) -> String {
    format!(
        "<< /ShadingType 2 /ColorSpace /DeviceRGB /Coords [0 0 100 0] /Extend [true true] \
         /Function << /FunctionType 2 /Domain [0 1] /C0 [{colour}] /C1 [{colour}] /N 1 >> >>"
    )
}

/// Table 95's reference dictionary, filled in: the target document's file, its page and its `/ID`.
///
/// `/F` and `/Page` are the two the table makes required; `/ID` is optional and is what this
/// reader matches on, because §14.4 states the match and a path states somebody else's
/// filesystem. `/Page 0` is the first page: §12.4.2 counts page indices from zero.
fn reference(id: Option<(&str, &str)>) -> String {
    let id = id.map_or_else(String::new, |(first, second)| {
        format!(" /ID [<{first}> <{second}>]")
    });
    format!("/Ref << /F << /Type /Filespec /F (target.pdf) /UF (target.pdf) >> /Page 0{id} >>")
}

/// Wraps a body of numbered objects in §7.5's header, cross-reference table and trailer.
fn assemble(body: &str, id: Option<(&str, &str)>) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    // §7.5.5 Table 15 makes `/ID` "Required in PDF 2.0 and later"; §14.4 is what it is for.
    let id = id.map_or_else(String::new, |(first, second)| {
        format!(" /ID [<{first}> <{second}>]")
    });
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R{id} >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// Interprets the containing document's only page against the supplied target documents.
fn interpret(reference: &str, supply: &Supply) -> pdf_model::Interpretation {
    let bytes = containing(reference);
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("the fixture's page");
    pdf_model::content::interpret_importing(
        &document,
        &page,
        &pdf_model::view::ViewState::of(&document),
        &pdf_model::FontCache::new(),
        supply,
    )
}

/// Interprets against a supply, under a view state a caller has set.
///
/// [`interpret`]'s one variant, and the only thing it varies is the state: §10.8.3's simulation
/// is a request a *reader* made of this program, so it arrives here and nowhere in either file.
fn interpret_under(
    reference: &str,
    supply: &Supply,
    state: &pdf_model::view::ViewState,
) -> pdf_model::Interpretation {
    let bytes = containing(reference);
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("the fixture's page");
    pdf_model::content::interpret_importing(
        &document,
        &page,
        state,
        &pdf_model::FontCache::new(),
        supply,
    )
}

/// A target document whose page is filled through one §8.6.6.5 `NChannel` space.
///
/// Its tint transform states red and its `/Colorants` state blue, so a reader that ignored
/// §10.8.3 draws red and one that simulated the inks does not — which is the whole
/// discrimination the test below rests on, and it is `colour_paths.rs`'s own fixture for the
/// same clause. Its `/ID` is [`TARGET_ID`], because that is what the proxy's `/Ref` matches.
fn target_with_a_spot() -> Vec<u8> {
    let page = "/Sep cs 1 0 scn 0 0 100 100 re f";
    let program = "{ pop pop 1 0 0 }";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /ColorSpace << /Sep 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{page}\nendstream\nendobj\n\
         5 0 obj\n[/DeviceN [/Spot1 /Spot2] /DeviceRGB 6 0 R 7 0 R]\nendobj\n\
         6 0 obj\n<< /FunctionType 4 /Domain [0 1 0 1] /Range [0 1 0 1 0 1] /Length {} >>\n\
         stream\n{program}\nendstream\nendobj\n\
         7 0 obj\n<< /Subtype /NChannel /Colorants 8 0 R >>\nendobj\n\
         8 0 obj\n<< /Spot1 [/Separation /Spot1 /DeviceRGB 9 0 R] \
         /Spot2 [/Separation /Spot2 /DeviceRGB 10 0 R] >>\nendobj\n\
         9 0 obj\n<< /FunctionType 2 /Domain [0 1] /C0 [1 1 1] /C1 [0 0 1] /N 1 >>\nendobj\n\
         10 0 obj\n<< /FunctionType 2 /Domain [0 1] /C0 [1 1 1] /C1 [0 1 0] /N 1 >>\nendobj\n",
        page.len() + 1,
        program.len() + 1
    );
    assemble(&body, Some(TARGET_ID))
}

/// §10.8.3's answer is the **reader's**, so it crosses into an imported page with the reader.
///
/// The clause conditions itself on what a reader asked this program for rather than on anything
/// either file states, which is the same sentence §12.5.3's magnification and §8.11.4.4's event
/// already have where they cross this boundary (ADR 1228). A page imported from another document
/// is drawn inside the page importing it, so the two drawn under different answers would be one
/// raster with two readings of one request.
///
/// The assertion is that the answer *reaches* the imported page, measured by removing it: with
/// the request off the separation comes out of its alternate space and with it on it does not.
/// A reader that dropped the answer at the boundary draws the same pixel both times.
#[test]
fn a_readers_separation_request_crosses_into_an_imported_page() {
    let (supply, refused) = Supply::read([("target.pdf", target_with_a_spot())], "the test's own");
    assert!(refused.is_empty(), "{refused:?}");

    let document = Document::open(containing("")).expect("the fixture is a valid PDF");
    let plain = pdf_model::view::ViewState::of(&document);
    let mut simulating = pdf_model::view::ViewState::of(&document);
    assert!(
        simulating.set_separation_simulation(true),
        "the default is off, so turning it on is a change"
    );

    let alternate = interpret_under(&reference(Some(TARGET_ID)), &supply, &plain);
    let simulated = interpret_under(&reference(Some(TARGET_ID)), &supply, &simulating);
    assert_eq!(said(&alternate), "", "{:?}", alternate.unsupported);
    assert_eq!(said(&simulated), "", "{:?}", simulated.unsupported);

    assert_eq!(
        at(&alternate, 10, 10),
        PROXY_RED,
        "the alternate space states red, which is what §8.6.6.4 draws without the request"
    );
    assert_ne!(
        at(&simulated, 10, 10),
        at(&alternate, 10, 10),
        "the reader asked for the ink to be simulated and the imported page did not hear it"
    );
}

/// The target document, supplied as a host would.
fn supplied() -> Supply {
    let (supply, refused) = Supply::read([("target.pdf", target())], "the test's own bytes");
    assert!(refused.is_empty(), "{refused:?}");
    assert_eq!(supply.len(), 1);
    supply
}

/// The fixture rendered at one pixel per unit, as RGBA rows.
fn raster(interpretation: &pdf_model::Interpretation) -> Vec<[u8; 4]> {
    let list = &interpretation.display_list;
    let target = TargetSpec::for_page(list, 1.0, 1 << 20).expect("a 100x100 target");
    let raster = render_cpu::CpuRasterizer::new()
        .rasterize(list, target)
        .expect("the fixture rasterises");
    raster
        .data
        .chunks_exact(4)
        .map(|bytes| [bytes[0], bytes[1], bytes[2], bytes[3]])
        .collect()
}

/// The RGBA pixel at **page** `(x, y)` of a 100-unit-square page, rastered one pixel per unit.
///
/// Page coordinates rather than device ones, because every assertion here is about where a
/// *document* put something. `TargetSpec::for_page` does the flip, about the page's height, and
/// stating it once here is what keeps trap 12a out of a file with a dozen of these.
fn at(interpretation: &pdf_model::Interpretation, x: usize, y: usize) -> [u8; 4] {
    raster(interpretation)[(99 - y) * 100 + x]
}

/// Whatever the interpretation said about §8.10.4, joined — empty where it said nothing.
fn said(interpretation: &pdf_model::Interpretation) -> String {
    interpretation
        .unsupported
        .iter()
        .filter_map(|report| match report {
            pdf_model::Unsupported::ReferenceXObject { detail } => Some(detail.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// §8.10.4.1's provision for a reader with no target document, exercised and pinned.
///
/// > PDF processors that do not recognise the Ref entry shall simply display or print the proxy
/// > as an ordinary form XObject.
///
/// *Simply* and *ordinary* are the load-bearing words, and the strongest reading of them is that
/// the entry changes nothing: the same proxy with and without `/Ref` produces the same display
/// list, command for command. The pixels are asserted as well as the list because "ordinary form
/// `XObject`" carries §8.10.1's step c) with it — a proxy is clipped by its own `/BBox` like any
/// other form, and a reader that exempted one would draw past it.
///
/// Nothing is reported, and that is the clause's doing rather than an oversight
/// (`doc/traps/instruments-and-reports.md`'s trap 11): §8.10.4.1 states this alternative, so there
/// is no gap to name. A report here would take every page holding a proxy out of the oracle's
/// comparison for a requirement the standard says is met.
#[test]
fn a_proxy_carrying_ref_is_drawn_as_an_ordinary_form_xobject_where_nothing_was_supplied() {
    let with = interpret(&reference(Some(TARGET_ID)), &Supply::NONE);
    let without = interpret("", &Supply::NONE);

    assert!(with.is_complete(), "{:?}", with.unsupported);
    assert!(without.is_complete(), "{:?}", without.unsupported);
    assert_eq!(
        with.display_list.commands(),
        without.display_list.commands(),
        "/Ref changed what the proxy draws"
    );
    assert_eq!(said(&with), "", "a supply nobody offered is not a gap");

    assert_eq!(at(&with, 10, 10), PROXY_RED, "the proxy paints");
    assert_eq!(
        at(&with, 60, 60),
        BLANK,
        "and is clipped by its own /BBox, as any form is"
    );
}

/// §8.10.4.1's other half: the referenced page, drawn where the proxy was.
///
/// > Those PDF processors that do implement reference XObjects shall use the proxy in place of the
/// > imported content if the latter is unavailable.
///
/// — so where it *is* available the imported content is what is drawn. And §8.10.4.1 says exactly
/// how:
///
/// > When the imported content replaces the proxy, it shall be transformed according to the proxy
/// > object's transformation matrix and clipped to the boundaries of its bounding box, as
/// > specified by the Matrix and BBox entries in the proxy's form dictionary
///
/// Four pixels, one per claim, and each excludes a different wrong answer: the proxy's own red
/// inside the box, the target's blue outside it, the containing page's green drawn from another
/// file's object 6, and a page that drew nothing at all.
#[test]
fn a_supplied_target_page_is_drawn_in_the_proxys_place() {
    let drawn = interpret(&reference(Some(TARGET_ID)), &supplied());

    assert_eq!(said(&drawn), "", "the page was found and is unchanged");
    assert_eq!(
        at(&drawn, 10, 10),
        TARGET_BLUE,
        "the referenced page's own content is what the proxy's place holds"
    );
    assert_eq!(
        at(&drawn, 60, 60),
        BLANK,
        "and it is clipped by the proxy's /BBox, which §8.10.4.1 makes the imported page's"
    );
    assert_eq!(
        at(&drawn, 70, 20),
        CONTAINING_MAGENTA,
        "the containing page's own shading is still this document's"
    );
    assert_ne!(
        at(&drawn, 10, 10),
        DECOY_GREEN,
        "the imported page's /Sh0 was looked up in the containing document's /Shading table"
    );
}

/// §8.10.4.3's first consideration, which is the only one of its two that binds a reader.
///
/// > When the page imported by a reference XObject contains annotations (see 12.5,
/// > "Annotations"), all annotations that contain a printable, unhidden, visible appearance stream
/// > (12.5.5, "Appearance streams") shall be included in the rendering of the imported page.
///
/// The target page's `Square` states `/F 4` — Table 167's Print, nothing hidden — and an
/// appearance stream that fills its own box yellow. "[I]ncluded in the rendering of the imported
/// page" is read at its strongest: the imported page is *rendered*, annotations and all, and that
/// rendering is then what §8.10.4.1 clips to the proxy's box. So the annotation is half inside
/// and half outside the box on purpose, and both halves are asserted — an appearance drawn over
/// the containing page the way this document's own annotations are would spill past (50, 50).
#[test]
fn an_imported_pages_annotation_is_drawn_and_clipped_with_the_page_it_is_on() {
    let drawn = interpret(&reference(Some(TARGET_ID)), &supplied());

    assert_eq!(
        at(&drawn, 35, 35),
        ANNOTATION_YELLOW,
        "the imported page's annotation appearance is part of its rendering"
    );
    assert_eq!(
        at(&drawn, 60, 60),
        BLANK,
        "and the part of it outside the proxy's /BBox is clipped away with the page"
    );
}

/// §14.4's match is on the identifier, and a file that is not the one named is refused by name.
///
/// > If the first identifier in the reference matches the first identifier in the referenced
/// > file's ID entry, and the last identifier in the reference matches the last identifier in the
/// > referenced file's ID entry, it is very likely that the correct and unchanged PDF file has
/// > been found.
///
/// A supply is not a permission to draw whatever is in it. The file offered here is the same file
/// as ever and the reference names a different one, so §8.10.4.1's "unavailable" is the truth and
/// the proxy is what is drawn — and this time it *is* reported, because a reader who supplied
/// files and got a grey box is owed the reason (trap 5).
#[test]
fn a_supplied_file_the_reference_does_not_name_is_refused_by_name() {
    let drawn = interpret(&reference(Some(("aaaa", "0304"))), &supplied());

    assert_eq!(at(&drawn, 10, 10), PROXY_RED, "the proxy is drawn");
    assert!(
        said(&drawn).contains("none of the 1 supplied target document(s) states its /ID"),
        "{}",
        said(&drawn)
    );
}

/// §14.4's second string: the right file, a different version of it — drawn, and said.
///
/// > If only the first identifier matches, a different version of the correct PDF file has been
/// > found.
///
/// Table 95 says what the entry is for in the same breath: it "allows it to warn the user if the
/// PDF file has changed since the reference was created". A warning is not a refusal, so the page
/// is drawn — and the sentence names the file, because a reader looking at a page that may not be
/// the page the producer saw has to be able to find out which file it came from.
#[test]
fn a_file_whose_changing_identifier_moved_is_drawn_and_said() {
    let drawn = interpret(&reference(Some(("0102", "9999"))), &supplied());

    assert_eq!(
        at(&drawn, 10, 10),
        TARGET_BLUE,
        "the correct file was found, so its page is drawn"
    );
    assert!(
        said(&drawn).contains("different version of the file"),
        "{}",
        said(&drawn)
    );
}

/// A reference with no `/ID` names nothing this reader can match, and says so.
///
/// Table 95 makes `/ID` optional, and this is the cost of leaving it out here: the only other
/// thing the dictionary offers is `/F`, a path into somebody else's filesystem, and matching on it
/// would let a *document* choose which of the host's files is opened for it. So the proxy is
/// drawn — §8.10.4.1's own answer — and the reason is stated rather than left to look like a
/// document with nothing in it.
#[test]
fn a_reference_with_no_identifier_imports_nothing_and_says_why() {
    let drawn = interpret(&reference(None), &supplied());

    assert_eq!(at(&drawn, 10, 10), PROXY_RED, "the proxy is drawn");
    assert!(said(&drawn).contains("states no /ID"), "{}", said(&drawn));
}

/// §8.10.4.1 Table 95's own sentence about a `/Page` that no longer exists.
///
/// > This reference is a weak one and may be inadvertently invalidated if the referenced page is
/// > changed or replaced in the target document after the reference is created.
///
/// The file is found and the page is not, which is the clause describing its own failure mode. The
/// proxy is drawn and the sentence names the file, so that a reader can tell this apart from the
/// file not being there at all — the two have different remedies.
#[test]
fn a_page_the_named_file_does_not_have_falls_back_to_the_proxy() {
    let stated = reference(Some(TARGET_ID)).replace("/Page 0", "/Page 7");
    let drawn = interpret(&stated, &supplied());

    assert_eq!(at(&drawn, 10, 10), PROXY_RED, "the proxy is drawn");
    assert!(
        said(&drawn).contains("that the file does not have"),
        "{}",
        said(&drawn)
    );
}

/// A file a host offered that is not a PDF is refused at the supply, by name.
///
/// `pdf_signature::trust::Supply`'s rule one crate over, and for its reason: a person who pointed
/// at a directory of two files and got one target document would otherwise be reading pages
/// imported under a supply they did not supply. The refusal carries the name the host used, which
/// is the only name this crate has for a file it cannot open.
#[test]
fn a_file_that_is_not_a_pdf_is_refused_at_the_supply_and_named() {
    let (supply, refused) = Supply::read(
        [
            ("target.pdf", target()),
            ("notes.txt", b"not a PDF at all".to_vec()),
        ],
        "the test's own bytes",
    );
    assert_eq!(supply.len(), 1, "the PDF is kept");
    assert_eq!(refused.len(), 1, "{refused:?}");
    assert!(
        refused[0].to_string().contains("notes.txt"),
        "{}",
        refused[0]
    );
}

/// A PDF with no `/ID` of its own is refused too, because nothing could ever name it.
///
/// §7.5.5 Table 15 makes the entry "Required in PDF 2.0 and later, or if an Encrypt entry is
/// present; optional otherwise", so a file without one is pre-2.0 or malformed — a fact about the
/// file. Keeping it would be keeping a document no reference can reach, which is a supply that
/// looks larger than it is.
#[test]
fn a_target_document_stating_no_identifier_is_refused() {
    let without = {
        let whole = String::from_utf8(target()).expect("the fixture is ASCII");
        whole
            .replace(&format!(" /ID [<{}> <{}>]", TARGET_ID.0, TARGET_ID.1), "")
            .into_bytes()
    };
    let (supply, refused) = Supply::read([("target.pdf", without)], "the test's own bytes");
    assert!(supply.is_empty(), "nothing can name it");
    assert_eq!(refused.len(), 1, "{refused:?}");
    assert!(
        refused[0].to_string().contains("states no /ID"),
        "{}",
        refused[0]
    );
}
