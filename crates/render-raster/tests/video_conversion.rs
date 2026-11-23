//! The device's video conversion against the processor's, per pixel (RFC 0015 section 6, round 4;
//! ADR 3529).
//!
//! Both compute `pdf_render::video`'s formula from the same integers — the luma sample and the
//! chroma interpolated in sixteenths, which is exact — and differ only in the arithmetic after:
//! the processor in `f64`, the device in `f32`. So the tolerance is a claim this file checks rather
//! than a margin it grants: **a channel may differ by one level, and only where the exact value
//! lies within a thousandth of a level of a half**, the one place an `f32` rounding error of about
//! 255 × 2⁻²⁴ can carry a value across the rounding boundary. Every other channel is equal.

#![expect(
    clippy::panic,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    reason = "test code: an explanatory panic is the intended failure mode, and its values are \
              levels and pixel counts"
)]

#[path = "../../../tools/conformance/test-support/gpu_lock.rs"]
mod gpu_lock;
#[path = "../../../tools/conformance/test-support/gpu_lock_hook.rs"]
mod gpu_lock_hook;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use pdf_media_worker::video::{Session, access_units};
use pdf_render::video::{Conversion, Planes, Signal, chroma_sixteenths};
use render_raster::QuorraRasterizer;

fn device() -> QuorraRasterizer {
    gpu_lock_hook::install();
    QuorraRasterizer::new_headless()
        .unwrap_or_else(|error| panic!("no adapter for raster: {error} — these tests do not skip"))
}

/// Planes of `width` by `height` whose every sample differs from its neighbours, from `seed`.
fn planes_of(width: u32, height: u32, seed: u32) -> [Vec<u8>; 3] {
    let chroma = (width.div_ceil(2) * height.div_ceil(2)) as usize;
    let mut state = seed.wrapping_mul(2_654_435_761).wrapping_add(1);
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        (state >> 24) as u8
    };
    [
        (0..width * height).map(|_| next()).collect(),
        (0..chroma).map(|_| next()).collect(),
        (0..chroma).map(|_| next()).collect(),
    ]
}

/// What a comparison found: channels compared, and those one level apart at a tie.
#[derive(Default, Debug)]
struct Compared {
    channels: u64,
    at_ties: u64,
}

/// The device's raster against the processor's, under the tolerance this file states.
fn compare(
    planes: &Planes<'_>,
    conversion: &Conversion,
    device: &[u8],
    oracle: &[u8],
    what: &str,
) -> Compared {
    assert_eq!(device.len(), oracle.len(), "{what}: the rasters' sizes");
    let affine = conversion.affine();
    let (chroma_width, chroma_height) = planes.chroma_size();
    let mut compared = Compared::default();
    for (index, (got, want)) in device
        .as_chunks::<4>()
        .0
        .iter()
        .zip(oracle.as_chunks::<4>().0)
        .enumerate()
    {
        let (x, y) = (index as u32 % planes.width, index as u32 / planes.width);
        let luma = f64::from(planes.y[index]);
        let blue = f64::from(chroma_sixteenths(
            planes.cb,
            chroma_width,
            chroma_height,
            conversion,
            x,
            y,
        )) - 2048.0;
        let red = f64::from(chroma_sixteenths(
            planes.cr,
            chroma_width,
            chroma_height,
            conversion,
            x,
            y,
        )) - 2048.0;
        for channel in 0..3 {
            compared.channels += 1;
            if got[channel] == want[channel] {
                continue;
            }
            let exact = affine.luma * (luma - affine.luma_offset)
                + affine.cb[channel] * blue
                + affine.cr[channel] * red;
            let from_tie = (exact - exact.floor() - 0.5).abs();
            assert!(
                got[channel].abs_diff(want[channel]) == 1 && from_tie < 1e-3,
                "{what}: pixel ({x}, {y}) channel {channel}: device {} oracle {} exact {exact}",
                got[channel],
                want[channel]
            );
            compared.at_ties += 1;
        }
        assert_eq!(got[3], 255, "{what}: pixel ({x}, {y}) is opaque");
    }
    compared
}

