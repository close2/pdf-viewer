//! How much of an image a magnified view actually shows, and how much was decoded to show it.
//!
//! Written to decide whether decoding *a region* of a huge image is worth building, and it
//! decided against (ADR 0373). Reduced resolution — §7.4.9 NOTE 3, ADRs 0233 and 0321 — answers
//! the zoomed-*out* half: the whole image on a screen with fewer pixels than the image has
//! samples. The other half is a reader magnifying a 212-megapixel scan, where the grid asks for
//! every sample and a window shows a fraction of a percent of them. That half is only worth
//! building if the population is more than one file, and this is the instrument that says.
//!
//! It is kept rather than being a round's scratch program because the refusal is only as good as
//! its population: a JPEG 2000 codestream of 64 megapixels that states a tile or precinct
//! partition would reopen the question, and this is the command that would find one.
//!
//! ```sh
//! cargo run --release -p pdf-model --example image_region_census -- doc/pdf.js/test/pdfs/*.pdf
//! ```
//!
//! # Two columns, because two questions have two different denominators
//!
//! **The stated side** reads every image `XObject`'s dictionary — `/Width`, `/Height` and the
//! codec at the end of its `/Filter` chain — for every object in the file, whichever page draws
//! it, and decodes nothing. That is the honest denominator for *how many large images exist*,
//! and it is cheap enough to run over a corpus of tens of thousands.
//!
//! **The drawn side** interprets **page one** and reads the placement out of the display list,
//! which is the only place a placement exists: ISO 32000-2 puts every image on the unit square
//! (§8.9.5.1), so how large it is drawn is the `Do`'s transform and nothing in the dictionary.
//! One page per document is a sample rather than a census of placements — a scan repeats its
//! page, so it is representative there, and a document whose big image is on page nine is
//! missed. The stated side is what says how much the sample misses.
//!
//! The two sides are joined by the grid, because a `Command::Image` holds a raster and not the
//! object it came from. A grid that names no dictionary — an inline image, or a JPEG 2000
//! codestream decoded at a reduced level, whose raster is deliberately not the grid the
//! dictionary states — is reported as `?` rather than attributed by guesswork.
//!
//! # What "the view needs" means here, and why the maximum magnification is the number used
//!
//! At magnification *z* the placement covers [`Grid::for_placement`]'s device grid — the same
//! function the backends use, so this census cannot disagree with the renderer about what "at
//! device resolution" means. Of that grid a window shows at most its own size, so the samples a
//! view needs are
//!
//! ```text
//! needed(z) = Π over both axes of  min(samples, device) × min(1, viewport ÷ device)
//! ```
//!
//! and what is decoded today is the whole raster, once, held in the display list. The ratio of
//! the two is the waste this item is about.
//!
//! `needed` is largest where the image exactly fills the window and falls away on both sides of
//! that, so the *worst* case is the largest magnification a person can reach — `viewer_core`'s
//! `ZOOM_RANGE`, which ends at 64. Quoting the waste there is deliberately the strongest case
//! the proposal can be given: at any zoom a reader is likelier to use, it is smaller.
//!
//! # The codec is reported because the proposal is one codec's
//!
//! Tiles, precincts and packets make a region decodable in JPEG 2000 without decoding the rest.
//! No other codec Table 6 names offers it: a `DCTDecode` or a `FlateDecode` raster has to be
//! decoded before a sample has a position at all. So a population that is large but holds no
//! JPEG 2000 is an argument for a different construction, not for this one.

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::{BTreeMap, BTreeSet};

use pdf_model::{Pages, interpret};
use pdf_render::{Command, Grid, ImageSource, Transform};
use pdf_syntax::{Dictionary, Document, Object, ObjectId};

/// The window `pdf-viewer` opens at, in device pixels.
///
/// `viewer-ui`'s `window.rs` asks winit for 800×1000 logical pixels. A larger window shows
/// proportionally more of a magnified image and moves every ratio below by that factor; it does
/// not move an image between buckets, because the buckets are orders of magnitude apart.
const VIEWPORT: (f32, f32) = (800.0, 1000.0);

/// The largest magnification a reader can reach — `viewer_core::open`'s `ZOOM_RANGE`.
const MAX_ZOOM: f32 = 64.0;

/// The waste at which an image is counted as one a region decode would help.
///
/// Sixteen decoded samples per sample shown, which is four magnifications past the point where
/// the image fills the window. Nothing hangs on the exact figure: the population's waste at
/// [`MAX_ZOOM`] spans several orders of magnitude, so the count moves by a handful either way.
const WASTEFUL: f64 = 16.0;

