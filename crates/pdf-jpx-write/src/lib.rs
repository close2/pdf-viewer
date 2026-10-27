//! Lossless JPEG 2000 writing, ISO/IEC 15444-1, for samples ISO 32000-2 gives no other home.
//!
//! One responsibility: turn integer samples into a JP2 file whose decode is those integers
//! exactly. It exists for §12.5.6.23's redaction, which destroys "that portion of the image
//! data" and must carry the rest unchanged: a `JPXDecode` component may be "between 1 to 38
//! inclusive" bits (§7.4.9), and Table 87 gives every other filter's samples at most sixteen,
//! so a cleared component deeper than that can only be written back as JPEG 2000 (ADR 2470).
//!
//! # The smallest conforming lossless encoder
//!
//! The codestream (`codestream`) has one tile, zero decomposition levels under the reversible
//! 5-3 filter, no quantisation, no component transformation and one quality layer — so the one
//! band is the samples themselves less their DC level shift, and the only coding left is Annex
//! D's bit modelling (`tier1`) through Annex C's MQ coder (`mq`) and Annex B's packets
//! (`tier2`). Nothing in it is lossy and nothing is optional, and it is no smaller than a
//! decoder must read anyway. The JP2 boxes around it are `file`'s.
//!
//! # How far it is proven, and why the bound is where it is
//!
//! Depths 1 to [`MAX_DEPTH`]. The proof is the round trip — every depth, read back sample for
//! sample by the tree's own decoder in `pdf-sandbox` — and that decoder carries a sample in an
//! `f32`, whose integers are exact to 2^24 and no further; a deeper component would be written
//! by a construction no reader here can check, so it is refused by name.
//!
//! # Where the texts are cited
//!
//! ISO/IEC 15444-1 is cited by clause and paraphrased (ADR 0187's discipline); only ISO 32000-2
//! is quoted.

#![forbid(unsafe_code)]

mod codestream;
mod file;
mod mq;
mod tier1;
mod tier2;

use std::fmt;

/// The deepest component this writer takes: the depth whose every integer the tree's decoder
/// reads back exactly, an `f32`'s 24 significant bits.
pub const MAX_DEPTH: u8 = 24;

/// The most components a codestream states: A.5.1's `Csiz` is at most 16 384.
pub const MAX_COMPONENTS: usize = 16_384;

/// One component's samples: its depth, and its integers row by row on the image's grid.
#[derive(Debug, Clone, Copy)]
pub struct Plane<'a> {
    /// Bits per sample, 1 to [`MAX_DEPTH`].
    pub depth: u8,
    /// Width × height unsigned integers, each below 2^`depth`.
    pub samples: &'a [u32],
}

/// The colour space a JP2 file's colour specification box enumerates (ISO/IEC 15444-1
/// I.5.3.3). ISO 32000-2 §7.4.9 has a stated `/ColorSpace` override it, and a file of this
/// writer's is always described by one, so the box is what JP2 requires and not what a reader
/// of the PDF uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Colour {
    /// Enumerated 17, one colour channel.
    Greyscale,
    /// Enumerated 16, three colour channels.
    Srgb,
}

impl Colour {
    /// The colour channels the space has.
    #[must_use]
    pub fn channels(self) -> usize {
        match self {
            Self::Greyscale => 1,
            Self::Srgb => 3,
        }
    }
}

/// How the last channel of a JP2 file with an opacity channel is defined (I.5.3.6), which ISO
/// 32000-2 Table 87's `/SMaskInData` 1 and 2 read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opacity {
    /// Opacity the colour channels were not multiplied by.
    Plain,
    /// Opacity already multiplied into the colour channels.
    Premultiplied,
}

