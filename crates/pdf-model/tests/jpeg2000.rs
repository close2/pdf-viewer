//! Every JPEG 2000 codestream in the corpus, decoded against ISO/IEC 15444-5's reference software.
//!
//! # Why this exists, and why it is not principle 5's forbidden move
//!
//! `CLAUDE.md` principle 5 says never to treat another library as the source of truth. That rule
//! is about **ISO 32000-2**, and this test is not about ISO 32000-2 at all: §7.4.9 says only that
//! `JPXDecode` data "shall be" a JPEG 2000 codestream and hands the whole of the decoding to
//! ISO/IEC 15444-1, which defines it exactly and leaves a decoder no latitude. Two decoders of
//! that standard either produce the same samples or one of them is wrong.
//!
//! `opj_decompress` is `OpenJPEG`'s, and `OpenJPEG` is the reference software ISO/IEC 15444-5
//! publishes for Part 1. That is a genuinely different status from `poppler` reading a PDF — and
//! the inference still runs the way principle 5 states it: **agreement is evidence that this tree
//! reads 15444-1 correctly, and a disagreement is a question to take back to 15444-1**, not a
//! target to move toward. The comparison is exact rather than tolerant for the same reason
//! `jbig2.rs`'s is: a codec has one right answer per codestream, so a tolerance would only hide
//! the differences worth finding.
//!
//! # What it found on its first run
//!
//! **Thirteen of the corpus's thirty codestreams decode to samples `OpenJPEG` does not produce**,
//! and the discriminator is exact: every one of the thirteen states `qntsty` 2 — scalar expounded
//! quantisation, ISO/IEC 15444-1's *irreversible* 9/7 path — and every one of the fourteen that
//! match states `qntsty` 0, the reversible 5/3 one. See
//! [`DIFFERS_FROM_THE_REFERENCE_SOFTWARE`] for the measurement, the one crossing that proves the
//! rule, and which way the samples move. The defect is `hayro-jpeg2000`'s and the write-up is
//! `doc/JPEG2000_FEEDBACK.md`.
//!
//! And it ruled a cause *out*, which is worth the same. `jp2k-resetprob.pdf` sat first on §3a's
//! undiagnosed ambiguous ranking at 5.03 bounds from the nearest reference. `opj_dump` says its
//! code-blocks carry `cblksty=0x2` — ISO/IEC 15444-1 Table A.19's RESET, "reset context
//! probabilities on coding pass boundaries", which is the coding option the file is named for and
//! exactly the sort of thing a decoder gets subtly wrong. Our decode of it is **byte-identical**
//! to `OpenJPEG`'s, so its remaining difference is not the codec: the 40×27 image is drawn into
//! 30×21 device pixels, which is `AMBIGUOUS_IMAGE_REDUCTION`'s subject. ADR 0161.
//!
//! # What it does not check
//!
//! What happens to the samples *after* the codec: §7.4.9's `/SMaskInData`, the `/ColorSpace`
//! override, `/Decode`. Those are ISO 32000-2's and belong to the clauses that state them. And it
//! cannot run at all where `OpenJPEG` is not installed, in which case it says so and passes — the
//! same contract every gate in this tree has with `pdftoppm` and `pdftotext`.

#![expect(
    clippy::print_stdout,
    reason = "test code: the survey output is the point of the run"
)]

use std::path::{Path, PathBuf};

use pdf_syntax::Document;

/// Codestreams whose samples match `OpenJPEG`'s exactly.
///
/// Ratcheted, like every other count in this tree: it may only rise.
const IDENTICAL: usize = 14;

/// Codestreams neither side puts in the same shape, so no comparison is possible.
///
/// Three, and all three are about the *instrument* rather than about either decoder.
/// `opj_decompress` writes a Netpbm whose maximum is the codestream's own precision, so
/// `issue12213.pdf`'s four-bit image comes back with a maximum of 15 and `issue19326.pdf`'s
/// sixteen-bit one with 65535, while this tree's decoder always produces eight bits (§7.4.9
/// leaves the depth to the decoder and the image pipeline here is eight-bit throughout — how the
/// two scale to eight bits is a question worth asking and not this test's). And `issue19517.pdf`
/// object 8 declares 12608×16806 in four channels, beyond the confined decoder's budget, so ours
/// comes back at a reduced resolution level (3152×4202, §7.4.9 NOTE 3) while the reference
/// decodes the full grid — same image, two of the codestream's own versions, not comparable
/// sample for sample.
const NOT_COMPARABLE: usize = 3;