/// The largest of [`MEGAPIXELS`], which is the size class the codecs are broken out for.
const HUGE_MEGAPIXELS: u64 = 64;

/// Megapixel thresholds the stated side is bucketed by.
const MEGAPIXELS: [u64; 4] = [1, 8, HUGE_MEGAPIXELS, 256];

/// How many of the largest images are named individually.
const LARGEST_NAMED: usize = 20;

/// One image `XObject`'s dictionary, read without decoding anything.
struct Stated {
    /// `/Width` × `/Height`.
    samples: u64,
    /// The last name in each such stream's `/Filter` chain, or `none` where a chain leaves
    /// samples.
    ///
    /// A set rather than one name because a grid can be stated by more than one stream — an
    /// image and the `/SMask` that belongs with it are two `XObject`s of one grid, and often of
    /// two codecs. Both have to be decoded, so both are the answer rather than an ambiguity.
    codecs: BTreeSet<String>,
}

impl Stated {
    /// The codecs, as one field.
    fn codec(&self) -> String {
        self.codecs.iter().cloned().collect::<Vec<_>>().join("+")
    }
}

/// One image the display list draws, with the placement that decides what a view of it needs.
///
/// One entry per *raster grid* per document rather than per command: `22060_A1_01_Plans.pdf`
/// runs `Do` on one 6.5-megapixel photograph thirteen times, and thirteen rows of one image
/// would make a population out of a page. [`Self::draws`] keeps the multiplicity, which is worth
/// having on its own — the interpreter decodes per `Do`.
struct Drawn {
    /// The document's file name.
    name: String,
    /// The raster the display list holds — after a JPEG 2000 reduction, where there was one.
    raster: (u32, u32),
    /// The device grid the largest of its placements asks for at magnification 1.
    ///
    /// The largest, because that is the placement whose magnified view shows the smallest share
    /// of the image, and this census is asked for the strongest case.
    placed: Grid,
    /// The codec the stated side names for this grid, or `?` where none matched.
    codec: String,
    /// Samples decoded to draw it once.
    decoded: f64,
    /// Samples a window shows at [`MAX_ZOOM`], under [`Self::placed`].
    needed: f64,
    /// How many painting operations on this page draw it.
    draws: usize,
}

impl Drawn {
    /// Decoded samples per sample a magnified view shows.
    fn waste(&self) -> f64 {
        self.decoded / self.needed
    }

    /// Samples a region decode would not have produced.
    ///
    /// The ratio alone ranks a 400×400 bitmap alongside a 200-megapixel scan, because at 64×
    /// magnification a window shows a couple of hundred samples of either. What decides whether
    /// a construction is worth building is the absolute quantity, which is this.
    fn saved(&self) -> f64 {
        (self.decoded - self.needed).max(0.0)
    }

    /// The magnification at which the tighter axis reaches one sample per device pixel.
    ///
    /// Beyond it a reader is magnifying samples rather than asking for new ones, which is where
    /// the region question is sharpest. A value over [`MAX_ZOOM`] means the viewer cannot get
    /// there at all and the full grid is never asked for — the case reduced resolution already
    /// answers.
    fn zoom_to_one_to_one(&self) -> f64 {
        let across = f64::from(self.raster.0) / f64::from(self.placed.width);
        let down = f64::from(self.raster.1) / f64::from(self.placed.height);
        across.max(down)
    }
}

/// The device grid `placement` asks for at magnification `zoom`.
///
/// Scaling the placement and asking [`Grid::for_placement`] is the renderer's own decision
/// rather than a restatement of it: a backend draws a magnified page by scaling exactly this
/// transform.
fn device_grid(placement: Transform, zoom: f32) -> Grid {
    Grid::for_placement(Transform {
        a: placement.a * zoom,
        b: placement.b * zoom,
        c: placement.c * zoom,
        d: placement.d * zoom,
        e: placement.e * zoom,
        f: placement.f * zoom,
    })
}

/// Samples of `raster` a [`VIEWPORT`]-sized window shows when the image is drawn on `device`.
///
/// Never below one sample: a placement collapsed to a line still shows something, and a
/// denominator of zero would name an infinite waste for an image nobody can see.
fn needed(raster: (u32, u32), device: Grid) -> f64 {
    fn axis(samples: u32, device: u32, viewport: f32) -> f64 {
        let device = f64::from(device);
        let across = f64::from(samples).min(device);
        let visible = (f64::from(viewport) / device).min(1.0);
        across * visible
    }
    (axis(raster.0, device.width, VIEWPORT.0) * axis(raster.1, device.height, VIEWPORT.1)).max(1.0)
}

