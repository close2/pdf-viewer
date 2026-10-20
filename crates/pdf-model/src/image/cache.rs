//! What one page's interpretation keeps of the images it decoded: rasters by stream identity
//! within a budget, and the masks beside them.

use std::sync::Arc;

use pdf_syntax::{Dictionary, Document, Object, ObjectId, Stream};

use crate::colour::Conversion;

use super::device_scale::SoftMaskAtDeviceScale;
use super::dispatch::decode_parts_in;
use super::soft_mask::{SoftMaskEntry, device_scaled_soft_mask, soft_mask_entry};
use super::{DecodesAhead, ImageError, InFlight, Parts, Picture};

/// Soft masks already read for the device to place, keyed by the object that states them.
///
/// # Why this exists, with the measurement that asked for it
///
/// The same reason [`crate::shading::Cache`] does, one clause over: a page draws the same
/// `XObject` many times and the samples do not depend on where. `issue16263.pdf` runs `Do` on
/// one image **55 times**, and its mask's `FlateDecode` stream inflates to 18.9 MB — so
/// reading it per painting operation put **750 MB** through one 960×540 page against 15.6 MB
/// before this route existed, and 27 MB with the cache. Nothing about the answer changes:
/// every input is the mask object's own — its packed bytes, its `/BitsPerComponent`, its
/// `/Decode` — which is what makes the object's identity a sound key here where a shading
/// needed its colour space in the key as well.
///
/// A mask with no object number of its own — one written directly into the image dictionary —
/// is not cached and is read each time, which is exact rather than approximately right.
///
/// # And, beside the masks, which of them are a stencil's shape
///
/// An *image's* alpha says which of §11.3.7.2's two quantities it carries for itself, in
/// [`pdf_render::Image`]'s own [`SampleAlpha`] field (ADR 1022). One mask cannot: §8.9.6.2's
/// stencil painted through a pattern becomes a §11.5.2 alpha mask on the fill that paints the
/// pattern (ADR 0151), and that mask is the stencil's *shape* where every other mask a command
/// carries is §11.6.4.3's opacity. Nothing in the display list can say so, so the interpreter
/// records which masks it built that way, here, beside its per-run knowledge about the masks
/// themselves (ADR 1017).
///
/// [`SampleAlpha`]: pdf_render::SampleAlpha
#[derive(Debug, Default)]
pub struct MaskCache {
    read: std::collections::BTreeMap<ObjectId, SoftMaskAtDeviceScale>,
    shape_masks: ShapeMasks,
}

impl MaskCache {
    /// Which of the soft masks built so far are a stencil's shape.
    pub(crate) fn shape_masks(&self) -> &ShapeMasks {
        &self.shape_masks
    }

    /// The record [`Self::shape_masks`] reads, for the interpreter to write as it draws.
    pub(crate) fn shape_masks_mut(&mut self) -> &mut ShapeMasks {
        &mut self.shape_masks
    }

    /// Reads a mask, reusing an earlier read of the same object.
    ///
    /// `base` is the grid of the parent's raster as decoded, which is what the routing
    /// decision is about; the routing runs on every call — only the *read* of the mask's
    /// packed bytes is cached, because that is the part whose inputs are all the mask
    /// object's own.
    pub(super) fn read(
        &mut self,
        document: &Document,
        dict: &Dictionary,
        resources: &Dictionary,
        base: (u32, u32),
    ) -> Option<SoftMaskAtDeviceScale> {
        if !matches!(
            soft_mask_entry(document, dict, resources, base),
            SoftMaskEntry::AtDeviceScale
        ) {
            return None;
        }
        let key = dict.get("SMask").and_then(Object::as_reference);
        if let Some(id) = key
            && let Some(mask) = self.read.get(&id)
        {
            return Some(mask.clone());
        }
        let mask = device_scaled_soft_mask(document, dict)?;
        if let Some(id) = key {
            self.read.insert(id, mask.clone());
        }
        Some(mask)
    }
}

