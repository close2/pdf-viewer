//! Reading the headers of JPEG 2000 data, without decoding a sample.
//!
//! ISO 32000-2 §7.4.9 hands the description of a `JPXDecode` image over to another standard:
//!
//! > The JPEG 2000 specifications define two widely used formats, JP2 and JPX, for packaging
//! > the compressed image data. JP2 is a subset of JPX. These packagings contain all the
//! > information needed to properly interpret the image data, including the colour space, bits
//! > per component, and image dimensions.
//!
//! That packaging is ISO/IEC 15444-1:2000 Annex I — a sequence of *boxes*, each a length, a
//! four-character type and a payload (I.4) — and the compressed data itself is a *codestream*
//! whose first marker segment states the image's size and per-component precision (A.5.1). This
//! module reads the boxes and that one marker segment. Nothing here decodes, allocates a
//! raster, or looks at a single wavelet coefficient.
//!
//! # Why a reader that decodes nothing
//!
//! Decoding already exists and is somewhere else: `crate::image` sends the bytes to
//! [`pdf_sandbox`], which runs an untrusted decoder in a separate confined process and returns a
//! finished eight-bit raster. That is the right shape for *drawing* and the wrong shape for
//! every other question, because it answers only what a rasteriser needs. The raster reports a
//! component count and one of five colour meanings; it cannot report which colour specification
//! method a `colr` box states, how many such boxes there are, what each one's approximation
//! field says, or the declared precision of each component — all of which are facts the data
//! states plainly in its first hundred bytes, before any coefficient is touched.
//!
//! So this is the cheap half of the same subject: the header facts, from the bytes in hand, with
//! no process, no budget and no decoder. `crate::image` keeps the decode.
//!
//! # What is read, and what is deliberately not
//!
//! Read: the `jp2h` superbox's `ihdr` (I.5.3.1), `bpcc` (I.5.3.2), `colr` (I.5.3.3) and `cdef`
//! (I.5.3.6) boxes, and the `SIZ` marker segment at the head of the first `jp2c` codestream
//! (A.5.1). Bare codestream data — no boxes at all — is read for its `SIZ` alone; §7.4.9 says a
//! PDF's data shall be a full JPX file structure, so that case is a malformed file being read
//! as far as it can be rather than a shape this module endorses.
//!
//! And, from ITU-T T.801 (the identical text of ISO/IEC 15444-2, held as `doc/md/T.801.md`),
//! what its M.9.2 names as the JPX baseline: the File Type box's brand and compatibility list
//! (I.5.2, M.9.2), the order of the top-level boxes (M.9.2.7), the Fragment List boxes of a
//! Fragment Table or a Cross-Reference box (M.11.3.1, M.9.2.5, M.9.2.6), the first Compositing
//! Layer Header box's Colour Group and Codestream Registration boxes (M.11.7.1, M.11.7.7,
//! M.9.2.2, M.9.2.4), the enumerated parameters and the Any ICC method's profile (M.11.7.3), and
//! the extended `Rsiz` with the `MCC` and `MCO` marker segments of every header in the first
//! codestream (A.2.1, A.3.8, A.3.9, M.9.2.3).
//!
//! Not read: the contents of `pclr` and `cmap` (I.5.3.4, I.5.3.5), which decide how codestream
//! components become channels through a palette, and every other marker segment. Whether a
//! `cmap` box is *present* is reported, because that alone decides whether a `cdef` box's channel
//! indices address components directly (I.5.3.6).
//!
//! # Limits a caller should know
//!
//! - **A palette is not applied.** [`Headers::colour_channels`] counts what `cdef` describes, or
//!   failing that what `ihdr` states, which for a palettised image is the count *before* the
//!   palette expands it.
//! - **A channel index is not resolved to a component.** Where [`Headers::component_mapping`] is
//!   true, I.5.3.5's box stands between the two and this module does not read it, so a caller
//!   must not read a [`Channel`]'s index as a component's.
//! - **Only the first codestream is read.** I.5.3.1 makes the first `jp2c` the one the `ihdr`
//!   box describes, so a file with several is read as that clause reads it.
//! - **Disagreement is reported, not resolved.** I.5.3.1 says a file whose `ihdr` contradicts
//!   its codestream is not a conforming file, and that a reader may prefer the codestream. Both
//!   statements are kept here, side by side, so a caller can say which one it is judging.
//! - **Only the first Compositing Layer Header box is read.** M.9.2.2 makes the first
//!   compositing layer the one a baseline reader renders, and it is the only one M.9.2 states a
//!   requirement about.

/// The `jp2h` superbox, I.5.3.
const JP2_HEADER: [u8; 4] = *b"jp2h";
/// The `ihdr` box, I.5.3.1.
const IMAGE_HEADER: [u8; 4] = *b"ihdr";
/// The `bpcc` box, I.5.3.2.
const BITS_PER_COMPONENT: [u8; 4] = *b"bpcc";
/// The `colr` box, I.5.3.3.
const COLOUR_SPECIFICATION: [u8; 4] = *b"colr";
/// The `cmap` box, I.5.3.5.
const COMPONENT_MAPPING: [u8; 4] = *b"cmap";
/// The `cdef` box, I.5.3.6.
const CHANNEL_DEFINITION: [u8; 4] = *b"cdef";
/// The `jp2c` contiguous codestream box, I.5.4.
const CODESTREAM: [u8; 4] = *b"jp2c";

/// The `ftyp` box, I.5.2.
const FILE_TYPE: [u8; 4] = *b"ftyp";
/// The Fragment Table box, T.801 M.11.3.
const FRAGMENT_TABLE: [u8; 4] = *b"ftbl";
/// The Fragment List box, T.801 M.11.3.1.
const FRAGMENT_LIST: [u8; 4] = *b"flst";
/// The Cross-Reference box, T.801 M.11.4.
const CROSS_REFERENCE: [u8; 4] = *b"cref";
/// The Codestream Header box, T.801 M.11.6.
const CODESTREAM_HEADER: [u8; 4] = *b"jpch";
/// The Compositing Layer Header box, T.801 M.11.7.
const LAYER_HEADER: [u8; 4] = *b"jplh";
/// The Colour Group box, T.801 M.11.7.1.
const COLOUR_GROUP: [u8; 4] = *b"cgrp";
/// The Codestream Registration box, T.801 M.11.7.7.
const REGISTRATION: [u8; 4] = *b"creg";
/// The Free box, T.801 M.11.20, whose contents that subclause tells every reader to ignore.
const FREE: [u8; 4] = *b"free";

/// The `SOC` marker that opens a codestream, Table A-2.
const SOC: [u8; 2] = [0xFF, 0x4F];
/// The `SIZ` marker, which A.5.1 requires immediately after `SOC`.
const SIZ: [u8; 2] = [0xFF, 0x51];
/// The `SOT` marker that opens a tile-part, Table A-2.
const SOT: [u8; 2] = [0xFF, 0x90];
/// The `SOD` marker that ends a tile-part header, Table A-2.
const SOD: [u8; 2] = [0xFF, 0x93];
/// The `MCC` marker, T.801 Table A.34.
const MCC: [u8; 2] = [0xFF, 0x75];
/// The `MCO` marker, T.801 Table A.40.
const MCO: [u8; 2] = [0xFF, 0x77];

/// T.801 Table A.2: the high bit of `Rsiz`, set where an extension of that part is present.
const EXTENDED: u16 = 0x8000;
/// T.801 Table A.2's bits whose extension is required to decode, with the table's names.
const REQUIRED_EXTENSIONS: [(u16, &str); 9] = [
    (0x0001, "variable DC offset"),
    (0x0002, "variable scalar quantization"),
    (0x0010, "single sample overlap"),
    (0x0020, "arbitrary decomposition style"),
    (0x0040, "arbitrary transformation kernel"),
    (0x0080, "whole sample symmetric transformation kernel"),
    (0x0100, "multiple component transformation"),
    (0x0400, "arbitrary shaped region of interest"),
    (0x0800, "precinct-dependent quantization"),
];
/// The name [`Codestream::required_extensions`] gives the multiple component transformation.
pub const MULTIPLE_COMPONENT_TRANSFORMATION: &str = "multiple component transformation";

/// A box header is at least a four-byte length and a four-byte type, I.4 Table I-1.
const BOX_HEADER: usize = 8;
/// A box whose `LBox` is 1 carries an eight-byte `XLBox` as well, I.4.
const EXTENDED_BOX_HEADER: usize = 16;
/// `Rsiz` through `Csiz` in the `SIZ` marker segment, Table A-9: 2 + 8×4 + 2 bytes.
const SIZ_FIXED: usize = 36;
/// `Ssiz`, `XRsiz` and `YRsiz` are one byte each, per component, Table A-9.
const SIZ_PER_COMPONENT: usize = 3;

/// Why JPEG 2000 data could not be read as far as its headers.
///
/// Every variant is a statement about the *bytes*, so a caller reporting one is reporting that
/// the data does not have the shape ISO/IEC 15444-1 gives it — not that this module declined.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HeaderError {
    /// The data begins with neither a JP2 box nor a codestream.
    #[error("the data is neither a JP2 file nor a JPEG 2000 codestream")]
    NotJpeg2000,
    /// A box states a length that does not fit in the data, or one the standard reserves.
    ///
    /// I.4 gives `LBox` three readings — a length, 1 meaning "see `XLBox`", and 0 meaning "to
    /// the end of the file" — and reserves 2 to 7, so a box claiming one of those is malformed
    /// rather than merely unknown.
    #[error("a box at byte {at} states a length of {length}, which the data cannot hold")]
    BoxLength {
        /// Where the box's `LBox` field begins.
        at: usize,
        /// The length it stated.
        length: u64,
    },
    /// A box this module reads is shorter than the fields its clause requires it to carry.
    #[error("the {name} box holds {held} bytes, short of the {needed} its fields require")]
    ShortBox {
        /// The box type, as its four characters.
        name: String,
        /// How many payload bytes it holds.
        held: usize,
        /// How many its fields require.
        needed: usize,
    },
    /// The codestream does not open with `SOC` followed by `SIZ`, which A.5.1 requires.
    #[error("the codestream does not open with the SOC and SIZ markers")]
    NoSizMarker,
    /// The `SIZ` marker segment is shorter than the component count it declares.
    #[error("the SIZ marker segment declares {components} components it does not carry")]
    ShortSiz {
        /// The `Csiz` value it declared.
        components: u16,
    },
}

/// The precision and sign of one component, ISO/IEC 15444-1:2000 Tables I-6 and A-11.
///
/// Both tables encode the same thing the same way — the low seven bits are the depth minus one,
/// the high bit is the sign — so one type serves `ihdr`'s `BPC`, `bpcc`'s `BPC`*ᵢ* and `SIZ`'s
/// `Ssiz`*ᵢ*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Depth {
    /// Bits per sample, counting the sign bit: 1 through 128 as the encoding allows.
    ///
    /// The tables define 1 to 38 and reserve the rest, so a value above 38 is a statement the
    /// standard does not define — which is exactly what a caller checking a range needs to see,
    /// rather than a refusal here.
    pub bits: u8,
    /// Whether the samples are signed, which is the encoding's high bit.
    pub signed: bool,
}