/// The last filter in a stream's `/Filter` chain, which is the codec where there is one.
fn codec_of(document: &Document, dict: &Dictionary) -> String {
    let filter = document.get_key(dict, "Filter");
    let last = match &filter {
        Object::Name(name) => Some(name.as_bytes().to_vec()),
        Object::Array(items) => items
            .last()
            .map(|item| document.resolve(item))
            .and_then(|item| item.as_name().map(|name| name.as_bytes().to_vec())),
        _ => None,
    };
    last.map_or_else(
        || "none".to_owned(),
        |name| String::from_utf8_lossy(&name).into_owned(),
    )
}

/// What a JPEG 2000 codestream says about the two cheap forms of random access.
///
/// ISO/IEC 15444-1 offers a decoder two ways to reach part of an image without decoding the
/// rest, and a codestream has to *state* them: a tile partition (A.5.1's SIZ) makes each tile
/// independently decodable, and a precinct partition (A.6.1's COD) makes a packet cover a
/// rectangle of a sub-band instead of the whole of it. One tile and no precincts means one
/// packet per (layer, resolution, component) over the whole image, and then there is no offset
/// in the codestream that says where a corner's coefficients begin.
struct Codestream {
    /// The reference grid, A.5.1's `Xsiz` − `XOsiz` by `Ysiz` − `YOsiz`.
    grid: (u32, u32),
    /// How many tiles the partition makes, across and down.
    tiles: (u32, u32),
    /// Whether A.6.1's `Scod` states a precinct partition.
    precincts: bool,
    /// A.6.1's progression order.
    progression: &'static str,
    /// Decomposition levels, which is how far the resolution progression reaches.
    levels: u8,
}

/// Reads two big-endian bytes at `at`.
fn be_u16(data: &[u8], at: usize) -> Option<u16> {
    let end = at.checked_add(2)?;
    Some(u16::from_be_bytes(data.get(at..end)?.try_into().ok()?))
}

/// Reads four big-endian bytes at `at`.
fn be_u32(data: &[u8], at: usize) -> Option<u32> {
    let end = at.checked_add(4)?;
    Some(u32::from_be_bytes(data.get(at..end)?.try_into().ok()?))
}

/// The codestream inside a JP2 container, or `data` itself where it is already one.
///
/// A `/JPXDecode` stream is either a raw codestream — which starts with A.4.1's SOC marker —
/// or ISO/IEC 15444-1 Annex I's JP2 file, whose boxes carry it in a `jp2c`. Only the top level
/// is walked, which is where I.5.4 puts the contiguous codestream box.
fn codestream_of(data: &[u8]) -> Option<&[u8]> {
    if data.starts_with(&[0xFF, 0x4F]) {
        return Some(data);
    }
    let mut at = 0_usize;
    while let Some(length) = be_u32(data, at) {
        let kind = data.get(at.checked_add(4)?..at.checked_add(8)?)?;
        let body = at.checked_add(8)?;
        if kind == b"jp2c" {
            return data.get(body..);
        }
        // A length of zero means the box runs to the end of the file, and one of 1 means the
        // real length is a 64-bit field this walk does not need to follow: neither can be
        // stepped over, so the walk stops rather than guessing.
        let length = usize::try_from(length).ok()?;
        if length <= 8 {
            return None;
        }
        at = at.checked_add(length)?;
    }
    None
}