/// Which of the soft masks this run built are a stencil's shape rather than an opacity
/// (ISO 32000-2 §8.9.6.2, §11.5.2, §11.6.4.2, §11.6.4.3).
///
/// §8.9.6.2's stencil painted through a pattern becomes a §11.5.2 alpha mask on the fill that
/// paints the pattern (ADR 0151). That mask *is* the stencil's shape — "the shape shall be 1.0
/// for painted areas and 0.0 for masked areas" — where every other mask a command carries is
/// §11.6.4.3's opacity, and a [`pdf_render::SoftMaskId`] cannot say which it is. A knockout
/// group states an element's shape by taking the opacity off it (§11.4.6), so this is the one
/// mask it must keep, and this record is what tells it so.
///
/// A stencil carrying an `/SMask` of its own is the one such mask that is not shape alone: the
/// mask the fill draws through is the stencil's shape times §11.6.5.2's opacity, which is what
/// the page is owed, and the record pairs it with a second mask holding the stencil alone,
/// which is what the knockout's shape is owed (ADR 1301).
///
/// An *image's* kind needs no record: [`pdf_render::Image`] carries its own [`SampleAlpha`],
/// decided by [`decode_parts`] where the masks are applied (ADR 1022). Until that field
/// existed this type held that too, keyed by the raster's identity, and the identity was the
/// cost of the substitute.
///
/// [`SampleAlpha`]: pdf_render::SampleAlpha
/// [`decode_parts`]: super::decode_parts
#[derive(Debug, Default)]
pub struct ShapeMasks {
    /// Each drawn mask, and the mask holding its stencil's shape — the same id where the
    /// stencil carried no soft mask and the drawn mask is its shape already.
    masks: Vec<(pdf_render::SoftMaskId, pdf_render::SoftMaskId)>,
}

impl ShapeMasks {
    /// Records that the soft mask `id` is a shape rather than an opacity: a stencil's, or the
    /// outline a tiling pattern's stroke is cut to (§11.6.4.2).
    pub(crate) fn record(&mut self, id: pdf_render::SoftMaskId) {
        self.masks.push((id, id));
    }

    /// Records that the soft mask `drawn` is a stencil's shape multiplied by its own
    /// `/SMask`'s opacity, and that `shape` holds the stencil alone.
    pub(crate) fn record_apart(
        &mut self,
        drawn: pdf_render::SoftMaskId,
        shape: pdf_render::SoftMaskId,
    ) {
        self.masks.push((drawn, shape));
    }

    /// The mask holding the shape of the stencil the soft mask `id` was built from, or `None`
    /// where `id` is §11.6.4.3's opacity and no stencil's.
    #[must_use]
    pub(crate) fn shape_of(&self, id: pdf_render::SoftMaskId) -> Option<pdf_render::SoftMaskId> {
        self.masks
            .iter()
            .find_map(|&(drawn, shape)| (drawn == id).then_some(shape))
    }
}

