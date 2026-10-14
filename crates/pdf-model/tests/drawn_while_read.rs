//! A page drawn from its display list while the list is still being read (ADR 1855).
//!
//! The interpreter hands its list over in parts (`pdf_model::progress::Parts`) and a drawer
//! paints each part into one target as it arrives (`render_cpu::Underway`). **The property that
//! decides whether this may ship is that the page it finishes is the page drawn whole, byte for
//! byte** — this backend is the oracle — and that every page where that is not known is told so
//! and drawn whole instead. Both halves are checked here: on pages built to be handed over, on
//! pages built to spoil the hand-over, and, ignored, over the first page of every tracked corpus
//! document.

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    reason = "test code: a fixture that does not open should fail loudly, the gate refuses to run \
              without the sandboxed decoder, the corpus sweep prints what it found, and the \
              fixture's coordinates are small counts"
)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use pdf_model::Pages;
use pdf_model::content::divide::{Division, Reading, last_pipeline_decline, last_reading};
use pdf_model::content::{Interpretation, displayed_size, interpret};
use pdf_model::progress::{Handed, Parts};
use pdf_render::{DisplayList, Raster, Rasterizer as _, TargetSpec};
use pdf_syntax::Document;
use render_cpu::{CpuRasterizer, Underway};

/// Pixel budget for a target, the corpus gates' own.
const PIXEL_BUDGET: u64 = 64 << 20;

/// A one-page document of `size` points square, with `resources` and `content`.
fn page_with(size: u32, resources: &str, content: &str) -> Vec<u8> {
    page_with_catalog(size, "", resources, content, "")
}

/// [`page_with`], the catalog stating `catalog` beside its pages and the file holding `objects`
/// after the four it always holds.
fn page_with_catalog(
    size: u32,
    catalog: &str,
    resources: &str,
    content: &str,
    objects: &str,
) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R {catalog} >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {size} {size}] \
         /Resources {resources} /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n{objects}",
        content.len().saturating_add(1)
    );
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let entries = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {entries}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {entries} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// `count` short strokes and fills over a page, each in a colour of its own, every one inside
/// its own `q … Q` as the witness states its marks.
fn marks(count: usize) -> String {
    let mut content = String::new();
    for index in 0..count {
        let (x, y) = ((index * 7) % 190, (index * 13) % 190);
        let shade = (index % 10) as f32 / 10.0;
        if index % 3 == 0 {
            let _ = writeln!(
                content,
                "q {shade} 0.2 0.5 rg 1 0 0 1 {x} {y} cm 0 0 6 4 re f Q"
            );
        } else {
            let _ = writeln!(
                content,
                "q {shade} 0.5 0.2 RG 0.6 w 1 0 0 1 {x} {y} cm 0 0 m 8 3 l 3 9 l S Q"
            );
        }
    }
    content
}

/// What drawing a page while it is read came to.
struct Drawn {
    list: Arc<DisplayList>,
    target: TargetSpec,
    parts: usize,
    ended: Result<pdf_render::Stated, &'static str>,
    finished: Result<Raster, &'static str>,
}

/// Interprets page one of `document` with a hand-over installed while a second thread draws
/// every part it is handed into one target, then finishes that target from the finished list.
fn drawn_while_read(document: &Document) -> Option<Drawn> {
    drawn_in_parts(document, &Parts::new())
}

/// [`drawn_while_read`], handed over to `parts`.
fn drawn_in_parts(document: &Document, parts: &Parts) -> Option<Drawn> {
    let page = Pages::new(document).get(0)?;
    let size = displayed_size(&page);
    let target = TargetSpec::for_page(&DisplayList::new(size), 1.0, PIXEL_BUDGET).ok()?;
    let (interpretation, (underway, handed, ended)): (Interpretation, _) =
        std::thread::scope(|scope| {
            let drawer = {
                let parts = parts.clone();
                scope.spawn(move || {
                    let mut underway = Underway::new(&CpuRasterizer::new(), target);
                    let mut handed = 0_usize;
                    loop {
                        match parts.take(Duration::from_mins(1)) {
                            Some(Handed::Part(part)) => {
                                handed += 1;
                                if let Some(underway) = underway.as_mut() {
                                    underway.take(part);
                                }
                            }
                            Some(Handed::Ended(ended)) => return (underway, handed, ended),
                            None => return (underway, handed, Err("nothing came for a minute")),
                        }
                    }
                })
            };
            let interpretation = parts.during(|| interpret(document, &page));
            (
                interpretation,
                drawer.join().expect("the drawer does not panic"),
            )
        });
    let list = Arc::new(interpretation.display_list);
    parts.keep(&list);
    assert!(parts.is(&list), "the list kept is the list the parts name");
    let finished = match (&ended, underway) {
        (Ok(_), Some(underway)) => underway.finish(&list),
        (Err(reason), _) => Err(*reason),
        (Ok(_), None) => Err("no target"),
    };
    Some(Drawn {
        list,
        target,
        parts: handed,
        ended,
        finished,
    })
}

