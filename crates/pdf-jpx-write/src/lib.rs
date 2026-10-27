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
//! # Signed components and the colour of a file
//!
//! A component's `Ssiz` marks it signed (ISO/IEC 15444-1 Table A-11), and G.1.2 shifts only an
//! unsigned component's DC level back, so a signed component's reconstructed value is negative
//! below zero. ISO 32000-2 §8.9.5.2 has an image's components "initially decomposed into
//! integers in the domain 0 to 2 n -1" and states no map from a signed value into that domain,
//! so the integers [`Plane`] carries are the domain's, the way this tree's decoder delivers them:
//! a signed component's value `s` is the integer `s + 2^(depth − 1)`, and a plane flagged signed
//! has only its `Ssiz` and `BPC` sign bits set — the coded bytes are an unsigned plane's.
//!
//! A file whose colour JP2 cannot enumerate — four channels of CMYK, or the colour specification
//! a producer's own data stated — is a JPX file ([`file()`] with [`Space::Cmyk`] or
//! [`Space::Stated`]), the one container that holds it.
//!
//! # Where the texts are cited
//!
//! ISO/IEC 15444-1 and ITU-T T.801 are cited by clause and paraphrased (ADR 0187's discipline);
//! only ISO 32000-2 is quoted.

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

/// One colour specification box as a source file stated it (ITU-T T.801 M.11.7.2), written
/// unchanged except for the approximation field's range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColourBox<'a> {
    /// `METH` (T.801 Table M.22): 1 enumerated, 2 restricted ICC, 3 any ICC, 4 vendor colour.
    pub method: u8,
    /// `PREC`, the precedence.
    pub precedence: i8,
    /// `APPROX` (T.801 Table M.23): 1 to 4. A JPX file may not state 0, the value a JP2 file sets
    /// there and its readers ignore (ISO/IEC 15444-1 I.5.3.3).
    pub approximation: u8,
    /// `METHDAT`, which may not be empty: for method 1 the four-byte `EnumCS` and its `EP`
    /// fields, for 2 and 3 the profile, for 4 the 16-byte UUID and its parameters.
    pub data: &'a [u8],
}

/// The colour a file states: what its colour specification box holds, and so whether the
/// container is JP2 or JPX.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Space<'a> {
    /// JP2 enumerated 17, one colour channel.
    Greyscale,
    /// JP2 enumerated 16, three colour channels.
    Srgb,
    /// JPX enumerated 12 (T.801 Table M.25), four colour channels.
    Cmyk,
    /// A colour specification box of the producer's, for any number of colour channels.
    Stated(ColourBox<'a>),
}

impl Space<'_> {
    /// The colour channels the space requires, or none where the box states the space.
    #[must_use]
    pub fn channels(self) -> Option<usize> {
        match self {
            Self::Greyscale => Some(1),
            Self::Srgb => Some(3),
            Self::Cmyk => Some(4),
            Self::Stated(_) => None,
        }
    }

    /// Whether the container is JPX rather than JP2.
    pub(crate) fn is_jpx(self) -> bool {
        matches!(self, Self::Cmyk | Self::Stated(_))
    }

    /// The `colr` box's contents (I.5.3.3, M.11.7.2).
    pub(crate) fn colour_box(self) -> Vec<u8> {
        let enumerated = |code: u32, approximation: u8| {
            let mut out = vec![1, 0, approximation];
            out.extend_from_slice(&code.to_be_bytes());
            out
        };
        match self {
            // I.5.3.3 sets APPROX to zero in a JP2 file and has a reader ignore it.
            Self::Greyscale => enumerated(17, 0),
            Self::Srgb => enumerated(16, 0),
            Self::Cmyk => enumerated(12, 1),
            Self::Stated(stated) => {
                let mut out = vec![
                    stated.method,
                    stated.precedence.to_be_bytes()[0],
                    stated.approximation,
                ];
                out.extend_from_slice(stated.data);
                out
            }
        }
    }

    /// The T.801 Table M.14 feature that names the space, where the table has one.
    pub(crate) fn feature(self) -> Option<u16> {
        match self {
            Self::Greyscale => Some(46),
            Self::Srgb => Some(45),
            Self::Cmyk => Some(55),
            Self::Stated(stated) => match (stated.method, stated.data.first_chunk::<4>()) {
                (1, Some(code)) => match u32::from_be_bytes(*code) {
                    12 => Some(55),
                    16 => Some(45),
                    17 => Some(46),
                    _ => None,
                },
                (2, _) => Some(43),
                (3, _) => Some(44),
                _ => None,
            },
        }
    }
}

