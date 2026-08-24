//! What §7.8.2's *other* content streams are read through, and that the two routes agree.
//!
//! ADR 0365 put a page's `/Contents` behind a fixed window; ADR 0427 does the same for the four
//! the same clause names beside it:
//!
//! > Content streams shall also be used to package sequences of instructions as self-contained
//! > graphical elements, such as forms (see 8.10, "Form XObjects"), patterns (8.7, "Patterns"),
//! > certain fonts (9.6.4, "Type 3 fonts"), and annotation appearances (12.5.5, "Appearance
//! > streams").
//!
//! **The routing rule is the decoded-stream memo's own condition** and it is pinned in
//! `pdf-syntax`'s own tests, next to the cache that states it. What is pinned here is the half
//! that cannot be checked there: a form the memo declines is *interpreted through the window*,
//! and the page it draws is the page the whole decode drew — command for command, with the same
//! form's bytes presented both ways in one test.
//!
//! Trap 8 is why the fixture is built by hand rather than found: no corpus document states a
//! form whose decode outgrows four mebibytes, and the rule is about the ones that do.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture should fail loudly, and the arithmetic is indices \
              into a fixture of five objects"
)]

use std::fmt::Write as _;
use std::io::Write as _;

use pdf_model::content::reader::NestedContent;
use pdf_syntax::{Document, Limits, ObjectId, StreamSource};

/// Content long enough that the memo declines its decode, with marks all through it.
///
/// Mostly §7.2.4 comments, so that four and a half mebibytes cost a scan rather than four
/// million operators — and every comment is longer than nothing and shorter than the window, so
/// the run crosses a refill inside a comment as well as between operators, which is the boundary
/// case a window gets wrong quietly.
fn long_form_content() -> Vec<u8> {
    let mut out = Vec::new();
    for index in 0..45_000 {
        if index % 500 == 0 {
            out.extend_from_slice(b"0 0 1 rg 10 20 30 40 re f\n");
        }
        out.push(b'%');
        out.extend_from_slice(&[b'p'; 97]);
        out.push(b'\n');
    }
    out
}

/// How the fixture's form states its bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Coding {
    /// No `/Filter` at all, so there is nothing to pump.
    Plain,
    /// `/FlateDecode`.
    Flate,
    /// `/LZWDecode`, at Table 8's default `/EarlyChange`.
    Lzw,
    /// `[/ASCIIHexDecode /FlateDecode]` — a *chain*, and the one §7.4.1's EXAMPLE 3 writes
    /// (with base-85 in place of hexadecimal) for a page's own marking instructions.
    HexFlate,
}

/// Encodes `content` as §7.4.4.2 codes, one literal code per byte, then EOD.
///
/// **A valid LZW stream that happens not to compress**, which is all this fixture needs: what is
/// under test is the *route*, and a stream whose decode outgrows the memo is what chooses it. The
/// width rule is written out rather than shared with the decoder, for the reason
/// `filter.rs`'s own packer gives — a test that asked the decoder what width to use would agree
/// with it however wrong both were.
fn lzw_literals(content: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut held: u32 = 0;
    let mut bits: u32 = 0;
    let mut width = 9u32;
    let mut next: u16 = 258;
    let mut previous = false;

    let codes = content
        .iter()
        .map(|&byte| u16::from(byte))
        .chain(std::iter::once(257));
    for code in codes {
        held = (held << width) | (u32::from(code) & ((1u32 << width) - 1));
        bits += width;
        while bits >= 8 {
            out.push(((held >> (bits - 8)) & 0xFF) as u8);
            bits -= 8;
        }
        if code == 257 {
            continue;
        }
        if previous && usize::from(next) < 4096 {
            next += 1;
            // Table 8's `/EarlyChange` default is 1.
            if width < 12 && u32::from(next) + 1 >= (1u32 << width) {
                width += 1;
            }
        }
        previous = true;
    }
    if bits > 0 {
        out.push(((held << (8 - bits)) & 0xFF) as u8);
    }
    out
}