/// The page drawn whole as one strip, which is the oracle's own drawing of it: `raster_golden`
/// holds this one, and a page divided into strips is the same picture only very nearly (ADRs
/// 0139, 1758), which is a property of the division and not of drawing while reading.
fn whole(list: &DisplayList, target: TargetSpec) -> Option<Raster> {
    CpuRasterizer::new()
        .with_strips(1)
        .rasterize(list, target)
        .ok()
}

#[test]
fn a_page_drawn_while_it_is_read_is_the_page_drawn_whole() {
    let document =
        Document::open(page_with(200, "<< >>", &marks(40_000))).expect("the fixture opens");
    let drawn = drawn_while_read(&document).expect("page one draws");
    assert!(
        drawn.parts > 1,
        "a page of 40 000 marks is handed over in parts, and was handed over in {}",
        drawn.parts
    );
    assert_eq!(
        drawn.ended.map(|stated| stated.commands),
        Ok(drawn.list.command_count())
    );
    let finished = drawn
        .finished
        .expect("every part was drawn and the page finished");
    let serial = whole(&drawn.list, drawn.target).expect("the page draws");
    assert!(
        finished.data == serial.data,
        "drawn while read is the page drawn whole"
    );
}

#[test]
fn a_page_smaller_than_a_part_hands_nothing_over_and_ends_whole() {
    let document = Document::open(page_with(200, "<< >>", &marks(100))).expect("the fixture opens");
    let drawn = drawn_while_read(&document).expect("page one draws");
    assert_eq!(drawn.parts, 0);
    let finished = drawn
        .finished
        .expect("an empty target finished from the list");
    let serial = whole(&drawn.list, drawn.target).expect("the page draws");
    assert_eq!(finished.data, serial.data);
}

/// §11.6.4.3's `/AIS` changed part-way down a page that has already painted translucent marks
/// under the other reading: the interpreter seals the marks before the change (ADR 1319), which
/// takes them back out of the list — so the parts that held them are not known to be the list's.
#[test]
fn a_command_taken_back_after_it_was_handed_over_spoils_the_hand_over() {
    let resources = "<< /ExtGState << /Shape << /AIS true /ca 0.5 >> \
                     /Opacity << /AIS false /ca 0.5 >> >> >>";
    let mut content = String::from("/Opacity gs\n");
    content.push_str(&marks(20_000));
    content.push_str("/Shape gs\n");
    content.push_str(&marks(20));
    let document = Document::open(page_with(200, resources, &content)).expect("the fixture opens");
    let drawn = drawn_while_read(&document).expect("page one draws");
    assert!(drawn.parts > 0, "parts were handed over before the change");
    assert!(
        drawn.ended.is_err() && drawn.finished.is_err(),
        "the hand-over is spoiled, so the page is drawn whole: {:?}",
        drawn.ended
    );
}

/// A division into `pieces` of every stream, however short — what a sweep asks for, so that
/// the join is asked of every page that has a cut rather than of the few long enough to divide.
const fn every_stream(pieces: usize) -> Division {
    Division {
        from: 0,
        pieces: Some(pieces),
        pipelined: true,
    }
}

/// Everything an interpretation states but its list, rendered whole so that no field is left out
/// of the comparison by a list of fields. The list is compared by its own equality, which is of
/// what it states: its rendering also prints each path's memo of its own hull, which says which
/// questions were asked of which copy of a clip and nothing about the page.
fn statement(interpretation: &Interpretation) -> String {
    let mut stated = interpretation.clone();
    stated.display_list = DisplayList::new(stated.display_list.page_size);
    format!("{stated:?}")
}

