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
//! Not read: the contents of `pclr` and `cmap` (I.5.3.4, I.5.3.5), which decide how codestream
//! components become channels through a palette, and every marker segment after `SIZ`. Whether a
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
//! - **ISO/IEC 15444-2 is not held by this project**, so nothing here knows what the JPX
//!   baseline feature set is, and the meaning of a `METH` value of 3 is left unstated rather
//!   than guessed: part 1's Table I-9 defines 1 and 2 and reserves the rest.

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

/// The `SOC` marker that opens a codestream, Table A-2.
const SOC: [u8; 2] = [0xFF, 0x4F];
/// The `SIZ` marker, which A.5.1 requires immediately after `SOC`.
const SIZ: [u8; 2] = [0xFF, 0x51];

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

/// One `colr` box, ISO/IEC 15444-1:2000 I.5.3.3.
///
/// A JP2 file holds at least one and may hold several, each describing the same colourspace by a
/// different method. Which one a *reader* uses is not this type's business: I.5.3.3 tells a
/// conforming JP2 reader to use the first, and other standards layered over JP2 say otherwise,
/// so all of them are kept and the choice is the caller's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColourSpecification<'a> {
    /// `METH`, the specification method. Table I-9 defines 1 and 2 and reserves the rest.
    pub method: u8,
    /// `PREC`, the precedence, which I.5.3.3 reserves and sets to zero.
    pub precedence: i8,
    /// `APPROX`, how well this specification approximates the intended colourspace.
    pub approximation: u8,
    /// `EnumCS`, present only where `METH` is 1.
    ///
    /// Table I-10 gives 16 (sRGB) and 17 (greyscale) as the values a conforming first edition
    /// file may state in its first `colr` box, and reserves the rest for later ISO use — which
    /// later work then assigned, so a number outside that pair is unknown here rather than
    /// wrong.
    pub enumerated: Option<u32>,
    /// `PROFILE`, the embedded ICC profile, present only where `METH` is 2.
    ///
    /// Borrowed rather than copied: a profile runs to hundreds of kilobytes, and a caller that
    /// wants it parsed hands these bytes to [`crate::icc::Profile::parse`].
    pub profile: Option<&'a [u8]>,
    /// Everything after `APPROX` that this module did not read as one of the two fields above.
    ///
    /// Non-empty only for a `METH` value part 1 reserves. Table I-9 says a conforming JP2 reader
    /// shall ignore such a box entirely, and this module keeps the bytes instead of discarding
    /// them because ISO 32000-2 §7.4.9 admits methods part 1 does not define.
    pub reserved: &'a [u8],
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
    /// `Rsiz`, the capabilities a decoder needs. Table A-10 defines only zero.
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
            match found.kind {
                JP2_HEADER => headers.read_jp2_header(found.payload)?,
                CODESTREAM if headers.codestream.is_none() => {
                    headers.codestream = Some(parse_codestream(found.payload)?);
                }
                _ => {}
            }
            next = next_box(data, found.end)?;
        }
        Ok(headers)
    }

    /// Reads the boxes inside the `jp2h` superbox, I.5.3.
    ///
    /// Iterative rather than recursive, and only one level deep, because I.5.3 names every box
    /// that may appear here and none of them is itself a superbox — so there is no depth for a
    /// hostile file to exhaust.
    fn read_jp2_header(&mut self, payload: &'a [u8]) -> Result<(), HeaderError> {
        let mut at = 0usize;
        while let Some(found) = next_box(payload, at)? {
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
                COLOUR_SPECIFICATION => self.colour.push(parse_colour(found.payload)?),
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

/// One `colr` box, I.5.3.3 Table I-11.
fn parse_colour(payload: &[u8]) -> Result<ColourSpecification<'_>, HeaderError> {
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

    // Table I-9 makes the remainder of the box a function of `METH`, and defines only two
    // values. A method it reserves leaves the remainder uninterpreted rather than guessed —
    // ISO 32000-2 §7.4.9 admits methods this edition does not define, and inventing a reading
    // for them here would be reading a standard this project does not hold.
    let (enumerated, profile) = match method {
        1 => {
            let value = rest
                .get(..ENUMERATED)
                .and_then(|bytes| <[u8; 4]>::try_from(bytes).ok())
                .map(u32::from_be_bytes)
                .ok_or_else(|| HeaderError::ShortBox {
                    name: name_of(COLOUR_SPECIFICATION),
                    held: payload.len(),
                    needed: NEEDED.saturating_add(ENUMERATED),
                })?;
            (Some(value), None)
        }
        2 => (None, Some(rest)),
        _ => (None, None),
    };

    Ok(ColourSpecification {
        method,
        // Signed, alone among the three fixed fields, which is I.5.3.3's own choice.
        precedence: i8::from_be_bytes([fields[1]]),
        approximation: fields[2],
        enumerated,
        profile,
        reserved: if enumerated.is_none() && profile.is_none() {
            rest
        } else {
            &[]
        },
    })
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
    })
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

    /// Table I-9 reserves every `METH` but 1 and 2, so nothing is invented for those bytes.
    #[test]
    fn a_reserved_method_leaves_its_remainder_uninterpreted() {
        let mut header = boxed(IMAGE_HEADER, &image_header(3, 7));
        header.extend_from_slice(&boxed(COLOUR_SPECIFICATION, &[3, 0, 1, 0xAA, 0xBB]));
        let data = jp2(&header, &[7, 7, 7]);

        let headers = Headers::parse(&data).expect("the file is well formed");
        assert_eq!(headers.colour[0].method, 3);
        assert_eq!(headers.colour[0].enumerated, None);
        assert_eq!(headers.colour[0].profile, None);
        assert_eq!(headers.colour[0].reserved, &[0xAA, 0xBB]);
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
