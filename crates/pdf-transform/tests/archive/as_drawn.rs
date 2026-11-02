//! A run of regular characters the window draws as the number on its front is written as that
//! number, the loss `content-as-drawn`, and the page stays text (`doc/adr/2638`).
//!
//! The expected values come from ISO 32000-2: §7.2.3 ("A sequence of consecutive regular
//! characters comprises a single token"), so `33600.272.58` is one token; §7.3.3, which spells a
//! real with one period, so that token is no number and the checker reads it as the operator
//! §7.8.2 makes of a keyword (`doc/adr/2619`); and from this reader's salvage of the front of such
//! a run (`doc/adr/1004`), which is what the window draws: `33600.272` from `33600.272.58` and
//! `454` from `454` followed by the byte 85h. A run whose front is no number, or a delimiter
//! standing where no object began, is not a run this reader draws, and its page keeps the fence's
//! answer.
use super::statement::{holds_fact, stated};
use super::*;
use pdf_archive::Flavour;
use pdf_transform::archive::Loss;

const OPERATORS: &str = "graphics/only-operators-the-base-standard-defines";

/// A simple font with no program and no descriptor, which the conversion embeds a shipped face
/// for (ISO 19005-2 section 6.2.11.4.1).
const HELVETICA: &str =
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>";

/// The front of RFC 1950's zlib framing around one RFC 1951 stored block of `whole` — the
/// encoding ISO 32000-2 §7.4.4.1 names for `FlateDecode` — cut after its first `kept` bytes, so the
/// block states a length that runs past what the stream holds.
fn a_stored_block_cut_after(whole: &[u8], kept: usize) -> Vec<u8> {
    let len = u16::try_from(whole.len()).expect("one stored block is at most 65535 bytes");
    // RFC 1950 section 2.2: CM 8, CINFO 7 and an FCHECK making the header a multiple of 31; RFC
    // 1951 section 3.2.4: BFINAL and BTYPE 00, then LEN and its one's complement NLEN.
    let mut out = vec![0x78, 0x01, 0x01];
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&(!len).to_le_bytes());
    out.extend_from_slice(&whole[..kept]);
    out
}

/// A page showing "Hello" in Helvetica, then `more`, its content stream written whole.
fn a_page_drawing(more: &[u8]) -> Vec<u8> {
    a_page_whose_stream_is(
        String::new(),
        [&b"BT /F1 12 Tf 10 100 Td (Hello) Tj ET "[..], more].concat(),
    )
}

/// A page showing whatever `data` draws, under `dict` added to its content stream's dictionary.
fn a_page_whose_stream_is(dict: String, data: Vec<u8>) -> Vec<u8> {
    Conforming {
        resources: "/Font << /F1 6 0 R >>".to_owned(),
        contents: Some((dict, data)),
        objects: vec![HELVETICA.to_owned()],
        ..Conforming::part_two()
    }
    .build()
}

/// The plan a shipped profile makes for `target`, with the authorisations its `discard` steps
/// stand for and the preservations its `append-statement` steps ask.
fn under(profile: &str, target: Target) -> ArchivePlan {
    let text = pdf_transform::profiles::named(profile)
        .expect("a shipped profile")
        .text;
    let config = pdf_transform::archive::Configuration::read(text, target).expect("it reads");
    ArchivePlan {
        authorised: config.authorisations(target),
        ..plan_from(text, target)
    }
}

/// Whether `haystack` holds `needle` as a run of bytes.
fn holds_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