/// A list's rendering without the memos each path keeps of its own hull and shape, which say what
/// was asked of a copy and nothing the list states (`Path`'s equality ignores them too).
fn without_memos(rendered: &str) -> String {
    let mut kept = String::with_capacity(rendered.len());
    let mut rest = rendered;
    while let Some(at) = [
        "hull: OnceLock(",
        "collapses: OnceLock(",
        "narrowest_rectangle: OnceLock(",
    ]
    .iter()
    .filter_map(|memo| rest.find(memo).map(|at| (at, memo.len())))
    .min()
    {
        let (at, opened) = at;
        kept.push_str(&rest[..at]);
        let mut depth = 1_usize;
        let mut end = at + opened;
        for (offset, byte) in rest.as_bytes()[at + opened..].iter().enumerate() {
            match byte {
                b'(' => depth += 1,
                b')' => depth -= 1,
                _ => {}
            }
            if depth == 0 {
                end = at + opened + offset + 1;
                break;
            }
        }
        rest = &rest[end..];
    }
    kept.push_str(rest);
    kept
}

/// Prints where two renderings first part, so that a failure names the field rather than the page.
fn print_parting(divided: &str, whole: &str) {
    let at = divided
        .bytes()
        .zip(whole.bytes())
        .position(|(one, other)| one != other)
        .unwrap_or_else(|| divided.len().min(whole.len()));
    let around = |text: &str| {
        let from = text.floor_char_boundary(at.saturating_sub(200));
        let to = text.ceil_char_boundary(at.saturating_add(200).min(text.len()));
        text.get(from..to).unwrap_or_default().to_owned()
    };
    println!("divided: {}\nwhole:   {}", around(divided), around(whole));
}

/// Page one of `document` read divided into `pieces` and read whole: how the divided read went,
/// and whether the two interpretations state the same page.
fn divided_against_whole(
    document: &Document,
    pieces: usize,
) -> Option<(Option<&'static str>, bool)> {
    let page = Pages::new(document).get(0)?;
    let undivided = Division::NEVER.during(|| interpret(document, &page));
    let divided = every_stream(pieces).during(|| interpret(document, &page));
    let reading = last_reading().map(|reading| match reading {
        Reading::Divided => None,
        Reading::Whole(reason) => Some(reason.0),
    });
    let divided_list = divided.display_list.clone();
    let (divided, whole) = (statement(&divided), statement(&undivided));
    // A list holding a NaN — a function's output a sample table states — is unequal to itself
    // by `==`, so a list that `==` does not match is matched by its rendering too, which prints
    // each NaN alike, with the paths' memos left out as equality leaves them.
    let (rendered, rendered_whole) = (
        without_memos(&format!("{divided_list:?}")),
        without_memos(&format!("{:?}", undivided.display_list)),
    );
    let same_list = divided_list == undivided.display_list || rendered == rendered_whole;
    if divided != whole {
        print_parting(&divided, &whole);
    } else if !same_list {
        print_parting(&rendered, &rendered_whole);
    }
    Some((reading?, divided == whole && same_list))
}

#[test]
fn a_page_read_in_pieces_is_the_page_read_whole() {
    // Inside one outer `q`, a clip, a line width and a colour at the page's own depth — the
    // spine each later piece composes its state from — then marks each inside its own `q … Q`,
    // then text, which is the last piece's.
    let mut content = String::from(
        "q 1 0 0 1 5 5 cm 0 0 180 180 re W n 0.4 w 0.1 0.6 0.2 RG
",
    );
    content.push_str(&marks(8_000));
    content.push_str(
        "0.9 0.1 0.1 rg 20 20 40 40 re f
",
    );
    content.push_str(&marks(8_000));
    content.push_str(
        "BT /F1 12 Tf 10 10 Td (Hello) Tj 0 14 Td (again) Tj ET Q
",
    );
    let resources = "<< /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >>";
    let document = Document::open(page_with(200, resources, &content)).expect("the fixture opens");
    for pieces in [2, 3, 7] {
        let (declined, same) = divided_against_whole(&document, pieces).expect("page one reads");
        assert_eq!(declined, None, "{pieces} pieces: the page divides");
        assert!(
            same,
            "{pieces} pieces: the page read in pieces is the page read whole"
        );
    }
}

#[test]
fn a_page_whose_state_the_join_does_not_carry_is_read_whole_and_says_why() {
    // §11.6.4.3's alpha source changed part-way down the page: the record of which reading the
    // marks painted under is the page's, and the piece that changes it declines the division.
    let resources = "<< /ExtGState << /Shape << /AIS true >> >> >>";
    let mut content = marks(2_000);
    content.push_str("/Shape gs\n");
    content.push_str(&marks(2_000));
    let document = Document::open(page_with(200, resources, &content)).expect("the fixture opens");
    let (declined, same) = divided_against_whole(&document, 4).expect("page one reads");
    assert!(same, "the page read whole is the page");
    assert!(
        declined.is_some_and(|reason| reason.contains("alpha readings")),
        "the decline names what declined it: {declined:?}"
    );
}

