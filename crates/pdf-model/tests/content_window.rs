//! What a page draws when its `/Contents` reaches the lexer a window at a time.
//!
//! A page's content stream is not decoded into one buffer any more: `content::reader` pumps it
//! into a fixed window and the interpreter reads through that (ADR 0365, `doc/todo/14`'s road
//! D). Every byte a page draws from is the same byte it drew from before — which is what the
//! corpus, the oracle and the readback of a thousand pages check — so what is left for a test
//! is the *boundary*, which no corpus document is guaranteed to place interestingly:
//!
//! - a token, a comment and a string that straddle a refill;
//! - the two things a bounded buffer cannot do, which are therefore the two it must say out
//!   loud (`ContentIssue::TokenTooLong` and `InlineImageError::Unbuffered`);
//! - a part whose damage the pump meets in the middle of the page rather than before it, which
//!   is ADR 0343's report arriving from a new place.
//!
//! Every fixture states its own arithmetic against `content::reader`'s two constants rather
//! than a number written here, so a round that re-measures the census moves them in one place.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a fixture that does not open should fail loudly, and every size here \
              is computed from a constant this crate exports"
)]

use std::fmt::Write as _;

use pdf_model::content::reader::{CEILING, LOOKAHEAD, WINDOW};
use pdf_model::page::ContentIssue;
use pdf_syntax::{Damage, Document};

/// A one-page PDF whose content stream is `operators`, uncompressed.
fn page(operators: &str) -> Document {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
         /Resources << >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{operators}\nendstream\nendobj\n",
        operators.len()
    );
    assemble(&body)
}

/// A one-page PDF whose `/Contents` is an array of two deflated parts.
fn page_of_deflated_parts(parts: &[Vec<u8>]) -> Document {
    let mut names = String::new();
    let mut objects: Vec<u8> = Vec::new();
    for (index, part) in parts.iter().enumerate() {
        let number = index + 4;
        let _ = write!(names, "{number} 0 R ");
        objects.extend_from_slice(
            format!(
                "{number} 0 obj\n<< /Filter /FlateDecode /Length {} >>\nstream\n",
                part.len()
            )
            .as_bytes(),
        );
        objects.extend_from_slice(part);
        objects.extend_from_slice(b"\nendstream\nendobj\n");
    }
    let head = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
         /Resources << >> /Contents [{names}] >>\nendobj\n"
    );
    let mut body = head.into_bytes();
    body.extend_from_slice(&objects);
    assemble_bytes(&body)
}

/// Deflates with the same library the decoder uses, which is what a producer would have done.
fn deflate(data: &[u8]) -> Vec<u8> {
    use std::io::Write as _;
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder.write_all(data).expect("in-memory write");
    encoder.finish().expect("finish")
}

/// Wraps a body of objects in a header, a cross-reference table and a trailer.
fn assemble(body: &str) -> Document {
    assemble_bytes(body.as_bytes())
}