impl Depth {
    /// Reads Table I-6's and Table A-11's shared one-byte encoding.
    fn from_byte(byte: u8) -> Self {
        Self {
            // The low seven bits hold at most 127, so this cannot overflow a `u8`;
            // `saturating_add` says so to the arithmetic lint rather than to a reader.
            bits: (byte & 0x7F).saturating_add(1),
            signed: byte & 0x80 != 0,
        }
    }
}

/// The `ihdr` box, ISO/IEC 15444-1:2000 I.5.3.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageHeader {
    /// `HEIGHT`, the image area's height in reference grid points.
    pub height: u32,
    /// `WIDTH`, the image area's width in reference grid points.
    pub width: u32,
    /// `NC`, the number of components in the codestream.
    pub components: u16,
    /// `BPC`, or `None` where it was 255 and the components vary in depth.
    ///
    /// I.5.3.1 requires a `bpcc` box in that case and I.5.3.2 forbids one in every other, so
    /// `None` here and [`Headers::component_depths`] being non-empty are the same statement made
    /// twice — which is what lets a caller notice when a file makes only one of them.
    pub depth: Option<Depth>,
    /// `C`, the compression type, which I.5.3.1 requires to be 7.
    pub compression: u8,
    /// `UnkC`, which is 1 where the producer did not know the real colourspace.
    pub colourspace_unknown: u8,
    /// `IPR`, which is 1 where the file carries an intellectual property rights box.
    pub rights: u8,
}

/// One `colr` box, ISO/IEC 15444-1:2000 I.5.3.3 and ITU-T T.801 M.11.7.2.
///
/// A JP2 file holds at least one and may hold several, each describing the same colourspace by a
/// different method. Which one a *reader* uses is not this type's business: I.5.3.3 tells a
/// conforming JP2 reader to use the first, and other standards layered over JP2 say otherwise,
/// so all of them are kept and the choice is the caller's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColourSpecification<'a> {
    /// `METH`, the specification method.
    ///
    /// Table I-9 of part 1 defines 1 and 2; T.801 Table M.22 adds 3, the Any ICC method, and 4,
    /// the Vendor Colour method, and reserves the rest.
    pub method: u8,
    /// `PREC`, the precedence, which I.5.3.3 reserves and sets to zero.
    pub precedence: i8,
    /// `APPROX`, how well this specification approximates the intended colourspace.
    pub approximation: u8,
    /// `EnumCS`, present only where `METH` is 1.
    ///
    /// Table I-10 gives 16 (sRGB) and 17 (greyscale) as the values a conforming first edition
    /// file may state in its first `colr` box, and T.801 Table M.25 assigns the others a JPX
    /// file may state.
    pub enumerated: Option<u32>,
    /// `EP`, the enumerated parameters after `EnumCS`, empty where the box states none.
    ///
    /// T.801 M.11.7.4 defines them for CIE Lab (14) and CIE Jab (19) alone, and makes their
    /// absence mean each field's default.
    pub parameters: &'a [u8],
    /// `PROFILE`, the embedded ICC profile, present only where `METH` is 2 or 3.
    ///
    /// Part 1 I.5.3.3 defines the restricted profile of method 2, and T.801 M.11.7.3.2 gives
    /// method 3 the same field holding any input profile. Borrowed rather than copied: a profile
    /// runs to hundreds of kilobytes, and a caller that wants it parsed hands these bytes to
    /// [`crate::icc::Profile::parse`].
    pub profile: Option<&'a [u8]>,
    /// Everything after `APPROX` that this module did not read as one of the fields above.
    ///
    /// Non-empty only for the Vendor Colour method and the values T.801 Table M.22 reserves,
    /// which the same table tells a conforming reader to ignore entirely where it does not
    /// understand them. The bytes are kept rather than discarded because ISO 32000-2 §7.4.9 admits a
    /// vendor-defined colour space.
    pub reserved: &'a [u8],
    /// Where the box's `TBox` field is in the data, for [`Headers::keeping_colour`].
    kind_at: usize,
    /// Where the box's `METH` field is in the data, for the same.
    method_at: usize,
}

impl ColourSpecification<'_> {
    /// T.801 Table M.22's value 3, the Any ICC method.
    pub const ANY_ICC: u8 = 3;
    /// Part 1 Table I-9's value 2, the Restricted ICC method.
    pub const RESTRICTED_ICC: u8 = 2;
    /// Part 1 Table I-9's value 1, the Enumerated method.
    pub const ENUMERATED: u8 = 1;
    /// T.801 Table M.25's value 14, CIE Lab.
    pub const CIELAB: u32 = 14;

    /// The illuminant a CIE Lab specification's values were computed under, or `None` for any
    /// other specification.
    ///
    /// T.801 M.11.7.4.1 lays `EP` out as six four-byte range and offset fields and then `IL`, and
    /// makes an omitted field its default, which for `IL` is CIE Illuminant D50. So a box whose
    /// `EP` stops before `IL` states D50.
    #[must_use]
    pub fn cielab_illuminant(&self) -> Option<Illuminant> {
        /// `RL`, `OL`, `RA`, `OA`, `RB` and `OB`, four bytes each, before `IL`. T.801 Table M.30.
        const RANGES: usize = 24;
        if self.enumerated != Some(Self::CIELAB) {
            return None;
        }
        Some(
            self.parameters
                .get(RANGES..RANGES.saturating_add(4))
                .and_then(|field| <[u8; 4]>::try_from(field).ok())
                .map_or(Illuminant::D50, Illuminant::from_field),
        )
    }
}

/// A CIE Lab specification's `IL` field: T.801 M.11.7.4.1 and T.801 Table M.29, which take their
/// codes from ITU-T T.4 Annex E, section E.6.7.
///
/// The field names an illuminant rather than stating the white point the Lab values are relative
/// to, so the white point is this type's to supply, and [`Self::white_point`] says where each one
/// comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Illuminant {
    /// CIE Illuminant D50, the field's default.
    D50,
    /// CIE Illuminant D65.
    D65,
    /// CIE Illuminant D75.
    D75,
    /// CIE standard illuminant A, coded `SA`.
    A,
    /// CIE illuminant C, coded `SC`.
    C,
    /// CIE illuminant F2.
    F2,
    /// CIE illuminant F7.
    F7,
    /// CIE illuminant F11.
    F11,
    /// An illuminant stated by its colour temperature alone, in kelvin: `CT` and two bytes.
    ColourTemperature(u16),
    /// Four bytes neither table codes.
    Unknown([u8; 4]),
}

impl Illuminant {
    /// The illuminant four `IL` bytes code.
    #[must_use]
    pub fn from_field(field: [u8; 4]) -> Self {
        match field {
            [0x00, 0x44, 0x35, 0x30] => Self::D50,
            [0x00, 0x44, 0x36, 0x35] => Self::D65,
            [0x00, 0x44, 0x37, 0x35] => Self::D75,
            [0x00, 0x00, 0x53, 0x41] => Self::A,
            [0x00, 0x00, 0x53, 0x43] => Self::C,
            [0x00, 0x00, 0x46, 0x32] => Self::F2,
            [0x00, 0x00, 0x46, 0x37] => Self::F7,
            [0x00, 0x46, 0x31, 0x31] => Self::F11,
            [b'C', b'T', high, low] => Self::ColourTemperature(u16::from_be_bytes([high, low])),
            other => Self::Unknown(other),
        }
    }

    /// The illuminant's white point, as CIE 1931 XYZ with `Y` at 1.0, or `None` where the code
    /// names no spectrum.
    ///
    /// Each value is a held text's or the CIE's own data's, never a renderer's (ADR 1713):
    ///
    /// - **D50** is the white point ITU-T T.4 section E.6.4 states for its default illuminant,
    ///   X 96.422, Y 100 and Z 82.521, scaled to `Y` = 1.
    /// - **D65** is the one ISO 32000-2 §8.6.5.4's EXAMPLE writes in a `Lab` space's
    ///   `/WhitePoint`, `[0.9505 1.00 1.0890]`.
    /// - **The other six** are the tristimulus sums of the CIE's published relative spectral
    ///   distributions against its 1931 2° colour-matching functions, at every wavelength both
    ///   tables state, rounded to five places; `data/cie/` holds the tables and
    ///   `every_white_point_is_the_cie_datas_own_sum` recomputes each value from them.
    ///
    /// - **A colour temperature** is the Planckian radiator's at that temperature, which is what
    ///   the CIE's vocabulary defines the term as: [`pdf_colour::planckian::white_point`].
    ///
    /// A code no table defines names nothing, and has no white point.
    #[must_use]
    pub fn white_point(self) -> Option<[f32; 3]> {
        match self {
            Self::D50 => Some([0.964_22, 1.0, 0.825_21]),
            Self::D65 => Some([0.950_5, 1.0, 1.089]),
            Self::D75 => Some([0.949_72, 1.0, 1.226_37]),
            Self::A => Some([1.098_5, 1.0, 0.355_85]),
            Self::C => Some([0.980_73, 1.0, 1.182_33]),
            Self::F2 => Some([0.991_86, 1.0, 0.673_94]),
            Self::F7 => Some([0.950_42, 1.0, 1.087_49]),
            Self::F11 => Some([1.009_61, 1.0, 0.643_51]),
            Self::ColourTemperature(kelvin) => {
                pdf_colour::planckian::white_point(f64::from(kelvin))
            }
            Self::Unknown(_) => None,
        }
    }
}

/// The `ftyp` box, ISO/IEC 15444-1:2000 I.5.2.
///
/// What a file says a reader needs. T.801 M.9.2 makes one entry of its compatibility list the
/// statement that a JPX baseline reader can open the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileType {
    /// `BR`, the brand.
    pub brand: [u8; 4],
    /// `MinV`, the minor version.
    pub version: u32,
    /// `CL`*ᵢ*, the compatibility list.
    pub compatibility: Vec<[u8; 4]>,
}

impl FileType {
    /// The `CL` value T.801 M.9.2 gives a file written so that a reader supporting only the JPX
    /// baseline set of features can open it.
    pub const JPX_BASELINE: [u8; 4] = *b"jpxb";
}

/// One `{OFF, LEN, DR}` tuple of a Fragment List box, T.801 Table M.17 in M.11.3.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fragment {
    /// `OFF`, the fragment's first byte, counted from the first byte of its file.
    pub offset: u64,
    /// `LEN`, the fragment's length.
    pub length: u32,
    /// `DR`, zero where the fragment is in this file and otherwise an index into the Data
    /// Reference box's URLs (M.11.2).
    pub reference: u16,
}

/// A Fragment List box and the box that holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FragmentList {
    /// `ftbl` where the list assembles a codestream (M.11.3), `cref` where it assembles a
    /// shared header or metadata box (M.11.4).
    pub container: [u8; 4],
    /// The index in [`Headers::top_level`] of the top-level box the list was read from: the
    /// Fragment Table box itself, or the Codestream Header or Compositing Layer Header box whose
    /// Cross-Reference box holds it. M.11.6 and M.11.7 number both kinds of header box by their
    /// order in the file, which is how T.801 M.9.2.6 tells the first layer's from the rest.
    pub box_index: usize,
    /// The fragments, in the order the list states them.
    pub fragments: Vec<Fragment>,
}