/// Codestreams whose samples differ from `OpenJPEG`'s, held by name in both directions.
///
/// # The discriminator is exact, and it named the defect
///
/// `opj_dump` on all 30: **every codestream here states `qntsty` 2 — scalar expounded
/// quantisation, which is ISO/IEC 15444-1's irreversible 9/7 path — and every codestream that
/// matches states `qntsty` 0, the reversible 5/3 one.** There is exactly one crossing, and it is
/// the one that proves the rule: `S2.pdf` object 35 is `qntsty` 2 and matches, and it is a
/// 316-byte 18×166 strip with a single quality layer, where a small reconstruction difference
/// rounds away. Layer count is *not* the discriminator: `issue5475.pdf` object 8 has one layer
/// and differs; `S2.pdf` objects 29 to 31 have five and six and match.
///
/// # Two defects, one clause, and both are fixed
///
/// ISO/IEC 15444-1's E-6 reconstructs a nonzero coefficient at `r · 2^(Mb − Nb)` above its
/// decoded magnitude, `r` conventionally ½. `Mb − Nb` is the count of magnitude bits never
/// coded. Both halves of that sentence were wrong in the decoder, and each hid the other:
///
/// 1. **The term was absent entirely.** `hayro-jpeg2000` 0.4.0's `Coefficient::get` returned the
///    truncated magnitude. The two-hundredth session measured the symptom — on `S2.pdf` object
///    17, two of every three differing samples moved toward the image's own mean and the standard
///    deviation fell from 0.2499 to 0.2399 — and named this as a *hypothesis*, because the
///    finding did not depend on it. It was right. Upstream `9cce046b` adds the term.
/// 2. **It was then skipped where `Mb − Nb` is zero**, which is a coefficient that was *fully*
///    decoded — and `2^0 = 1`, so the term is `r` itself rather than nothing, because the
///    quantisation interval of width Δ still surrounds the value. Invisible on coarsely
///    quantised images, where most coefficients are truncated and (1) dominates; the whole error
///    on finely quantised ones. Found in the three-hundred-and-eleventh session by bisecting on
///    resolution — `issue5475.pdf` object 8 has `numresolutions=2`, so decoding at `-r 1` stops
///    at the LL sub-band with no 9/7 synthesis at all, and the disagreement was still there —
///    then confirmed by the residual being symmetric and confined to fractional parts in
///    (0.25, 0.75), which is two floats a quarter-level apart and nothing else. Fixed in
///    `close2/hayro` `2a1abd14`, offered upstream.
///
/// **Without quantisation the term must not be applied at all**: there is no interval, a fully
/// decoded coefficient is exact, and offsetting it by half moves a lossless image. Applying (2)
/// unconditionally takes `S2.pdf` objects 29 to 31 from byte-identical to 19 131 samples wrong by
/// up to 5 — which is how that half of the fix earned its condition.
///
/// | | 0.4.0 | `9cce046b` | both |
/// |---|---|---|---|
/// | `S2.pdf` object 17 | 298 229 differ, worst by 52 | 53 286, worst by 3 | **325, worst by 1** |
/// | `S2.pdf` object 33 | 102 139, worst by 87 | 63, worst by 1 | 63, worst by 1 |
/// | `issue5475.pdf` object 8 | 91 144, worst by 2 | 91 144, worst by 2 | **48, worst by 1** |
/// | `issue5481.pdf` object 5 | 1 076 388, worst by 4 | 1 076 388, worst by 4 | **546, worst by 1** |
/// | `issue5549.pdf` object 11 | 965 165, worst by 5 | 965 165, worst by 5 | **2 494, worst by 1** |
///
/// Roughly **3.4 million differing samples become 5 900, and nothing exceeds one level**.
///
/// # What is left, and why the list is still thirteen
///
/// **Not one codestream became byte-identical**, through either fix. The population has never
/// moved; the magnitude of its error has, by a factor of 87. What remains is one level on 0.02%
/// to 0.1% of a plate's samples, and whether that is a third defect or the last place of two
/// `f32` pipelines is **not established** — a precision ladder is the instrument for that, and
/// nobody has run it. Two causes are ruled out and worth not re-testing: the final rounding mode
/// (half away from zero here, `lrintf` under the default mode there — forcing half-to-even moves
/// two counts the wrong way), and FMA, which `math::mul_add` already `cfg`s off on a target
/// without it.
///
/// Held by name in both directions: a codestream leaving this list means something improved and
/// the improvement should be recorded, and one arriving means something regressed.
const DIFFERS_FROM_THE_REFERENCE_SOFTWARE: [&str; 13] = [
    "S2.pdf object 17",
    "S2.pdf object 18",
    "S2.pdf object 19",
    "S2.pdf object 20",
    "S2.pdf object 21",
    "S2.pdf object 22",
    "S2.pdf object 32",
    "S2.pdf object 33",
    "S2.pdf object 34",
    "issue5475.pdf object 8",
    "issue5481.pdf object 5",
    "issue5481.pdf object 43",
    "issue5549.pdf object 11",
];