/// §8.4.5's `gs` composed into a later piece's state, and §8.11.3.2's optional content decided
/// for a piece that begins inside a section: one layer on, one off by the default configuration
/// (§8.11.4.3's `/OFF`), each open across many cuts, a section with an `/MCID` closed inside a
/// piece, and a property list written inline (§14.6.2) — ADR 1894.
#[test]
fn a_page_under_gs_and_inside_optional_content_is_read_in_pieces_as_the_page_read_whole() {
    let resources = "<< /ExtGState << /Half << /ca 0.5 /CA 0.5 /LW 2 /LC 1 >> >> \
                     /Properties << /On 5 0 R /Off 6 0 R >> >>";
    let catalog = "/OCProperties << /OCGs [5 0 R 6 0 R] /D << /OFF [6 0 R] >> >> \
                   /MarkInfo << /Marked true >>";
    let objects = "5 0 obj\n<< /Type /OCG /Name (on) >>\nendobj\n\
                   6 0 obj\n<< /Type /OCG /Name (off) >>\nendobj\n";
    let mut content = String::from("q /Half gs /OC /On BDC\n");
    content.push_str(&marks(3_000));
    content.push_str("/Span << /MCID 0 /Lang (en) >> BDC 0 0 9 9 re f EMC\n");
    content.push_str("/Layer BMC\n");
    content.push_str(&marks(3_000));
    content.push_str("EMC EMC /OC /Off BDC\n");
    content.push_str(&marks(3_000));
    content.push_str("EMC /P <</MCID 1>> DP Q\n");
    content.push_str(&marks(1_000));
    let document = Document::open(page_with_catalog(
        200, catalog, resources, &content, objects,
    ))
    .expect("the fixture opens");
    for pieces in [2, 5, 9] {
        let (declined, same) = divided_against_whole(&document, pieces).expect("page one reads");
        assert_eq!(declined, None, "{pieces} pieces: the page divides");
        assert!(
            same,
            "{pieces} pieces: the page read in pieces is the page read whole"
        );
    }
}

/// A section that records an extent — an `/MCID` — open across every cut is the one marked
/// content a piece still declines, and it says so.
#[test]
fn a_page_whose_whole_content_is_one_tagged_section_is_read_whole_and_says_why() {
    let mut content = String::from("/P << /MCID 0 >> BDC\n");
    content.push_str(&marks(4_000));
    content.push_str("EMC\n");
    let document = Document::open(page_with(200, "<< >>", &content)).expect("the fixture opens");
    let (declined, same) = divided_against_whole(&document, 4).expect("page one reads");
    assert!(same, "the page read whole is the page");
    assert!(
        declined.is_some_and(|reason| reason.contains("plain sections")),
        "the decline names what declined it: {declined:?}"
    );
}

#[test]
fn a_page_read_in_pieces_is_drawn_while_it_is_read() {
    let document =
        Document::open(page_with(200, "<< >>", &marks(40_000))).expect("the fixture opens");
    let drawn = every_stream(4)
        .during(|| drawn_while_read(&document))
        .expect("page one draws");
    assert!(
        drawn.parts > 0,
        "the first piece hands its list over in parts"
    );
    let finished = drawn
        .finished
        .expect("the hand-over ends with the joined list");
    let serial = whole(&drawn.list, drawn.target).expect("the page draws");
    assert!(
        finished.data == serial.data,
        "drawn while read in pieces is the page drawn whole"
    );
}

#[test]
fn a_hand_over_nothing_claims_ends_with_its_call() {
    let parts = Parts::new();
    parts.during(|| ());
    assert!(matches!(
        parts.take(Duration::ZERO),
        Some(Handed::Ended(Err(_)))
    ));
}

/// The corpus sweep's part size: small enough that a first page of a few dozen commands is handed
/// over in parts, which at [`pdf_model::progress::FIRST_PART`] only four of the tracked first pages
/// are.
const SMALL_PARTS: usize = 8;

/// Every tracked corpus document, sorted; `None` when the submodule is not checked out.
fn corpus() -> Option<Vec<PathBuf>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&root)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|extension| extension == "pdf"))
        .collect();
    files.sort();
    (!files.is_empty()).then_some(files)
}

