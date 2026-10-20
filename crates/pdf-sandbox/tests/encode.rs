//! The one encode, asked of the confined worker: what crosses, what comes back, and the budget
//! measured where it acts (ADR 2138).
//!
//! A test binary of its own because one of these sets the process-wide isolation policy, and the
//! confinement tests beside it read that policy from other threads.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::unwrap_used,
    reason = "test code over grids of a few thousand pixels a side and a frame header whose six \
              bytes are sliced out two lines above their indices; a test's unwrap fails as \
              loudly as an assertion and names the line"
)]

use pdf_sandbox::{
    DctComponents, DctParameters, Decoded, Isolation, Request, Sandbox, SandboxError,
};

/// A gradient of `channels` interleaved components.
fn ramp(width: u32, height: u32, channels: u32) -> Vec<u8> {
    (0..width * height * channels)
        .map(|index| u8::try_from(index % 251).unwrap())
        .collect()
}

fn rgb(width: u32, height: u32) -> DctParameters {
    DctParameters {
        width,
        height,
        components: DctComponents::Rgb,
        quality: 80,
    }
}

/// ISO/IEC 10918-1 B.2.2's frame header after SOF0: precision, lines, samples a line, components.
fn frame(jpeg: &[u8]) -> Option<(u8, u16, u16, u8)> {
    let at = jpeg.windows(2).position(|pair| pair == [0xFF, 0xC0])?;
    let header = jpeg.get(at + 4..at + 10)?;
    Some((
        header[0],
        u16::from_be_bytes([header[1], header[2]]),
        u16::from_be_bytes([header[3], header[4]]),
        header[5],
    ))
}

/// The worker answers an encode with a baseline frame stating the grid and the components it was
/// given — the parameters §7.4.8 says "shall be stored in the encoded data".
#[test]
fn the_confined_worker_encodes_a_frame_stating_its_grid() {
    let samples = ramp(120, 90, 3);
    let Decoded::Dct(jpeg) = Sandbox::shared()
        .decode(&Request::Dct {
            samples: &samples,
            parameters: rgb(120, 90),
        })
        .unwrap()
    else {
        panic!("an encode is answered with a JPEG");
    };
    assert_eq!(frame(&jpeg), Some((8, 90, 120, 3)));

    let grey = ramp(33, 17, 1);
    let jpeg = pdf_sandbox::encode_dct(
        &grey,
        DctParameters {
            components: DctComponents::Gray,
            ..rgb(33, 17)
        },
    )
    .unwrap();
    assert_eq!(frame(&jpeg), Some((8, 17, 33, 1)));
}

/// The isolation flag moves where the encode happens and nothing else.
#[test]
fn the_two_isolation_settings_encode_the_same_bytes() {
    let samples = ramp(64, 64, 3);
    let confined = pdf_sandbox::encode_dct(&samples, rgb(64, 64)).unwrap();
    pdf_sandbox::set_isolation(Isolation::InProcess);
    let here = pdf_sandbox::encode_dct(&samples, rgb(64, 64));
    pdf_sandbox::set_isolation(Isolation::Sandboxed);
    assert_eq!(here.unwrap(), confined);
}

/// A refusal is the encoder's sentence, and the worker that said it answers the next request.
#[test]
fn a_refused_encode_leaves_the_worker_answering() {
    let error = pdf_sandbox::encode_dct(&[0; 5], rgb(4, 4)).unwrap_err();
    assert!(
        matches!(&error, SandboxError::Undecodable { detail } if detail.contains("48 samples")),
        "{error}"
    );
    assert!(pdf_sandbox::encode_dct(&ramp(8, 8, 3), rgb(8, 8)).is_ok());
}

/// The budget, measured where it acts: the largest square one encode is given — 4729×4729 in
/// three components, 19 141 samples under 2^26 — encodes inside the worker's
/// gigabyte of address space, and a grid two pixels a row wider is past it and refused by name
/// before the encoder allocates.
#[test]
fn the_largest_grid_the_budget_admits_encodes_under_the_workers_ceiling() {
    let side = 4729;
    let samples = vec![0x80; side * side * 3];
    let side = u32::try_from(side).unwrap();
    // The confined worker named explicitly, so that the isolation test beside this one cannot move
    // the encode into this process under `cargo test`'s threads.
    let sandbox = Sandbox::shared();
    let Decoded::Dct(jpeg) = sandbox
        .decode(&Request::Dct {
            samples: &samples,
            parameters: rgb(side, side),
        })
        .unwrap()
    else {
        panic!("an encode is answered with a JPEG");
    };
    assert_eq!(frame(&jpeg).map(|frame| frame.1), Some(4729));
    assert!(sandbox.confinement().unwrap().is_enforced());

    let past = sandbox
        .decode(&Request::Dct {
            samples: &[],
            parameters: rgb(side + 2, side),
        })
        .unwrap_err();
    assert!(
        matches!(&past, SandboxError::Undecodable { detail } if detail.contains("past the")),
        "{past}"
    );
}