/// How many bytes of decoded rasters [`RasterCache`] holds for one interpretation.
///
/// **Derived from a replay of the recorded call sequence, the way ADR 0317's 4 MiB was.** Every
/// `Do` on an image over the pdf.js corpus's page ones was recorded — 4789 calls in 228 documents,
/// each with the key below, the bytes the entry charges and the bytes of base raster the decode
/// produced — and a least-recently-used simulation run over that sequence at each budget. The
/// second of those two is the work, so it is the column that decides:
///
/// | budget | base raster decoded | decode calls | evictions |
/// |---|---|---|---|
/// | none — the code before this | 1796.8 MB | 4789 | — |
/// | 8 MiB | 1635.7 MB | 4544 | 124 |
/// | 16 MiB | 1635.7 MB | 4544 | 105 |
/// | 32 MiB | **781.5 MB** | 4466 | 13 |
/// | 64 MiB | 781.5 MB | 4466 | 4 |
/// | unbounded | 781.5 MB | 4466 | 0 |
///
/// **The knee is at 32 MiB and nothing above it buys a byte on this corpus**, because the repeats
/// that cost anything are a handful of large photographs: the witness `22060_A1_01_Plans.pdf` is
/// four 2480×2630 rasters of **26.1 MB** each drawn nine times apiece, which is why 8 and 16 MiB
/// save almost nothing — the entry does not fit at all. The median entry in the whole corpus is
/// 1 KB and seven of the 4466 exceed 32 MiB.
///
/// **The two columns are worth keeping apart, and one document is why.** `issue16263.pdf` repeats
/// a `Do` forty times over an entry charged 18.9 MB and decodes **no base raster at all** for it:
/// that 18.9 MB is §11.6.5.2's packed mask, which [`MaskCache`] has shared since ADR 0210, and
/// counting it as work saved would have credited this cache with a saving somebody else already
/// made. Its peak resident memory is unmoved by this cache, which is what says so.
///
/// **The alternative ADR 0374 weighed — a cache of the *last* raster only — was simulated on
/// the same sequence**: 804.2 MB over 4534 decodes. It serves the witness's nine-in-a-row exactly,
/// because the witness's nine are consecutive, and it is 2.9% short of the table above on a corpus
/// where nothing interleaves two large images. What it cannot do is the interleave, which is one
/// `Do` order away from the shape the witness already has.
///
/// **The doubling to 64 MiB is headroom for a shape the corpus does not contain and a page can
/// state as easily as the witness's**: two large images drawn alternately. At 32 MiB two 26.1 MB
/// rasters thrash and every `Do` decodes; at 64 MiB both are held. The cost of being wrong in this
/// direction is bounded by the paragraph below, and the cost of being wrong in the other is a page
/// that gets nothing at all.
///
/// # Why this is not ADR 0256's "below 10 MB" band
///
/// That band governs what an *open document* holds — the object cache, the decoded-stream memo,
/// the readback cache — and this is not that. It is per interpretation, it dies with the page's
/// display list, and **it spends no memory that list was not already spending**: every raster held
/// here is the raster a `Command::Image` in the list under construction is holding, because the
/// same `Arc` goes into both. Before this cache the nine `Do`s of one photograph put nine separate
/// 26.1 MB allocations into the list. ADR 0374 has the peak-resident measurement, and it moves in
/// the direction that follows from that sentence rather than the one a cache usually moves in.
///
/// §10.5's transfer does not break that sharing: it rides on the mark and a backend applies it
/// over the finished raster (ADR 1125), so no transferred copy of the samples is made for this
/// budget to bound.
const RASTER_BUDGET: usize = 64 << 20;

