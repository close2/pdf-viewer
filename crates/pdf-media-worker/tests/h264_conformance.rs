//! ITU-T H.264.1's conformance bitstreams, decoded by the worker's H.264 session and compared with
//! the reference reconstruction each ships beside it (ADR 3527).
//!
//! **The criterion is bit-exactness**, and it is the standard's rather than this test's: H.264's
//! decoding process is specified to the sample, so a conforming decoder's output pictures equal the
//! reference decoder's exactly, and H.264.1 states its bitstreams' conformance against their
//! reconstructed pictures. Neither ITU-T H.264 nor H.264.1 is held in `doc/`, so the clause is not
//! quoted here; the verdict EXACT is every reference frame reproduced byte for byte, in output order,
//! and no frame extra.
//!
//! An interlaced stream is reported as handed off — the worker's own refusal by name — and any other
//! refusal in the decoder's words. The progressive streams this walk finds EXACT are the subset the
//! worker is held to; the others are reported, not asserted, because the decoder is a dependency
//! whose verdicts RFC 0012 section 3.2 measured and this tree does not patch.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "an ignored walk over files a person fetched: a missing one fails loudly, and its \
              sizes are a few megabytes"
)]

use std::path::{Path, PathBuf};

use pdf_media_worker::video::{Picture, Session, VideoError, access_units};

/// The extensions H.264.1's zips give their bitstreams.
const STREAM_EXTENSIONS: [&str; 7] = ["264", "jsv", "26l", "avc", "jvt", "h264", "bit"];

/// The extensions H.264.1's zips give their reference reconstructions: raw 8-bit 4:2:0 planes in
/// output order, named for the picture format by some contributors.
const REFERENCE_EXTENSIONS: [&str; 3] = ["yuv", "qcif", "cif"];

#[test]
// not a gate: it reads streams the tree does not hold (`tools/fetch-h264-conformance.sh` fetches
// them pinned by hash), and prints a verdict a person reads; the decoder's verdicts are measured
// here, not changed.
#[ignore = "reads ITU-T H.264.1's streams from PDF_MEDIA_H264_STREAMS, which are not committed"]
fn the_conformance_streams_decode_against_their_references() {
    let streams = PathBuf::from(std::env::var("PDF_MEDIA_H264_STREAMS").expect("the streams"));
    let mut names: Vec<PathBuf> = std::fs::read_dir(&streams)
        .expect("the streams' directory")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_dir())
        .collect();
    names.sort();
    let (mut exact, mut handed, mut other) = (0usize, 0usize, 0usize);
    for directory in &names {
        let name = directory
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        let verdict = verdict(directory);
        match verdict.split_whitespace().next() {
            Some("EXACT") => exact = exact.saturating_add(1),
            Some("HANDED-OFF") => handed = handed.saturating_add(1),
            _ => other = other.saturating_add(1),
        }
        println!("{name}: {verdict}");
    }
    println!(
        "{} streams: {exact} EXACT, {handed} interlaced and handed off, {other} otherwise",
        names.len()
    );
    assert!(
        exact > 0,
        "no stream decoded exactly: the walk read nothing"
    );
}

/// One stream's verdict.
fn verdict(directory: &Path) -> String {
    let files: Vec<PathBuf> = std::fs::read_dir(directory).map_or_else(
        |_| Vec::new(),
        |entries| {
            entries
                .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                .collect()
        },
    );
    let extension = |path: &Path| {
        path.extension().map_or_else(String::new, |extension| {
            extension.to_string_lossy().to_ascii_lowercase()
        })
    };
    let Some(stream) = files
        .iter()
        .find(|path| STREAM_EXTENSIONS.contains(&extension(path).as_str()))
    else {
        return "NO-STREAM in the zip this walk reads".to_owned();
    };
    let references: Vec<&PathBuf> = files
        .iter()
        .filter(|path| REFERENCE_EXTENSIONS.contains(&extension(path).as_str()))
        .collect();
    if references.is_empty() {
        return "NO-REFERENCE in the zip".to_owned();
    }
    let bytes = std::fs::read(stream).expect("the stream");
    let mut session = Session::elementary();
    let mut pictures: Vec<Picture> = Vec::new();
    for unit in access_units(&bytes) {
        match session.access_unit(unit) {
            Ok(out) => pictures.extend(out),
            Err(VideoError::Interlaced(what)) => return format!("HANDED-OFF ({what})"),
            Err(VideoError::Refused(sentence)) => {
                return format!("REFUSED after {} pictures: {sentence}", pictures.len());
            }
        }
    }
    pictures.extend(session.finish());
    let decoded: Vec<u8> = pictures
        .iter()
        .flat_map(|picture| [&picture.y, &picture.u, &picture.v])
        .flatten()
        .copied()
        .collect();
    let frame = pictures.first().map_or(1, |picture| {
        picture.y.len() + picture.u.len() + picture.v.len()
    });
    let mut nearest = String::new();
    for reference in references {
        let expected = std::fs::read(reference).expect("the reference");
        if expected == decoded {
            return format!("EXACT {} pictures", pictures.len());
        }
        let first = decoded
            .iter()
            .zip(&expected)
            .position(|(ours, theirs)| ours != theirs)
            .unwrap_or_else(|| decoded.len().min(expected.len()));
        nearest = format!(
            "DIFFERS from {}: {} pictures for {} reference frames, first difference in frame {}",
            reference
                .file_name()
                .map_or_else(String::new, |name| name.to_string_lossy().into_owned()),
            pictures.len(),
            expected.len() / frame.max(1),
            first / frame.max(1)
        );
    }
    nearest
}
