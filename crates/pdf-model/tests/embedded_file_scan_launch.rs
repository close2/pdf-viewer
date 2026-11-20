//! Page one of an encrypted document whose `/EFF` differs from `/StmF` is drawn without the pass
//! over every indirect object that finds its embedded file streams (ADR 3241).
//!
//! ISO 32000-2 §7.6.6 Table 20's `/EFF` names the crypt filter "used when decrypting embedded
//! file streams that do not have their own crypt filter specifier", and Table 44 makes an
//! embedded file stream's `/Type` "(Optional)", so an untyped stream is one only where a file
//! specification's `/EF` or `/RF` names it (Table 43). ADR 3234 finds those streams by a pass over
//! the file's indirect objects. A stream reached as a page's `/Contents`, as an `XObject`, a
//! pattern, a shading or a font program is reached in a role Table 43 does not give an embedded
//! file, so its filter is `/StmF` without the pass, and CLAUDE.md's "a 500-page document must
//! reach its first page no slower than a 5-page one" holds for these files too.
//!
//! The fixture is the revision-5 container `pdf-syntax`'s `tests/encryption.rs` builds around a
//! real producer's AES-256 ciphertext (`doc/pdf.js`'s one revision-5 document): its five
//! constants are copied here, so the content stream decrypting to the producer's operators is
//! a statement about the key rather than about this code's self-consistency. AES-256 under
//! revision 5 keys every object with the file key itself (§7.6.3.3 Algorithm 1.A), so each page
//! carries its own copy of the same ciphertext and the file grows with its page count.

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture should fail loudly, and its object numbers and \
              hexadecimal digits are small enough that no sum overflows"
)]

use std::fmt::Write as _;
use std::time::{Duration, Instant};

use pdf_syntax::object::ObjectId;
use pdf_syntax::{Document, Limits};

/// The revision-5 document's `/U` (48 significant bytes).
const USER_ENTRY: &str = "1A535BA22F2DD1CF9F37CF7540FB3A2D0F82D9EA49DB2F1E1CFD5118952D3DCC\
                          34C814AD2CC29313002D79DE31DF86A0";

/// The same document's `/UE`: the file encryption key, wrapped under the user password.
const USER_ENCRYPTION: &str = "EE4139E2E22728C58B69A73CB2AEA4F0A8F759F895C5D1295751820432A238B6";

/// The same document's `/Perms`, keyed by the file key.
const PERMS: &str = "CB21FA8DCEEDECEA128E0F81D834B379";

/// `/O` for an owner password `pdf-syntax`'s `tests/encryption.rs` names and computed outside
/// this tree; nothing here authenticates against it, and Table 21 requires the entry.
const OWNER_ENTRY: &str = "37339979E2EE6D29992924840F9851C2C5DE0F106CD37200D4ABC619ACB7097E\
                           01020304050607081112131415161718";

/// `/OE` for the same owner password.
const OWNER_ENCRYPTION: &str = "368A47EE4DDABB0D0B78025BE58C6F309D4DDCC4C8CD41FF7B719A113031A183";

/// The same document's page-one content stream: AES-256 ciphertext, initialisation vector first.
const CONTENT: &str = "C7DB7E278725358D2C49C2E3484E065C57C1E330102E61843D4122C6FBFAAAF6\
                       2B72D613B1D048882805E6C9CA6D82CE269138F7A09D517C45736167E16541E4\
                       44C976BA99CC2762A4615B507BFE2D1992E48B81A11B1E125147D4B1B57E6111\
                       7A0E37E5C9ADC29113557CD22468871A733DC200FB4739FEDD3BFE3C32E02FB9\
                       138D72145C7ADA1FDDBF363C0356D0AAEA672FA0A46470AD92BB99454A5A1F58";

/// Operators [`CONTENT`] decrypts to, among others.
const CONTENT_PLAINTEXT: &[u8] = b"(repro1a: AES-256 R5, password 'passwoert' with umlauts) Tj";

/// The user password of the document those constants came from.
const USER_PASSWORD: &str = "p\u{E4}ssw\u{F6}rt";

/// The attachment's bytes. `/EFF /Identity` leaves them as the file holds them.
const ATTACHMENT: &[u8] = b"an attachment in the clear";