fn both(
    device: &mut QuorraRasterizer,
    planes: &Planes<'_>,
    conversion: &Conversion,
    what: &str,
) -> Compared {
    let on_device = device
        .convert_picture(planes, conversion)
        .unwrap_or_else(|error| panic!("{what}: the device refused: {error}"));
    let oracle = render_cpu::video::convert_picture(planes, conversion)
        .unwrap_or_else(|error| panic!("{what}: the oracle refused: {error}"));
    compare(planes, conversion, &on_device.data, &oracle.data, what)
}

/// Hand-built planes of an odd size, so that the last chroma column and row are held, through
/// every siting, both ranges and each matrix the code draws, and the unspecified one's choice.
#[test]
fn hand_built_planes_match_the_oracle() {
    let mut device = device();
    let (width, height) = (333, 187);
    let mut total = Compared::default();
    for (seed, matrix) in [1u8, 4, 5, 7, 9, 2].into_iter().enumerate() {
        for full_range in [false, true] {
            for siting in 0..=5 {
                let [y, cb, cr] = planes_of(width, height, seed as u32 * 12 + u32::from(siting));
                let planes = Planes {
                    width,
                    height,
                    y: &y,
                    cb: &cb,
                    cr: &cr,
                };
                let signal = Signal {
                    full_range,
                    matrix_coefficients: matrix,
                    chroma_sample_loc_type: siting,
                    ..Signal::default()
                };
                let conversion = Conversion::for_signal(signal, height).expect("drawn");
                let what = format!("matrix {matrix}, full range {full_range}, siting {siting}");
                let compared = both(&mut device, &planes, &conversion, &what);
                total.channels += compared.channels;
                total.at_ties += compared.at_ties;
            }
        }
    }
    println!(
        "{}: {} channels, {} one level apart at a tie",
        device.adapter_description(),
        total.channels,
        total.at_ties
    );
}

/// The device refuses planes that are not their size's before it uploads anything.
#[test]
fn planes_of_the_wrong_size_are_refused_on_the_device_too() {
    let mut device = device();
    let planes = Planes {
        width: 4,
        height: 4,
        y: &[0; 16],
        cb: &[0; 4],
        cr: &[0; 3],
    };
    let conversion = Conversion::for_signal(Signal::default(), 4).expect("drawn");
    assert!(device.convert_picture(&planes, &conversion).is_err());
}

/// The conformance streams' pictures (`tools/fetch-h264-conformance.sh`, ADR 3527),
/// the first ten of each stream the worker's decoder reads, each by its own signal, on the device
/// against the oracle. `PDF_MEDIA_H264_STREAMS` names the streams' directory.
#[test]
// not a gate: it reads streams the tree does not hold (`tools/fetch-h264-conformance.sh` fetches
// them pinned by hash, ADR 3527) and prints counts a person reads; the same comparison on
// hand-built planes, `hand_built_planes_match_the_oracle`, runs in every nextest of this crate.
#[ignore = "not a gate: needs the fetched H.264.1 streams"]
fn the_conformance_streams_pictures_match_the_oracle() {
    let root = PathBuf::from(std::env::var("PDF_MEDIA_H264_STREAMS").expect("the streams"));
    let mut directories: Vec<PathBuf> = std::fs::read_dir(&root)
        .expect("the streams' directory")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_dir())
        .collect();
    directories.sort();
    let mut device = device();
    let (mut streams, mut pictures, mut total) = (0, 0, Compared::default());
    for directory in &directories {
        let Some(stream) = std::fs::read_dir(directory).ok().and_then(|entries| {
            entries
                .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                .find(|path| {
                    path.extension().is_some_and(|extension| {
                        matches!(
                            extension.to_ascii_lowercase().to_str(),
                            Some("264" | "h264" | "jsv" | "jvt" | "avc" | "26l" | "bit")
                        )
                    })
                })
        }) else {
            continue;
        };
        let bytes = std::fs::read(&stream).expect("the stream");
        let mut session = Session::elementary();
        let mut decoded = Vec::new();
        for unit in access_units(&bytes) {
            match session.access_unit(unit) {
                Ok(out) => decoded.extend(out),
                Err(_) => break,
            }
            if decoded.len() >= 10 {
                break;
            }
        }
        if decoded.len() < 10 {
            decoded.extend(session.finish());
        }
        if decoded.is_empty() {
            continue;
        }
        streams += 1;
        let name = directory
            .file_name()
            .map(|name| name.to_string_lossy().into_owned());
        for (index, picture) in decoded.iter().take(10).enumerate() {
            let planes = Planes {
                width: picture.width as u32,
                height: picture.height as u32,
                y: &picture.y,
                cb: &picture.u,
                cr: &picture.v,
            };
            let signal = Signal {
                full_range: picture.signal.full_range,
                colour_primaries: picture.signal.colour_primaries,
                transfer_characteristics: picture.signal.transfer_characteristics,
                matrix_coefficients: picture.signal.matrix_coefficients,
                chroma_sample_loc_type: picture.signal.chroma_sample_loc_type,
            };
            let conversion = match Conversion::for_signal(signal, planes.height) {
                Ok(conversion) => conversion,
                Err(refused) => {
                    println!("{name:?}: refused, {refused}");
                    break;
                }
            };
            let compared = both(
                &mut device,
                &planes,
                &conversion,
                &format!("{name:?} picture {index}"),
            );
            pictures += 1;
            total.channels += compared.channels;
            total.at_ties += compared.at_ties;
        }
        if index_signal(&decoded) != Signal::default() {
            println!("{name:?}: {:?}", index_signal(&decoded));
        }
    }
    println!(
        "{}: {streams} streams, {pictures} pictures, {} channels, {} one level apart at a tie",
        device.adapter_description(),
        total.channels,
        total.at_ties
    );
    assert!(pictures > 0, "the walk compared nothing");
}