impl From<Colour> for Space<'_> {
    fn from(colour: Colour) -> Self {
        match colour {
            Colour::Greyscale => Self::Greyscale,
            Colour::Srgb => Self::Srgb,
        }
    }
}

/// Whether plane `index` is flagged signed in `signed`, which is empty where none is.
pub(crate) fn is_signed(signed: &[bool], index: usize) -> bool {
    signed.get(index).copied().unwrap_or(false)
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
    /// The colour channels do not number what the space has (`Space::Cmyk` takes four).
    SpaceChannels {
        /// The channels the space requires.
        expected: usize,
        /// The colour planes given.
        planes: usize,
    },
    /// A colour specification box whose method, approximation or contents a JPX file cannot
    /// state (Tables M.22 and M.23: methods 1 to 4, approximations 1 to 4, and METHDAT of at
    /// least the four bytes of `EnumCS` or the 16 of a vendor UUID where the method has them).
    ColourBox,
    /// Sign flags that number neither the planes nor none.
    Signed {
        /// The flags given.
        flags: usize,
        /// The planes, opacity included.
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
            Self::SpaceChannels { expected, planes } => {
                write!(
                    f,
                    "{planes} colour planes for a space of {expected} channels"
                )
            }
            Self::ColourBox => write!(
                f,
                "a colour specification box a JPX file cannot state: METH 1 to 4, APPROX 1 to 4 \
                 and a METHDAT of the length its method has"
            ),
            Self::Signed { flags, planes } => {
                write!(f, "{flags} sign flags for {planes} planes")
            }
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

/// Checks `planes` against a `width` × `height` grid, and `signed` against the planes.
fn validate(width: u32, height: u32, planes: &[Plane<'_>], signed: &[bool]) -> Result<(), Error> {
    if width == 0 || height == 0 {
        return Err(Error::EmptyGrid);
    }
    if planes.is_empty() || planes.len() > MAX_COMPONENTS {
        return Err(Error::Components(planes.len()));
    }
    if !signed.is_empty() && signed.len() != planes.len() {
        return Err(Error::Signed {
            flags: signed.len(),
            planes: planes.len(),
        });
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
    validate(width, height, planes, &[])?;
    Ok(codestream::write(width, height, planes, &[]))
}

/// A JP2 file (ISO/IEC 15444-1 Annex I) holding `colour_planes` in `colour`, and an opacity
/// channel after them where `opacity` gives one, losslessly.
///
/// # Errors
///
/// [`Error`] where the grid is empty, the colour planes do not number the space's channels, or
/// a plane does not fit the grid or its depth.
pub fn jp2(
    grid: (u32, u32),
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
    file(grid, colour.into(), colour_planes, opacity, &[])
}

/// A file for `colour_planes` in `space`, and an opacity channel after them where `opacity`
/// gives one, losslessly: JP2 (Annex I) where `space` is one JP2 enumerates, and JPX (ITU-T
/// T.801 Annex M) otherwise.
///
/// `signed` flags the planes whose `Ssiz` states a signed component, colour planes first and
/// the opacity plane last, or is empty where every plane is unsigned. A flagged plane's
/// integers are still the domain's (the module documentation), so only the sign bits differ.
///
/// # Errors
///
/// [`Error`] where the grid is empty, the colour planes do not number the space's channels, a
/// plane does not fit the grid or its depth, the flags do not number the planes, or a stated
/// colour box is one a JPX file cannot hold.
pub fn file(
    (width, height): (u32, u32),
    space: Space<'_>,
    colour_planes: &[Plane<'_>],
    opacity: Option<(Plane<'_>, Opacity)>,
    signed: &[bool],
) -> Result<Vec<u8>, Error> {
    match space.channels() {
        Some(expected) if expected != colour_planes.len() => {
            return Err(Error::SpaceChannels {
                expected,
                planes: colour_planes.len(),
            });
        }
        None if colour_planes.is_empty() => {
            return Err(Error::Components(0));
        }
        _ => {}
    }
    if let Space::Stated(stated) = space {
        let data_needed = match stated.method {
            1 => 4,
            2 | 3 => 1,
            4 => 16,
            _ => return Err(Error::ColourBox),
        };
        if !(1..=4).contains(&stated.approximation) || stated.data.len() < data_needed {
            return Err(Error::ColourBox);
        }
    }
    let mut planes = colour_planes.to_vec();
    if let Some((plane, _)) = opacity {
        planes.push(plane);
    }
    validate(width, height, &planes, signed)?;
    let stream = codestream::write(width, height, &planes, signed);
    Ok(file::write(
        (width, height),
        &planes,
        signed,
        space,
        opacity.map(|(_, kind)| kind),
        &stream,
    ))
}