/// What the first Compositing Layer Header box says, T.801 M.11.7.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstLayer<'a> {
    /// The `colr` boxes of its Colour Group box, M.11.7.1, empty where it has none — in which
    /// case the layer's colour is the JP2 Header box's specifications, as M.11.7 says.
    pub colour: Vec<ColourSpecification<'a>>,
    /// The `CDN`*ᵢ* of its Codestream Registration box, M.11.7.7, or `None` where it has none —
    /// in which case the same subclause makes the layer one codestream.
    pub codestreams: Option<Vec<u16>>,
    /// The type of every box found within it, in file order: its own boxes, the boxes of its
    /// Colour Group box, and for a Cross-Reference box the `Rtyp` it names — M.11.7 treats a
    /// cross-referenced box as stored in the header box itself, and T.801 M.9.2.7's second
    /// sentence asks which boxes are found there.
    pub contents: Vec<[u8; 4]>,
}

/// One description in the `cdef` box, ISO/IEC 15444-1:2000 I.5.3.6.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Channel {
    /// `Cn`, the index of the channel this description is about.
    pub channel: u16,
    /// `Typ`, the channel's kind. Table I-16: 0 is colour, 1 opacity, 2 premultiplied opacity.
    pub kind: u16,
    /// `Asoc`, the colour this channel is associated with.
    pub association: u16,
}

impl Channel {
    /// Table I-16's value 0: this channel carries colour image data.
    pub const COLOUR: u16 = 0;
}

/// The `SIZ` marker segment, ISO/IEC 15444-1:2000 A.5.1.
///
/// The codestream's own statement of its size and precision, which I.5.3.1 makes redundant with
/// the `ihdr` box and authoritative where the two disagree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Codestream {
    /// `Rsiz`, the capabilities a decoder needs.
    ///
    /// Part 1's Table A-10 defines zero; T.801 Table A.2 sets the high bit where an extension of
    /// its own is present and one bit below it for each extension, which
    /// [`Self::required_extensions`] reads.
    pub capabilities: u16,
    /// `Xsiz`, the reference grid's width.
    pub grid_width: u32,
    /// `Ysiz`, the reference grid's height.
    pub grid_height: u32,
    /// `XOsiz`, the image area's horizontal offset into the grid.
    pub x_offset: u32,
    /// `YOsiz`, the image area's vertical offset into the grid.
    pub y_offset: u32,
    /// `Ssiz`*ᵢ* for each component, in codestream order. Its length is `Csiz`.
    pub depths: Vec<Depth>,
    /// Every `MCC` marker segment series, T.801 A.3.8, from the main header and from every
    /// tile-part header, in the order they were read.
    pub collections: Vec<Collection>,
    /// `Nmco` of every `MCO` marker segment, T.801 A.3.9, from the same headers.
    pub orderings: Vec<u8>,
}

/// The facts of one `MCC` marker segment series T.801 M.9.2.3 asks about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Collection {
    /// `Qmcc`, the number of component collections.
    pub count: u16,
    /// The first collection's `Xmcc` transform type, the two low bits of T.801 Table A.35:
    /// 0 dependency, 1 decorrelation, 3 wavelet. `None` where the series ends before the field.
    pub kind: Option<u8>,
    /// Whether the first collection's `Tmcc` marks its array-based transform reversible, Table
    /// A.38. `None` where the series ends before the field, or the collection is not array-based.
    pub reversible: Option<bool>,
}

impl Collection {
    /// The value T.801 Table A.35 gives an array-based decorrelation transform.
    pub const DECORRELATION: u8 = 1;
}

impl Codestream {
    /// `Csiz`, the number of components, which is the length of [`Self::depths`].
    ///
    /// A `u16` because Table A-9 bounds `Csiz` at 16 384 and the parse allocates one entry per
    /// component, so the conversion cannot lose anything.
    #[must_use]
    pub fn components(&self) -> u16 {
        u16::try_from(self.depths.len()).unwrap_or(u16::MAX)
    }

    /// The T.801 Table A.2 extensions `Rsiz` says are *required* to decode this codestream.
    ///
    /// Empty where the high bit is clear, which is every codestream using part 1's capabilities
    /// alone. T.801 Table A.2 marks three of its bits useful rather than required — trellis coded
    /// quantization, visual masking and the non-linear point transformation — and its note c)
    /// says such data can still be decoded without them, so they are not listed.
    #[must_use]
    pub fn required_extensions(&self) -> Vec<&'static str> {
        if self.capabilities & EXTENDED == 0 {
            return Vec::new();
        }
        REQUIRED_EXTENSIONS
            .iter()
            .filter(|(bit, _)| self.capabilities & bit != 0)
            .map(|(_, name)| *name)
            .collect()
    }
}

/// What JPEG 2000 data says about itself before anything is decoded.
///
/// Every field is what the bytes stated, unreconciled. [`Self::colour_channels`] and
/// [`Self::component_depths`] are the two derived readings, and they say in their own
/// documentation which clause derives them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Headers<'a> {
    /// The `ihdr` box, absent from data that is a bare codestream.
    pub image: Option<ImageHeader>,
    /// Every `colr` box, in the order the file states them.
    pub colour: Vec<ColourSpecification<'a>>,
    /// The `bpcc` box's per-component depths, empty where the file states no such box.
    pub bits_per_component: Vec<Depth>,
    /// The `cdef` box's descriptions, empty where the file states no such box.
    pub channels: Vec<Channel>,
    /// Whether the file states a `cmap` box, I.5.3.5.
    ///
    /// Its contents are not read, but its *presence* is what decides whether a `cdef` box's
    /// channel indices address codestream components directly: I.5.3.6 says a reader maps
    /// component *i* to channel *i* where there is no such box, and that this box says otherwise
    /// where there is one. So a caller reading [`Self::channels`] as component indices has to
    /// know, and this is how it is told.
    pub component_mapping: bool,
    /// The first codestream's `SIZ` marker segment.
    pub codestream: Option<Codestream>,
    /// The `ftyp` box, absent from a bare codestream.
    pub file_type: Option<FileType>,
    /// The type of every top-level box, in file order.
    pub top_level: Vec<[u8; 4]>,
    /// Where each box of [`Self::top_level`] lies in the data, first byte of its header to one
    /// past its last byte, in the same order — the positions T.801 M.9.2.6 compares a fragment's
    /// offset against.
    pub top_level_spans: Vec<std::ops::Range<usize>>,
    /// The type of every box within the JP2 Header box, in file order.
    pub jp2_header_contents: Vec<[u8; 4]>,
    /// The type of every box found within the first Codestream Header box, read as
    /// [`FirstLayer::contents`] is, or `None` where the file states none.
    ///
    /// M.11.6 applies Codestream Header box *i* to codestream *i*, and M.9.2.2 makes the first
    /// layer's one codestream the file's first, so this is the box T.801 M.9.2.7 calls the
    /// Codestream Header box associated with the first compositing layer.
    pub first_codestream_header: Option<Vec<[u8; 4]>>,
    /// Every Fragment List box of a top-level Fragment Table box, and of a Cross-Reference box
    /// inside a top-level Codestream Header or Compositing Layer Header box.
    pub fragment_lists: Vec<FragmentList>,
    /// The first Compositing Layer Header box, absent where the file states none.
    pub first_layer: Option<FirstLayer<'a>>,
}

impl<'a> Headers<'a> {
    /// Reads the JP2 boxes and the first codestream's `SIZ` marker segment.
    ///
    /// Unknown boxes are skipped, which is what I.8 asks of a reader; a box this module does
    /// know but that is too short for its own fields is an error, because reading past it would
    /// mean reporting a field the file did not state.
    ///
    /// # Errors
    ///
    /// [`HeaderError`] when the data is not JPEG 2000 at all, when a box states a length the
    /// data cannot hold, or when a box or marker segment this module reads is truncated.
    pub fn parse(data: &'a [u8]) -> Result<Self, HeaderError> {
        let mut headers = Self {
            image: None,
            colour: Vec::new(),
            bits_per_component: Vec::new(),
            channels: Vec::new(),
            component_mapping: false,
            codestream: None,
            file_type: None,
            top_level: Vec::new(),
            top_level_spans: Vec::new(),
            jp2_header_contents: Vec::new(),
            first_codestream_header: None,
            fragment_lists: Vec::new(),
            first_layer: None,
        };

        // §7.4.9 requires a full JPX file structure, so boxes are the shape expected. Data that
        // opens with `SOC` instead is a bare codestream, which some producers write and which is
        // still readable for everything A.5.1 states.
        if data.starts_with(&SOC) {
            headers.codestream = Some(parse_codestream(data)?);
            return Ok(headers);
        }

        // Whether the data is JPEG 2000 at all is decided by its first box, and only there:
        // arbitrary bytes read as a box of absurd length, and answering that with a length
        // complaint would tell a caller the file was nearly right. A box that fails later is a
        // different claim — the data *is* shaped like a JP2 file and one of its boxes is
        // broken — and keeps its own error.
        let Some(first) = next_box(data, 0).map_err(|_| HeaderError::NotJpeg2000)? else {
            return Err(HeaderError::NotJpeg2000);
        };

        let mut next = Some(first);
        while let Some(found) = next {
            let index = headers.top_level.len();
            headers.top_level.push(found.kind);
            headers.top_level_spans.push(found.start..found.end);
            match found.kind {
                JP2_HEADER => headers.read_jp2_header(found.payload, found.payload_at)?,
                CODESTREAM if headers.codestream.is_none() => {
                    headers.codestream = Some(parse_codestream(found.payload)?);
                }
                FILE_TYPE if headers.file_type.is_none() => {
                    headers.file_type = Some(parse_file_type(found.payload)?);
                }
                FRAGMENT_TABLE => {
                    headers.read_fragment_lists(found.kind, index, found.payload)?;
                }
                CODESTREAM_HEADER => {
                    headers.read_cross_references(index, found.payload)?;
                    if headers.first_codestream_header.is_none() {
                        headers.first_codestream_header = Some(contents_of(found.payload)?);
                    }
                }
                LAYER_HEADER => {
                    headers.read_cross_references(index, found.payload)?;
                    if headers.first_layer.is_none() {
                        headers.first_layer =
                            Some(parse_first_layer(found.payload, found.payload_at)?);
                    }
                }
                _ => {}
            }
            next = next_box(data, found.end)?;
        }
        Ok(headers)
    }

