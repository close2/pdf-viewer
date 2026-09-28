//! Whether `pdf-ccitt` decodes every `CCITTFaxDecode` stream in the corpus to the scan lines
//! `hayro-ccitt` decodes, and how long each takes.
//!
//! ADR 1349 moved ISO 32000-2 §7.4.6 onto this tree's own decoder, so that Table 11's
//! `/DamagedRowsBeforeError` could be built; this is the measurement that decision rests on. Every
//! stream object whose filter chain ends in `CCITTFaxDecode` — an image `XObject`, a `/Mask` or an
//! `/SMask` — is decoded by both, with the parameters `pdf_model::image` would send, and the two
//! are compared scan line by scan line, bit for bit. Where they disagree the line is printed with
//! the first row that differs and each decoder's error, which is where the reading of T.4 and T.6
//! gets checked: `hayro-ccitt` is evidence about that reading, never its definition (principle 5).
//!
//! The concealment itself is not compared, because the other decoder has none: both decode at
//! `/DamagedRowsBeforeError` zero, and how many streams state the entry above zero where Table 11
//! lets it apply is counted beside.
//!
//! **What this does not reach**: an inline image (§8.9.7) is inside a content stream, and finding
//! its data is the interpreter's; `tests/inline_images.rs` and the corpus gates reach those.
//!
//! ```sh
//! cargo run --release -p pdf-model --example ccitt_decoder_census -- <directory>... 2>/dev/null
//! ```
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]
#![allow(clippy::print_stdout, clippy::print_stderr)]
#![allow(
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    reason = "counters and milliseconds over a corpus, measured and summarised in one pass; a \
              measurement rather than a shipped path"
)]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use pdf_syntax::{Dictionary, Document, Object, ObjectId};

/// The pixel bound `pdf_sandbox::decode` refuses above, so that no image is decoded here that the
/// filter would not decode.
const MAX_PIXELS: u64 = 1 << 28;

/// Every `.pdf` under the paths named, however deep.
fn documents(roots: &[String]) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending: Vec<PathBuf> = roots.iter().map(PathBuf::from).collect();
    while let Some(path) = pending.pop() {
        if path.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&path) {
                pending.extend(entries.flatten().map(|entry| entry.path()));
            }
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
        {
            found.push(path);
        }
    }
    found.sort();
    found
}

/// Table 11's entries as `pdf_model::image` resolves them.
struct Resolved {
    k: i64,
    columns: u32,
    rows: u32,
    end_of_line: bool,
    encoded_byte_align: bool,
    end_of_block: bool,
    damaged_rows: i64,
}

fn resolve(document: &Document, parms: Option<&Dictionary>, height: u32) -> Resolved {
    let integer = |key: &str, default: i64| -> i64 {
        parms
            .map(|parms| document.get_key(parms, key))
            .and_then(|value| value.as_integer())
            .unwrap_or(default)
    };
    let flag = |key: &str, default: bool| -> bool {
        match parms.map(|parms| document.get_key(parms, key)) {
            Some(Object::Boolean(value)) => value,
            _ => default,
        }
    };
    let end_of_block = flag("EndOfBlock", true);
    let stated_rows = u32::try_from(integer("Rows", 0)).unwrap_or(0);
    // `pdf_model::image::ccitt_rows`: `/Rows` binds only under `/EndOfBlock` false.
    let rows = match stated_rows {
        0 => height,
        _ if end_of_block => height,
        rows => rows,
    };
    Resolved {
        k: integer("K", 0),
        columns: u32::try_from(integer("Columns", 1728)).unwrap_or(0),
        rows,
        end_of_line: flag("EndOfLine", false),
        encoded_byte_align: flag("EncodedByteAlign", false),
        end_of_block,
        damaged_rows: integer("DamagedRowsBeforeError", 0),
    }
}

/// Scan lines packed a bit a pel, black set, one `Vec` a line.
#[derive(Default)]
struct Packed {
    lines: Vec<Vec<u8>>,
    line: Vec<u8>,
    filled: u32,
}