#[test]
fn a_run_the_window_draws_as_a_number_is_written_as_that_number_and_the_page_stays_text() {
    let source = a_page_drawing(b"q 33600.272.58 0 m 10 10 l S Q");
    for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
        let (report, output) = convert_with_plan(&source, &under("as-if-printed", target));
        let Decision::Authorised { losses, .. } = decision(&report, OPERATORS) else {
            panic!("{:?}", decision(&report, OPERATORS));
        };
        assert!(
            losses.iter().eq([Loss::ContentAsDrawn]),
            "{}",
            losses.words()
        );
        let output = output.expect("a file is written");
        assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
        let conversion = conversion(&report);
        assert!(
            conversion.photographed.is_empty(),
            "no page is photographed"
        );
        let [written] = conversion.runs_written_as_drawn.as_slice() else {
            panic!("one run: {:?}", conversion.runs_written_as_drawn);
        };
        assert_eq!(written.pages, [1]);
        assert_eq!(written.run, b"33600.272.58");
        assert_eq!(written.number, b"33600.272");
        // The producer's operators around the run are the producer's, byte for byte.
        let contents = decoded_contents(&output);
        assert!(
            holds_bytes(&contents, b"q 33600.272 0 m 10 10 l S Q"),
            "{}",
            String::from_utf8_lossy(&contents)
        );
        assert!(!holds_bytes(&contents, b"33600.272.58"));
        let json = conversion.to_json().render();
        assert!(json.contains("33600.272.58"), "{json}");
        // The file's own `xmpMM:History` names the run and the number.
        let packet = String::from_utf8(document_packet(&output)).expect("the packet is UTF-8");
        assert!(packet.contains("33600.272.58"), "{packet}");
    }
}

// The plant: a `)` where no string began is a delimiter (§7.2.3), not a run this reader draws as a
// number, so the page it stands on is photographed for the operator row as before — and the run
// beside it is still written, because the photograph replaces only the page's own content.
#[test]
fn a_stray_delimiter_beside_a_run_keeps_its_page_photographed() {
    let source = a_page_drawing(b"q 33600.272.58 0 m 10 10 l S Q )");
    let target = Target::Two(Level::B);
    let (report, output) = convert_with_plan(&source, &under("as-if-printed", target));
    let Decision::Authorised { losses, .. } = decision(&report, OPERATORS) else {
        panic!("{:?}", decision(&report, OPERATORS));
    };
    assert!(
        losses.iter().any(|loss| loss == Loss::PageAsImage),
        "{}",
        losses.words()
    );
    let output = output.expect("a file is written");
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    let pages = &conversion(&report).photographed;
    assert_eq!(pages.len(), 1, "{pages:?}");
    assert_eq!(pages[0].requirements, [OPERATORS]);
    let [written] = conversion(&report).runs_written_as_drawn.as_slice() else {
        panic!("one run: {:?}", conversion(&report).runs_written_as_drawn);
    };
    assert_eq!(written.run, b"33600.272.58");
}

// A FlateDecode stream cut inside its one stored block (RFC 1951 section 3.2.4) decodes to the
// bytes before the cut, which every reader draws; the run that ends them is written as `454`.
#[test]
fn a_run_ending_a_damaged_stream_is_written_as_the_number_it_draws() {
    let front = b"BT /F1 12 Tf 10 100 Td (Hello) Tj ET 1 0 0 1 10 454\x85".to_vec();
    let whole = [&front[..], b" cm 0 0 m 5 5 l S"].concat();
    let cut = a_stored_block_cut_after(&whole, front.len());
    let source = a_page_whose_stream_is("/Filter /FlateDecode".to_owned(), cut);
    let target = Target::Two(Level::B);
    let (report, output) = convert_with_plan(&source, &under("as-if-printed", target));
    let Decision::Authorised { losses, .. } = decision(&report, OPERATORS) else {
        panic!("{:?}", decision(&report, OPERATORS));
    };
    assert!(
        losses.iter().eq([Loss::ContentAsDrawn]),
        "{}",
        losses.words()
    );
    let output = output.expect("a file is written");
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    let conversion = conversion(&report);
    let [written] = conversion.runs_written_as_drawn.as_slice() else {
        panic!("one run: {:?}", conversion.runs_written_as_drawn);
    };
    assert_eq!(written.run, b"454\x85");
    assert_eq!(written.number, b"454");
    let contents = decoded_contents(&output);
    assert!(
        contents.ends_with(b"1 0 0 1 10 454"),
        "{}",
        String::from_utf8_lossy(&contents)
    );
    assert!(conversion.to_json().render().contains("454\\\\x85"));
}