/// What the codestream's SIZ and COD marker segments state.
fn markers(data: &[u8]) -> Option<Codestream> {
    let data = codestream_of(data)?;
    let mut at = 2_usize;
    let mut grid = None;
    let mut tiles = None;
    let mut coding = None;
    // A.4.2's SOT ends the main header, and everything this reads is in front of it.
    while let Some(marker) = be_u16(data, at) {
        if marker == 0xFF90 || marker == 0xFF93 {
            break;
        }
        let length = usize::from(be_u16(data, at.checked_add(2)?)?);
        let segment = at.checked_add(4)?;
        match marker {
            // A.5.1's SIZ: the reference grid, then the tile the partition repeats.
            0xFF51 => {
                let image = (
                    be_u32(data, segment.checked_add(2)?)?,
                    be_u32(data, segment.checked_add(6)?)?,
                );
                let offset = (
                    be_u32(data, segment.checked_add(10)?)?,
                    be_u32(data, segment.checked_add(14)?)?,
                );
                let tile = (
                    be_u32(data, segment.checked_add(18)?)?,
                    be_u32(data, segment.checked_add(22)?)?,
                );
                let tile_offset = (
                    be_u32(data, segment.checked_add(26)?)?,
                    be_u32(data, segment.checked_add(30)?)?,
                );
                grid = Some((
                    image.0.saturating_sub(offset.0),
                    image.1.saturating_sub(offset.1),
                ));
                tiles = Some((
                    image
                        .0
                        .saturating_sub(tile_offset.0)
                        .div_ceil(tile.0.max(1)),
                    image
                        .1
                        .saturating_sub(tile_offset.1)
                        .div_ceil(tile.1.max(1)),
                ));
            }
            // A.6.1's COD: the coding style, whose Scod bit 0 states a precinct partition.
            0xFF52 => {
                let style = *data.get(segment)?;
                let progression = match data.get(segment.checked_add(1)?)? {
                    0 => "LRCP",
                    1 => "RLCP",
                    2 => "RPCL",
                    3 => "PCRL",
                    4 => "CPRL",
                    _ => "?",
                };
                coding = Some((
                    style & 1 == 1,
                    progression,
                    *data.get(segment.checked_add(5)?)?,
                ));
            }
            _ => {}
        }
        at = at.checked_add(2)?.checked_add(length)?;
    }
    let (grid, tiles) = (grid?, tiles?);
    let (precincts, progression, levels) = coding?;
    Some(Codestream {
        grid,
        tiles,
        precincts,
        progression,
        levels,
    })
}

/// Every image `XObject` in the file, by the grid it states.
fn stated(
    document: &Document,
    name: &str,
    codestreams: &mut Vec<(String, Codestream)>,
) -> BTreeMap<(u32, u32), Stated> {
    let mut by_grid: BTreeMap<(u32, u32), Stated> = BTreeMap::new();
    for number in document.xref().object_numbers() {
        let object = document.get(ObjectId {
            number,
            generation: 0,
        });
        let Some(stream) = object.as_stream() else {
            continue;
        };
        let dict = &stream.dict;
        if document
            .get_key(dict, "Subtype")
            .as_name()
            .is_none_or(|name| name.as_bytes() != b"Image")
        {
            continue;
        }
        let dimension = |key: &str| {
            document
                .get_key(dict, key)
                .as_integer()
                .and_then(|value| u32::try_from(value).ok())
                .unwrap_or(0)
        };
        let (width, height) = (dimension("Width"), dimension("Height"));
        if width == 0 || height == 0 {
            continue;
        }
        let codec = codec_of(document, dict);
        // Only for JPEG 2000, and only through `image_stream`, which applies the filters in
        // front of the codec and stops there: the markers are in the codestream's own bytes,
        // and inflating every `FlateDecode` raster to find out it is not one would make this
        // census cost what a decode costs.
        if codec == "JPXDecode"
            && let Some(image) = document.image_stream(stream)
            && let Some(codestream) = markers(&image.data)
        {
            codestreams.push((name.to_owned(), codestream));
        }
        by_grid
            .entry((width, height))
            .or_insert(Stated {
                samples: u64::from(width).saturating_mul(u64::from(height)),
                codecs: BTreeSet::new(),
            })
            .codecs
            .insert(codec);
    }
    by_grid
}

/// Every image command in `commands`, including the ones inside transparency groups.
fn images<'a>(commands: &'a [Command], into: &mut Vec<(&'a ImageSource, Transform)>) {
    for command in commands {
        match command {
            Command::Image {
                image, transform, ..
            } => into.push((image, *transform)),
            Command::Group { commands, .. } => images(commands, into),
            _ => {}
        }
    }
}

/// What the stated side accumulates over the documents it is given.
#[derive(Default)]
struct StatedTally {
    /// Image `XObject`s of a distinct grid, over every document.
    images: u64,
    /// How many of them are at or above each [`MEGAPIXELS`] threshold.
    buckets: [u64; MEGAPIXELS.len()],
    /// Which codecs the ones at or above [`HUGE_MEGAPIXELS`] state.
    codecs: BTreeMap<String, u64>,
    /// The largest few, by sample count: (samples, document, codec, grid).
    largest: Vec<(u64, String, String, (u32, u32))>,
}