/// The first picture's signal, as the renderer's type.
fn index_signal(decoded: &[pdf_media_worker::video::Picture]) -> Signal {
    decoded
        .first()
        .map_or_else(Signal::default, |picture| Signal {
            full_range: picture.signal.full_range,
            colour_primaries: picture.signal.colour_primaries,
            transfer_characteristics: picture.signal.transfer_characteristics,
            matrix_coefficients: picture.signal.matrix_coefficients,
            chroma_sample_loc_type: picture.signal.chroma_sample_loc_type,
        })
}

/// One 1080-line picture converted forty times on the device and on the processor,
/// after five unmeasured, with each lane's median. Run it alone, under the lock the device holds.
#[test]
// not a gate: a measurement whose figures ADR 3529 records, read by a person; a time is no
// verdict a test can hold on a shared machine.
#[ignore = "not a gate: a measurement"]
fn a_1080_line_picture_s_conversion_is_timed_on_both_lanes() {
    let mut device = device();
    let (width, height) = (1920, 1080);
    let [y, cb, cr] = planes_of(width, height, 7);
    let planes = Planes {
        width,
        height,
        y: &y,
        cb: &cb,
        cr: &cr,
    };
    let signal = Signal {
        matrix_coefficients: 1,
        ..Signal::default()
    };
    let conversion = Conversion::for_signal(signal, height).expect("drawn");
    let median = |mut samples: Vec<Duration>| {
        samples.sort();
        samples[samples.len() / 2]
    };
    let (mut total, mut on_device, mut read_back, mut processor) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for round in 0..45 {
        let started = Instant::now();
        device
            .convert_picture(&planes, &conversion)
            .expect("converted");
        let elapsed = started.elapsed();
        let cost = device.last_video_cost();
        let started = Instant::now();
        render_cpu::video::convert_picture(&planes, &conversion).expect("converted");
        let cpu = started.elapsed();
        if round >= 5 {
            total.push(elapsed);
            on_device.push(cost.on_device);
            read_back.push(cost.read_back);
            processor.push(cpu);
        }
    }
    println!(
        "{}: 1920x1080, median of 40 — device {:?} (on the device {:?}, read back {:?}), \
         processor {:?} at {} rayon threads",
        device.adapter_description(),
        median(total),
        median(on_device),
        median(read_back),
        median(processor),
        std::env::var("RAYON_NUM_THREADS").unwrap_or_else(|_| "the default".to_owned())
    );
}