/// A one-page document whose only content is `/Fx Do`, with `Fx` carrying `content`.
///
/// `coding` says what the form's stream states for `/Filter`, which is the only thing that
/// differs between the arms: the *same* instructions, decoded by the same filter table, reaching
/// the interpreter by the routes `Document::nested_content_source` chooses between.
fn document_with_form(content: &[u8], coding: Coding) -> Document {
    let (data, filter) = match coding {
        Coding::Plain => (content.to_vec(), ""),
        Coding::Flate => {
            let mut encoder =
                flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
            encoder.write_all(content).expect("in-memory write");
            (encoder.finish().expect("finish"), " /Filter /FlateDecode")
        }
        Coding::Lzw => (lzw_literals(content), " /Filter /LZWDecode"),
        Coding::HexFlate => {
            let mut encoder =
                flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
            encoder.write_all(content).expect("in-memory write");
            let deflated = encoder.finish().expect("finish");
            let mut hexed = Vec::with_capacity(deflated.len() * 2 + 1);
            for byte in deflated {
                let _ = write!(hexed_string(&mut hexed), "{byte:02X}");
            }
            hexed.push(b'>');
            (hexed, " /Filter [/ASCIIHexDecode /FlateDecode]")
        }
    };

    let mut objects: Vec<Vec<u8>> = vec![
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_vec(),
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_vec(),
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
          /Resources << /XObject << /Fx 5 0 R >> >> /Contents 4 0 R >>\nendobj\n"
            .to_vec(),
        b"4 0 obj\n<< /Length 7 >>\nstream\n/Fx Do\nendstream\nendobj\n".to_vec(),
    ];
    let mut form = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 612 792] /Resources << >>{filter} \
         /Length {} >>\nstream\n",
        data.len()
    )
    .into_bytes();
    form.extend_from_slice(&data);
    form.extend_from_slice(b"\nendstream\nendobj\n");
    objects.push(form);

    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for object in &objects {
        offsets.push(out.len());
        out.extend_from_slice(object);
    }
    let xref_at = out.len();
    let mut tail = String::new();
    let size = offsets.len() + 1;
    let _ = writeln!(tail, "xref\n0 {size}");
    tail.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(tail, "{offset:010} 00000 n ");
    }
    let _ = write!(
        tail,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.extend_from_slice(tail.as_bytes());
    Document::open_with_limits(out, Limits::DEFAULT).expect("the fixture opens")
}

/// A `fmt::Write` over a byte buffer, so that the hexadecimal armour is written where it goes.
struct Hexed<'a>(&'a mut Vec<u8>);

impl std::fmt::Write for Hexed<'_> {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        self.0.extend_from_slice(text.as_bytes());
        Ok(())
    }
}

/// See [`Hexed`].
fn hexed_string(out: &mut Vec<u8>) -> Hexed<'_> {
    Hexed(out)
}

/// Page one's display list, as the digest example compares them.
fn commands(document: &Document) -> String {
    let page = pdf_model::Pages::new(document)
        .get(0)
        .expect("the fixture has a page");
    format!(
        "{:?}",
        pdf_model::interpret(document, &page)
            .display_list
            .commands()
    )
}

/// Which route object 5 takes, asked of the document that holds it.
fn route(document: &Document) -> StreamSource {
    let object = document.get(ObjectId::new(5, 0));
    let stream = object.as_stream().expect("object 5 is the form");
    document
        .nested_content_source(stream)
        .expect("the form decodes")
}

/// A form the memo declines draws exactly what the same form drawn whole draws.
///
/// **The assertion that discriminates is the first one.** Comparing the two display lists alone
/// would pass just as happily if the window were never reached — which is how a route decision
/// stops being tested without anything going red — so the route is asserted first and the
/// pictures second.
#[test]
fn a_form_read_through_the_window_draws_what_the_whole_decode_draws() {
    let content = long_form_content();
    let windowed = document_with_form(&content, Coding::Flate);
    let whole = document_with_form(&content, Coding::Plain);

    assert!(
        matches!(route(&windowed), StreamSource::Pumped(_)),
        "a form whose decode outgrows the memo is read through the window"
    );
    assert!(
        matches!(route(&whole), StreamSource::Whole(_)),
        "and one with no filter at all has nothing to pump"
    );

    let through_the_window = commands(&windowed);
    let all_at_once = commands(&whole);
    assert!(
        through_the_window.contains("Fill"),
        "the fixture has to draw something for this to be a comparison"
    );
    assert_eq!(
        through_the_window, all_at_once,
        "the window changed what the form draws"
    );
}