    /// A copy of `data` in which every `colr` box of the JP2 Header box but the one at `keep` is
    /// a Free box, and the kept one states `method` in place of its own where that is given.
    ///
    /// What a reader does with a colour specification it is to ignore, said in the format's own
    /// vocabulary: T.801 M.11.20 makes a Free box's contents meaningless and tells every reader
    /// to ignore it, and a box keeps its length when only its type changes, so no offset
    /// anywhere in the file moves. ISO 32000-2 §7.4.9 is the reason a caller asks: a dictionary
    /// that states `/ColorSpace` has the data's specifications ignored, and one that does not
    /// has a processor choose among them. Rewriting the method is for the Any ICC method, whose
    /// field T.801 M.11.7.3.2 lays out as part 1's restricted one.
    ///
    /// `data` must be the bytes these headers were parsed from.
    #[must_use]
    pub fn keeping_colour(&self, data: &[u8], keep: Option<usize>, method: Option<u8>) -> Vec<u8> {
        let mut out = data.to_vec();
        for (index, colour) in self.colour.iter().enumerate() {
            let at = colour.kind_at;
            if Some(index) == keep {
                if let (Some(method), Some(byte)) = (method, out.get_mut(colour.method_at)) {
                    *byte = method;
                }
                continue;
            }
            if let Some(kind) = out.get_mut(at..at.saturating_add(4)) {
                kind.copy_from_slice(&FREE);
            }
        }
        out
    }

    /// Reads the Fragment List box of a Fragment Table or Cross-Reference box, M.11.3 and M.11.4.
    fn read_fragment_lists(
        &mut self,
        container: [u8; 4],
        box_index: usize,
        payload: &[u8],
    ) -> Result<(), HeaderError> {
        let mut at = 0usize;
        while let Some(found) = next_box(payload, at)? {
            if found.kind == FRAGMENT_LIST {
                self.fragment_lists.push(FragmentList {
                    container,
                    box_index,
                    fragments: parse_fragments(found.payload)?,
                });
            }
            at = found.end;
        }
        Ok(())
    }

    /// Reads the Cross-Reference boxes one level inside a header superbox, M.11.4.
    ///
    /// A Cross-Reference box is `Rtyp` followed by a Fragment List box, so it is read by
    /// stepping over the four-byte field.
    fn read_cross_references(
        &mut self,
        box_index: usize,
        payload: &[u8],
    ) -> Result<(), HeaderError> {
        /// `Rtyp`, the referenced box type, T.801 Table M.18.
        const REFERENCED_TYPE: usize = 4;
        let mut at = 0usize;
        while let Some(found) = next_box(payload, at)? {
            if found.kind == CROSS_REFERENCE {
                let list = found.payload.get(REFERENCED_TYPE..).unwrap_or_default();
                self.read_fragment_lists(CROSS_REFERENCE, box_index, list)?;
            }
            at = found.end;
        }
        Ok(())
    }

    /// Reads the boxes inside the `jp2h` superbox, I.5.3.
    ///
    /// Iterative rather than recursive, and only one level deep, because I.5.3 names every box
    /// that may appear here and none of them is itself a superbox — so there is no depth for a
    /// hostile file to exhaust.
    fn read_jp2_header(&mut self, payload: &'a [u8], base: usize) -> Result<(), HeaderError> {
        let mut at = 0usize;
        while let Some(found) = next_box(payload, at)? {
            self.jp2_header_contents.push(found.kind);
            match found.kind {
                IMAGE_HEADER if self.image.is_none() => {
                    self.image = Some(parse_image_header(found.payload)?);
                }
                BITS_PER_COMPONENT if self.bits_per_component.is_empty() => {
                    self.bits_per_component = found
                        .payload
                        .iter()
                        .copied()
                        .map(Depth::from_byte)
                        .collect();
                }
                COLOUR_SPECIFICATION => {
                    self.colour.push(parse_colour(found.payload, base, &found)?);
                }
                CHANNEL_DEFINITION if self.channels.is_empty() => {
                    self.channels = parse_channels(found.payload)?;
                }
                COMPONENT_MAPPING => self.component_mapping = true,
                _ => {}
            }
            at = found.end;
        }
        Ok(())
    }

    /// How many of the image's channels carry colour, I.5.3.1 and I.5.3.6.
    ///
    /// The `ihdr` box's `NC` counts *every* channel, opacity included. I.5.3.6 is what separates
    /// them: where auxiliary channels exist the file shall state a `cdef` box giving each channel
    /// a `Typ`, and Table I-16's value 0 is the colour ones. Where there is no `cdef` box that
    /// same clause says the codestream holds colour channels only and they are in colour order,
    /// so `NC` is the answer.
    ///
    /// A channel may be described more than once — I.5.3.6 permits it, for a channel associated
    /// with several colours — so distinct `Cn` values are counted rather than descriptions.
    ///
    /// `None` for data stating neither a `cdef` box nor an `ihdr` box nor a codestream.
    #[must_use]
    pub fn colour_channels(&self) -> Option<u32> {
        if !self.channels.is_empty() {
            let mut seen: Vec<u16> = self
                .channels
                .iter()
                .filter(|channel| channel.kind == Channel::COLOUR)
                .map(|channel| channel.channel)
                .collect();
            seen.sort_unstable();
            seen.dedup();
            return Some(u32::try_from(seen.len()).unwrap_or(u32::MAX));
        }
        self.image
            .map(|image| u32::from(image.components))
            .or_else(|| {
                self.codestream
                    .as_ref()
                    .map(|codestream| u32::from(codestream.components()))
            })
    }

    /// The declared precision of each component, in codestream order.
    ///
    /// Three statements of the same fact, in the order I.5.3.1 and I.5.3.2 arrange them: the
    /// `bpcc` box where the components differ, the `ihdr` box's single `BPC` repeated where they
    /// do not, and the codestream's own `Ssiz`*ᵢ* where there are no boxes at all.
    ///
    /// Empty where nothing states a depth.
    #[must_use]
    pub fn component_depths(&self) -> Vec<Depth> {
        if !self.bits_per_component.is_empty() {
            return self.bits_per_component.clone();
        }
        if let Some((image, depth)) = self.image.and_then(|image| Some((image, image.depth?))) {
            return vec![depth; usize::from(image.components)];
        }
        self.codestream
            .as_ref()
            .map_or_else(Vec::new, |codestream| codestream.depths.clone())
    }
}

/// One box the walk reached, I.4.
struct Found<'a> {
    /// `TBox`, the four characters that name the box's type.
    kind: [u8; 4],
    /// `DBox`, the box's contents.
    payload: &'a [u8],
    /// Where the box's `LBox` field is, in the slice the walk read.
    start: usize,
    /// Where `DBox` begins, in the same slice.
    payload_at: usize,
    /// Where the next box begins.
    end: usize,
}

/// Reads the box beginning at `at`, or `None` where the data is exhausted.
///
/// I.4's three readings of `LBox` are all honoured, and each of them advances `at` by at least
/// [`BOX_HEADER`] — which is why this walk terminates without a visit counter.
fn next_box(data: &[u8], at: usize) -> Result<Option<Found<'_>>, HeaderError> {
    let Some(rest) = data.get(at..) else {
        return Ok(None);
    };
    if rest.len() < BOX_HEADER {
        // Fewer than eight bytes left is the ordinary end of a sequence of boxes: a box cannot
        // begin here, and trailing padding is not an error the caller can act on.
        return Ok(None);
    }
    let stated = u32::from_be_bytes([rest[0], rest[1], rest[2], rest[3]]);
    let kind = [rest[4], rest[5], rest[6], rest[7]];

    let (header, length) = match stated {
        // I.4 gives zero to a box whose length was not known when it was written: such a box
        // holds every remaining byte, and is therefore the last one.
        0 => (BOX_HEADER, rest.len()),
        1 => {
            let extended = rest
                .get(BOX_HEADER..EXTENDED_BOX_HEADER)
                .and_then(|bytes| <[u8; 8]>::try_from(bytes).ok())
                .map(u64::from_be_bytes)
                .ok_or(HeaderError::BoxLength { at, length: 1 })?;
            let length = usize::try_from(extended).map_err(|_| HeaderError::BoxLength {
                at,
                length: extended,
            })?;
            (EXTENDED_BOX_HEADER, length)
        }
        // I.4 reserves 2 to 7, so a box claiming one of them states a length it cannot have.
        2..=7 => {
            return Err(HeaderError::BoxLength {
                at,
                length: u64::from(stated),
            });
        }
        other => (
            BOX_HEADER,
            usize::try_from(other).map_err(|_| HeaderError::BoxLength {
                at,
                length: u64::from(other),
            })?,
        ),
    };

    if length < header || length > rest.len() {
        return Err(HeaderError::BoxLength {
            at,
            length: u64::try_from(length).unwrap_or(u64::MAX),
        });
    }
    let payload = rest.get(header..length).unwrap_or_default();
    Ok(Some(Found {
        kind,
        payload,
        start: at,
        payload_at: at.saturating_add(header),
        // `length` is at least `BOX_HEADER`, so every step of the walk makes progress.
        end: at.saturating_add(length),
    }))
}

/// The `ihdr` box's fixed fields, I.5.3.1 Table I-5.
fn parse_image_header(payload: &[u8]) -> Result<ImageHeader, HeaderError> {
    /// `HEIGHT`, `WIDTH`, `NC`, `BPC`, `C`, `UnkC` and `IPR`: 4 + 4 + 2 + 1 + 1 + 1 + 1.
    const NEEDED: usize = 14;
    let fields = payload.get(..NEEDED).ok_or_else(|| HeaderError::ShortBox {
        name: name_of(IMAGE_HEADER),
        held: payload.len(),
        needed: NEEDED,
    })?;
    let bits = fields[10];
    Ok(ImageHeader {
        height: u32::from_be_bytes([fields[0], fields[1], fields[2], fields[3]]),
        width: u32::from_be_bytes([fields[4], fields[5], fields[6], fields[7]]),
        components: u16::from_be_bytes([fields[8], fields[9]]),
        // I.5.3.1 makes 255 the signal that the components differ, in which case that clause
        // requires the `bpcc` box of I.5.3.2 to state each one instead.
        depth: (bits != 0xFF).then(|| Depth::from_byte(bits)),
        compression: fields[11],
        colourspace_unknown: fields[12],
        rights: fields[13],
    })
}

/// One `colr` box, I.5.3.3 Table I-11 and T.801 Table M.24.
///
/// `base` is where the slice `found` was read from begins in the whole data, so that the box's
/// own fields can be found again by [`Headers::keeping_colour`].
fn parse_colour<'a>(
    payload: &'a [u8],
    base: usize,
    found: &Found<'_>,
) -> Result<ColourSpecification<'a>, HeaderError> {
    /// `METH`, `PREC` and `APPROX`, which every `colr` box carries whatever its method.
    const NEEDED: usize = 3;
    /// `EnumCS` is a four-byte big endian unsigned integer.
    const ENUMERATED: usize = 4;

    let fields = payload.get(..NEEDED).ok_or_else(|| HeaderError::ShortBox {
        name: name_of(COLOUR_SPECIFICATION),
        held: payload.len(),
        needed: NEEDED,
    })?;
    let method = fields[0];
    let rest = payload.get(NEEDED..).unwrap_or_default();

    // T.801 Table M.22 makes the remainder of the box a function of `METH`. The Vendor Colour
    // method and the values the table reserves leave it uninterpreted: the first is a vendor's own
    // definition, and the table tells a reader to ignore the second outright.
    let (enumerated, parameters, profile) = match method {
        ColourSpecification::ENUMERATED => {
            let value = rest
                .get(..ENUMERATED)
                .and_then(|bytes| <[u8; 4]>::try_from(bytes).ok())
                .map(u32::from_be_bytes)
                .ok_or_else(|| HeaderError::ShortBox {
                    name: name_of(COLOUR_SPECIFICATION),
                    held: payload.len(),
                    needed: NEEDED.saturating_add(ENUMERATED),
                })?;
            // M.11.7.3.1: the `EP` field is every byte after `EnumCS` to the end of the box.
            (
                Some(value),
                rest.get(ENUMERATED..).unwrap_or_default(),
                None,
            )
        }
        ColourSpecification::RESTRICTED_ICC | ColourSpecification::ANY_ICC => {
            (None, &[][..], Some(rest))
        }
        _ => (None, &[][..], None),
    };

    Ok(ColourSpecification {
        method,
        // Signed, alone among the three fixed fields, which is I.5.3.3's own choice.
        precedence: i8::from_be_bytes([fields[1]]),
        approximation: fields[2],
        enumerated,
        parameters,
        profile,
        reserved: if enumerated.is_none() && profile.is_none() {
            rest
        } else {
            &[]
        },
        kind_at: base.saturating_add(found.start).saturating_add(4),
        method_at: base.saturating_add(found.payload_at),
    })
}

