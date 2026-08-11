//! What the press a page composites in *is*, for the pages §11.4.7 sends to four components.
//!
//! `doc/todo/23`'s two largest rows are one question asked twice: a page whose blending colour
//! space has four components that are **not this tree's assumed process inks**. §11.4.7 makes
//! the page group's `/CS` the space "[a]ll page-level compositing" happens in, and §11.7.2 says
//! what that space is when it is stated indirectly:
//!
//! > If an isolated transparency group or page has an ICCBased 'CMYK' colour space , DeviceCMYK
//! > shall be redefined within the transparency group to be the same as the blending colour
//! > space and references to the process colourants Cyan , Magenta , Yellow and Black are
//! > defined to be references to the corresponding colourants in the blending colour space,
//! > even where the actual or simulated output device is not CMYK.
//!
//! Annex P — informative, and the standard's own algorithm for this question — puts the two
//! indirections in order: an isolated group with a device blending space "first apply[ies] the
//! default colour space mechanism", and a page group with no parent inherits "from the output
//! device, or from the output intent".
//!
//! So the press a page composites in comes from one of four places, and this census counts
//! them and then asks the question that decides whether honouring them needs a dependency:
//! **can this tree already evaluate the profile that names the press?** `pdf_model::icc` has
//! read `A2B0`/`A2B1` in three encodings since ADR 0009. A profile it cannot read is not a gap
//! but §8.6.5.5's own answer:
//!
//! > If this entry is omitted and the PDF reader does not understand the ICC profile data, the
//! > colour space that shall be used is DeviceGray , DeviceRGB , or DeviceCMYK , depending on
//! > whether the value of N is 1 , 3 , or 4 , respectively.
//!
//! which for four components is the space this tree already composites in (ADRs 0262, 0263).
//!
//! ```sh
//! cargo run --release -p pdf-model --example press_census -- corpus-cache/safedocs/*/0000/*.pdf
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::BTreeMap;

use pdf_model::colour::ColourSpace;
use pdf_syntax::{Dictionary, Document, Object};

/// How a page's blending colour space was arrived at, and what it is.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Press {
    /// No page group, or one whose `/CS` is a three-component RGB space: not a departure.
    Device,
    /// `/DeviceCMYK` with nothing naming what it is — ADR 0263's assumed process inks.
    AssumedInks,
    /// `/DeviceCMYK` remapped by §8.6.5.6's `/DefaultCMYK`, to a space of this description.
    DefaultCmyk(String),
    /// `/DeviceCMYK` with §14.11.5's output intent naming the press.
    OutputIntent(String),
    /// A page group `/CS` that is itself a four-component CIE-based space.
    StatedFour(String),
    /// A page group `/CS` of one component — §11.3.4 per component, with one of them.
    StatedOne(String),
    /// Anything else this tree names as a departure: a `Separation`, a `DeviceN`, a `Lab`.
    Other(String),
}

impl Press {
    /// The row this belongs in.
    fn row(&self) -> &'static str {
        match self {
            Self::Device => "device (not a departure)",
            Self::AssumedInks => "/DeviceCMYK, assumed inks (drawn since ADR 0263)",
            Self::DefaultCmyk(_) => "a press named by §8.6.5.6's /DefaultCMYK",
            Self::OutputIntent(_) => "a press named by §14.11.5's output intent",
            Self::StatedFour(_) => "four components stated by the page group itself",
            Self::StatedOne(_) => "one component stated by the page group",
            Self::Other(_) => "another space this tree departs from",
        }
    }
}

/// What was learned about the ICC profile behind a press, where there is one.
#[derive(Debug, Default)]
struct ProfileFacts {
    /// Whether `pdf_model::icc` parses it into a transform it can evaluate.
    evaluable: bool,
    /// The four-character tag signatures the profile's own tag table lists.
    tags: Vec<String>,
    /// The header's data colour space, `CMYK` and the rest.
    space: String,
    /// The header's device class: `prtr`, `mntr`, `spac`.
    class: String,
    /// The header's major version number.
    version: u8,
    /// How many bytes the decoded profile is.
    bytes: usize,
    /// The decoded profile itself, kept only so that `--sample` can evaluate it.
    data: Vec<u8>,
}

/// How closely a grid of `side` samples of `profile` reproduces the profile itself.
///
/// The number that chooses `pdf_model::colour`'s `PRESS_SIDE`: a backend interpolates a table
/// rather than running a colour transform per pixel, so the table has to be fine enough that
/// interpolating it and evaluating the profile are the same answer. Printed for every press in
/// the population under `--sample`, in levels of 255, so the side is chosen from what the web's
/// profiles actually do rather than from one of them.
fn sampling_gap(profile: &pdf_model::icc::Profile, side: usize) -> f32 {
    sampling_gap_in(profile, side, false)
}

