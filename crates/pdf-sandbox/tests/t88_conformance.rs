//! ITU-T T.88 Annex K's conformance data, decoded through this crate's `JBIG2Decode` filter.
//!
//! Annex K ships ten codestreams with the bitmap the Recommendation's sample software reconstructs
//! from each, and states for every one that the reconstruction equals the original image (Table
//! K.1, mean squared error 0). The annex is informative, and the data is evidence about T.88, not
//! T.88: **seven of the ten streams depart from the clauses they exercise**, because the sample
//! encoder that wrote them does, and the sample decoder that drew their references departs the same
//! way (ADR 1459 section 2 names each departure with its clause). A conforming decoder refuses
//! those, so they are held here by name with the clause they break, and the day one of them starts
//! matching its reference this test fails and says to read the clause again.
//!
//! Each stream reaches the filter the way ISO 32000-2 §7.4.7 says a PDF carries it: the file
//! header, the end-of-page and the end-of-file segments dropped, the page's segments given page
//! association 1, the page-0 segments in a separate stream for Table 12's `/JBIG2Globals`, and one
//! image per page. And each is decoded under both isolations, which must agree.
//!
//! **The data is not in the repository.** Its copyright notice licenses it for conformance testing
//! of an implementation and nothing wider, which an Apache-2.0 tree cannot carry
//! (`doc/third-party-data.md`; `doc/questions/Q209` asks the owner). So this reads it from
//! `$T88_CONFORMANCE_DATA`, or from where the round that fetched it unpacked it, and prints a
//! sentence and passes where neither holds it.
//!
//! The count of streams that decode to their reference is printed, not written down anywhere: it
//! is what `doc/conformance/ledger.toml`'s §7.4.7 row points at.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::panic,
    reason = "offsets into codestreams and bitmaps of at most a few hundred kilobytes, each \
              bounded by a length read two lines before it, and helpers called only from the test: \
              a test should fail loudly, and an overflow, an `expect` or a `panic` here does"
)]

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use pdf_sandbox::{Bilevel, Decoded, Isolation, Request, Sandbox};

/// Where Annex K's files are read from when `$T88_CONFORMANCE_DATA` does not say.
///
/// The directory `JBIG2_ConformanceData-A20180829.zip` unpacks to, inside the Recommendation's own
/// zip (`doc/third-party-data.md` has the zip's address and hash).
const DEFAULT_DIRECTORY: &str = "/home/AI/specs/T.88/Software/conf/JBIG2_ConformanceData-A20180829";

/// The environment variable naming the directory instead.
const DIRECTORY_VARIABLE: &str = "T88_CONFORMANCE_DATA";