/// The embedded file stream's object number in [`document`].
const EMBEDDED_FILE: u32 = 4;

/// Page one's annotation's appearance stream, in [`document`].
const APPEARANCE: u32 = 7;

/// Page one's content stream, in [`document`].
const PAGE_ONE_CONTENT: u32 = 9;

/// The object number of the first page; each page is followed by its content stream.
const FIRST_PAGE: usize = 8;

/// Bytes from a hexadecimal constant, white space ignored.
fn hex(text: &str) -> Vec<u8> {
    let digits: Vec<u8> = text
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .map(|byte| match byte {
            b'0'..=b'9' => byte - b'0',
            b'A'..=b'F' => byte - b'A' + 10,
            other => panic!("{} is not an upper-case hexadecimal digit", other as char),
        })
        .collect();
    digits
        .as_chunks::<2>()
        .0
        .iter()
        .map(|[high, low]| high * 16 + low)
        .collect()
}

/// A PDF string in §7.3.4.3's hexadecimal form.
fn hex_string(bytes: &[u8]) -> String {
    let mut out = String::from("<");
    for byte in bytes {
        let _ = write!(out, "{byte:02X}");
    }
    out.push('>');
    out
}

/// A stream object around `data`.
fn stream(data: &[u8]) -> Vec<u8> {
    let mut out = format!("<< /Length {} >>\nstream\n", data.len()).into_bytes();
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream");
    out
}

/// A document of `pages` pages, each with its own encrypted content stream, and one untyped
/// embedded file stream that a file specification's `/EF` names. Page one also carries a
/// square annotation whose appearance stream is the same ciphertext, as a form `XObject`.
///
/// `/StmF` is the standard filter and `/EFF` is `/Identity`, the direction of review 2131's
/// B06; `/StrF` is `/Identity` so that the name tree's strings need no cipher here.
fn document(pages: usize) -> Vec<u8> {
    let mut kids = String::new();
    for index in 0..pages {
        let _ = write!(kids, "{} 0 R ", FIRST_PAGE + 2 * index);
    }
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R \
           /Names << /EmbeddedFiles << /Names [(a.txt) 3 0 R] >> >> >>"
            .to_vec(),
        format!("<< /Type /Pages /Kids [{kids}] /Count {pages} >>").into_bytes(),
        format!("<< /Type /Filespec /F (a.txt) /UF (a.txt) /EF << /F {EMBEDDED_FILE} 0 R >> >>")
            .into_bytes(),
        stream(ATTACHMENT),
        format!(
            "<< /Filter /Standard /V 5 /R 5 /Length 256 \
             /CF << /StdCF << /CFM /AESV3 /Length 32 /AuthEvent /DocOpen >> >> \
             /StmF /StdCF /StrF /Identity /EFF /Identity \
             /O {} /U {} /OE {} /UE {} /Perms {} /P -4 /EncryptMetadata true >>",
            hex_string(&hex(OWNER_ENTRY)),
            hex_string(&hex(USER_ENTRY)),
            hex_string(&hex(OWNER_ENCRYPTION)),
            hex_string(&hex(USER_ENCRYPTION)),
            hex_string(&hex(PERMS)),
        )
        .into_bytes(),
        format!("<< /Type /Annot /Subtype /Square /Rect [10 10 450 190] /AP << /N {APPEARANCE} 0 R >> >>")
            .into_bytes(),
    ];
    let content = hex(CONTENT);
    let mut appearance = format!(
        "<< /Subtype /Form /BBox [0 0 440 180] /Length {} >>\nstream\n",
        content.len()
    )
    .into_bytes();
    appearance.extend_from_slice(&content);
    appearance.extend_from_slice(b"\nendstream");
    objects.push(appearance);
    for index in 0..pages {
        let annotations = if index == 0 { "/Annots [6 0 R]" } else { "" };
        objects.push(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 460 200] {annotations} \
                 /Contents {} 0 R >>",
                FIRST_PAGE + 1 + 2 * index
            )
            .into_bytes(),
        );
        objects.push(stream(&content));
    }
    assemble(&objects, "/Root 1 0 R /Encrypt 5 0 R")
}