/// The corpus documents, or `None` when the submodule is not checked out.
fn corpus() -> Option<Vec<PathBuf>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
    let mut found: Vec<PathBuf> = std::fs::read_dir(root)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "pdf"))
        .collect();
    found.sort();
    (!found.is_empty()).then_some(found)
}

/// Whether `OpenJPEG`'s decompressor is on this machine.
fn have_openjpeg() -> bool {
    std::process::Command::new("opj_decompress")
        .arg("-h")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}

/// One decoded image: interleaved 8-bit colour components, opacity dropped.
#[derive(Debug)]
struct Samples {
    width: u32,
    height: u32,
    components: usize,
    data: Vec<u8>,
}

/// Decodes a codestream with `OpenJPEG`.
///
/// The intermediate is a Netpbm file because that is the one raster family `opj_decompress`
/// writes that needs no library to read back: a short ASCII header and the samples. It picks the
/// member itself — `P5` for one component, `P6` for three, `P7` (PAM) where there is an opacity
/// channel — so all three are read here.
fn openjpeg(codestream: &[u8], stem: &str) -> Result<Samples, String> {
    let dir = std::env::temp_dir().join(format!("pdfviewer-jpx-{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    // The extension decides how `opj_decompress` reads the input: a raw codestream is `.j2k`, a
    // JP2 container `.jp2`. §7.4.9 permits either, and the first two bytes say which — a raw
    // codestream begins with the SOC marker `FF 4F` and a JP2 file with a signature box.
    let raw = matches!(codestream.first_chunk::<2>(), Some(&[0xFF, 0x4F]));
    let input = dir.join(format!("{stem}.{}", if raw { "j2k" } else { "jp2" }));
    // `opj_decompress` chooses between PGM, PPM and PAM by what the codestream holds, and takes
    // the *directory* of the name it is given, so the extension asked for here is only a hint.
    let output = dir.join(format!("{stem}.pnm"));
    std::fs::write(&input, codestream).map_err(|error| error.to_string())?;
    let status = std::process::Command::new("opj_decompress")
        .arg("-i")
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|error| error.to_string())?;
    if !status.success() {
        return Err(format!("opj_decompress exited {status}"));
    }
    let written = [output.clone(), output.with_extension("pam")]
        .into_iter()
        .find(|path| path.exists())
        .ok_or_else(|| "opj_decompress wrote nothing".to_owned())?;
    let bytes = std::fs::read(&written).map_err(|error| error.to_string())?;
    let _ = std::fs::remove_file(&input);
    let _ = std::fs::remove_file(&written);
    read_netpbm(&bytes)
}

/// Splits an ASCII header into whitespace-separated tokens, honouring `#` comments.
fn tokens(bytes: &[u8], wanted: usize) -> Result<(Vec<String>, usize), String> {
    let mut fields = Vec::new();
    let mut at = 0usize;
    while fields.len() < wanted {
        while bytes.get(at).is_some_and(u8::is_ascii_whitespace) {
            at = at.saturating_add(1);
        }
        if bytes.get(at) == Some(&b'#') {
            while bytes.get(at).is_some_and(|byte| *byte != b'\n') {
                at = at.saturating_add(1);
            }
            continue;
        }
        let start = at;
        while bytes
            .get(at)
            .is_some_and(|byte| !byte.is_ascii_whitespace())
        {
            at = at.saturating_add(1);
        }
        if start == at {
            return Err("truncated Netpbm header".to_owned());
        }
        fields.push(String::from_utf8_lossy(bytes.get(start..at).unwrap_or_default()).into_owned());
    }
    Ok((fields, at.saturating_add(1)))
}

/// Reads a binary PGM, PPM or PAM.
fn read_netpbm(bytes: &[u8]) -> Result<Samples, String> {
    let magic = bytes.get(..2).ok_or_else(|| "empty file".to_owned())?;
    let (width, height, depth, maximum, at) = match magic {
        b"P5" | b"P6" => {
            let (fields, at) = tokens(bytes, 4)?;
            let number = |index: usize| -> Result<u32, String> {
                fields
                    .get(index)
                    .ok_or_else(|| "short header".to_owned())?
                    .parse::<u32>()
                    .map_err(|error| error.to_string())
            };
            let depth = if magic == b"P5" { 1usize } else { 3 };
            (number(1)?, number(2)?, depth, number(3)?, at)
        }
        b"P7" => {
            // PAM's header is `KEY VALUE` lines ending in `ENDHDR`, which the token split
            // above reads as a flat list.
            let end = bytes
                .windows(7)
                .position(|window| window == b"ENDHDR\n")
                .ok_or_else(|| "PAM header has no ENDHDR".to_owned())?;
            let fields: Vec<String> = String::from_utf8_lossy(bytes.get(..end).unwrap_or_default())
                .split_ascii_whitespace()
                .map(str::to_owned)
                .collect();
            let value = |key: &str| -> Result<u32, String> {
                let index = fields
                    .iter()
                    .position(|field| field == key)
                    .ok_or_else(|| format!("PAM header has no {key}"))?;
                fields
                    .get(index.saturating_add(1))
                    .ok_or_else(|| format!("PAM {key} has no value"))?
                    .parse::<u32>()
                    .map_err(|error| error.to_string())
            };
            let depth = usize::try_from(value("DEPTH")?).map_err(|error| error.to_string())?;
            (
                value("WIDTH")?,
                value("HEIGHT")?,
                depth,
                value("MAXVAL")?,
                end.saturating_add(7),
            )
        }
        other => {
            return Err(format!(
                "not a binary Netpbm: {:?}",
                String::from_utf8_lossy(other)
            ));
        }
    };
    if maximum != 255 {
        return Err(format!("maximum {maximum}, not 255"));
    }
    // PAM carries the opacity channel that PGM and PPM cannot; it is dropped so that both sides
    // of the comparison hold colour components only.
    let colour = if depth == 2 || depth == 4 {
        depth.saturating_sub(1)
    } else {
        depth
    };
    let mut data = Vec::new();
    for pixel in bytes.get(at..).unwrap_or_default().chunks(depth) {
        data.extend_from_slice(pixel.get(..colour).unwrap_or_default());
    }
    Ok(Samples {
        width,
        height,
        components: colour,
        data,
    })
}

/// Our decode of a codestream, with opacity dropped for the same reason.
fn ours(codestream: &[u8]) -> Result<Samples, String> {
    let decoded = pdf_sandbox::decode(&pdf_sandbox::Request::Jpx {
        data: codestream,
        indices: false,
    })
    .map_err(|error| error.to_string())?;
    let pdf_sandbox::Decoded::Raster(raster) = decoded else {
        return Err("not a raster".to_owned());
    };
    let channels = raster.channels();
    let colour = usize::from(raster.components);
    let mut data = Vec::new();
    for pixel in raster.data.chunks(channels) {
        data.extend_from_slice(pixel.get(..colour).unwrap_or_default());
    }
    Ok(Samples {
        width: raster.width,
        height: raster.height,
        components: colour,
        data,
    })
}

/// Every `/JPXDecode` codestream in one document, with the object number that holds it.
fn codestreams(document: &Document) -> Vec<(u32, std::sync::Arc<[u8]>)> {
    let mut found = Vec::new();
    for number in document.xref().object_numbers() {
        let object = document.get(pdf_syntax::ObjectId {
            number,
            generation: 0,
        });
        let Some(stream) = object.as_stream() else {
            continue;
        };
        let Some(image) = document.image_stream(stream) else {
            continue;
        };
        if image
            .codec
            .as_ref()
            .is_some_and(|name| name.as_slice() == b"JPXDecode")
        {
            found.push((number, image.data));
        }
    }
    found
}

/// What comparing one codestream against the reference software concluded.
enum Verdict {
    /// Sample for sample the same.
    Identical(String),
    /// Same shape, different samples — the line names how many and by how much.
    Differing(String),
    /// One side would not produce comparable samples at all, with the reason.
    Incomparable(String),
}

/// Decodes one codestream both ways and says how they compare.
fn compare(codestream: &[u8], name: &str, number: u32) -> Verdict {
    let at = format!("{name} object {number}");
    let stem = format!("{}-{number}", name.replace('.', "_"));
    let reference = match openjpeg(codestream, &stem) {
        Ok(reference) => reference,
        Err(error) => return Verdict::Incomparable(format!("{at}: opj_decompress: {error}")),
    };
    let mine = match ours(codestream) {
        Ok(mine) => mine,
        Err(error) => return Verdict::Incomparable(format!("{at}: ours: {error}")),
    };
    let shape_of = |image: &Samples| (image.width, image.height, image.components);
    if shape_of(&mine) != shape_of(&reference) {
        return Verdict::Incomparable(format!(
            "{at}: {}x{}x{} against {}x{}x{}",
            mine.width,
            mine.height,
            mine.components,
            reference.width,
            reference.height,
            reference.components
        ));
    }
    let mut worst = 0u8;
    let mut apart = 0usize;
    for (a, b) in mine.data.iter().zip(reference.data.iter()) {
        let difference = a.abs_diff(*b);
        if difference != 0 {
            apart = apart.saturating_add(1);
            worst = worst.max(difference);
        }
    }
    let shape = format!("{}x{}x{}", mine.width, mine.height, mine.components);
    if worst == 0 {
        Verdict::Identical(format!("{at}: {shape}"))
    } else {
        Verdict::Differing(format!(
            "{at}: {shape}, {apart} of {} samples differ, worst by {worst}",
            mine.data.len()
        ))
    }
}

/// Fails the gate if this build cannot reach the sandboxed image decoder.
///
/// `CCITTFaxDecode`, `JBIG2Decode` and `JPXDecode` are decoded by a separate program, and Cargo
/// does not build another package's binaries when it tests this one (trap 10). A build without
/// it draws every other image and none of those three, so what follows would be a measurement of
/// the build rather than of the tree — which is exactly what moved the accessibility census's
/// ratchet by nine elements while four rounds read the difference as something else
/// (ADR 0557, trap 16).
#[expect(
    clippy::panic,
    reason = "a gate that cannot decode the images it is measuring must stop rather than \
              print a number about a different program"
)]
fn require_the_sandbox() {
    if let Err(error) = pdf_model::image::sandboxed_decoder() {
        panic!(
            "the sandboxed image decoder is not available, so the counts below would be \
             wrong: {error}"
        );
    }
}