/// Refuses to measure without the sandboxed decoder — trap 10, ADR 0557 — for the reason every
/// corpus gate does: a page whose JBIG2, JPX or CCITT image the worker did not decode holds no
/// command for it on either side of this comparison, so a pass taken without the worker is a pass
/// of the build and not of the tree.
fn require_the_sandbox() {
    if let Err(error) = pdf_model::image::sandboxed_decoder() {
        panic!(
            "the sandboxed image decoder is not available, so the comparison below would be of the \
             build rather than of the tree: {error}"
        );
    }
}

/// The first page of every tracked corpus document drawn while it is read: every page that
/// finishes is the page drawn whole, and every page that does not says why.
#[test]
#[ignore = "the whole tracked corpus interpreted and drawn twice; run explicitly, under the gates \
            profile"]
fn every_tracked_first_page_drawn_while_read_is_the_page_drawn_whole() {
    require_the_sandbox();
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    let mut finished = 0_usize;
    let mut handed = 0_usize;
    let mut whole_instead: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut different = Vec::new();
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        // Parts of a few commands, so that every page with more than a handful is handed over in
        // many parts and the property is asked of the population rather than of the few pages
        // long enough to be handed over at the shipped size.
        let Some(drawn) = drawn_in_parts(&document, &Parts::in_parts_of(SMALL_PARTS)) else {
            continue;
        };
        let Some(expected) = whole(&drawn.list, drawn.target) else {
            continue;
        };
        if drawn.parts > 0 {
            handed += 1;
        }
        match drawn.finished {
            Ok(raster) if raster.data == expected.data => finished += 1,
            Ok(_) => different.push(path.display().to_string()),
            Err(reason) => *whole_instead.entry(reason).or_default() += 1,
        }
    }
    println!(
        "{} documents: {finished} first pages finished from their parts, byte for byte the page \
         drawn whole ({handed} handed over in at least one part); drawn whole instead: \
         {whole_instead:?}",
        files.len()
    );
    assert!(
        different.is_empty(),
        "pages drawn while read that are not the page drawn whole: {different:?}"
    );
}

/// The first page of every tracked corpus document read in pieces and read whole (ADR 1880):
/// every page that divides states what the whole reading states, every one that does not says
/// why, and the pages that divide are drawn while read in pieces as the page drawn whole.
#[test]
#[ignore = "the whole tracked corpus interpreted three times and drawn twice; run explicitly, \
            under the gates profile"]
fn every_tracked_first_page_read_in_pieces_is_the_page_read_whole() {
    require_the_sandbox();
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    let mut divided = 0_usize;
    let mut divided_as_shipped = 0_usize;
    let mut pipelined = 0_usize;
    let mut fell_back: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut drawn_same = 0_usize;
    let mut declined: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut different = Vec::new();
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let Some((reading, same)) = divided_against_whole(&document, 4) else {
            continue;
        };
        // Whether the pipeline settled the page, or handed it to the plan after the inflate.
        let pipeline = last_pipeline_decline();
        // The division a viewer gets, which reads only a stream of a mebibyte or more in pieces:
        // a page it divides is a page whose list the corpus gates' arms would see (ADR 1863).
        if let Some(page) = Pages::new(&document).get(0) {
            let _ = interpret(&document, &page);
            if last_reading() == Some(Reading::Divided) {
                divided_as_shipped += 1;
            }
        }
        if !same {
            different.push(path.display().to_string());
        }
        match reading {
            None => {
                divided += 1;
                match pipeline {
                    None => pipelined += 1,
                    Some(reason) => *fell_back.entry(reason.0).or_default() += 1,
                }
            }
            Some(reason) => {
                *declined.entry(reason).or_default() += 1;
                continue;
            }
        }
        let Some(drawn) =
            every_stream(4).during(|| drawn_in_parts(&document, &Parts::in_parts_of(SMALL_PARTS)))
        else {
            continue;
        };
        let Some(expected) = whole(&drawn.list, drawn.target) else {
            continue;
        };
        match drawn.finished {
            Ok(raster) if raster.data == expected.data => drawn_same += 1,
            Ok(_) => different.push(format!("{} (drawn while read)", path.display())),
            Err(reason) => *declined.entry(reason).or_default() += 1,
        }
    }
    println!(
        "{} documents: {divided} first pages read in four pieces, each stating what the whole \
         reading states; {drawn_same} of them drawn while read byte for byte the page drawn \
         whole; {pipelined} of them with the inflate pipelined in, the rest divided after it: \
         {fell_back:#?}; {divided_as_shipped} divided as shipped; read whole instead: {declined:#?}",
        files.len()
    );
    assert!(
        different.is_empty(),
        "pages read in pieces that are not the page read whole: {different:?}"
    );
}