impl StatedTally {
    /// Adds one document's image dictionaries.
    fn add(&mut self, name: &str, by_grid: &BTreeMap<(u32, u32), Stated>) {
        for (grid, image) in by_grid {
            self.images = self.images.saturating_add(1);
            for (bucket, threshold) in self.buckets.iter_mut().zip(MEGAPIXELS) {
                if image.samples >= threshold.saturating_mul(1_000_000) {
                    *bucket = bucket.saturating_add(1);
                }
            }
            if image.samples >= HUGE_MEGAPIXELS.saturating_mul(1_000_000) {
                let seen = self.codecs.entry(image.codec()).or_default();
                *seen = seen.saturating_add(1);
            }
            self.largest
                .push((image.samples, name.to_owned(), image.codec(), *grid));
        }
        self.largest.sort_by_key(|entry| std::cmp::Reverse(entry.0));
        self.largest.truncate(LARGEST_NAMED);
    }

    /// Prints what it holds.
    fn report(&self) {
        println!("stated: every image XObject in every object, decoded from nothing");
        println!("  {} image(s) of a distinct grid", self.images);
        for (bucket, threshold) in self.buckets.iter().zip(MEGAPIXELS) {
            println!("  {bucket} at or above {threshold} Mpx");
        }
        println!(
            "  codecs at or above {HUGE_MEGAPIXELS} Mpx: {:?}",
            self.codecs
        );
        println!("  largest:");
        for (samples, name, codec, grid) in &self.largest {
            println!(
                "    {name}: {}x{} = {samples} samples, {codec}",
                grid.0, grid.1
            );
        }
    }
}

/// The images page one draws, with what each of them costs and what a view of it shows.
///
/// Answers whether there was a page to interpret at all, which is what the caller counts.
fn drawn_on_page_one(
    document: &Document,
    name: &str,
    by_grid: &BTreeMap<(u32, u32), Stated>,
    into: &mut Vec<Drawn>,
    deferred: &mut usize,
    unread: &mut usize,
) -> bool {
    let Some(page) = Pages::new(document).get(0) else {
        return false;
    };
    let interpretation = interpret(document, &page);
    let mut found = Vec::new();
    images(interpretation.display_list.commands(), &mut found);
    let mut by_raster: BTreeMap<(u32, u32), Drawn> = BTreeMap::new();
    for (source, placement) in found {
        // A deferred source is a base image and a mask combined where the scale is known (ADR
        // 0210). Asking it for a one-sample grid gives the base's own grid back, because the
        // combination is on the finer of the two — which is the raster the decode produced,
        // and so the number this census is about.
        let raster = match source {
            ImageSource::Decoded(image) => (image.width, image.height),
            ImageSource::AtDeviceScale(image) => {
                *deferred = deferred.saturating_add(1);
                let combined = image.samples(Grid {
                    width: 1,
                    height: 1,
                });
                (combined.width, combined.height)
            }
            // `ImageSource` is `#[non_exhaustive]`. A variant added after this census was
            // written is counted rather than guessed at, so that a number printed here never
            // quietly stops covering what the display list holds.
            _ => {
                *unread = unread.saturating_add(1);
                continue;
            }
        };
        let placed = device_grid(placement, 1.0);
        let entry = by_raster.entry(raster).or_insert_with(|| Drawn {
            name: name.to_owned(),
            raster,
            placed,
            codec: by_grid
                .get(&raster)
                .map_or_else(|| "?".to_owned(), Stated::codec),
            decoded: f64::from(raster.0) * f64::from(raster.1),
            needed: f64::INFINITY,
            draws: 0,
        });
        entry.draws = entry.draws.saturating_add(1);
        if u64::from(placed.width).saturating_mul(u64::from(placed.height))
            >= u64::from(entry.placed.width).saturating_mul(u64::from(entry.placed.height))
        {
            entry.placed = placed;
        }
        entry.needed = entry
            .needed
            .min(needed(raster, device_grid(placement, MAX_ZOOM)));
    }
    into.extend(by_raster.into_values());
    true
}