/// Every corpus JPEG 2000 codestream decodes to exactly what `OpenJPEG` decodes it to.
#[test]
fn every_corpus_codestream_decodes_to_the_reference_softwares_samples() {
    require_the_sandbox();
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    if !have_openjpeg() {
        println!("skipped: opj_decompress is not installed");
        return;
    }

    let mut identical = Vec::new();
    let mut differing = Vec::new();
    let mut incomparable = Vec::new();
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        for (number, codestream) in codestreams(&document) {
            match compare(&codestream, &name, number) {
                Verdict::Identical(entry) => identical.push(entry),
                Verdict::Differing(entry) => differing.push(entry),
                Verdict::Incomparable(entry) => incomparable.push(entry),
            }
        }
    }

    println!(
        "{} codestreams byte-identical to OpenJPEG's decode:",
        identical.len()
    );
    for entry in &identical {
        println!("  {entry}");
    }
    println!("{} differing:", differing.len());
    for entry in &differing {
        println!("  {entry}");
    }
    println!("{} not comparable:", incomparable.len());
    for entry in &incomparable {
        println!("  {entry}");
    }

    let named: Vec<&str> = differing
        .iter()
        .map(|entry| entry.split(':').next().unwrap_or(entry))
        .collect();
    assert_eq!(
        named, DIFFERS_FROM_THE_REFERENCE_SOFTWARE,
        "held by name in both directions: a codestream leaving this list has been fixed, and one          arriving in it used to agree"
    );
    assert_eq!(
        identical.len(),
        IDENTICAL,
        "codestreams matching `OpenJPEG` sample for sample: {identical:?}"
    );
    assert_eq!(
        incomparable.len(),
        NOT_COMPARABLE,
        "codestreams neither side puts in the same shape: {incomparable:?}"
    );
}