/// sRGB's own transfer function and its inverse, IEC 61966-2-1's.
fn decode_srgb(value: f32) -> f32 {
    if value <= 0.040_45 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

/// The inverse of [`decode_srgb`].
fn encode_srgb(value: f32) -> f32 {
    if value <= 0.003_130_8 {
        value * 12.92
    } else {
        1.055f32.mul_add(value.powf(1.0 / 2.4), -0.055)
    }
}

/// [`sampling_gap`], optionally with the grid held in linear light rather than in sRGB.
fn sampling_gap_in(profile: &pdf_model::icc::Profile, side: usize, linear: bool) -> f32 {
    let last = side.saturating_sub(1);
    #[expect(
        clippy::cast_precision_loss,
        reason = "a grid index below 65 and the side itself, both exact in f32"
    )]
    let at = |index: usize| index as f32 / last as f32;
    let mut grid = Vec::with_capacity(side.saturating_pow(4));
    for black in 0..side {
        for yellow in 0..side {
            for magenta in 0..side {
                for cyan in 0..side {
                    let colour = profile.to_rgb(&[at(cyan), at(magenta), at(yellow), at(black)]);
                    let held = [colour.r, colour.g, colour.b];
                    grid.push(if linear {
                        [
                            decode_srgb(held[0]),
                            decode_srgb(held[1]),
                            decode_srgb(held[2]),
                        ]
                    } else {
                        held
                    });
                }
            }
        }
    }
    let Some(space) = pdf_render::BlendingSpace::new(side, grid.into()) else {
        return f32::NAN;
    };
    let mut worst = 0.0f32;
    for step in 0..20_000usize {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a residue below 101, exact in f32"
        )]
        let axis = |factor: usize| (step.saturating_mul(factor) % 101) as f32 / 100.0;
        let inks = [axis(7), axis(13), axis(29), axis(47)];
        let mut sampled = space.convert(inks[0], inks[1], inks[2], inks[3]);
        if linear {
            for channel in &mut sampled {
                *channel = encode_srgb(*channel);
            }
        }
        let direct = profile.to_rgb(&inks);
        for (got, want) in sampled.iter().zip([direct.r, direct.g, direct.b]) {
            worst = worst.max((got - want).abs() * 255.0);
        }
    }
    worst
}

#[expect(
    clippy::too_many_lines,
    reason = "one census printing one table; splitting it would hide what is counted"
)]
fn main() {
    let mut opened = 0_usize;
    let mut rows: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut evaluable: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut with_b2a: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut spaces: BTreeMap<String, usize> = BTreeMap::new();
    let mut classes: BTreeMap<String, usize> = BTreeMap::new();
    let mut lines: Vec<String> = Vec::new();
    let mut gaps: Vec<(String, f32, f32, f32)> = Vec::new();
    let sampling = std::env::args().any(|argument| argument == "--sample");

    for path in std::env::args().skip(1).filter(|name| name != "--sample") {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        opened = opened.saturating_add(1);
        let name = path.rsplit('/').next().unwrap_or(&path).to_owned();
        let pages = pdf_model::Pages::new(&document);
        // Page one, which is the page the survey this census follows renders and reports.
        let Some(page) = pages.get(0) else {
            continue;
        };
        let (press, profile) = press_of(&document, &page.dict, &page.resources);
        let counter = rows.entry(press.row()).or_default();
        *counter = counter.saturating_add(1);
        if matches!(press, Press::Device) {
            continue;
        }
        if let Some(facts) = &profile {
            if facts.evaluable {
                let counter = evaluable.entry(press.row()).or_default();
                *counter = counter.saturating_add(1);
            }
            if facts.tags.iter().any(|tag| tag.starts_with("B2A")) {
                let counter = with_b2a.entry(press.row()).or_default();
                *counter = counter.saturating_add(1);
            }
            let counter = spaces.entry(facts.space.clone()).or_default();
            *counter = counter.saturating_add(1);
            let counter = classes.entry(facts.class.clone()).or_default();
            *counter = counter.saturating_add(1);
        }
        if sampling
            && let Some(profile) = profile
                .as_ref()
                .and_then(|facts| pdf_model::icc::Profile::parse(&facts.data))
            && profile.channels() == 4
        {
            gaps.push((
                name.clone(),
                sampling_gap(&profile, 9),
                sampling_gap(&profile, 17),
                sampling_gap_in(&profile, 17, true),
            ));
        }
        lines.push(format!(
            "  {name}: {} | {}",
            describe(&press),
            profile.as_ref().map_or_else(
                || "no ICC profile behind it".to_owned(),
                |facts| format!(
                    "profile {} bytes, {} v{}, class {}, evaluable {}, tags [{}]",
                    facts.bytes,
                    facts.space,
                    facts.version,
                    facts.class,
                    facts.evaluable,
                    facts.tags.join(" ")
                )
            )
        ));
    }

    println!("{opened} document(s) opened");
    for (row, count) in &rows {
        let good = evaluable.get(row).copied().unwrap_or(0);
        let b2a = with_b2a.get(row).copied().unwrap_or(0);
        println!("  {count:>6}  {row}  (evaluable A2B {good}, carrying B2A {b2a})");
    }
    if sampling {
        let worst = |which: fn(&(String, f32, f32, f32)) -> f32| {
            gaps.iter()
                .map(which)
                .fold(0.0f32, |held, gap| if gap > held { gap } else { held })
        };
        println!(
            "  {} press(es) sampled: worst gap of 255, sRGB 9 {:.2}, sRGB 17 {:.2}, \
             linear 17 {:.2}",
            gaps.len(),
            worst(|row| row.1),
            worst(|row| row.2),
            worst(|row| row.3)
        );
        for (name, nine, seventeen, thirty_three) in &gaps {
            println!(
                "  sampled {name}: sRGB 9 {nine:.2}, sRGB 17 {seventeen:.2}, \
                 linear 17 {thirty_three:.2}"
            );
        }
    }
    println!("  profile data colour spaces: {spaces:?}");
    println!("  profile device classes: {classes:?}");
    for line in &lines {
        println!("{line}");
    }
}