/// The same, for a body holding a filter's binary output.
fn assemble_bytes(body: &[u8]) -> Document {
    let mut out: Vec<u8> = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    let mut rest = body;
    while !rest.is_empty() {
        let end = rest
            .windows(7)
            .position(|window| window == b"endobj\n")
            .map_or(rest.len(), |at| at + 7);
        offsets.push(out.len());
        out.extend_from_slice(rest.get(..end).unwrap_or_default());
        rest = rest.get(end..).unwrap_or_default();
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
    let mut tail = format!("xref\n0 {size}\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(tail, "{offset:010} 00000 n ");
    }
    let _ = write!(
        tail,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.extend_from_slice(tail.as_bytes());
    Document::open(out).expect("the fixture opens")
}

/// What page one reported.
fn issues(document: &Document) -> Vec<ContentIssue> {
    let page = pdf_model::Pages::new(document)
        .get(0)
        .expect("the fixture has a page");
    page.content_with_report(document).1
}

/// How many commands page one drew.
fn commands(document: &Document) -> usize {
    let page = pdf_model::Pages::new(document)
        .get(0)
        .expect("the fixture has a page");
    pdf_model::interpret(document, &page)
        .display_list
        .commands()
        .len()
}

/// Every rectangle draws, wherever the refills fall.
///
/// The stream is several windows long and the operators are of every awkward shape at once: a
/// string with white space in it, a comment, and a name. A window boundary lands somewhere
/// inside all of them, and the count is what says none was cut — a token read as two tokens
/// draws a rectangle short, and a comment read as content draws one too many.
#[test]
fn a_stream_of_many_windows_draws_every_rectangle_in_it() {
    const RECTANGLES: usize = 20_000;
    let mut content = String::new();
    for index in 0..RECTANGLES {
        let _ = write!(
            content,
            "% rectangle {index}, a comment of its own\n\
             /Artifact BMC (a string with spaces and a ) escaped) Tj EMC\n\
             0 0 0 rg {} {} 10 10 re f\n",
            index % 500,
            index % 700
        );
    }
    assert!(
        content.len() > 8 * WINDOW,
        "the fixture must span several windows: {} bytes",
        content.len()
    );
    let document = page(&content);
    assert_eq!(issues(&document), Vec::new());
    assert_eq!(
        commands(&document),
        RECTANGLES,
        "one fill per rectangle, whatever the window boundaries cut across"
    );
}

/// A single token longer than the window is reported, stepped over, and the page draws on.
///
/// The one thing a bounded buffer cannot do. `examples/token_window_census` measured 225 775 555
/// content-stream tokens in 39 976 documents and the largest is 390.16 KiB, so this fixture
/// states something no document has been seen to state — and the point is that it is *said*
/// rather than cut, because a truncated token would put bytes the file never wrote in front of
/// the interpreter.
#[test]
fn a_token_longer_than_the_window_is_reported_rather_than_cut() {
    let mut content = String::from("0 0 0 rg 10 10 20 20 re f\n(");
    content.push_str(&"a".repeat(CEILING + CEILING / 2));
    content.push_str(") Tj\n50 50 20 20 re f\n");
    let document = page(&content);
    let page_one = pdf_model::Pages::new(&document)
        .get(0)
        .expect("the fixture has a page");
    let interpretation = pdf_model::interpret(&document, &page_one);

    // Reported by *interpretation* rather than by `content_with_report`, and the difference is
    // real: assembling the parts finds nothing wrong with them, and it is the lexer asking for
    // a token that meets the buffer's ceiling.
    assert!(
        format!("{:?}", interpretation.unsupported)
            .contains(&format!("TokenTooLong {{ limit: {CEILING} }}")),
        "the reader says which bound it met: {:?}",
        interpretation.unsupported
    );
    assert_eq!(issues(&document), Vec::new(), "the parts assemble cleanly");
    assert_eq!(
        interpretation.display_list.commands().len(),
        2,
        "and both rectangles — the one before the token and the one after it — are drawn"
    );
}

/// An inline image whose data outruns the lookahead is refused by name, not read short.
///
/// The second thing a bounded buffer cannot do, and §8.9.7 is why it is a *different* answer
/// from [`pdf_model::inline_image::InlineImageError::NoTerminator`]: that one says the file
/// states no `EI`, and this one says this reader stopped looking. The clause itself asks for
/// small images — "it should be used only for small images (4096 bytes or less)" — and the
/// census puts the largest that exists at 9.01 MiB, so this fixture states twenty mebibytes of
/// image data with no terminator in it at all.
#[test]
fn an_inline_image_longer_than_the_lookahead_is_refused_by_name() {
    let mut content: Vec<u8> =
        b"0 0 0 rg 10 10 20 20 re f\nBI /W 8 /H 8 /BPC 8 /CS /G /F /Fl ID ".to_vec();
    // No `E` anywhere in the data, so nothing in it can be read as the terminator.
    content.extend(std::iter::repeat_n(b'x', LOOKAHEAD + 4 * 1024 * 1024));
    let document = page_of_deflated_parts(&[deflate(&content)]);
    let page_one = pdf_model::Pages::new(&document)
        .get(0)
        .expect("the fixture has a page");
    let reported = format!(
        "{:?}",
        pdf_model::interpret(&document, &page_one).unsupported
    );

    assert!(
        reported.contains("longer than the"),
        "the image is refused as unbuffered rather than as having no terminator: {reported}"
    );
    assert!(
        !reported.contains("no EI ends the image data"),
        "and not as a file that states no EI: {reported}"
    );
}

/// Table 31's array is one stream, and a token may straddle the join.
///
/// ISO 32000-2 §7.7.3.3, Table 31's `/Contents` row:
///
/// > If the value is an array, the effect shall be as if all of the streams in the array were
/// > concatenated with at least one white-space character added between the streams' data, in
/// > order, to form a single stream.
///
/// So the parts chain into one reader. This fixture puts an operator's operands in one part
/// and the operator in the next, with both deflated, which is the case the pump has to carry
/// across a part boundary as well as across a window boundary.
#[test]
fn two_deflated_parts_are_one_stream_across_the_join() {
    let document = page_of_deflated_parts(&[
        deflate(b"0 0 0 rg 10 10 100 100"),
        deflate(b"re f 200 200 50 50 re f"),
    ]);
    assert_eq!(issues(&document), Vec::new());
    assert_eq!(
        commands(&document),
        2,
        "the rectangle whose operator is in the second part is drawn"
    );
}

/// A deflated part that stops short draws its prefix and says so, from inside the page.
///
/// ADR 0343's report, produced as the pump goes: under a window the damage is met in the
/// middle of the page rather than before it, and `kept` still says how much of the stream the
/// page is drawing. Suppress the report and a page that was cut short is indistinguishable
/// from a page meant to be sparse.
#[test]
fn a_truncated_deflated_part_reports_its_damage_with_what_it_kept() {
    let whole = b"0 0 0 rg 10 10 100 100 re f\n200 200 50 50 re f\n";
    let compressed = deflate(whole);
    let cut = compressed
        .get(..compressed.len() - 4)
        .expect("the fixture is longer than four bytes")
        .to_vec();
    let document = page_of_deflated_parts(&[cut]);

    let reported = issues(&document);
    let [
        ContentIssue::Damaged {
            index,
            damage,
            kept,
            ref filters,
        },
    ] = reported[..]
    else {
        panic!("a truncated part must report its damage: {reported:?}");
    };
    assert_eq!((index, damage), (0, Damage::Truncated));
    assert_eq!(filters, &vec!["FlateDecode".to_owned()]);
    assert!(
        kept > 0 && kept <= whole.len(),
        "kept is what the page is drawing: {kept}"
    );
    assert!(
        commands(&document) > 0,
        "and what did inflate is on the page"
    );
}

/// A derived length past the end of the window is a request for more bytes, not a wrong answer.
///
/// §8.9.7 makes an inline image's data a stream's data:
///
/// > The bytes between the ID operator and a white-space token, but before the EI operator shall
/// > be treated the same as a stream object's data ( see 7.3.8, "Stream objects"), even though
/// > they do not follow the standard stream syntax.
///
/// and §7.3.8.2 says such an extent is inferable rather than guessable:
///
/// > Finally, streams are used to represent many objects from whose attributes a length can be
/// > inferred. All of these constraints shall be consistent.
///
/// So for unfiltered samples the byte count is arithmetic — `pdf_model::inline_image` computes it
/// and its own module comment says the forward search for `EI` "is only reached for *filtered*
/// data with no `/L`". **Through a window that sentence was false**: the arithmetic answer could
/// not be *checked* against the `EI` it predicts while the buffer was shorter than the image, so
/// the derived length was dropped and the search ran — and the search stopped at the first
/// whitespace-delimited `EI` the samples happened to spell.
///
/// The witness is a crawled architectural drawing whose second inline image is 1024×716 in
/// `/DeviceRGB`: 2 199 552 bytes of samples that spell ` EI ` 817 411 bytes in, so 63% of the
/// picture was lost and the remaining 1.4 MB was tokenised as content operators (session 619).
///
/// This fixture is that shape in miniature: an image two windows long whose samples spell an
/// `EI` in the first one. Getting it wrong draws a short image *and* executes the rest as
/// operators, so the marker rectangle after `EI` is what says the stream resumed where it should.
#[test]
fn a_derived_length_beyond_the_window_grows_the_window_rather_than_guessing() {
    const WIDE: usize = 256;
    let tall = WINDOW * 2 / WIDE;
    let mut content: Vec<u8> = format!("BI /W {WIDE} /H {tall} /BPC 8 /CS /G ID ").into_bytes();
    let samples = WIDE * tall;
    content.extend(std::iter::repeat_n(b'\xC0', samples));
    // A whitespace-delimited `EI` inside the first window, which is the only thing a search can
    // find and is not where the data ends.
    let decoy = content.len() - samples + WINDOW / 2;
    content
        .get_mut(decoy..decoy + 4)
        .expect("the samples are longer than half a window")
        .copy_from_slice(b" EI ");
    content.extend_from_slice(b" EI\n0 0 0 rg 10 10 20 20 re f\n");

    let document = page_of_deflated_parts(&[deflate(&content)]);
    let page_one = pdf_model::Pages::new(&document)
        .get(0)
        .expect("the fixture has a page");
    let interpretation = pdf_model::interpret(&document, &page_one);
    let reported = format!("{:?}", interpretation.unsupported);
    assert_eq!(
        reported, "[]",
        "the image is whole and the stream resumes at its EI, so there is nothing to report"
    );

    let drawn = interpretation.display_list.commands();
    let image = drawn
        .iter()
        .find_map(|command| match command {
            pdf_render::Command::Image { image, .. } => Some(image.clone()),
            _ => None,
        })
        .expect("the inline image is drawn");
    let pdf_render::ImageSource::Decoded(decoded) = &image else {
        panic!("an unfiltered inline image is decoded rather than deferred");
    };
    assert_eq!(
        (decoded.width as usize, decoded.height as usize),
        (WIDE, tall),
        "every sample the dictionary describes is in the image"
    );
    assert_eq!(
        drawn.len(),
        2,
        "the image and the rectangle after EI: {drawn:?}"
    );
}