/// Numbered objects in a file with a §7.5.4 cross-reference table whose offsets are those of the
/// bytes as laid down, so the reader takes a well-formed file's path.
fn assemble(objects: &[Vec<u8>], trailer_entries: &str) -> Vec<u8> {
    let mut out: Vec<u8> = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let table_at = out.len();
    let size = objects.len() + 1;
    out.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in &offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!("trailer\n<< /Size {size} {trailer_entries} >>\nstartxref\n{table_at}\n%%EOF\n")
            .as_bytes(),
    );
    out
}

/// The fixture opened with its user password.
fn open(bytes: Vec<u8>) -> Document {
    Document::open_with_password(bytes, Limits::default(), USER_PASSWORD)
        .expect("the user password opens the fixture")
}

/// Page one interpreted, as the viewer's launch path interprets it before it draws.
fn interpret_page_one(document: &Document) -> pdf_model::Interpretation {
    let page = pdf_model::Pages::new(document)
        .get(0)
        .expect("the fixture has a page one");
    pdf_model::interpret(document, &page)
}

/// The stream `number`'s data as the document decrypted it.
fn data_of(document: &Document, number: u32) -> Vec<u8> {
    document
        .get(ObjectId::new(number, 0))
        .as_stream()
        .expect("the object is a stream")
        .data
        .to_vec()
}

/// Whether `data` carries the producer's operators, which no wrong key gives.
fn holds_plaintext(data: &[u8]) -> bool {
    data.windows(CONTENT_PLAINTEXT.len())
        .any(|window| window == CONTENT_PLAINTEXT)
}

/// Page one's content stream is reached as `/Contents` and its annotation's appearance stream
/// as `/AP`'s `/N`, neither of which Table 43 makes an embedded file, so both are decrypted under
/// `/StmF` without the pass that finds `/EF`'s streams. The attachment, asked for by number with
/// no role, still takes the pass and `/EFF`.
#[test]
fn page_one_is_drawn_without_the_embedded_file_pass() {
    let document = open(document(500));
    let _ = interpret_page_one(&document);
    assert!(
        !document.embedded_file_streams_collected(),
        "page one's streams are reached in roles Table 43 does not give an embedded file"
    );
    assert!(
        holds_plaintext(&data_of(&document, PAGE_ONE_CONTENT)),
        "page one's content stream takes /StmF, the producer's AES-256 key"
    );
    assert!(
        holds_plaintext(&data_of(&document, APPEARANCE)),
        "page one's appearance stream takes /StmF too"
    );

    assert_eq!(
        data_of(&document, EMBEDDED_FILE),
        ATTACHMENT,
        "the stream /EF names takes /EFF /Identity, with or without Table 44's optional /Type"
    );
    assert!(
        document.embedded_file_streams_collected(),
        "a stream reached by number alone is decided by the pass"
    );
}

/// Opening the fixture and interpreting page one, the median of `runs`.
fn time_to_page_one(bytes: &[u8], runs: usize) -> Duration {
    let mut samples: Vec<Duration> = (0..runs)
        .map(|_| {
            let start = Instant::now();
            let document = open(bytes.to_vec());
            let _ = interpret_page_one(&document);
            start.elapsed()
        })
        .collect();
    samples.sort();
    samples[runs / 2]
}

/// CLAUDE.md principle 2's "a 500-page document must reach its first page no slower than a
/// 5-page one", for the files ADR 3234 priced. A clock, so it is a reading rather than a gate:
/// `cargo test --release -p pdf-model --test syntax_suite embedded_file_scan_launch --
/// --include-ignored --nocapture`.
// not a gate: a clock read by hand for ADR 3241's A/B; the band on time-to-first-page is
// doc/checks/launch-path.toml's, and the test beside this one holds the structural claim (no
// pass before page one) on every run.
#[test]
#[ignore = "a clock; run in release by hand"]
fn time_to_page_one_against_page_count() {
    for pages in [5, 500, 5000] {
        let bytes = document(pages);
        let median = time_to_page_one(&bytes, 31);
        println!(
            "pages {pages:>5}  bytes {:>8}  open + page one median {:>9.1} us",
            bytes.len(),
            median.as_secs_f64() * 1e6
        );
    }
}