/// A one-line description of a press, for the per-document listing.
fn describe(press: &Press) -> String {
    match press {
        Press::Device => "device".to_owned(),
        Press::AssumedInks => "/DeviceCMYK with assumed inks".to_owned(),
        Press::DefaultCmyk(what) => format!("/DefaultCMYK -> {what}"),
        Press::OutputIntent(what) => format!("output intent -> {what}"),
        Press::StatedFour(what) | Press::StatedOne(what) | Press::Other(what) => {
            format!("page group /CS -> {what}")
        }
    }
}

/// The blending space a page composites in, and the profile behind it where there is one.
///
/// §11.4.7 gives the page group's `/CS` the whole page, Annex P puts §8.6.5.6's default
/// mechanism in front of a device space and §14.11.5's output intent behind it, and
/// `pdf_model`'s own `device_cmyk_is_a_named_press` asks the same two questions in the same
/// order. This is that reading, printed rather than acted on.
fn press_of(
    document: &Document,
    page: &Dictionary,
    resources: &Dictionary,
) -> (Press, Option<ProfileFacts>) {
    let attributes = document.get_key(page, "Group");
    let Some(attributes) = attributes.as_dict() else {
        return (Press::Device, None);
    };
    // §8.10.3 Table 94's `/S`, which is what makes the dictionary a transparency group.
    if document
        .get_key(attributes, "S")
        .as_name()
        .is_none_or(|name| name.as_bytes() != b"Transparency")
    {
        return (Press::Device, None);
    }
    let entry = document.get_key(attributes, "CS");
    if matches!(entry, Object::Null) {
        return (Press::Device, None);
    }
    let parsed = ColourSpace::parse(document, &entry, &Dictionary::new());
    let described = describe_object(&entry, parsed.as_ref());
    match parsed {
        // The three-component RGB spaces are what this device already composites in.
        Some(ColourSpace::Rgb | ColourSpace::CalRgb { .. }) => (Press::Device, None),
        Some(ColourSpace::Icc { ref profile }) if profile.channels() == 3 => (Press::Device, None),
        Some(ColourSpace::Cmyk) => {
            // Annex P's note: a device blending space applies the default mechanism first.
            let default = document
                .get_key(resources, "ColorSpace")
                .as_dict()
                .map_or(Object::Null, |dict| document.get_key(dict, "DefaultCMYK"));
            if !matches!(document.resolve(&default), Object::Null) {
                let space = ColourSpace::parse(document, &default, &Dictionary::new());
                if !matches!(space, Some(ColourSpace::Cmyk) | None) {
                    let facts = icc_facts(document, &default);
                    return (
                        Press::DefaultCmyk(describe_object(&default, space.as_ref())),
                        facts,
                    );
                }
            }
            if let Some((described, facts)) = output_intent(document) {
                return (Press::OutputIntent(described), Some(facts));
            }
            (Press::AssumedInks, None)
        }
        Some(ref space) if space.components() == 4 => {
            // The report this tree makes today reads "the document names the press its
            // DeviceCMYK is" for a page in this row that *also* carries a four-component
            // output intent, because `device_cmyk_is_a_named_press` asks the document before
            // the page group. The space here is the page group's own, so the intent is not
            // what names it, and the misattribution is counted rather than assumed.
            let also = if output_intent(document).is_some() {
                " [also carries a four-component output intent]"
            } else {
                ""
            };
            (
                Press::StatedFour(format!("{described}{also}")),
                icc_facts(document, &entry),
            )
        }
        Some(ref space) if space.components() == 1 => {
            (Press::StatedOne(described), icc_facts(document, &entry))
        }
        _ => (Press::Other(described), icc_facts(document, &entry)),
    }
}