/// Base rasters already decoded, keyed by everything the decode depends on.
///
/// # Why this exists, with the measurement that asked for it
///
/// [`decode_parts`] runs at every `Do`, so a page that draws one image `XObject` thirty-six times
/// decodes it thirty-six times. `22060_A1_01_Plans.pdf` is one page with four 2480×2630
/// `DCTDecode` photographs in an ICC-based space, drawn nine times each: **3.23 of the 3.24
/// seconds** it took to interpret page one were inside that call, and eight of every nine calls
/// reproduced a raster the interpreter had already produced (ADR 0373 measured it,
/// ADR 0374 took it).
///
/// What it is worth on that page, in instructions rather than in seconds because the machine was
/// carrying five other rounds — `examples/callgrind_interpret` under `RAYON_NUM_THREADS=1`:
/// **58 665 139 034 → 6 544 740 674, −88.8%**. Peak resident over the same interpretation:
/// **1031.8 MB → 235.6 MB**. On an ordinary page — ISO 32000-2's page 101, fifty repetitions,
/// which draws no image at all — 1 235 040 931 → 1 234 947 809, which is the −0.008% of a
/// `Vec` that is never touched.
///
/// # What the key claims
///
/// [`MaskCache`] above can key on an object number because every input of a mask is the mask
/// object's own. A base image's raster is not like that: [`decode_parts`] reads five things, and
/// four of them are in the key.
///
/// - **The stream**, by [`StreamIdentity`] — the identity of its allocation for a stream reached
///   through a resource dictionary, and a digest of its content for §8.9.7's inline image, which
///   has no allocation that outlives one `BI`. For the first of the two the entry *holds* the
///   [`Arc`], which is what makes an address a sound name. ADR 0317's `a_pin_is_not_a_copy`
///   argument, exactly: a freed buffer's address is handed to the next allocation, so an entry
///   naming an address it does not keep alive could answer a lookup for a different stream.
///   Holding it makes that impossible rather than unlikely.
/// - **What the decode reads of the resource dictionary in force**, because §8.6.5.1 resolves a
///   colour space named `/CS0` through it and §8.6.5.6's `/DefaultGray`, `/DefaultRGB` and
///   `/DefaultCMYK` reach even the device names. One stream under two resource dictionaries is two
///   pictures. **Both lookups are inside one of §7.8.3 Table 34's eight entries**, `ColorSpace`:
///
///   > A dictionary that maps each resource name to either the name of a device-dependent colour
///   > space or an array describing a colour space
///
///   — and §8.6.5.6 puts the defaults in the `ColorSpace` subdictionary of the current resource
///   dictionary, so that entry is the whole of what a decode can read, and it is what an entry
///   holds and compares, by value: two equal entries name the same objects of the same document,
///   so equality is the claim that the lookups agree. [`RasterCache::parts`] makes the claim
///   structural rather than a reading of `colour.rs`: the decode is *handed* a resource
///   dictionary holding that one entry, so a lookup of anything else finds nothing on a miss and
///   nothing on a hit alike. **An entry holding the whole dictionary would escape
///   `RASTER_BUDGET`**: a fuzzed page naming 10 260 one-row image `XObject`s through one
///   dictionary would clone that dictionary — about a mebibyte, since it names all 10 260 — into
///   every entry while charging the eight bytes of samples, 10.60 GiB to interpret (ADR 0798
///   measured it, ADR 0791 took this).
/// - **The fill colour**, because §8.9.6.2's stencil "does not specify colours; instead, it
///   designates places on the page that should either be marked with the current colour or masked
///   out", so the same stencil under two fill colours is two rasters. Compared as bit patterns,
///   which is conservative in the one direction that matters: `-0.0` and `0.0` paint alike and
///   miss, and no two distinct colours can collide.
/// - **How the samples are converted** (`crate::colour::Conversion`), which is two things. What
///   they are composited into, because §11.4.7's four-component page is interpreted twice, once
///   per half (ADR 0262), and because a `/Luminosity` mask group paints in the ink §10.4.2.3
///   weighs rather than in colour (ADR 0220). And whether §8.6.5.9's black point compensation
///   applies, because §8.9.5.1 Table 87's `/Intent` is an image's *own* rendering intent — so one
///   page can draw one stream under both answers. Both change *within* an interpretation, which
///   is why neither can be left to the cache's lifetime the way the fifth input is.
///
/// The fifth input is the [`Document`], and it is not in the key because the cache belongs to one
/// interpretation of one page and cannot outlive it.
///
/// `tests::a_raster_is_not_shared_*` is one test per component, each of which fails if its
/// component is dropped from the key.
///
/// # What is not cached
///
/// A decode that **failed**, for [`crate::shading::Cache`]'s reason: an error is rare, the caller
/// reports it, and remembering one would mean deciding whether it is a property of the object or
/// of the moment. And a raster larger than `RASTER_BUDGET`, which is the eviction arithmetic
/// falling out rather than a rule of its own — it is inserted, it evicts everything including
/// itself, and the page pays what it always paid.
///
/// [`decode_parts`]: super::decode_parts
#[derive(Debug, Default)]
pub struct RasterCache {
    /// Least recently used first; a hit moves its entry to the end.
    ///
    /// A `Vec` rather than a map because the byte budget keeps this short — the entries are
    /// whole rasters — and because the cheap components of the key are compared first, so a
    /// probe that misses costs one pointer comparison per entry. A map would need the key
    /// constructed, and constructing it means cloning the `/ColorSpace` entry per `Do`.
    ///
    /// **"Keeps this short" is a claim about [`StreamIdentity`] rather than about the budget**,
    /// and it is why that type exists: a probe is linear in the entries, so an image draw that
    /// can only ever *add* one makes a page quadratic in its own image count. A stream reached
    /// through a resource dictionary cannot do that — its allocation recurs, so it adds one
    /// entry however often it is drawn — and an inline image named by its address added one per
    /// `BI`. ADR 0399 measured what that cost the two documents that found it.
    entries: Vec<Cached>,
    /// The sum of the entries' [`Cached::bytes`], so that eviction is arithmetic.
    held: usize,
    /// Decodes started before their `Do`, where the interpretation started any (ADR 1321).
    ///
    /// Asked on a miss and never on a hit, so an entry here is exactly what the decode below
    /// would have produced, arriving earlier: [`DecodesAhead`] says why the two are one answer.
    ahead: Option<Arc<DecodesAhead>>,
}