/// Prints the drawn side: what page one's images cost against what a magnified view shows.
fn report_drawn(drawn: &mut [Drawn], deferred: usize, unread: usize) {
    println!(
        "drawn: page one's images, against an {}x{} window at {MAX_ZOOM}x",
        VIEWPORT.0, VIEWPORT.1
    );
    let operations: usize = drawn.iter().map(|image| image.draws).sum();
    println!(
        "  {} raster(s) over {operations} painting operation(s), {deferred} deferred, {unread} \
         of a variant this census does not read",
        drawn.len()
    );
    for threshold in MEGAPIXELS {
        let floor = threshold.saturating_mul(1_000_000);
        let mut by_codec: BTreeMap<&str, usize> = BTreeMap::new();
        let mut count = 0_usize;
        let mut wasteful = 0_usize;
        let mut saved = 0.0_f64;
        for image in drawn
            .iter()
            .filter(|image| image.decoded >= as_float(floor))
        {
            count = count.saturating_add(1);
            saved += image.saved();
            if image.waste() >= WASTEFUL {
                wasteful = wasteful.saturating_add(1);
            }
            let seen = by_codec.entry(image.codec.as_str()).or_default();
            *seen = seen.saturating_add(1);
        }
        println!(
            "  decoded >= {threshold:>3} Mpx: {count:>5} image(s), {wasteful} of them wasting \
             {WASTEFUL}x or more, {saved:.0} samples not shown  {by_codec:?}"
        );
    }

    drawn.sort_by(|a, b| b.saved().total_cmp(&a.saved()));
    println!("  the {LARGEST_NAMED} that would save the most:");
    for image in drawn.iter().take(LARGEST_NAMED) {
        println!(
            "    {}: {}x{} {} decoded {} time(s), placed {}x{} px, 1:1 at {:.2}x, shows {:.0} \
             samples at {MAX_ZOOM}x, waste {:.0}x, saves {:.0}",
            image.name,
            image.raster.0,
            image.raster.1,
            image.codec,
            image.draws,
            image.placed.width,
            image.placed.height,
            image.zoom_to_one_to_one(),
            image.needed,
            image.waste(),
            image.saved()
        );
    }
}

/// A sample count as the float the drawn side measures in.
///
/// Exact for every count a raster can hold: [`pdf_model`]'s own bound refuses an image beyond
/// 2^28 samples, and an `f64` carries integers to 2^53.
fn as_float(samples: u64) -> f64 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "see this function's own paragraph: the bound is 2^28 and the mantissa is 53 \
                  bits, so nothing this census counts is rounded"
    )]
    {
        samples as f64
    }
}

/// Prints what the JPEG 2000 codestreams state about reaching part of an image.
///
/// Counted over all of them and listed for the largest, which is where the question is: a
/// thumbnail carrying a tile partition it does not need says nothing about whether a scan has
/// one.
fn report_codestreams(codestreams: &mut [(String, Codestream)]) {
    let tiled = codestreams
        .iter()
        .filter(|(_, stream)| stream.tiles.0.saturating_mul(stream.tiles.1) > 1)
        .count();
    let precincts = codestreams
        .iter()
        .filter(|(_, stream)| stream.precincts)
        .count();
    println!("JPEG 2000: what the codestreams state about reaching part of one");
    println!(
        "  {} codestream(s), {tiled} stating more than one tile, {precincts} stating a precinct \
         partition",
        codestreams.len()
    );
    codestreams.sort_by_key(|(_, stream)| {
        std::cmp::Reverse(u64::from(stream.grid.0).saturating_mul(u64::from(stream.grid.1)))
    });
    for (name, stream) in codestreams.iter().take(LARGEST_NAMED) {
        println!(
            "    {name}: {}x{}, {}x{} tile(s), precincts {}, {}, {} decomposition level(s)",
            stream.grid.0,
            stream.grid.1,
            stream.tiles.0,
            stream.tiles.1,
            if stream.precincts { "yes" } else { "no" },
            stream.progression,
            stream.levels
        );
    }
}

fn main() {
    let mut documents = 0_usize;
    let mut pages = 0_usize;
    let mut tally = StatedTally::default();
    let mut codestreams: Vec<(String, Codestream)> = Vec::new();
    let mut drawn: Vec<Drawn> = Vec::new();
    let mut deferred = 0_usize;
    let mut unread = 0_usize;

    for path in std::env::args().skip(1) {
        let name = std::path::Path::new(&path)
            .file_name()
            .map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned());
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        documents = documents.saturating_add(1);

        let by_grid = stated(&document, &name, &mut codestreams);
        tally.add(&name, &by_grid);
        if drawn_on_page_one(
            &document,
            &name,
            &by_grid,
            &mut drawn,
            &mut deferred,
            &mut unread,
        ) {
            pages = pages.saturating_add(1);
        }
    }

    println!("{documents} document(s) opened, {pages} first page(s) interpreted");
    println!();
    tally.report();
    println!();
    report_drawn(&mut drawn, deferred, unread);
    println!();
    report_codestreams(&mut codestreams);
}