/// §14.11.5's first usable output intent, described, with the facts about its profile.
fn output_intent(document: &Document) -> Option<(String, ProfileFacts)> {
    let catalog = document.catalog().ok()?;
    let intents = document.get_key(&catalog, "OutputIntents");
    for intent in intents.as_array()? {
        let intent = document.resolve(intent);
        let Some(dict) = intent.as_dict() else {
            continue;
        };
        let profile = document.get_key(dict, "DestOutputProfile");
        let Some(stream) = profile.as_stream() else {
            continue;
        };
        let Some(data) = document.decoded_stream_data(stream) else {
            continue;
        };
        let facts = facts_of(&data);
        // `pdf_model`'s own `device_cmyk_is_a_named_press` takes the first intent whose profile
        // parses *and* has four components, which is the count `/DeviceCMYK` has.
        if facts.evaluable && facts.channels() == 4 {
            let identifier = document
                .get_key(dict, "OutputConditionIdentifier")
                .as_string()
                .map_or_else(
                    || "unnamed".to_owned(),
                    |text| String::from_utf8_lossy(text).into_owned(),
                );
            return Some((
                format!("{identifier} ({} components)", facts.channels()),
                facts,
            ));
        }
    }
    None
}

/// The facts about the ICC profile an `[/ICCBased stream]` object names, if it is one.
fn icc_facts(document: &Document, entry: &Object) -> Option<ProfileFacts> {
    let object = document.resolve(entry);
    let array = object.as_array()?;
    let head = array.first().map(|first| document.resolve(first))?;
    if head.as_name()?.as_bytes() != b"ICCBased" {
        return None;
    }
    let stream = array.get(1).map(|second| document.resolve(second))?;
    let data = document.decoded_stream_data(stream.as_stream()?)?;
    Some(facts_of(&data))
}

impl ProfileFacts {
    /// How many components the header's data colour space has.
    fn channels(&self) -> usize {
        match self.space.as_str() {
            "GRAY" => 1,
            "CMYK" => 4,
            _ => 3,
        }
    }
}

/// Reads an ICC profile's header and tag table, and asks `pdf_model::icc` whether it parses.
///
/// The tag table is read here rather than through `icc::Profile` because the question is what
/// the profile *holds* — §8.6.5.5 requires a blending-space profile to carry `B2A` as well as
/// `A2B`, and whether the files that state one actually do is a fact about the population.
fn facts_of(data: &[u8]) -> ProfileFacts {
    let mut facts = ProfileFacts {
        evaluable: pdf_model::icc::Profile::parse(data).is_some(),
        bytes: data.len(),
        data: data.to_vec(),
        ..ProfileFacts::default()
    };
    let text = |range: std::ops::Range<usize>| {
        data.get(range)
            .map(|bytes| String::from_utf8_lossy(bytes).trim().to_owned())
            .unwrap_or_default()
    };
    if data.get(36..40) != Some(b"acsp") {
        "not a profile".clone_into(&mut facts.space);
        return facts;
    }
    facts.space = text(16..20);
    facts.class = text(12..16);
    facts.version = data.get(8).copied().unwrap_or(0);
    let count = u32_at(data, 128).unwrap_or(0) as usize;
    for index in 0..count.min(256) {
        let Some(at) = 132usize.checked_add(index.saturating_mul(12)) else {
            break;
        };
        let Some(signature) = data.get(at..at.saturating_add(4)) else {
            break;
        };
        facts
            .tags
            .push(String::from_utf8_lossy(signature).trim().to_owned());
    }
    facts
}

/// A big-endian `u32` at `offset`.
fn u32_at(data: &[u8], offset: usize) -> Option<u32> {
    let bytes = data.get(offset..offset.checked_add(4)?)?;
    Some(u32::from_be_bytes([
        *bytes.first()?,
        *bytes.get(1)?,
        *bytes.get(2)?,
        *bytes.get(3)?,
    ]))
}

/// What a colour space object says it is, with the component count this crate reads from it.
fn describe_object(entry: &Object, parsed: Option<&ColourSpace>) -> String {
    let written = match entry {
        Object::Name(name) => format!("/{}", String::from_utf8_lossy(name.as_bytes())),
        Object::Array(items) => match items.first() {
            Some(Object::Name(name)) => {
                format!("[/{} …]", String::from_utf8_lossy(name.as_bytes()))
            }
            _ => "an array-formed space".to_owned(),
        },
        _ => "an indirect space".to_owned(),
    };
    match parsed {
        Some(space) => format!("{written} ({} components)", space.components()),
        None => format!("{written} (unreadable)"),
    }
}