/// The same, for `LZWDecode`, which is the sharper bomb of §7.4's five.
///
/// **`Document::pumping` is one function on purpose**, so a filter gaining a pump reaches a page's
/// `/Contents` and §7.8.2's other four at once; what this asserts is the half that costs
/// something if it is wrong — the form is *routed* through the window and the page it draws is
/// the page the whole decode drew. LZW reaches about 1365:1 on a long run of one byte, which is
/// why it is the one taken first.
#[test]
fn an_lzw_form_is_read_through_the_window_and_draws_what_the_whole_decode_draws() {
    let content = long_form_content();
    let windowed = document_with_form(&content, Coding::Lzw);
    let whole = document_with_form(&content, Coding::Plain);

    assert!(
        matches!(route(&windowed), StreamSource::Pumped(_)),
        "an LZW form whose decode outgrows the memo is read through the window"
    );

    let through_the_window = commands(&windowed);
    assert!(
        through_the_window.contains("Fill"),
        "the fixture has to draw something for this to be a comparison"
    );
    assert_eq!(
        through_the_window,
        commands(&whole),
        "the window changed what the LZW form draws"
    );
}

/// The same, for a **chain** — which is the arrangement that escaped road D entirely.
///
/// `Document::pumping` used to grant a window to a single `FlateDecode` or `LZWDecode` and hand
/// every chain back whole, so an author defeated the road by wrapping the bomb in a second
/// filter: ADR 0586 measured the same bomb at 257 µs under the decoded-stream budget and 6.93 s
/// over it, about 25 000×, bought with padding. §7.4.1's own EXAMPLE 3 writes that arrangement
/// for a page's marking instructions, which is why it cannot be dismissed as hostile-only.
/// ADR 0587.
#[test]
fn a_chained_form_is_read_through_the_window_and_draws_what_the_whole_decode_draws() {
    let content = long_form_content();
    let windowed = document_with_form(&content, Coding::HexFlate);
    let whole = document_with_form(&content, Coding::Plain);

    assert!(
        matches!(route(&windowed), StreamSource::Pumped(_)),
        "a chain whose every stage pumps is read through the window"
    );

    let through_the_window = commands(&windowed);
    assert!(
        through_the_window.contains("Fill"),
        "the fixture has to draw something for this to be a comparison"
    );
    assert_eq!(
        through_the_window,
        commands(&whole),
        "the window changed what the chained form draws"
    );
}

/// §8.7.3.1's cell is windowed like the other three, which is what ADR 0430 bought.
///
/// **The exception it replaces was found by fuzzing.** A mutated tiling pattern the `page` target
/// reached took 0.24 s held and 9.0 s windowed, because the cell's content stream was run once
/// per site painted and `MAX_TILES` allows four thousand of them — so `NestedContent::of`'s
/// premise, that a decode the memo declines is re-run on every read anyway, was false for exactly
/// this one of the four. The cell is now interpreted **once** and every other site is its commands
/// displaced, so the premise holds again and the type that kept the exception is gone. What this
/// asserts is that the routing constructor is the one a tiling cell now takes, and
/// `hostile_budgets.rs` asserts the reading itself happens once.
#[test]
fn a_tiling_cell_is_windowed_like_the_form_of_the_same_size() {
    let content = long_form_content();
    let document = document_with_form(&content, Coding::Flate);
    let object = document.get(ObjectId::new(5, 0));
    let stream = object.as_stream().expect("object 5 is the form");

    assert!(
        matches!(route(&document), StreamSource::Pumped(_)),
        "the routing constructor windows a decode the memo would decline"
    );
    assert!(
        NestedContent::of(&document, stream, "a tiling pattern's cell".to_owned())
            .expect("it decodes")
            .windowed(),
        "and a tiling cell is built by that same constructor since ADR 0430"
    );
}

/// A form the memo *keeps* is decoded whole, and reading it twice runs the filter once.
///
/// The other half of the rule, and the reason it is the memo's condition rather than a number of
/// this test's own: §11.6.6's paired runs interpret one form two and three times, so a route
/// that pumped every form would re-inflate it for each. This is what says the ordinary form is
/// untouched.
#[test]
fn a_form_the_memo_keeps_is_decoded_once_however_often_it_is_read() {
    let document = document_with_form(b"0 0 1 rg 10 20 30 40 re f\n", Coding::Flate);
    assert!(
        matches!(route(&document), StreamSource::Whole(_)),
        "an ordinary form is held whole"
    );
    let before = document.decoded_streams().hits;
    let _ = route(&document);
    let _ = route(&document);
    assert_eq!(
        document.decoded_streams().hits,
        before + 2,
        "every read after the first is a cache read"
    );
}