/// What a stream, or the part of one a case decodes, is held to.
#[derive(Debug, Clone, Copy)]
enum Expected {
    /// Its reference, bit for bit.
    Matches,
    /// Nothing: the stream departs from T.88 at the clause named, so a conforming decoder refuses
    /// it or draws something else. A match is a failure — it means the decoder has grown the
    /// sample software's habit.
    Departs(&'static str),
    /// A refusal from the filter, which says out loud what the codec this build carries would draw
    /// wrongly, until the fork takes the patch named under `doc/patches/`. The codec is asked
    /// directly as well, and its matching the reference is a failure that says the fork has taken
    /// the patch and the filter's refusal is to be deleted (ADR 1459).
    WaitsOn(&'static str),
    /// Nothing either way: ISO 32000-2 §7.4.7 excludes the feature from a PDF's `JBIG2Decode`.
    Outside(&'static str),
}

impl Expected {
    /// The expectation in the words the printed table carries.
    fn held(self) -> String {
        match self {
            Self::Matches => "held to its reference".to_owned(),
            Self::Departs(clause) => format!("departs from T.88 at {clause}"),
            Self::WaitsOn(patch) => format!("waits on doc/patches/{patch}"),
            Self::Outside(reason) => format!("outside the PDF profile: {reason}"),
        }
    }
}

/// Which segments of a stream's page a case decodes.
#[derive(Debug, Clone, Copy)]
enum Part {
    /// All of them.
    Whole,
    /// The page information and the immediate generic region only, held to the reference with the
    /// rectangle of the stream's text region cleared. The text region and its symbol dictionary
    /// depart from T.88; the generic region does not, and outside the text region's rectangle it
    /// is the whole page. The rectangle is the text region's own region segment information field
    /// (T.88 section 7.4.1): 37 by 8 pixels at (4, 1) in every stream this is used for.
    GenericRegion,
    /// The stream as Annex H.1 prints it, which `codeStreamTest1_TT1.jb2` is but for two bytes.
    AnnexH,
}

/// One image to decode and what it is held to.
#[derive(Debug, Clone, Copy)]
struct Case {
    /// The codestream's file name, without `.jb2`.
    stream: &'static str,
    /// Its page, counted from 1 as its segments' page association counts.
    page: u32,
    part: Part,
    expected: Expected,
}

/// Where `codeStreamTest1_TT1.jb2` differs from Annex H.1's printing of the same datastream: the
/// first two bytes of its text region's symbol ID Huffman table (T.88 section 7.4.3.1.7), at file
/// offset `0x9B`.
const ANNEX_H_OFFSET: usize = 0x9B;
/// The two bytes the file holds there. As filed, RUNCODE0 and RUNCODE1 get the one-bit codes, the
/// three symbol ID code lengths decode as 1, 1 and 0, and the "p" the text region draws has no code.
const FILE_HOLDS: [u8; 2] = [0x11, 0x00];
/// The two bytes Annex H.1 prints there. Its own walk through the segment decodes them as code
/// lengths 2, 2 and 1 for the "p", the "c" and the "a" — which is what the reference shows.
const ANNEX_H_PRINTS: [u8; 2] = [0x01, 0x10];

/// The ten streams of T.88 Table K.1, their pages, and the parts held separately.
const CASES: [Case; 16] = [
    // T.88 Table K.1 item 1: Annex H.1's three pages — Huffman and MMR coding, then arithmetic, then
    // refinement and aggregation.
    Case {
        stream: "codeStreamTest1_TT1",
        page: 1,
        part: Part::Whole,
        expected: Expected::Departs(
            "7.4.3.1.7: the file's symbol ID Huffman table gives the third symbol no code; Annex \
             H.1 prints two other bytes at 0x9B",
        ),
    },
    Case {
        stream: "codeStreamTest1_TT1",
        page: 1,
        part: Part::AnnexH,
        expected: Expected::Matches,
    },
    Case {
        stream: "codeStreamTest1_TT1",
        page: 2,
        part: Part::Whole,
        expected: Expected::Matches,
    },
    Case {
        stream: "codeStreamTest1_TT1",
        page: 3,
        part: Part::Whole,
        expected: Expected::Matches,
    },
    // Item 2: a Huffman-coded symbol dictionary.
    Case {
        stream: "codeStreamTest1_TT2",
        page: 1,
        part: Part::Whole,
        expected: Expected::Departs(
            "6.5.9: SDHUFFBMSIZE is not the collective bitmap's length and each symbol is MMR-coded \
             alone; 6.5.10: the export runs never reach the symbol count; 7.4.3.2: the text \
             region refers to no symbol dictionary",
        ),
    },
    // Item 3: the same, arithmetic-coded.
    Case {
        stream: "codeStreamTest1_TT3",
        page: 1,
        part: Part::Whole,
        expected: Expected::Departs(
            "6.5.10: the export runs are two zeros and never reach the symbol count; 7.4.3.2: the \
             text region refers to no symbol dictionary",
        ),
    },
    Case {
        stream: "codeStreamTest1_TT3",
        page: 1,
        part: Part::GenericRegion,
        expected: Expected::Matches,
    },
    // Item 4: a generic region with template 1.
    Case {
        stream: "codeStreamTest1_TT4",
        page: 1,
        part: Part::Whole,
        expected: Expected::Departs(
            "6.5.10: the export runs are two zeros; 7.4.3.2: the text region refers to no symbol \
             dictionary",
        ),
    },
    Case {
        stream: "codeStreamTest1_TT4",
        page: 1,
        part: Part::GenericRegion,
        expected: Expected::Matches,
    },
    // Item 5: a symbol dictionary refining symbols.
    Case {
        stream: "codeStreamTest1_TT5",
        page: 1,
        part: Part::Whole,
        expected: Expected::Departs(
            "6.5.5 step 4: no OOB ends the refinement dictionary's height class; 6.5.5 step 1: it \
             refers to no dictionary yet exports more symbols than it defines; 6.5.10; 7.4.3.2",
        ),
    },
    // Item 6: a text region refining symbol instances.
    Case {
        stream: "codeStreamTest2_TT6",
        page: 1,
        part: Part::Whole,
        expected: Expected::Departs(
            "6.4.9: no instance T is coded although SBSTRIPS is 4, because SBREFINE is 1; 6.5.10; \
             7.4.3.2",
        ),
    },
    // Item 7: a generic region with the extended template.
    Case {
        stream: "codeStreamTest1_TT7",
        page: 1,
        part: Part::Whole,
        expected: Expected::Departs(
            "6.5.10: the export runs are two zeros; 7.4.3.2: the text region refers to no symbol \
             dictionary",
        ),
    },
    Case {
        stream: "codeStreamTest1_TT7",
        page: 1,
        part: Part::GenericRegion,
        expected: Expected::WaitsOn("hayro-jbig2-extended-template.patch"),
    },
    // Item 8: a text region in colour.
    Case {
        stream: "codeStreamTest3_TT8",
        page: 1,
        part: Part::Whole,
        expected: Expected::Outside(
            "§7.4.7 requires COLEXTFLAG to be 0, and this text region's is 1",
        ),
    },
    // Items 9 and 10: one page of generic region, MMR-coded and arithmetic-coded.
    Case {
        stream: "F01_200_TT9",
        page: 1,
        part: Part::Whole,
        expected: Expected::Matches,
    },
    Case {
        stream: "F01_200_TT10",
        page: 1,
        part: Part::Whole,
        expected: Expected::Matches,
    },
];

/// One segment of a JBIG2 file in the sequential organisation (T.88 sections 7.2 and D.1).
#[derive(Debug)]
struct Segment {
    /// The segment's type, the low six bits of its header flags (7.2.3).
    kind: u8,
    /// Its page association (7.2.6).
    page: u32,
    /// Header and data, with the page association already set to 1 where it was not 0.
    bytes: Vec<u8>,
}

/// T.88 section 7.3's segment types this file needs by name.
const PAGE_INFORMATION: u8 = 48;
const END_OF_PAGE: u8 = 49;
const END_OF_FILE: u8 = 51;
const IMMEDIATE_GENERIC_REGION: u8 = 38;
const IMMEDIATE_LOSSLESS_GENERIC_REGION: u8 = 39;

/// A big-endian 32-bit field.
fn read_u32(bytes: &[u8], at: usize) -> u32 {
    u32::from_be_bytes(bytes[at..at + 4].try_into().expect("four bytes make a u32"))
}

/// The segments of a JBIG2 file, each made ready for §7.4.7's embedded organisation.
///
/// The file header is T.88 section D.4's: an eight-byte identifier, a flags byte whose bit 0 says
/// the sequential organisation and whose bit 1 says the page count is unknown, and otherwise four
/// bytes of page count. Every Annex K stream is sequential, and this asserts it rather than
/// reorganising a random-access one.
fn segments(file: &[u8]) -> Vec<Segment> {
    assert_eq!(&file[..8], b"\x97JB2\r\n\x1a\n", "T.88 D.4.1's identifier");
    let flags = file[8];
    assert_eq!(
        flags & 1,
        1,
        "Annex K's streams use the sequential organisation"
    );
    let mut at = if flags & 2 == 0 { 13 } else { 9 };

    let mut out = Vec::new();
    while at < file.len() {
        let start = at;
        let number = read_u32(file, at);
        let header_flags = file[at + 4];
        at += 5;
        // 7.2.4: three bits of count, or all three set and a four-byte count with a retain bit for
        // each referred-to segment and this one.
        let short_count = usize::from(file[at] >> 5);
        let referred = if short_count == 7 {
            let count = usize::try_from(read_u32(file, at) & 0x1fff_ffff).expect("fits");
            at += 4 + (count + 8) / 8;
            count
        } else {
            at += 1;
            short_count
        };
        // 7.2.5: each referred-to number is one, two or four bytes, by this segment's own number.
        let width = match number {
            0..=256 => 1,
            257..=65536 => 2,
            _ => 4,
        };
        at += referred * width;
        // 7.2.6: the page association is four bytes where bit 6 of the flags says so.
        let long_page = header_flags & 0x40 != 0;
        let association_at = at - start;
        let page = if long_page {
            read_u32(file, at)
        } else {
            u32::from(file[at])
        };
        at += if long_page { 4 } else { 1 };
        let length = usize::try_from(read_u32(file, at)).expect("fits");
        at += 4 + length;

        let mut bytes = file[start..at].to_vec();
        // §7.4.7: "the value of its segment page association field shall be set to 1".
        if page != 0 {
            if long_page {
                bytes[association_at..association_at + 4].copy_from_slice(&1_u32.to_be_bytes());
            } else {
                bytes[association_at] = 1;
            }
        }
        out.push(Segment {
            kind: header_flags & 0x3f,
            page,
            bytes,
        });
    }
    out
}

/// One page of a file as §7.4.7 puts it in a PDF: the image `XObject`'s stream and the
/// `/JBIG2Globals` stream.
///
/// §7.4.7: "The JBIG2 file header, end-of-page segments, and end-of-file segment shall not be
/// present", the page's segments are its stream, and the segments of page 0 are the globals.
fn embedded(segments: &[Segment], page: u32, part: Part) -> (Vec<u8>, Vec<u8>) {
    let kept = |segment: &&Segment| !matches!(segment.kind, END_OF_PAGE | END_OF_FILE);
    let in_part = |segment: &&Segment| match part {
        Part::Whole | Part::AnnexH => true,
        Part::GenericRegion => matches!(
            segment.kind,
            PAGE_INFORMATION | IMMEDIATE_GENERIC_REGION | IMMEDIATE_LOSSLESS_GENERIC_REGION
        ),
    };
    let data = segments
        .iter()
        .filter(|segment| segment.page == page)
        .filter(kept)
        .filter(in_part)
        .flat_map(|segment| segment.bytes.iter().copied())
        .collect();
    let globals = match part {
        Part::GenericRegion => Vec::new(),
        Part::Whole | Part::AnnexH => segments
            .iter()
            .filter(|segment| segment.page == 0)
            .filter(kept)
            .flat_map(|segment| segment.bytes.iter().copied())
            .collect(),
    };
    (data, globals)
}

/// A reference bitmap: one `bool` a pixel, true where it is ink, rows from the top.
#[derive(Debug, PartialEq, Eq)]
struct Picture {
    width: u32,
    height: u32,
    ink: Vec<bool>,
}

/// Reads one of Annex K's `.bmp` files: the Windows bitmap format, uncompressed, one bit a pixel
/// through a palette or 24 bits a pixel, rows bottom-up where the height is positive and padded to
/// four bytes. A pixel is ink where its colour is not white — which for the one 24-bit reference,
/// item 8's coloured text, is the shape a bilevel decoder draws.
fn bitmap_file(bytes: &[u8]) -> Picture {
    assert_eq!(&bytes[..2], b"BM");
    let little = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().expect("four bytes"));
    let offset = usize::try_from(little(10)).expect("fits");
    let width = little(18);
    let stated_height = i32::from_le_bytes(bytes[22..26].try_into().expect("four bytes"));
    let bits = u16::from_le_bytes([bytes[28], bytes[29]]);
    assert!(little(30) == 0, "an uncompressed bitmap");
    let height = stated_height.unsigned_abs();
    let width_usize = usize::try_from(width).expect("fits");
    let stride = (width_usize * usize::from(bits)).div_ceil(32) * 4;
    let white = |colour: &[u8]| colour.iter().all(|channel| *channel == 0xff);

    let mut ink = Vec::with_capacity(width_usize * usize::try_from(height).expect("fits"));
    for y in 0..height {
        let stored = if stated_height > 0 { height - 1 - y } else { y };
        let row = offset + usize::try_from(stored).expect("fits") * stride;
        for x in 0..width_usize {
            let colour = match bits {
                1 => {
                    let index = (bytes[row + x / 8] >> (7 - x % 8)) & 1;
                    let entry = 54 + usize::from(index) * 4;
                    &bytes[entry..entry + 3]
                }
                24 => &bytes[row + x * 3..row + x * 3 + 3],
                other => panic!("Annex K's references are 1- or 24-bit, not {other}-bit"),
            };
            ink.push(!white(colour));
        }
    }
    Picture { width, height, ink }
}

/// The filter's output as a [`Picture`]. A set bit is white: the filter's sense is `DeviceGray`'s,
/// not JBIG2's (`decode.rs`'s `PackedRows`).
fn decoded(image: &Bilevel) -> Picture {
    let row_bytes = usize::try_from(image.width.div_ceil(8)).expect("fits");
    let mut ink = Vec::new();
    for y in 0..usize::try_from(image.height).expect("fits") {
        for x in 0..usize::try_from(image.width).expect("fits") {
            ink.push((image.rows[y * row_bytes + x / 8] >> (7 - x % 8)) & 1 == 0);
        }
    }
    Picture {
        width: image.width,
        height: image.height,
        ink,
    }
}

/// The codec's own answer, asked without the filter in front of it.
struct CodecPicture {
    ink: Vec<bool>,
}

impl hayro_jbig2::Decoder for CodecPicture {
    fn push_pixel(&mut self, black: bool) {
        self.ink.push(black);
    }

    fn push_pixel_chunk(&mut self, black: bool, chunk_count: u32) {
        let pixels = usize::try_from(chunk_count).expect("fits") * 8;
        self.ink.extend(std::iter::repeat_n(black, pixels));
    }

    fn next_line(&mut self) {}
}

/// Whether `hayro_jbig2` alone decodes the embedded streams to `reference`.
fn codec_matches(data: &[u8], globals: &[u8], reference: &Picture) -> bool {
    let globals = (!globals.is_empty()).then_some(globals);
    let Ok(image) = hayro_jbig2::Image::new_embedded(data, globals) else {
        return false;
    };
    let mut picture = CodecPicture { ink: Vec::new() };
    image.decode(&mut picture).is_ok()
        && (image.width(), image.height()) == (reference.width, reference.height)
        && picture.ink == reference.ink
}

/// The reference with the text region's rectangle cleared: what the generic region alone draws.
fn without_text_region(mut picture: Picture) -> Picture {
    let (left, top, width, height) = (4, 1, 37, 8);
    for y in top..top + height {
        for x in left..left + width {
            let at = y * usize::try_from(picture.width).expect("fits") + x;
            picture.ink[at] = false;
        }
    }
    picture
}

/// What one decode came to, in words a table can carry.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Matches,
    Differs(String),
    Refused(String),
}

impl Outcome {
    fn of(result: Result<Decoded, pdf_sandbox::SandboxError>, reference: &Picture) -> Self {
        match result {
            Err(error) => Self::Refused(error.to_string()),
            Ok(Decoded::Raster(_)) => Self::Differs("a raster came back from JBIG2".to_owned()),
            Ok(Decoded::Bilevel(image)) => {
                let picture = decoded(&image);
                if (picture.width, picture.height) != (reference.width, reference.height) {
                    Self::Differs(format!(
                        "{}x{} where the reference is {}x{}",
                        picture.width, picture.height, reference.width, reference.height
                    ))
                } else if picture == *reference {
                    Self::Matches
                } else {
                    let wrong = picture
                        .ink
                        .iter()
                        .zip(&reference.ink)
                        .filter(|(ours, theirs)| ours != theirs)
                        .count();
                    Self::Differs(format!("{wrong} pixels differ"))
                }
            }
        }
    }
}

/// The directory holding Annex K's files, or `None` where this machine has none.
fn directory() -> Option<PathBuf> {
    let named = std::env::var_os(DIRECTORY_VARIABLE).map(PathBuf::from);
    let directory = named.unwrap_or_else(|| PathBuf::from(DEFAULT_DIRECTORY));
    directory
        .join("codeStreamTest1_TT1.jb2")
        .is_file()
        .then_some(directory)
}

/// The reference bitmap for one page of one stream: T.88 Table K.1's "reconstructed image", named
/// `<stream>_TT<page - 1, two digits>.bmp`.
fn reference(directory: &Path, case: &Case) -> Picture {
    let name = format!("{}_TT{:02}.bmp", case.stream, case.page - 1);
    let picture =
        bitmap_file(&std::fs::read(directory.join(&name)).expect("T.88 Table K.1 names it"));
    match case.part {
        Part::Whole | Part::AnnexH => picture,
        Part::GenericRegion => without_text_region(picture),
    }
}

/// The codestream a case decodes, before §7.4.7's embedding.
fn codestream(directory: &Path, case: &Case) -> Vec<u8> {
    let path = directory.join(format!("{}.jb2", case.stream));
    let mut file = std::fs::read(path).expect("T.88 Table K.1 names it");
    if matches!(case.part, Part::AnnexH) {
        let held = &mut file[ANNEX_H_OFFSET..ANNEX_H_OFFSET + 2];
        assert_eq!(
            held, FILE_HOLDS,
            "the file differs from Annex H.1 where and as this test says it does"
        );
        held.copy_from_slice(&ANNEX_H_PRINTS);
    }
    file
}

#[test]
fn annex_k_streams_against_their_reference_bitmaps() {
    let Some(directory) = directory() else {
        println!(
            "skipped: ITU-T T.88 Annex K's conformance data is not on this machine. It is not in \
             the repository (doc/third-party-data.md); unpack JBIG2_ConformanceData-A20180829.zip \
             from the Recommendation's zip and name the directory with ${DIRECTORY_VARIABLE}"
        );
        return;
    };

    let mut table = String::new();
    let mut failures = Vec::new();
    // Each whole stream's name, and whether every page of it decoded to its reference.
    let mut streams: Vec<(&str, bool)> = Vec::new();
    for case in &CASES {
        let reference = reference(&directory, case);
        let (data, globals) = embedded(
            &segments(&codestream(&directory, case)),
            case.page,
            case.part,
        );
        let request = Request::Jbig2 {
            data: &data,
            globals: &globals,
        };

        pdf_sandbox::set_isolation(Isolation::InProcess);
        let here = Outcome::of(pdf_sandbox::decode(&request), &reference);
        pdf_sandbox::set_isolation(Isolation::Sandboxed);
        let confined = Outcome::of(Sandbox::shared().decode(&request), &reference);

        let name = format!("{} page {} ({:?})", case.stream, case.page, case.part);
        writeln!(table, "{name:44} {here:?} — {}", case.expected.held()).expect("a String");
        if here != confined {
            failures.push(format!(
                "{name}: in process {here:?}, confined {confined:?} — the isolations disagree"
            ));
        }
        if matches!(case.part, Part::Whole) {
            let matched = here == Outcome::Matches;
            match streams
                .iter_mut()
                .find(|(stream, _)| *stream == case.stream)
            {
                Some((_, every_page)) => *every_page &= matched,
                None => streams.push((case.stream, matched)),
            }
        }
        if let Expected::WaitsOn(patch) = case.expected
            && codec_matches(&data, &globals, &reference)
        {
            failures.push(format!(
                "{name}: the codec now decodes it to its reference, so the fork has taken \
                 doc/patches/{patch}: delete the filter's refusal in decode.rs (ADR 1459) and hold \
                 this as Matches"
            ));
        }
        match (case.expected, &here) {
            (Expected::Matches, Outcome::Matches) => {}
            (Expected::Matches, other) => {
                failures.push(format!("{name}: held to its reference, and {other:?}"));
            }
            (Expected::Departs(clause), Outcome::Matches) => failures.push(format!(
                "{name} departs from T.88 ({clause}) and now matches the sample software's \
                 reference: read the clause again before believing either"
            )),
            (Expected::WaitsOn(patch), Outcome::Matches) => failures.push(format!(
                "{name} matches through the filter, which was to refuse it until the fork took \
                 doc/patches/{patch}"
            )),
            (Expected::WaitsOn(patch), Outcome::Differs(how)) => failures.push(format!(
                "{name}: drawn wrongly ({how}) where the filter was to refuse it out loud until \
                 the fork took doc/patches/{patch}"
            )),
            (Expected::Departs(_) | Expected::WaitsOn(_) | Expected::Outside(_), _) => {}
        }
    }
    println!("{table}");
    let decoding = streams.iter().filter(|(_, every_page)| *every_page).count();
    println!(
        "{decoding} of Annex K's {} streams decode to their reference bitmaps on every page",
        streams.len()
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