/// How a cached raster names the stream it was decoded from.
///
/// Two kinds of stream reach [`RasterCache`] and only one of them has an allocation that
/// outlives the draw, which is the distinction this type is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamIdentity {
    /// A stream the content stream named through a resource dictionary, identified by the
    /// address of its allocation, which the entry holds alive. §7.8.3 makes that sound: the
    /// dictionary hands out the same object, so the same `Do` twice is the same [`Arc`] twice.
    Allocation,
    /// §8.9.7's inline image, identified by a digest of its content because it has no address
    /// that recurs.
    ///
    /// > This type of image shall be defined directly within the content stream in which it
    /// > will be painted rather than as a separate object.
    ///
    /// So there is no object for a resource dictionary to hand out twice: the stream is *built*
    /// at every `BI`, out of bytes in the content stream, and its address names one draw rather
    /// than one image.
    ///
    /// The digest is not the whole of the comparison — the content is compared exactly beside it,
    /// the way `pdf_render::DisplayList::add_clip` compares a clip it found by digest — so a
    /// collision costs a decode and never an answer about a different image.
    Content(u64),
}

impl StreamIdentity {
    /// The identity of §8.9.7's inline image, from what it holds.
    ///
    /// The dictionary's length rather than the dictionary: §8.9.7's dictionaries are Table 91's
    /// dozen keys and it is the data that separates two hatchings of one page. The exact
    /// comparison in `Cached::answers` is what decides an entry, so this only has to spread.
    #[must_use]
    pub fn of_inline(stream: &Stream) -> Self {
        use std::hash::{Hash as _, Hasher as _};
        let mut hasher = std::hash::DefaultHasher::new();
        stream.data.hash(&mut hasher);
        stream.dict.len().hash(&mut hasher);
        Self::Content(hasher.finish())
    }
}

/// A stream on its way into [`RasterCache::parts`], under the name an entry for it is keyed by.
///
/// The two travel together because they are one fact — *which image is this* — and the whole of
/// ADR 0399's defect was a caller that had the stream and did not have the name. So the two
/// constructors are the only way to build one, and each says which of §7.8.3's route and §8.9.7's
/// the stream came by.
#[derive(Debug, Clone, Copy)]
pub struct NamedStream<'a> {
    /// The stream to decode.
    pub stream: &'a Arc<Stream>,
    /// How an entry for it is named.
    pub identity: StreamIdentity,
}

impl<'a> NamedStream<'a> {
    /// A stream a resource dictionary handed out, named by the address of its allocation.
    #[must_use]
    pub fn allocation(stream: &'a Arc<Stream>) -> Self {
        Self {
            stream,
            identity: StreamIdentity::Allocation,
        }
    }

    /// §8.9.7's inline image, named by a digest of its content.
    #[must_use]
    pub fn inline(stream: &'a Arc<Stream>) -> Self {
        Self {
            identity: StreamIdentity::of_inline(stream),
            stream,
        }
    }
}

/// One decoded raster, and the claim about which `Do` it answers.
#[derive(Debug)]
struct Cached {
    /// How this entry names its stream.
    identity: StreamIdentity,
    /// The stream these samples came from, **held** so that no other stream can take its
    /// address. See [`RasterCache`] for why identity is the address rather than an object
    /// number.
    stream: Arc<Stream>,
    /// What the decode read of the resource dictionary the image was drawn from: §7.8.3
    /// Table 34's `/ColorSpace` entry, unresolved, or [`Object::Null`] where there is none.
    /// [`RasterCache`] says why that entry is the whole of it, and why it is not the dictionary.
    colour_spaces: Object,
    /// The fill colour, as bit patterns.
    fill: [u32; 4],
    /// How the samples were converted: the target, and §8.6.5.9's black point.
    into: Conversion,
    /// The answer.
    parts: Parts,
    /// What this entry charges against [`RASTER_BUDGET`]: the samples, and the `/ColorSpace`
    /// entry it holds. The second term is what makes the budget a bound on the entry rather than
    /// on the samples — ADR 0243's shape, one bound doing two jobs, which here is ten thousand eight-byte rasters
    /// holding ten thousand mebibyte dictionaries (ADR 0798).
    bytes: usize,
}

/// The key of §7.8.3 Table 34's entry an image decode reads, and the one it is handed.
pub(super) const COLOUR_SPACES: &str = "ColorSpace";

/// What a resource dictionary without a `/ColorSpace` entry is compared as.
static NO_COLOUR_SPACES: Object = Object::Null;