impl Packed {
    fn push(&mut self, black: bool, count: u32) {
        for _ in 0..count {
            if self.filled.is_multiple_of(8) {
                self.line.push(0);
            }
            if black {
                *self.line.last_mut().unwrap() |= 0x80 >> (self.filled % 8);
            }
            self.filled += 1;
        }
    }

    fn end(&mut self) {
        self.lines.push(std::mem::take(&mut self.line));
        self.filled = 0;
    }
}

impl hayro_ccitt::Decoder for Packed {
    fn push_pixels(&mut self, white: bool, count: u32) {
        self.push(!white, count);
    }

    fn next_line(&mut self) {
        self.end();
    }
}

/// `pdf-ccitt`'s lines, packed the same way.
struct Ours {
    packed: Packed,
    columns: u32,
}

impl pdf_ccitt::Rows for Ours {
    fn row(&mut self, changes: &[u32]) {
        let mut at = 0;
        let mut black = false;
        for &change in changes.iter().chain(std::iter::once(&self.columns)) {
            self.packed.push(black, change - at);
            at = change;
            black = !black;
        }
        self.packed.end();
    }
}

/// One decoder's answer: its whole lines, whether it ended on an error, and the time it took.
struct Answer {
    lines: Vec<Vec<u8>>,
    error: Option<String>,
    spent: Duration,
}

fn hayro(data: &[u8], resolved: &Resolved) -> Answer {
    use hayro_ccitt::{DecodeSettings, DecoderContext, EncodingMode};
    let settings = DecodeSettings {
        columns: resolved.columns,
        rows: resolved.rows,
        end_of_block: resolved.end_of_block,
        end_of_line: resolved.end_of_line,
        rows_are_byte_aligned: resolved.encoded_byte_align,
        encoding: match resolved.k {
            ..0 => EncodingMode::Group4,
            0 => EncodingMode::Group3_1D,
            k => EncodingMode::Group3_2D {
                k: u32::try_from(k).unwrap_or(u32::MAX),
            },
        },
        invert_black: false,
    };
    let mut packed = Packed::default();
    let started = Instant::now();
    let result = hayro_ccitt::decode(data, &mut packed, &mut DecoderContext::new(settings));
    let spent = started.elapsed();
    Answer {
        lines: packed.lines,
        error: result.err().map(|error| error.to_string()),
        spent,
    }
}

fn ours(data: &[u8], resolved: &Resolved) -> Answer {
    let parameters = pdf_ccitt::Parameters {
        coding: pdf_ccitt::Coding::from_k(resolved.k),
        columns: resolved.columns,
        rows: resolved.rows,
        end_of_line: resolved.end_of_line,
        encoded_byte_align: resolved.encoded_byte_align,
        end_of_block: resolved.end_of_block,
        damaged_rows_before_error: 0,
    };
    let mut rows = Ours {
        packed: Packed::default(),
        columns: resolved.columns,
    };
    let started = Instant::now();
    let result = pdf_ccitt::decode(data, &parameters, &mut rows);
    let spent = started.elapsed();
    Answer {
        lines: rows.packed.lines,
        error: result.err().map(|error| error.to_string()),
        spent,
    }
}

/// Whether the raw bytes name the filter at all, so the documents that do not are never parsed.
fn mentions_the_filter(bytes: &[u8]) -> bool {
    const NEEDLE: &[u8] = b"CCITTFaxDecode";
    bytes.windows(NEEDLE.len()).any(|window| window == NEEDLE)
}