/// Why samples were not written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The grid has no sample: A.5.1 has `Xsiz` and `Ysiz` greater than the offsets, here 0.
    EmptyGrid,
    /// No component, or more than `Csiz` can state.
    Components(usize),
    /// The colour channels do not number what the colour space has.
    ColourChannels {
        /// The colour space.
        colour: Colour,
        /// The colour planes given.
        planes: usize,
    },
    /// A depth outside 1 to [`MAX_DEPTH`].
    Depth {
        /// The component.
        plane: usize,
        /// Its depth.
        depth: u8,
    },
    /// A plane whose sample count is not the grid's.
    Samples {
        /// The component.
        plane: usize,
        /// The grid's sample count.
        expected: usize,
        /// The plane's.
        found: usize,
    },
    /// A sample at or above 2^depth.
    Sample {
        /// The component.
        plane: usize,
        /// The sample's index.
        index: usize,
        /// Its value.
        value: u32,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyGrid => write!(f, "a JPEG 2000 image needs at least one sample"),
            Self::Components(count) => write!(
                f,
                "{count} components, where a codestream states 1 to {MAX_COMPONENTS}"
            ),
            Self::ColourChannels { colour, planes } => write!(
                f,
                "{planes} colour planes for a space of {} channels",
                colour.channels()
            ),
            Self::Depth { plane, depth } => write!(
                f,
                "component {plane} is {depth} bits, where this writer takes 1 to {MAX_DEPTH}"
            ),
            Self::Samples {
                plane,
                expected,
                found,
            } => write!(
                f,
                "component {plane} has {found} samples on a grid of {expected}"
            ),
            Self::Sample {
                plane,
                index,
                value,
            } => write!(
                f,
                "sample {index} of component {plane} is {value}, past its depth"
            ),
        }
    }
}

impl std::error::Error for Error {}

/// Checks `planes` against a `width` × `height` grid.
fn validate(width: u32, height: u32, planes: &[Plane<'_>]) -> Result<(), Error> {
    if width == 0 || height == 0 {
        return Err(Error::EmptyGrid);
    }
    if planes.is_empty() || planes.len() > MAX_COMPONENTS {
        return Err(Error::Components(planes.len()));
    }
    let expected =
        usize::try_from(u64::from(width).saturating_mul(u64::from(height))).unwrap_or(usize::MAX);
    for (index, plane) in planes.iter().enumerate() {
        if !(1..=MAX_DEPTH).contains(&plane.depth) {
            return Err(Error::Depth {
                plane: index,
                depth: plane.depth,
            });
        }
        if plane.samples.len() != expected {
            return Err(Error::Samples {
                plane: index,
                expected,
                found: plane.samples.len(),
            });
        }
        let limit = 1u32 << plane.depth;
        if let Some((at, value)) = plane
            .samples
            .iter()
            .enumerate()
            .find(|(_, value)| **value >= limit)
        {
            return Err(Error::Sample {
                plane: index,
                index: at,
                value: *value,
            });
        }
    }
    Ok(())
}

/// A raw ISO/IEC 15444-1 codestream for `planes` on a `width` × `height` grid, losslessly.
///
/// # Errors
///
/// [`Error`] where the grid is empty or a plane does not fit it or its depth.
pub fn codestream(width: u32, height: u32, planes: &[Plane<'_>]) -> Result<Vec<u8>, Error> {
    validate(width, height, planes)?;
    Ok(codestream::write(width, height, planes))
}

/// A JP2 file (ISO/IEC 15444-1 Annex I) holding `colour_planes` in `colour`, and an opacity
/// channel after them where `opacity` gives one, losslessly.
///
/// # Errors
///
/// [`Error`] where the grid is empty, the colour planes do not number the space's channels, or
/// a plane does not fit the grid or its depth.
pub fn jp2(
    (width, height): (u32, u32),
    colour: Colour,
    colour_planes: &[Plane<'_>],
    opacity: Option<(Plane<'_>, Opacity)>,
) -> Result<Vec<u8>, Error> {
    if colour_planes.len() != colour.channels() {
        return Err(Error::ColourChannels {
            colour,
            planes: colour_planes.len(),
        });
    }
    let mut planes = colour_planes.to_vec();
    if let Some((plane, _)) = opacity {
        planes.push(plane);
    }
    validate(width, height, &planes)?;
    let stream = codestream::write(width, height, &planes);
    Ok(file::write(
        (width, height),
        &planes,
        colour,
        opacity.map(|(_, kind)| kind),
        &stream,
    ))
}