/// The `ftyp` box's fields, I.5.2 Table I-3.
fn parse_file_type(payload: &[u8]) -> Result<FileType, HeaderError> {
    /// `BR` and `MinV`, four bytes each.
    const NEEDED: usize = 8;
    let fields = payload.get(..NEEDED).ok_or_else(|| HeaderError::ShortBox {
        name: name_of(FILE_TYPE),
        held: payload.len(),
        needed: NEEDED,
    })?;
    Ok(FileType {
        brand: [fields[0], fields[1], fields[2], fields[3]],
        version: u32::from_be_bytes([fields[4], fields[5], fields[6], fields[7]]),
        compatibility: payload
            .get(NEEDED..)
            .unwrap_or_default()
            .chunks_exact(4)
            .map(|entry| [entry[0], entry[1], entry[2], entry[3]])
            .collect(),
    })
}

/// A Fragment List box's tuples, T.801 Table M.17 in M.11.3.1.
fn parse_fragments(payload: &[u8]) -> Result<Vec<Fragment>, HeaderError> {
    /// `NF`, a two-byte count.
    const COUNT: usize = 2;
    /// `OFF`, `LEN` and `DR`: eight, four and two bytes.
    const EACH: usize = 14;
    let count = payload
        .get(..COUNT)
        .and_then(|bytes| <[u8; 2]>::try_from(bytes).ok())
        .map(u16::from_be_bytes)
        .ok_or_else(|| HeaderError::ShortBox {
            name: name_of(FRAGMENT_LIST),
            held: payload.len(),
            needed: COUNT,
        })?;
    let needed = COUNT.saturating_add(usize::from(count).saturating_mul(EACH));
    let body = payload
        .get(COUNT..needed)
        .ok_or_else(|| HeaderError::ShortBox {
            name: name_of(FRAGMENT_LIST),
            held: payload.len(),
            needed,
        })?;
    Ok(body
        .chunks_exact(EACH)
        .map(|entry| Fragment {
            offset: u64::from_be_bytes([
                entry[0], entry[1], entry[2], entry[3], entry[4], entry[5], entry[6], entry[7],
            ]),
            length: u32::from_be_bytes([entry[8], entry[9], entry[10], entry[11]]),
            reference: u16::from_be_bytes([entry[12], entry[13]]),
        })
        .collect())
}

/// The first Compositing Layer Header box's Colour Group and Codestream Registration boxes.
///
/// `base` is where `payload` begins in the whole data. The Colour Group box is one more level
/// down, so its `colr` boxes are found at that box's own payload offset added on.
fn parse_first_layer(payload: &[u8], base: usize) -> Result<FirstLayer<'_>, HeaderError> {
    /// `XS` and `YS`, two bytes each, T.801 M.11.7.7.
    const GRID: usize = 4;
    /// `CDN`, `XR`, `YR`, `XO` and `YO`: two bytes and four of one byte.
    const EACH: usize = 6;
    let mut layer = FirstLayer {
        colour: Vec::new(),
        codestreams: None,
        contents: contents_of(payload)?,
    };
    let mut at = 0usize;
    while let Some(found) = next_box(payload, at)? {
        match found.kind {
            COLOUR_GROUP if layer.colour.is_empty() => {
                let group_base = base.saturating_add(found.payload_at);
                let mut inner = 0usize;
                while let Some(colour) = next_box(found.payload, inner)? {
                    if colour.kind == COLOUR_SPECIFICATION {
                        layer
                            .colour
                            .push(parse_colour(colour.payload, group_base, &colour)?);
                    }
                    inner = colour.end;
                }
            }
            REGISTRATION if layer.codestreams.is_none() => {
                layer.codestreams = Some(
                    found
                        .payload
                        .get(GRID..)
                        .unwrap_or_default()
                        .chunks_exact(EACH)
                        .map(|entry| u16::from_be_bytes([entry[0], entry[1]]))
                        .collect(),
                );
            }
            _ => {}
        }
        at = found.end;
    }
    Ok(layer)
}

/// The type of every box found within a Codestream Header or Compositing Layer Header box.
///
/// Its own boxes in file order, each Colour Group box followed by the boxes it holds (M.11.7.1),
/// and a Cross-Reference box as the `Rtyp` it names, because M.11.6 and M.11.7 both treat the box
/// a cross-reference points to as though the header box held it. Two levels and no more: M.11.7.1 names nothing a Colour Group box holds but Colour
/// Specification boxes, and M.11.4 forbids a Cross-Reference box pointing at another.
fn contents_of(payload: &[u8]) -> Result<Vec<[u8; 4]>, HeaderError> {
    let mut contents = Vec::new();
    let mut at = 0usize;
    while let Some(found) = next_box(payload, at)? {
        match found.kind {
            CROSS_REFERENCE => {
                if let Some(named) = found
                    .payload
                    .get(..4)
                    .and_then(|bytes| <[u8; 4]>::try_from(bytes).ok())
                {
                    contents.push(named);
                }
            }
            COLOUR_GROUP => {
                contents.push(found.kind);
                let mut inner = 0usize;
                while let Some(held) = next_box(found.payload, inner)? {
                    contents.push(held.kind);
                    inner = held.end;
                }
            }
            kind => contents.push(kind),
        }
        at = found.end;
    }
    Ok(contents)
}

/// The `cdef` box's array of channel descriptions, I.5.3.6 Table I-19.
fn parse_channels(payload: &[u8]) -> Result<Vec<Channel>, HeaderError> {
    /// `N`, the count of descriptions, is a two-byte big endian unsigned integer.
    const COUNT: usize = 2;
    /// Each description is `Cn`, `Typ` and `Asoc`, two bytes each.
    const EACH: usize = 6;

    let count = payload
        .get(..COUNT)
        .and_then(|bytes| <[u8; 2]>::try_from(bytes).ok())
        .map(u16::from_be_bytes)
        .ok_or_else(|| HeaderError::ShortBox {
            name: name_of(CHANNEL_DEFINITION),
            held: payload.len(),
            needed: COUNT,
        })?;
    let needed = COUNT.saturating_add(usize::from(count).saturating_mul(EACH));
    let body = payload
        .get(COUNT..needed)
        .ok_or_else(|| HeaderError::ShortBox {
            name: name_of(CHANNEL_DEFINITION),
            held: payload.len(),
            needed,
        })?;
    Ok(body
        .chunks_exact(EACH)
        .map(|entry| Channel {
            channel: u16::from_be_bytes([entry[0], entry[1]]),
            kind: u16::from_be_bytes([entry[2], entry[3]]),
            association: u16::from_be_bytes([entry[4], entry[5]]),
        })
        .collect())
}

/// The `SIZ` marker segment at the head of a codestream, A.5.1 Table A-9.
///
/// A.5 puts `SIZ` immediately after `SOC` in the main header and permits exactly one of each, so
/// there is no scan: the segment is either where the clause says it is or the data is not a
/// codestream this module can read.
fn parse_codestream(data: &[u8]) -> Result<Codestream, HeaderError> {
    /// `SOC`, `SIZ` and `Lsiz` are two bytes each, so the parameters begin at byte six.
    const PARAMETERS: usize = 6;

    if !data.starts_with(&SOC) || data.get(2..4) != Some(&SIZ) {
        return Err(HeaderError::NoSizMarker);
    }
    let fixed = data
        .get(PARAMETERS..PARAMETERS.saturating_add(SIZ_FIXED))
        .ok_or(HeaderError::NoSizMarker)?;
    let components = u16::from_be_bytes([fixed[34], fixed[35]]);

    let per_component = usize::from(components).saturating_mul(SIZ_PER_COMPONENT);
    let tail = data
        .get(PARAMETERS.saturating_add(SIZ_FIXED)..)
        .and_then(|rest| rest.get(..per_component))
        .ok_or(HeaderError::ShortSiz { components })?;

    Ok(Codestream {
        capabilities: u16::from_be_bytes([fixed[0], fixed[1]]),
        grid_width: u32::from_be_bytes([fixed[2], fixed[3], fixed[4], fixed[5]]),
        grid_height: u32::from_be_bytes([fixed[6], fixed[7], fixed[8], fixed[9]]),
        x_offset: u32::from_be_bytes([fixed[10], fixed[11], fixed[12], fixed[13]]),
        y_offset: u32::from_be_bytes([fixed[14], fixed[15], fixed[16], fixed[17]]),
        depths: tail
            .chunks_exact(SIZ_PER_COMPONENT)
            .map(|entry| Depth::from_byte(entry[0]))
            .collect(),
        collections: Vec::new(),
        orderings: Vec::new(),
    })
    .map(|mut codestream| {
        read_component_transforms(data, &mut codestream);
        codestream
    })
}