fn main() {
    let roots: Vec<String> = std::env::args().skip(1).collect();
    let roots = if roots.is_empty() {
        vec![
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../doc/pdf.js/test/pdfs")
                .to_string_lossy()
                .into_owned(),
        ]
    } else {
        roots
    };
    let files = documents(&roots);
    let (mut scanned, mut with_filter, mut streams) = (0_usize, 0_usize, 0_usize);
    let (mut identical, mut same_lines_other_ending, mut different) = (0_usize, 0_usize, 0_usize);
    let (mut too_large, mut concealment_applies) = (0_usize, 0_usize);
    let (mut theirs_total, mut ours_total) = (Duration::ZERO, Duration::ZERO);
    let (mut lines_total, mut pels_total) = (0_u64, 0_u64);
    for path in &files {
        scanned += 1;
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        if !mentions_the_filter(&bytes) {
            continue;
        }
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        with_filter += 1;
        for number in document.xref().object_numbers() {
            let object = document.get(ObjectId {
                number,
                generation: 0,
            });
            let Object::Stream(stream) = object else {
                continue;
            };
            if document.image_codec(&stream).as_deref() != Some(b"CCITTFaxDecode".as_slice()) {
                continue;
            }
            let Some(image) = document.image_stream(&stream) else {
                continue;
            };
            let height = document
                .get_key(&stream.dict, "Height")
                .as_integer()
                .and_then(|height| u32::try_from(height).ok())
                .unwrap_or(0);
            let resolved = resolve(&document, image.parms.as_ref(), height);
            if resolved.columns == 0
                || u64::from(resolved.columns) * u64::from(resolved.rows.max(height)) > MAX_PIXELS
            {
                too_large += 1;
                continue;
            }
            streams += 1;
            if resolved.damaged_rows > 0 && resolved.end_of_line && resolved.k >= 0 {
                concealment_applies += 1;
            }
            let theirs = hayro(&image.data, &resolved);
            let mine = ours(&image.data, &resolved);
            theirs_total += theirs.spent;
            ours_total += mine.spent;
            lines_total += mine.lines.len() as u64;
            pels_total += mine.lines.len() as u64 * u64::from(resolved.columns);
            // A line the other decoder was inside when it stopped is not one it delivered.
            let theirs_whole: &[Vec<u8>] = &theirs.lines;
            if theirs_whole == mine.lines.as_slice() {
                if theirs.error.is_some() == mine.error.is_some() {
                    identical += 1;
                } else {
                    same_lines_other_ending += 1;
                    println!(
                        "ENDING\t{}\t{number}\tK {} cols {} rows {} eol {} align {} eob {}\t\
                         lines {}\thayro: {:?}\tours: {:?}",
                        path.display(),
                        resolved.k,
                        resolved.columns,
                        resolved.rows,
                        resolved.end_of_line,
                        resolved.encoded_byte_align,
                        resolved.end_of_block,
                        mine.lines.len(),
                        theirs.error,
                        mine.error
                    );
                }
            } else {
                different += 1;
                let first = theirs_whole
                    .iter()
                    .zip(&mine.lines)
                    .position(|(a, b)| a != b)
                    .unwrap_or_else(|| theirs_whole.len().min(mine.lines.len()));
                println!(
                    "DIFFER\t{}\t{number}\tK {} cols {} rows {} eol {} align {} eob {}\t\
                     hayro {} lines {:?}\tours {} lines {:?}\tfirst differing line {first}",
                    path.display(),
                    resolved.k,
                    resolved.columns,
                    resolved.rows,
                    resolved.end_of_line,
                    resolved.encoded_byte_align,
                    resolved.end_of_block,
                    theirs_whole.len(),
                    theirs.error,
                    mine.lines.len(),
                    mine.error
                );
            }
        }
    }
    println!(
        "documents {scanned}, naming the filter {with_filter}; CCITT streams {streams} \
         (+{too_large} over the pixel bound or with no columns): identical {identical}, same \
         lines ending differently {same_lines_other_ending}, different lines {different}; \
         /DamagedRowsBeforeError applying above zero {concealment_applies}"
    );
    println!(
        "decode time: hayro-ccitt {:.1} ms, pdf-ccitt {:.1} ms over {lines_total} lines, \
         {pels_total} pels",
        theirs_total.as_secs_f64() * 1e3,
        ours_total.as_secs_f64() * 1e3
    );
}
