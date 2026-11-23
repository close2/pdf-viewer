//! Fuzzes the confined media worker's H.264 path: an MP4 file's video track read by
//! `pdf_media_worker::mp4::movie_video_track` and its samples decoded by
//! `pdf_media_worker::video::Session`, or the input taken as an Annex B elementary stream (ADR 3526).
//!
//! The input is bytes an untrusted document embedded. The first byte chooses the reading; beyond
//! never panicking — overflow checks stay on in this profile — what comes back is checked against
//! the bounds the host relies on:
//!
//! - **every access unit lies inside the file**, and no more than [`MAX_ACCESS_UNITS`];
//! - **no more parameter-set bytes than [`MAX_PARAMETER_SET_BYTES`]**;
//! - **every picture is at most [`MAX_PICTURE_PIXELS`]**, its planes exactly its size's.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pdf_media_worker::mp4::{MAX_ACCESS_UNITS, MAX_PARAMETER_SET_BYTES, movie_video_track};
use pdf_media_worker::video::{MAX_PICTURE_PIXELS, Picture, Session, access_units};

/// The most access units one input decodes: a fuzzer's input is small, and a stream of empty
/// units is still bounded by this rather than by the input's length.
const MOST_UNITS: usize = 64;

fn check(picture: &Picture) {
    let luma = picture.width.saturating_mul(picture.height);
    let chroma = picture
        .width
        .div_ceil(2)
        .saturating_mul(picture.height.div_ceil(2));
    assert!(luma <= MAX_PICTURE_PIXELS, "a picture past the bound");
    assert_eq!(picture.y.len(), luma, "a luma plane that is not its size");
    assert_eq!(
        picture.u.len(),
        chroma,
        "a chroma plane that is not its size"
    );
    assert_eq!(
        picture.v.len(),
        chroma,
        "a chroma plane that is not its size"
    );
}

fuzz_target!(|input: &[u8]| {
    let Some((&mode, data)) = input.split_first() else {
        return;
    };
    if mode & 1 == 0 {
        let mut session = Session::elementary();
        for unit in access_units(data).into_iter().take(MOST_UNITS) {
            let Ok(pictures) = session.access_unit(unit) else {
                return;
            };
            pictures.iter().for_each(check);
        }
        session.finish().iter().for_each(check);
        return;
    }
    let length = u64::try_from(data.len()).unwrap_or(u64::MAX);
    let Ok(track) = movie_video_track(data, length) else {
        return;
    };
    assert!(
        track.access_units.len() <= MAX_ACCESS_UNITS,
        "access units past the bound"
    );
    assert!(
        track.access_units.iter().all(|unit| unit
            .offset
            .checked_add(u64::from(unit.size))
            .is_some_and(|end| end <= length)),
        "an access unit past the file's end"
    );
    let held: usize = track.parameter_sets.iter().map(Vec::len).sum();
    assert!(
        held <= MAX_PARAMETER_SET_BYTES,
        "parameter sets past the bound"
    );
    let Ok(mut session) = Session::for_track(track.length_size, &track.parameter_sets) else {
        return;
    };
    for unit in track.access_units.iter().take(MOST_UNITS) {
        let start = usize::try_from(unit.offset).unwrap_or(usize::MAX);
        let end = start.saturating_add(usize::try_from(unit.size).unwrap_or(usize::MAX));
        let Some(sample) = data.get(start..end) else {
            return;
        };
        let Ok(pictures) = session.sample(sample) else {
            return;
        };
        pictures.iter().for_each(check);
    }
    session.finish().iter().for_each(check);
});