impl RasterCache {
    /// What the cache currently charges against `RASTER_BUDGET`, in bytes.
    ///
    /// For a test of the charge: an entry is charged what it holds, so this is bounded by the
    /// budget and the entries' own cost is bounded by this.
    #[must_use]
    pub fn held(&self) -> usize {
        self.held
    }

    /// Answers misses from `ahead` as well, for as long as the interpretation that started it
    /// runs (ADR 1321).
    pub(crate) fn decode_ahead(&mut self, ahead: Arc<DecodesAhead>) {
        self.ahead = Some(ahead);
    }

    /// Stops asking the table [`Self::decode_ahead`] installed, once its decodes are over.
    pub(crate) fn stop_ahead(&mut self) {
        self.ahead = None;
    }

    /// Decodes an image `XObject`, reusing an earlier decode of the same one under the same
    /// state.
    ///
    /// Everything about the decode itself is [`decode_parts`]'s; this adds the lookup and the
    /// eviction. `masks` is the caller's [`MaskCache`], which a hit does not touch — it is a
    /// memo of the same kind one level down, so a raster answered from here needed no mask read
    /// to answer it.
    ///
    /// # Errors
    ///
    /// See [`ImageError`].
    ///
    /// [`decode_parts`]: super::decode_parts
    pub fn parts(
        &mut self,
        document: &Document,
        image: NamedStream<'_>,
        resources: &Dictionary,
        fill: pdf_render::Color,
        into: &Conversion,
        masks: &mut MaskCache,
    ) -> Result<Parts, ImageError> {
        let NamedStream { stream, identity } = image;
        let fill = [
            fill.r.to_bits(),
            fill.g.to_bits(),
            fill.b.to_bits(),
            fill.a.to_bits(),
        ];
        let colour_spaces = resources.get(COLOUR_SPACES).unwrap_or(&NO_COLOUR_SPACES);
        if let Some(at) = self
            .entries
            .iter()
            .position(|entry| entry.answers(stream, identity, colour_spaces, fill, into))
        {
            // Moved to the end because recency is what the eviction below reads, and a `Vec`
            // says it by position: the entry taken out and pushed back is the most recent.
            let entry = self.entries.remove(at);
            let parts = entry.parts.clone();
            self.entries.push(entry);
            return Ok(parts);
        }

        // The decode is handed §7.8.3 Table 34's `/ColorSpace` entry and nothing else of the
        // resource dictionary, so that the entry below holds, by construction, everything the
        // decode could have read: a lookup of any other entry finds nothing here and nothing on
        // a hit alike. Taken back out afterwards rather than cloned twice.
        let mut read = Dictionary::new();
        if let Some(entry) = resources.get(COLOUR_SPACES) {
            read.insert(
                pdf_syntax::Name::new(COLOUR_SPACES.as_bytes()),
                entry.clone(),
            );
        }
        // Only a stream named through a resource dictionary is ever decoded ahead: §8.9.7's
        // inline image is built at its `BI` and has no earlier moment to be decoded at.
        let ahead = match (identity, self.ahead.as_ref()) {
            (StreamIdentity::Allocation, Some(ahead)) => ahead.take(stream, colour_spaces, into),
            _ => None,
        };
        // The cut's signal (ADR 1481): nothing of this page is decoded ahead, this thread is not
        // one of the pool's, whose threads a frame divided across it would otherwise find busy,
        // and there is a pool to divide it across — on one thread the pass is all the cut adds.
        let in_flight = if self.ahead.is_none()
            && rayon::current_thread_index().is_none()
            && rayon::current_num_threads() >= 2
        {
            InFlight::Alone
        } else {
            InFlight::AmongOthers
        };
        let parts = match ahead {
            Some(answer) => answer?,
            None => decode_parts_in(
                document,
                stream,
                (&read, fill_of(fill), into),
                masks,
                in_flight,
            )?,
        };
        let colour_spaces = read.remove(COLOUR_SPACES).unwrap_or(Object::Null);
        let bytes = parts.bytes().saturating_add(footprint(&colour_spaces));
        self.entries.push(Cached {
            identity,
            stream: Arc::clone(stream),
            colour_spaces,
            fill,
            into: into.clone(),
            parts: parts.clone(),
            bytes,
        });
        self.held = self.held.saturating_add(bytes);
        while self.held > RASTER_BUDGET && !self.entries.is_empty() {
            let evicted = self.entries.remove(0);
            self.held = self.held.saturating_sub(evicted.bytes);
        }
        Ok(parts)
    }
}