/// Reads the `MCC` and `MCO` marker segments of the main header and of every tile-part header,
/// T.801 A.3.8 and A.3.9.
///
/// A walk over marker segments and nothing else: each has a two-byte length after its marker
/// (A.1.4), and a tile-part's data is stepped over whole by its `SOT`'s `Psot` (A.4.2). A
/// malformed length ends the walk rather than failing the parse, because what the codestream
/// has already stated about itself is still what it stated; A.5.1's `SIZ` is the only segment
/// this module refuses a codestream for.
///
/// Every step advances by at least the four bytes of a marker and its length, or by a
/// tile-part's non-zero `Psot`, so the walk ends.
fn read_component_transforms(data: &[u8], codestream: &mut Codestream) {
    /// A marker and its two-byte length.
    const MARKER_AND_LENGTH: usize = 4;
    /// `Isot`, `Psot`, `TPsot` and `TNsot` follow the `SOT` marker's length, A.4.2.
    const PSOT_AT: usize = 6;

    let mut at = 0usize;
    // Collected per header, because T.801 A.3.8 joins a series only within one header.
    let mut series: Vec<(u8, u16, Vec<u8>)> = Vec::new();
    let mut tile_part: Option<usize> = None;
    while let Some(marker) = data.get(at..at.saturating_add(2)) {
        if marker == SOC {
            at = at.saturating_add(2);
            continue;
        }
        if marker == SOD {
            flush_series(&mut series, codestream);
            // `Psot` counts from the first byte of the `SOT` marker; zero means the tile-part
            // runs to the end of the codestream, which ends the walk.
            let Some(start) = tile_part else {
                break;
            };
            let psot = data
                .get(start.saturating_add(PSOT_AT)..start.saturating_add(PSOT_AT + 4))
                .and_then(|bytes| <[u8; 4]>::try_from(bytes).ok())
                .map_or(0, u32::from_be_bytes);
            if psot == 0 {
                break;
            }
            at = start.saturating_add(usize::try_from(psot).unwrap_or(usize::MAX));
            tile_part = None;
            continue;
        }
        let Some(length) = data
            .get(at.saturating_add(2)..at.saturating_add(MARKER_AND_LENGTH))
            .and_then(|bytes| <[u8; 2]>::try_from(bytes).ok())
            .map(u16::from_be_bytes)
        else {
            break;
        };
        let length = usize::from(length);
        if length < 2 {
            break;
        }
        let Some(segment) =
            data.get(at.saturating_add(MARKER_AND_LENGTH)..at.saturating_add(2 + length))
        else {
            break;
        };
        if marker == SOT {
            flush_series(&mut series, codestream);
            tile_part = Some(at);
        } else if marker == MCC {
            // `Zmcc` (two bytes) and `Imcc` (one) open every segment of a series.
            if let [z_high, z_low, index, rest @ ..] = segment {
                series.push((*index, u16::from_be_bytes([*z_high, *z_low]), rest.to_vec()));
            }
        } else if marker == MCO
            && let Some(stages) = segment.first()
        {
            codestream.orderings.push(*stages);
        }
        at = at.saturating_add(2).saturating_add(length);
    }
    flush_series(&mut series, codestream);
}

/// Joins the `MCC` segments one header held into series and reads each, T.801 A.3.8.
///
/// A series is the segments sharing an `Imcc`, appended in `Zmcc` order; the first carries
/// `Ymcc` and `Qmcc` before the collections (T.801 Table A.34).
fn flush_series(series: &mut Vec<(u8, u16, Vec<u8>)>, codestream: &mut Codestream) {
    series.sort_by_key(|(index, z, _)| (*index, *z));
    let mut joined: Vec<(u8, Vec<u8>)> = Vec::new();
    for (index, _, bytes) in series.drain(..) {
        match joined.last_mut() {
            Some((last, stream)) if *last == index => stream.extend_from_slice(&bytes),
            _ => joined.push((index, bytes)),
        }
    }
    for (_, stream) in joined {
        codestream.collections.push(read_collection(&stream));
    }
}

/// The facts of one joined `MCC` series: `Ymcc`, `Qmcc`, then the first collection's fields.
fn read_collection(stream: &[u8]) -> Collection {
    /// `Ymcc` and `Qmcc`, two bytes each.
    const COUNTS: usize = 4;
    /// T.801 Tables A.36 and A.37: the high bit of `Nmcc` and `Mmcc` makes each index two
    /// bytes.
    const WIDE: u16 = 0x8000;

    let count = stream
        .get(2..COUNTS)
        .and_then(|bytes| <[u8; 2]>::try_from(bytes).ok())
        .map_or(0, u16::from_be_bytes);
    let mut collection = Collection {
        count,
        kind: None,
        reversible: None,
    };
    let Some(kind) = stream.get(COUNTS).copied() else {
        return collection;
    };
    collection.kind = Some(kind & 0b11);
    let read_u16 = |at: usize| {
        stream
            .get(at..at.saturating_add(2))
            .and_then(|bytes| <[u8; 2]>::try_from(bytes).ok())
            .map(u16::from_be_bytes)
    };
    let indices = |field: u16| {
        usize::from(field & !WIDE).saturating_mul(if field & WIDE == 0 { 1 } else { 2 })
    };
    let at = COUNTS.saturating_add(1);
    let Some(inputs) = read_u16(at) else {
        return collection;
    };
    let at = at.saturating_add(2).saturating_add(indices(inputs));
    let Some(outputs) = read_u16(at) else {
        return collection;
    };
    let at = at.saturating_add(2).saturating_add(indices(outputs));
    // T.801 Table A.38: the top byte's lowest bit of the 24-bit `Tmcc` marks an array-based
    // transform reversible. A wavelet-based collection's `Tmcc` is read by T.801 Table A.39 and
    // states no such bit.
    if kind & 0b10 == 0 {
        collection.reversible = stream.get(at).map(|top| top & 1 == 1);
    }
    collection
}