// `as-shown` discards at the operator row, which authorises `content-as-drawn` and nothing that
// removes an operator: a page whose one unlisted operator is no run keeps the fence's refusal.
#[test]
fn an_operator_that_is_no_run_keeps_the_fence_under_as_shown() {
    let source = a_page_drawing(b"1 2 Frobnicate");
    let (report, output) = convert_with_plan(&source, &under("as-shown", Target::Two(Level::B)));
    assert!(
        matches!(
            decision(&report, OPERATORS),
            Decision::Refused(Because::TheFence(_))
        ),
        "{:?}",
        decision(&report, OPERATORS)
    );
    assert!(output.is_none());
    assert!(conversion(&report).runs_written_as_drawn.is_empty());
}

#[test]
fn state_every_loss_names_each_run_on_its_appended_page() {
    let source = a_page_drawing(b"q 33600.272.58 0 m 3NT-DE290.58 10 l S Q");
    let target = Target::Two(Level::B);
    let (report, output) = convert_with_plan(&source, &under("state-every-loss", target));
    let output = output.expect("a file is written");
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    let conversion = conversion(&report);
    assert_eq!(conversion.runs_written_as_drawn.len(), 2);
    let statement = conversion.statement.as_ref().expect("a statement page");
    assert!(
        statement.sites.contains(&OPERATORS),
        "{:?}",
        statement.sites
    );
    let page = stated(&output, statement);
    for fact in [
        "Loss: content-as-drawn",
        "Pages: 1",
        "Run as the producer wrote it: 33600.272.58",
        "Number written: 33600.272 (ISO 32000-2 7.3.3)",
        "Run as the producer wrote it: 3NT-DE290.58",
        "Number written: 3 (ISO 32000-2 7.3.3)",
    ] {
        assert!(holds_fact(&page, fact), "{fact}:\n{page}");
    }
}

// The bound: each run is at least two bytes, a digit and what the number leaves out, so the runs
// one stream yields are at most half its length, and one conversion writes at most 65536 of them
// (`as_drawn::MOST_RUNS`, ISO 32000-2 setting no limit of its own). At the bound every run is
// written and the file's own history names the first sixteen and counts the rest; one past it,
// none is, and the operator row keeps its refusal.
#[test]
fn the_runs_one_conversion_writes_are_bounded_and_tested_at_the_bound() {
    const MOST_RUNS: usize = 1 << 16;
    let target = Target::Two(Level::B);
    let at_the_bound = b"1.2.3 ".repeat(MOST_RUNS);
    assert!(MOST_RUNS <= at_the_bound.len() / 2);
    let (report, output) = convert_with_plan(
        &a_page_drawing(&at_the_bound),
        &under("as-if-printed", target),
    );
    let output = output.expect("a file is written");
    assert_eq!(conversion(&report).runs_written_as_drawn.len(), MOST_RUNS);
    let packet = String::from_utf8(document_packet(&output)).expect("the packet is UTF-8");
    assert!(
        packet.contains(&format!("and {} more", MOST_RUNS - 16)),
        "{}",
        &packet[..packet.len().min(4000)]
    );
    let past = b"1.2.3 ".repeat(MOST_RUNS + 1);
    let (report, _) = convert_with_plan(&a_page_drawing(&past), &under("as-if-printed", target));
    assert!(conversion(&report).runs_written_as_drawn.is_empty());
    assert!(
        !matches!(
            decision(&report, OPERATORS),
            Decision::Authorised { losses, .. } if losses.iter().any(|loss| loss == Loss::ContentAsDrawn)
        ),
        "{:?}",
        decision(&report, OPERATORS)
    );
}