/// A fill colour back from the bit patterns the key holds.
///
/// The round trip is exact — `to_bits` and `from_bits` are inverse for every value including the
/// NaNs — so the decode is handed the caller's own colour and not a copy of it that rounds.
fn fill_of(bits: [u32; 4]) -> pdf_render::Color {
    pdf_render::Color {
        r: f32::from_bits(bits[0]),
        g: f32::from_bits(bits[1]),
        b: f32::from_bits(bits[2]),
        a: f32::from_bits(bits[3]),
    }
}

impl Cached {
    /// Whether this entry is an answer to the `Do` described.
    ///
    /// The three cheap components are compared before the `/ColorSpace` entry, which is what
    /// keeps a probe that misses to one pointer comparison: no two entries share an address
    /// unless they differ in something else, so at most one comparison of that entry — a
    /// reference, usually, and a dictionary at worst — happens per probe.
    ///
    /// [`StreamIdentity`] goes first for the same reason. For an allocation it is one
    /// discriminant and the address decides; for §8.9.7's inline image it is the digest, and a
    /// probe reaches the exact comparison of the content only where the digest already agreed.
    fn answers(
        &self,
        stream: &Arc<Stream>,
        identity: StreamIdentity,
        colour_spaces: &Object,
        fill: [u32; 4],
        into: &Conversion,
    ) -> bool {
        self.identity == identity
            && match identity {
                StreamIdentity::Allocation => Arc::ptr_eq(&self.stream, stream),
                // Exact, the way `DisplayList::add_clip` is exact about a clip it found by
                // digest: the digest above narrows the search and this decides it, so two
                // inline images that collide cost a decode rather than each other's raster.
                StreamIdentity::Content(_) => *self.stream == **stream,
            }
            && self.fill == fill
            && self.into == *into
            && self.colour_spaces == *colour_spaces
    }
}

/// What holding an object costs, for [`RASTER_BUDGET`]'s arithmetic.
///
/// The value itself and what it owns on the heap, walked rather than asked of the allocator
/// because nothing in this tree asks the allocator anything. Two of the terms are over-charges
/// in the direction [`Parts::bytes`] already chooses: a name's bytes are behind an [`Arc`] the
/// parser may share, and a stream's data is the document's, held once however many entries name
/// it. Over-charging bounds; under-charging is the defect this function exists to end.
fn footprint(object: &Object) -> usize {
    let owned = match object {
        Object::String(bytes) => bytes.len(),
        Object::Name(name) => name.as_bytes().len(),
        Object::Array(items) => items.iter().map(footprint).fold(0, usize::saturating_add),
        Object::Dictionary(dict) => dictionary_footprint(dict),
        Object::Stream(stream) => stream
            .data
            .len()
            .saturating_add(dictionary_footprint(&stream.dict)),
        Object::Null
        | Object::Boolean(_)
        | Object::Integer(_)
        | Object::Real(_)
        | Object::Reference(_) => 0,
    };
    size_of::<Object>().saturating_add(owned)
}

/// [`footprint`] over a dictionary's entries: each key's handle and bytes, and each value.
fn dictionary_footprint(dict: &Dictionary) -> usize {
    dict.iter()
        .map(|(key, value)| {
            size_of::<pdf_syntax::Name>()
                .saturating_add(key.as_bytes().len())
                .saturating_add(footprint(value))
        })
        .fold(0, usize::saturating_add)
}

impl Parts {
    /// What holding this costs, for [`RASTER_BUDGET`]'s arithmetic.
    ///
    /// A deferred mask's packed bytes are counted beside the base raster's even though
    /// [`MaskCache`] is usually holding the same allocation for the same interpretation: the
    /// budget's job is to bound what a page can hold, and over-charging bounds it, while
    /// under-charging would let a mask nobody else holds — one written inline, which
    /// [`MaskCache`] declines to remember — go unbudgeted.
    fn bytes(&self) -> usize {
        match &self.picture {
            Picture::Complete(image) => image.data.len(),
            Picture::Masked { base, opacity } => base.data.len().saturating_add(opacity.data.len()),
        }
    }
}