/// A box type as the four characters a clause names it by, for a report.
fn name_of(kind: [u8; 4]) -> String {
    String::from_utf8_lossy(&kind).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a box: `LBox`, `TBox`, payload. I.4.
    fn boxed(kind: [u8; 4], payload: &[u8]) -> Vec<u8> {
        let length = u32::try_from(payload.len().saturating_add(BOX_HEADER))
            .expect("the test data is small");
        let mut bytes = length.to_be_bytes().to_vec();
        bytes.extend_from_slice(&kind);
        bytes.extend_from_slice(payload);
        bytes
    }

    /// An `ihdr` payload, I.5.3.1 Table I-5.
    fn image_header(components: u16, bits: u8) -> Vec<u8> {
        let mut payload = 480u32.to_be_bytes().to_vec();
        payload.extend_from_slice(&640u32.to_be_bytes());
        payload.extend_from_slice(&components.to_be_bytes());
        payload.extend_from_slice(&[bits, 7, 0, 0]);
        payload
    }

    /// A `colr` payload for `METH` 1, I.5.3.3 Table I-11.
    fn enumerated_colour(approximation: u8, space: u32) -> Vec<u8> {
        let mut payload = vec![1, 0, approximation];
        payload.extend_from_slice(&space.to_be_bytes());
        payload
    }

    /// A minimal codestream: `SOC`, then a `SIZ` segment with one `Ssiz` per component. A.5.1.
    fn codestream(depths: &[u8]) -> Vec<u8> {
        let components = u16::try_from(depths.len()).expect("the test data is small");
        let mut parameters = 0u16.to_be_bytes().to_vec();
        for value in [640u32, 480, 0, 0, 640, 480, 0, 0] {
            parameters.extend_from_slice(&value.to_be_bytes());
        }
        parameters.extend_from_slice(&components.to_be_bytes());
        for depth in depths {
            parameters.extend_from_slice(&[*depth, 1, 1]);
        }
        let length =
            u16::try_from(parameters.len().saturating_add(2)).expect("the test data is small");

        let mut bytes = SOC.to_vec();
        bytes.extend_from_slice(&SIZ);
        bytes.extend_from_slice(&length.to_be_bytes());
        bytes.extend_from_slice(&parameters);
        bytes
    }

    /// A whole JP2 file: signature, file type, header superbox, codestream. I.5.
    fn jp2(header: &[u8], depths: &[u8]) -> Vec<u8> {
        let mut bytes = boxed(*b"jP  ", &[0x0D, 0x0A, 0x87, 0x0A]);
        bytes.extend_from_slice(&boxed(*b"ftyp", b"jp2 \0\0\0\0jp2 "));
        bytes.extend_from_slice(&boxed(JP2_HEADER, header));
        bytes.extend_from_slice(&boxed(CODESTREAM, &codestream(depths)));
        bytes
    }

    /// The shape ISO/IEC 15444-1 I.5 gives a JP2 file, read end to end.
    #[test]
    fn a_jp2_file_states_its_header_and_its_codestream() {
        let mut header = boxed(IMAGE_HEADER, &image_header(3, 7));
        header.extend_from_slice(&boxed(COLOUR_SPECIFICATION, &enumerated_colour(0, 16)));
        let data = jp2(&header, &[7, 7, 7]);

        let headers = Headers::parse(&data).expect("the file is well formed");
        let image = headers.image.expect("an ihdr box was stated");
        assert_eq!((image.width, image.height), (640, 480));
        assert_eq!(image.components, 3);
        assert_eq!(
            image.depth,
            Some(Depth {
                bits: 8,
                signed: false
            })
        );
        assert_eq!(headers.colour.len(), 1);
        assert_eq!(headers.colour[0].method, 1);
        assert_eq!(headers.colour[0].enumerated, Some(16));
        assert_eq!(headers.colour_channels(), Some(3));
        assert_eq!(
            headers.codestream.as_ref().map(Codestream::components),
            Some(3)
        );
    }

    /// I.5.3.3 permits several `colr` boxes, and every one of them is kept.
    #[test]
    fn several_colour_specifications_are_all_kept() {
        let mut header = boxed(IMAGE_HEADER, &image_header(3, 7));
        header.extend_from_slice(&boxed(COLOUR_SPECIFICATION, &enumerated_colour(1, 16)));
        header.extend_from_slice(&boxed(COLOUR_SPECIFICATION, &enumerated_colour(0, 17)));
        let data = jp2(&header, &[7, 7, 7]);

        let headers = Headers::parse(&data).expect("the file is well formed");
        let approximations: Vec<u8> = headers
            .colour
            .iter()
            .map(|colour| colour.approximation)
            .collect();
        assert_eq!(approximations, vec![1, 0]);
    }

    /// I.5.3.1's `BPC` of 255 defers to I.5.3.2's `bpcc` box, and `component_depths` follows it.
    #[test]
    fn varying_depths_come_from_the_bits_per_component_box() {
        let mut header = boxed(IMAGE_HEADER, &image_header(3, 0xFF));
        header.extend_from_slice(&boxed(BITS_PER_COMPONENT, &[7, 7, 0x0B]));
        header.extend_from_slice(&boxed(COLOUR_SPECIFICATION, &enumerated_colour(0, 16)));
        let data = jp2(&header, &[7, 7, 0x0B]);

        let headers = Headers::parse(&data).expect("the file is well formed");
        assert_eq!(headers.image.and_then(|image| image.depth), None);
        let bits: Vec<u8> = headers
            .component_depths()
            .iter()
            .map(|depth| depth.bits)
            .collect();
        assert_eq!(bits, vec![8, 8, 12]);
    }

    /// I.5.3.1's `BPC` where it is a single value: one depth, repeated for every component.
    #[test]
    fn one_depth_is_repeated_across_the_components() {
        let mut header = boxed(IMAGE_HEADER, &image_header(4, 7));
        header.extend_from_slice(&boxed(COLOUR_SPECIFICATION, &enumerated_colour(0, 16)));
        let data = jp2(&header, &[7, 7, 7, 7]);

        let headers = Headers::parse(&data).expect("the file is well formed");
        assert_eq!(headers.component_depths().len(), 4);
    }

    /// I.5.3.6's `Typ` of 0 is what makes a channel a colour channel; opacity does not count.
    #[test]
    fn an_opacity_channel_is_not_a_colour_channel() {
        let mut definitions = 4u16.to_be_bytes().to_vec();
        for (channel, kind, association) in [(0u16, 0u16, 1u16), (1, 0, 2), (2, 0, 3), (3, 1, 0)] {
            definitions.extend_from_slice(&channel.to_be_bytes());
            definitions.extend_from_slice(&kind.to_be_bytes());
            definitions.extend_from_slice(&association.to_be_bytes());
        }
        let mut header = boxed(IMAGE_HEADER, &image_header(4, 7));
        header.extend_from_slice(&boxed(COLOUR_SPECIFICATION, &enumerated_colour(0, 16)));
        header.extend_from_slice(&boxed(CHANNEL_DEFINITION, &definitions));
        let data = jp2(&header, &[7, 7, 7, 7]);

        let headers = Headers::parse(&data).expect("the file is well formed");
        assert_eq!(headers.image.map(|image| image.components), Some(4));
        assert_eq!(headers.colour_channels(), Some(3));
    }

    /// I.5.3.6 lets one channel carry several descriptions, and they are one channel.
    #[test]
    fn a_channel_described_twice_is_counted_once() {
        let mut definitions = 2u16.to_be_bytes().to_vec();
        for (channel, kind, association) in [(0u16, 0u16, 1u16), (0, 0, 2)] {
            definitions.extend_from_slice(&channel.to_be_bytes());
            definitions.extend_from_slice(&kind.to_be_bytes());
            definitions.extend_from_slice(&association.to_be_bytes());
        }
        let mut header = boxed(IMAGE_HEADER, &image_header(1, 7));
        header.extend_from_slice(&boxed(COLOUR_SPECIFICATION, &enumerated_colour(0, 17)));
        header.extend_from_slice(&boxed(CHANNEL_DEFINITION, &definitions));
        let data = jp2(&header, &[7]);

        let headers = Headers::parse(&data).expect("the file is well formed");
        assert_eq!(headers.colour_channels(), Some(1));
    }

    /// I.5.3.5's box stands between channels and components, and its presence is reported.
    #[test]
    fn a_component_mapping_box_is_noticed_without_being_read() {
        let mut header = boxed(IMAGE_HEADER, &image_header(3, 7));
        header.extend_from_slice(&boxed(COLOUR_SPECIFICATION, &enumerated_colour(0, 16)));
        let plain = jp2(&header, &[7, 7, 7]);
        assert!(
            !Headers::parse(&plain)
                .expect("the file is well formed")
                .component_mapping
        );

        header.extend_from_slice(&boxed(
            COMPONENT_MAPPING,
            &[0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 2],
        ));
        let mapped = jp2(&header, &[7, 7, 7]);
        assert!(
            Headers::parse(&mapped)
                .expect("the file is well formed")
                .component_mapping
        );
    }

    /// Data that is a bare codestream still states A.5.1's parameters, and only those.
    #[test]
    fn a_bare_codestream_is_read_for_its_siz_alone() {
        let data = codestream(&[7, 7, 7]);
        let headers = Headers::parse(&data).expect("the codestream is readable");
        assert!(headers.image.is_none());
        assert!(headers.colour.is_empty());
        assert_eq!(headers.colour_channels(), Some(3));
    }

    /// T.801 Table M.22 reserves every `METH` above 4, so nothing is invented for those bytes.
    #[test]
    fn a_reserved_method_leaves_its_remainder_uninterpreted() {
        let mut header = boxed(IMAGE_HEADER, &image_header(3, 7));
        header.extend_from_slice(&boxed(COLOUR_SPECIFICATION, &[5, 0, 1, 0xAA, 0xBB]));
        let data = jp2(&header, &[7, 7, 7]);

        let headers = Headers::parse(&data).expect("the file is well formed");
        assert_eq!(headers.colour[0].method, 5);
        assert_eq!(headers.colour[0].enumerated, None);
        assert_eq!(headers.colour[0].profile, None);
        assert_eq!(headers.colour[0].reserved, &[0xAA, 0xBB]);
    }

    /// T.801 M.11.7.3.2 gives the Any ICC method part 1's profile field.
    #[test]
    fn the_any_icc_method_carries_a_profile() {
        let mut header = boxed(IMAGE_HEADER, &image_header(3, 7));
        header.extend_from_slice(&boxed(COLOUR_SPECIFICATION, &[3, 0, 1, 0xAA, 0xBB]));
        let data = jp2(&header, &[7, 7, 7]);

        let headers = Headers::parse(&data).expect("the file is well formed");
        assert_eq!(headers.colour[0].profile, Some(&[0xAA, 0xBB][..]));
        assert!(headers.colour[0].reserved.is_empty());
    }

    /// T.801 M.11.7.3.1: the `EP` field is every byte after `EnumCS`.
    #[test]
    fn enumerated_parameters_are_the_rest_of_the_box() {
        let mut colour = enumerated_colour(1, 14);
        colour.extend_from_slice(&[0, 0, 0, 100]);
        let mut header = boxed(IMAGE_HEADER, &image_header(3, 7));
        header.extend_from_slice(&boxed(COLOUR_SPECIFICATION, &colour));
        let data = jp2(&header, &[7, 7, 7]);

        let headers = Headers::parse(&data).expect("the file is well formed");
        assert_eq!(headers.colour[0].enumerated, Some(14));
        assert_eq!(headers.colour[0].parameters, &[0, 0, 0, 100]);
    }

    /// T.801 M.11.7.4.1: `IL` is the seventh four-byte field of a CIE Lab box's `EP`, coded as
    /// T.801 Table M.29 and T.4 section E.6.7 code it, and D50 where the box stops before it.
    #[test]
    fn a_cielab_box_states_its_illuminant() {
        let illuminant = |parameters: &[u8]| {
            let mut colour = enumerated_colour(0, ColourSpecification::CIELAB);
            colour.extend_from_slice(parameters);
            let mut header = boxed(IMAGE_HEADER, &image_header(3, 7));
            header.extend_from_slice(&boxed(COLOUR_SPECIFICATION, &colour));
            let data = jp2(&header, &[7, 7, 7]);
            Headers::parse(&data)
                .expect("the file is well formed")
                .colour[0]
                .cielab_illuminant()
        };
        let ranges = [0u8; 24];
        let with = |code: [u8; 4]| {
            let mut parameters = ranges.to_vec();
            parameters.extend_from_slice(&code);
            illuminant(&parameters)
        };
        assert_eq!(illuminant(&[]), Some(Illuminant::D50));
        assert_eq!(illuminant(&ranges), Some(Illuminant::D50));
        for (code, named) in [
            ([0x00, 0x44, 0x35, 0x30], Illuminant::D50),
            ([0x00, 0x44, 0x36, 0x35], Illuminant::D65),
            ([0x00, 0x44, 0x37, 0x35], Illuminant::D75),
            ([0x00, 0x00, 0x53, 0x41], Illuminant::A),
            ([0x00, 0x00, 0x53, 0x43], Illuminant::C),
            ([0x00, 0x00, 0x46, 0x32], Illuminant::F2),
            ([0x00, 0x00, 0x46, 0x37], Illuminant::F7),
            ([0x00, 0x46, 0x31, 0x31], Illuminant::F11),
            // M.11.7.4.1's own example of a colour temperature, 7500 K.
            (
                [0x43, 0x54, 0x1D, 0x4C],
                Illuminant::ColourTemperature(7500),
            ),
            ([1, 2, 3, 4], Illuminant::Unknown([1, 2, 3, 4])),
        ] {
            assert_eq!(with(code), Some(named), "{code:02x?}");
        }
        assert_eq!(
            Illuminant::ColourTemperature(7500).white_point(),
            pdf_colour::planckian::white_point(7500.0)
        );
        assert_eq!(Illuminant::ColourTemperature(0).white_point(), None);
        assert_eq!(Illuminant::Unknown([1, 2, 3, 4]).white_point(), None);

        let mut srgb = enumerated_colour(0, 16);
        srgb.extend_from_slice(&[0; 28]);
        let mut header = boxed(IMAGE_HEADER, &image_header(3, 7));
        header.extend_from_slice(&boxed(COLOUR_SPECIFICATION, &srgb));
        let data = jp2(&header, &[7, 7, 7]);
        let headers = Headers::parse(&data).expect("the file is well formed");
        assert_eq!(
            headers.colour[0].cielab_illuminant(),
            None,
            "sRGB has no illuminant"
        );
    }

    /// Every CIE-derived white point of [`Illuminant::white_point`] is the tristimulus sum of the
    /// CIE's own spectral distribution against its 1931 2° colour-matching functions, at every
    /// wavelength both tables state, `Y` scaled to 1 — recomputed here from `data/cie/`, so a
    /// mistyped digit fails rather than tinting every Lab image under that illuminant.
    ///
    /// D65 is §8.6.5.4's EXAMPLE's rather than a sum, and is held to the CIE's within the four
    /// places the EXAMPLE prints, give or take the CCIR value's last digit.
    #[test]
    fn every_white_point_is_the_cie_datas_own_sum() {
        let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/cie");
        let table = |name: &str| -> Vec<Vec<f64>> {
            std::fs::read_to_string(directory.join(name))
                .unwrap_or_else(|error| panic!("data/cie/{name}: {error}"))
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(|line| {
                    line.trim()
                        .split(',')
                        .map(|field| field.parse::<f64>().expect("a number"))
                        .collect()
                })
                .collect()
        };
        let observer: std::collections::BTreeMap<i64, [f64; 3]> = table("CIE_xyz_1931_2deg.csv")
            .into_iter()
            .map(|row| {
                #[expect(clippy::cast_possible_truncation, reason = "a wavelength in nm")]
                let at = row[0].round() as i64;
                (at, [row[1], row[2], row[3]])
            })
            .collect();
        let white = |name: &str, column: usize| -> [f64; 3] {
            let mut sum = [0.0f64; 3];
            for row in table(name) {
                #[expect(clippy::cast_possible_truncation, reason = "a wavelength in nm")]
                let at = row[0].round() as i64;
                if let Some(functions) = observer.get(&at) {
                    for (axis, function) in functions.iter().enumerate() {
                        sum[axis] += row[column] * function;
                    }
                }
            }
            [sum[0] / sum[1], 1.0, sum[2] / sum[1]]
        };
        for (illuminant, file, column) in [
            (Illuminant::A, "CIE_std_illum_A_1nm.csv", 1),
            (Illuminant::C, "CIE_illum_C.csv", 1),
            (Illuminant::D75, "CIE_illum_D75.csv", 1),
            (Illuminant::F2, "CIE_illum_FLs.csv", 2),
            (Illuminant::F7, "CIE_illum_FLs.csv", 7),
            (Illuminant::F11, "CIE_illum_FLs.csv", 11),
        ] {
            let stated = illuminant.white_point().expect("a standard illuminant");
            let summed = white(file, column);
            for axis in 0..3 {
                assert!(
                    (f64::from(stated[axis]) - summed[axis]).abs() < 6e-6,
                    "{illuminant:?} axis {axis}: {stated:?} against the data's {summed:?}"
                );
            }
        }
        let example = Illuminant::D65
            .white_point()
            .expect("a standard illuminant");
        let summed = white("CIE_std_illum_D65.csv", 1);
        for axis in 0..3 {
            assert!(
                (f64::from(example[axis]) - summed[axis]).abs() < 2e-4,
                "D65 axis {axis}: {example:?} against the data's {summed:?}"
            );
        }
    }

    /// The file type box and the order of the top-level boxes, I.5.2 and T.801 M.9.2.7.
    #[test]
    fn the_file_type_and_the_box_order_are_read() {
        let mut header = boxed(IMAGE_HEADER, &image_header(3, 7));
        header.extend_from_slice(&boxed(COLOUR_SPECIFICATION, &enumerated_colour(0, 16)));
        let data = jp2(&header, &[7, 7, 7]);

        let headers = Headers::parse(&data).expect("the file is well formed");
        let file_type = headers.file_type.expect("an ftyp box was stated");
        assert_eq!(file_type.brand, *b"jp2 ");
        assert_eq!(file_type.compatibility, vec![*b"jp2 "]);
        assert_eq!(
            headers.top_level,
            vec![*b"jP  ", FILE_TYPE, JP2_HEADER, CODESTREAM]
        );
    }

    /// A Fragment Table box's list, T.801 M.11.3.1.
    #[test]
    fn a_fragment_table_is_read_for_its_fragments() {
        let mut list = 2u16.to_be_bytes().to_vec();
        for (offset, length, reference) in [(100u64, 20u32, 0u16), (400, 30, 1)] {
            list.extend_from_slice(&offset.to_be_bytes());
            list.extend_from_slice(&length.to_be_bytes());
            list.extend_from_slice(&reference.to_be_bytes());
        }
        let mut data = jp2(&boxed(IMAGE_HEADER, &image_header(3, 7)), &[7, 7, 7]);
        data.extend_from_slice(&boxed(FRAGMENT_TABLE, &boxed(FRAGMENT_LIST, &list)));

        let headers = Headers::parse(&data).expect("the file is well formed");
        assert_eq!(headers.fragment_lists.len(), 1);
        assert_eq!(headers.fragment_lists[0].container, FRAGMENT_TABLE);
        assert_eq!(
            headers.fragment_lists[0].fragments[1],
            Fragment {
                offset: 400,
                length: 30,
                reference: 1
            }
        );
    }

    /// The first Compositing Layer Header box's colour group and registration, T.801 M.11.7.
    #[test]
    fn the_first_layer_states_its_colour_and_its_codestreams() {
        let group = boxed(
            COLOUR_GROUP,
            &boxed(COLOUR_SPECIFICATION, &enumerated_colour(1, 3)),
        );
        let mut registration = vec![0, 1, 0, 1];
        for stream in [0u16, 2] {
            registration.extend_from_slice(&stream.to_be_bytes());
            registration.extend_from_slice(&[1, 1, 0, 0]);
        }
        let mut layer = group;
        layer.extend_from_slice(&boxed(REGISTRATION, &registration));
        let mut data = jp2(&boxed(IMAGE_HEADER, &image_header(3, 7)), &[7, 7, 7]);
        data.extend_from_slice(&boxed(LAYER_HEADER, &layer));

        let headers = Headers::parse(&data).expect("the file is well formed");
        let first = headers.first_layer.expect("a jplh box was stated");
        assert_eq!(first.colour[0].enumerated, Some(3));
        assert_eq!(first.codestreams, Some(vec![0, 2]));
    }

    /// What the JP2 Header box and the first header boxes hold, and where each top-level box
    /// lies, T.801 M.9.2.6 and M.9.2.7.
    #[test]
    fn header_contents_and_box_spans_are_read() {
        let mut reference = COLOUR_GROUP.to_vec();
        let mut list = 1u16.to_be_bytes().to_vec();
        list.extend_from_slice(&0u64.to_be_bytes());
        list.extend_from_slice(&8u32.to_be_bytes());
        list.extend_from_slice(&0u16.to_be_bytes());
        reference.extend_from_slice(&boxed(FRAGMENT_LIST, &list));
        let mut codestream_header = boxed(BITS_PER_COMPONENT, &[7, 7, 7]);
        codestream_header.extend_from_slice(&boxed(CROSS_REFERENCE, &reference));
        let layer = boxed(
            COLOUR_GROUP,
            &boxed(COLOUR_SPECIFICATION, &enumerated_colour(1, 16)),
        );
        let mut data = jp2(&boxed(IMAGE_HEADER, &image_header(3, 7)), &[7, 7, 7]);
        data.extend_from_slice(&boxed(CODESTREAM_HEADER, &codestream_header));
        data.extend_from_slice(&boxed(LAYER_HEADER, &layer));

        let headers = Headers::parse(&data).expect("the file is well formed");
        assert_eq!(headers.jp2_header_contents, vec![IMAGE_HEADER]);
        assert_eq!(
            headers.first_codestream_header,
            Some(vec![BITS_PER_COMPONENT, COLOUR_GROUP])
        );
        assert_eq!(
            headers.first_layer.expect("a jplh box was stated").contents,
            vec![COLOUR_GROUP, COLOUR_SPECIFICATION]
        );
        assert_eq!(headers.top_level_spans.len(), headers.top_level.len());
        assert_eq!(headers.top_level_spans[0].start, 0);
        assert_eq!(
            headers.top_level_spans.last().map(|span| span.end),
            Some(data.len())
        );
        let list = &headers.fragment_lists[0];
        assert_eq!(list.container, CROSS_REFERENCE);
        assert_eq!(headers.top_level[list.box_index], CODESTREAM_HEADER);
    }

    /// Every `colr` box but the kept one becomes a Free box, and nothing moves.
    #[test]
    fn keeping_one_colour_specification_frees_the_others() {
        let mut header = boxed(IMAGE_HEADER, &image_header(3, 7));
        header.extend_from_slice(&boxed(COLOUR_SPECIFICATION, &enumerated_colour(0, 19)));
        header.extend_from_slice(&boxed(COLOUR_SPECIFICATION, &[3, 0, 0, 0xAA]));
        let data = jp2(&header, &[7, 7, 7]);

        let headers = Headers::parse(&data).expect("the file is well formed");
        let kept =
            headers.keeping_colour(&data, Some(1), Some(ColourSpecification::RESTRICTED_ICC));
        assert_eq!(kept.len(), data.len());
        let again = Headers::parse(&kept).expect("the copy is well formed");
        assert_eq!(again.colour.len(), 1);
        assert_eq!(again.colour[0].method, ColourSpecification::RESTRICTED_ICC);
        assert_eq!(again.colour[0].profile, Some(&[0xAA][..]));

        let none = headers.keeping_colour(&data, None, None);
        assert!(
            Headers::parse(&none)
                .expect("well formed")
                .colour
                .is_empty()
        );
    }

    /// `Rsiz`'s extended bits, T.801 Table A.2, and the `MCC` and `MCO` segments A.3.8 and A.3.9.
    #[test]
    fn the_extended_capabilities_and_the_component_transforms_are_read() {
        let mut stream = codestream(&[7, 7, 7]);
        stream[6..8].copy_from_slice(&0x8104u16.to_be_bytes());
        // MCC: Zmcc 0, Imcc 1, Ymcc 0, Qmcc 1; Xmcc 1; Nmcc 3 with 8-bit indices; Mmcc 3; Tmcc.
        let mut mcc = vec![
            0, 0, 1, 0, 0, 0, 1, 1, 0, 3, 0, 1, 2, 0, 3, 0, 1, 2, 0, 0, 1,
        ];
        let length = u16::try_from(mcc.len() + 2).expect("small");
        let mut segment = MCC.to_vec();
        segment.extend_from_slice(&length.to_be_bytes());
        segment.append(&mut mcc);
        segment.extend_from_slice(&[0xFF, 0x77, 0, 4, 1, 1]);
        segment.extend_from_slice(&[0xFF, 0x90, 0, 10, 0, 0, 0, 0, 0, 0, 0, 1, 0xFF, 0x93]);
        stream.extend_from_slice(&segment);

        let codestream = parse_codestream(&stream).expect("well formed");
        assert_eq!(
            codestream.required_extensions(),
            vec![MULTIPLE_COMPONENT_TRANSFORMATION]
        );
        assert_eq!(
            codestream.collections,
            vec![Collection {
                count: 1,
                kind: Some(Collection::DECORRELATION),
                reversible: Some(false),
            }]
        );
        assert_eq!(codestream.orderings, vec![1]);
    }

    /// I.4's `LBox` of 0 makes a box run to the end of the data, and the walk still terminates.
    #[test]
    fn a_box_of_unknown_length_runs_to_the_end() {
        let header = boxed(IMAGE_HEADER, &image_header(3, 7));
        let mut data = boxed(*b"jP  ", &[0x0D, 0x0A, 0x87, 0x0A]);
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&JP2_HEADER);
        data.extend_from_slice(&header);

        let headers = Headers::parse(&data).expect("the file is well formed");
        assert_eq!(headers.image.map(|image| image.components), Some(3));
    }

    /// A box claiming more bytes than the data holds is a length error, not a silent stop.
    #[test]
    fn a_box_longer_than_the_data_is_refused() {
        let mut data = boxed(*b"jP  ", &[0x0D, 0x0A, 0x87, 0x0A]);
        data.extend_from_slice(&1000u32.to_be_bytes());
        data.extend_from_slice(&JP2_HEADER);

        assert!(matches!(
            Headers::parse(&data),
            Err(HeaderError::BoxLength { .. })
        ));
    }

    /// I.4 reserves `LBox` values 2 through 7, and a box stating one is malformed.
    #[test]
    fn a_reserved_box_length_is_refused() {
        let mut data = boxed(*b"jP  ", &[0x0D, 0x0A, 0x87, 0x0A]);
        data.extend_from_slice(&4u32.to_be_bytes());
        data.extend_from_slice(&JP2_HEADER);

        assert!(matches!(
            Headers::parse(&data),
            Err(HeaderError::BoxLength { length: 4, .. })
        ));
    }

    /// Bytes that are neither a box nor a codestream are refused rather than read as empty.
    #[test]
    fn arbitrary_bytes_are_not_jpeg_2000() {
        assert_eq!(
            Headers::parse(b"not a JPEG 2000 file at all"),
            Err(HeaderError::NotJpeg2000)
        );
        assert_eq!(Headers::parse(&[]), Err(HeaderError::NotJpeg2000));
        assert_eq!(Headers::parse(&[0xFF, 0x4F]), Err(HeaderError::NoSizMarker));
    }

    /// An `ihdr` box too short for Table I-5's fields is an error rather than a guess.
    #[test]
    fn a_truncated_image_header_is_refused() {
        let header = boxed(IMAGE_HEADER, &[0, 0, 0]);
        let data = jp2(&header, &[7]);
        assert!(matches!(
            Headers::parse(&data),
            Err(HeaderError::ShortBox { .. })
        ));
    }

    /// Tables I-6 and A-11: the low seven bits are the depth minus one, the high bit the sign.
    #[test]
    fn the_depth_encoding_is_the_one_both_tables_state() {
        assert_eq!(
            Depth::from_byte(0),
            Depth {
                bits: 1,
                signed: false
            }
        );
        assert_eq!(
            Depth::from_byte(0x25),
            Depth {
                bits: 38,
                signed: false
            }
        );
        assert_eq!(
            Depth::from_byte(0xA5),
            Depth {
                bits: 38,
                signed: true
            }
        );
        // 0x28 is the value the veraPDF witness for the bit-depth rule states, and 41 is the
        // depth it means — outside the 1 to 38 the tables define.
        assert_eq!(Depth::from_byte(0x28).bits, 41);
    }
}
