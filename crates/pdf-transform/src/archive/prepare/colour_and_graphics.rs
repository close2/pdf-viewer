//! The colour and graphics family of the preparations: the output intent ISO 19005-2 section
//! 6.2.3 asks for, its destination profile, and the `DeviceN` `/DefaultCMYK` of section 6.2.4.3.
//!
//! They are one family because each answers a row whose requirement identifier begins
//! `graphics/`, and each depends on the same reading of the document: which device families its
//! pages paint in, and which profile, if any, the file already holds for them. The family's
//! headings are `decision/colour_and_graphics`'s, section 4 of `doc/pdfa-blocking-issues.md`.

use super::super::decision::{Because, Loss};
use super::super::{ArchivePlan, COMPRESSION_LEVEL};
use super::{NO_SPARE_OBJECT, Spare};
use crate::json::Value;
use pdf_archive::survey::DeviceFamily;
use pdf_model::icc::Identification;
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};
use pdf_syntax::serialize::flate_encode;
use std::collections::BTreeSet;

/// The RGB profile this program ships.
///
/// `doc/questions/A18`: ship the standard sRGB profile, with a flag to override it.
/// `data/icc/PROVENANCE.md` records which of the ICC's four sRGB profiles this is and why — the
/// short answer being that ISO 19005-2 section 6.2.4.2 names the ICC editions a profile may
/// conform to and the ICC's headline v4 download conforms to none of them. It is `static` data,
/// so it costs no parse time until something asks for it.
pub(in crate::archive) const SRGB: &[u8] = include_bytes!("../../../../../data/icc/sRGB2014.icc");

/// The CMYK profile this program ships.
///
/// `doc/adr/1153`: the same decision as [`SRGB`]'s, taken for the other family a destination
/// profile can be — ISO 19005-2 section 6.2.4.3 licenses `DeviceCMYK` through an output intent
/// whose profile is CMYK, and the RGB one cannot answer that row whatever else it does.
/// `data/icc/PROVENANCE.md` carries the edition argument for a `prtr` class, the licence read
/// out of the file's own `cprt` tag, and the alternatives declined. Also `static`.
pub(in crate::archive) const CMYK: &[u8] =
    include_bytes!("../../../../../data/icc/GRACoL2006_Coated1v2.icc");

/// The profile of this renderer's own assumed process inks: an output profile over CMYK whose
/// `A2B` is `pdf_colour::colour::assumed_inks_profile`'s sampling of the ink cube every
/// uncharacterised `DeviceCMYK` colour is drawn through (ADRs 0263, 2700).
///
/// The owner's answer A780: an archive that licenses `DeviceCMYK` its source never characterised
/// states the inks the source was drawn with, so that it draws as the source did, here and in a
/// reader that follows the profile; only a profile that accepts an assumed real press
/// (`press-assumed`, which `as-if-printed` authorises) states [`CMYK`] instead (ADR 2701). Checked
/// in, and regenerated and compared by a test of `pdf-colour`'s, so nothing computes it at run
/// time. `static` data, like the other two.
pub(in crate::archive) const ASSUMED_INKS: &[u8] =
    include_bytes!("../../../../../data/icc/quorra-assumed-inks.icc");

/// Why the `DeviceN` `/DefaultCMYK` could not be built for this document.
///
/// §10.4.2.5's transform produces `DeviceRGB`, so the alternate space this construction needs is
/// an **RGB** ICC profile: writing the transform's three numbers into a space of any other family
/// would be asserting an arithmetic the clause does not state.
const NO_RGB_ALTERNATE: &str = "the DeviceN DefaultCMYK this file needs states ISO 32000-2 \
     §10.4.2.5's transform, whose result is RGB, so its alternate space has to be an RGB ICC \
     profile — and the profile this conversion has is of another colour family. Supply the \
     press's CMYK profile with --output-intent-profile and it answers the clause instead";

/// Why no output intent could be prepared for this document.
const NO_USABLE_PROFILE: &str = "an output intent needs a destination profile that is an ICC \
     profile of an output or monitor class over grey, RGB or CMYK, and this conversion has none: \
     either the profile supplied with --output-intent-profile is not one, or the file already \
     holds a destination profile that is not — and ISO 19005 requires every entry of an \
     OutputIntents array to name the same profile object, so a second one cannot be added beside \
     it";

/// Where an output intent's destination profile came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileSource {
    /// One of the two profiles this program ships — `doc/questions/A18`'s default.
    ///
    /// Which of them is `shipped_for`'s decision, and the report says which by printing the
    /// profile's own `desc` tag and colour space beside this word rather than by having two.
    Shipped,
    /// One the caller supplied.
    Supplied,
    /// One the document already held.
    ///
    /// Not a choice this conversion made: ISO 19005 requires every entry of an `OutputIntents`
    /// array that states a destination profile to state the *same* object, so a file that already
    /// holds one decides what a new entry names.
    AlreadyInTheFile,
}

impl ProfileSource {
    /// A stable word for the report's machine-readable form.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Shipped => "shipped",
            Self::Supplied => "supplied",
            Self::AlreadyInTheFile => "already-in-the-file",
        }
    }

    /// Where the profile came from, in one clause for a person.
    #[must_use]
    pub const fn describe(self) -> &'static str {
        match self {
            Self::Shipped => "a profile this program ships",
            Self::Supplied => "the profile you supplied",
            Self::AlreadyInTheFile => "the destination profile this file already held",
        }
    }
}

/// The destination profile a conversion's output intent names, as the report states it.
///
/// **The `cprt` tag is the load-bearing field.** `doc/pdf-a-conversion-limits.md` section 10.1
/// records the ICC's own guidance that a profile's copyright owner and terms of use live in its
/// header's creator field and its `cprt` tag, and turns that into a rule: a user who embeds
/// somebody else's press profile is told whose it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DestinationProfile {
    /// Where it came from.
    pub source: ProfileSource,
    /// The profile's `desc` tag: what it calls itself.
    pub describes: Option<String>,
    /// Its `cprt` tag: whose profile it is, and on what terms.
    pub copyright: Option<String>,
    /// The colour space its own header states.
    pub space: String,
}

impl DestinationProfile {
    /// The profile as JSON.
    pub(in crate::archive) fn to_json(&self) -> Value {
        Value::Object(vec![
            ("source".to_owned(), Value::text(self.source.word())),
            (
                "describes".to_owned(),
                self.describes
                    .as_ref()
                    .map_or(Value::Null, |text| Value::text(text.clone())),
            ),
            (
                "copyright".to_owned(),
                self.copyright
                    .as_ref()
                    .map_or(Value::Null, |text| Value::text(text.clone())),
            ),
            ("space".to_owned(), Value::text(self.space.clone())),
        ])
    }
}

/// The output intent a conversion is in a position to add.
pub(in crate::archive) struct Intent {
    /// The object the destination profile is, in the *source's* numbering.
    pub(in crate::archive) destination: ObjectId,
    /// The profile stream this conversion adds, where it adds one.
    pub(in crate::archive) written: Option<Object>,
    /// The colour family the profile's own header states.
    pub(in crate::archive) family: DeviceFamily,
    /// What the report says about the profile.
    pub(in crate::archive) reported: DestinationProfile,
    /// Whether the profile this conversion adds is [`ASSUMED_INKS`], the press this renderer
    /// already drew the source's `DeviceCMYK` through (ADR 2700).
    pub(in crate::archive) assumed_inks: bool,
}

/// The `/DefaultCMYK` a conversion is in a position to write, and the objects it needs.
///
/// ISO 19005-2 section 6.2.4.3's second licence, built from the standard and nothing else:
/// §8.6.6.5's `DeviceN` over the four names that clause reserves for a CMYK device's process
/// colourants, §7.10.5's PostScript calculator function stating §10.4.2.5's conversion, and the
/// ICC sRGB profile the output intent would have named as the alternate space §8.6.6.4 says the
/// tint transform's output is interpreted in.
pub(in crate::archive) struct DefaultCmyk {
    /// The colour space array, in the *source's* numbering.
    pub(in crate::archive) space: ObjectId,
    /// The two objects this conversion adds for it.
    pub(in crate::archive) written: [(ObjectId, Object); 2],
}

/// The two rows that say a page drew in `DeviceRGB` where nothing licensed it.
///
/// ISO 19005-2 section 6.2.4.3 and ISO 19005-4 section 6.2.4.3, one row per part.
pub(super) const DEVICE_RGB_ROWS: [&str; 2] = [
    "graphics/device-rgb-needs-a-default-or-an-rgb-output-intent",
    "graphics/device-rgb-needs-a-default-a-blending-space-or-an-rgb-output-intent",
];

/// The two rows that say a page drew in `DeviceCMYK` where nothing licensed it.
///
/// The same two subclauses. `DeviceGray` has rows of its own and they are deliberately absent
/// from both lists: each part licenses grey through a PDF/A output intent of *any* family, so a
/// grey page asks nothing of the choice below.
pub(super) const DEVICE_CMYK_ROWS: [&str; 2] = [
    "graphics/device-cmyk-needs-a-default-or-a-cmyk-output-intent",
    "graphics/device-cmyk-needs-a-default-a-blending-space-or-a-cmyk-output-intent",
];

/// Which of the two shipped profiles this document's own device colour asks for.
///
/// **One document, one destination profile.** ISO 19005-2 section 6.2.3 and ISO 19005-4
/// section 6.2.3 require every entry of one `OutputIntents` array that states a
/// `DestOutputProfile` to name the *same* object, and section 6.2.4.3 of each licenses a device
/// colour space only through a destination profile of that space's own family. A page drawing in
/// both `DeviceRGB` and `DeviceCMYK` therefore cannot be licensed by output intents at all —
/// whichever profile is embedded, one of the two rows stays failed. So this is not a preference
/// between two good profiles; it is which row gets answered.
///
/// The rule is one sentence: **the CMYK profile where the document's unlicensed device colour is
/// CMYK and none of it is RGB, and the RGB profile otherwise.** What settles the tie is that only
/// the CMYK row has a second licence — ISO 19005-2 section 6.2.4.3 admits a `DeviceN`-based
/// `/DefaultCMYK`, which [`prepare_default_cmyk`] builds out of §10.4.2.5's own transform and
/// which `Answer::CmykUnderPartTwo` takes exactly when the profile in hand is not CMYK. Under
/// part 2, then, this order licenses a mixed document *completely* and the other order does not.
/// Part 4 states no such second licence, so a mixed document there keeps one refusal whichever
/// way the choice goes; taking the same order in both parts is what makes this one rule rather
/// than two, and part 4's CMYK row is then refused with the sentence that asks the operator for
/// the press's profile (`super::super::supplied_cmyk`, `doc/adr/2130`).
///
/// `doc/adr/1153` is the argument, including why part 4's page-level output intents are not the
/// answer to a document whose RGB and CMYK live on different pages.
///
/// **Which CMYK profile** is the owner's answer A780 (ADR 2701): [`ASSUMED_INKS`], which states
/// the inks this renderer drew the source through, unless the profile in force authorises the
/// loss `press-assumed` — a real press assumed for colour the file never characterised, which
/// `as-if-printed` accepts as what a print scanned back in would show — and then [`CMYK`].
fn shipped_for(plan: &ArchivePlan, failed: &BTreeSet<&'static str>) -> &'static [u8] {
    let cmyk = DEVICE_CMYK_ROWS.iter().any(|id| failed.contains(id));
    let rgb = DEVICE_RGB_ROWS.iter().any(|id| failed.contains(id));
    match (cmyk && !rgb, plan.authorised.grants(Loss::PressAssumed)) {
        (false, _) => SRGB,
        (true, true) => CMYK,
        (true, false) => ASSUMED_INKS,
    }
}

/// The `/OutputIntents` array the catalog states, resolved to its entries.
pub(in crate::archive) fn output_intent_entries(
    document: &Document,
    catalog: &Dictionary,
) -> Vec<Object> {
    document
        .get_key(catalog, "OutputIntents")
        .as_array()
        .map(<[Object]>::to_vec)
        .unwrap_or_default()
}

/// The colour family a profile's header names, where it is one of ISO 19005's three.
fn family_of(stated: &Identification) -> Option<DeviceFamily> {
    // ISO 19005-2 section 6.2.3 and ISO 19005-4 section 6.2.3 admit an output or a monitor
    // profile and no other class, and grey, RGB or CMYK and no other space. `pdf_archive` judges
    // both of a file's own profile; this asks the same question of one about to be written, so
    // that a conversion cannot add a profile the validator would then reject.
    if stated.class != *b"prtr" && stated.class != *b"mntr" {
        return None;
    }
    match &stated.space {
        b"GRAY" => Some(DeviceFamily::Gray),
        b"RGB " => Some(DeviceFamily::Rgb),
        b"CMYK" => Some(DeviceFamily::Cmyk),
        _ => None,
    }
}

/// Prepares the output intent: the profile it names, and where that profile comes from.
pub(super) fn prepare_intent(
    plan: &ArchivePlan,
    document: &Document,
    catalog: &Dictionary,
    failed: &BTreeSet<&'static str>,
    spare: &mut Spare,
) -> Result<Intent, Because> {
    let wrong = Because::NotBuiltYet(NO_USABLE_PROFILE);
    // ISO 19005-2 section 6.2.3 and ISO 19005-4 section 6.2.3: where an OutputIntents array holds
    // more than one entry, every entry stating a destination profile states the *same* object. So
    // a file that already holds one decides what a new entry may name, and adding a second
    // profile beside it is not open to this conversion at all.
    if let Some(destination) = held_destination_profile(document, catalog) {
        let Object::Stream(stream) = document.get(destination) else {
            return Err(wrong);
        };
        let data = document.decoded_stream_data(&stream).ok_or(wrong)?;
        let stated = Identification::read(&data).ok_or(wrong)?;
        let family = family_of(&stated).ok_or(wrong)?;
        return Ok(Intent {
            destination,
            written: None,
            family,
            reported: DestinationProfile {
                source: ProfileSource::AlreadyInTheFile,
                space: stated.space_name(),
                describes: stated.description,
                copyright: stated.copyright,
            },
            assumed_inks: false,
        });
    }

    let (source, bytes) = match &plan.profile {
        Some(supplied) => (ProfileSource::Supplied, supplied.to_vec()),
        None => (ProfileSource::Shipped, shipped_for(plan, failed).to_vec()),
    };
    let assumed_inks = bytes == ASSUMED_INKS;
    let stated = Identification::read(&bytes).ok_or(wrong)?;
    let family = family_of(&stated).ok_or(wrong)?;
    let destination = spare
        .take(document)
        .ok_or(Because::NotBuiltYet(NO_SPARE_OBJECT))?;
    Ok(Intent {
        destination,
        written: Some(profile_stream(&bytes, family)),
        family,
        reported: DestinationProfile {
            source,
            space: stated.space_name(),
            describes: stated.description,
            copyright: stated.copyright,
        },
        assumed_inks,
    })
}

/// A destination profile this reader does not draw through, and the profile written in its place.
///
/// ISO 19005-2 and ISO 19005-4 section 6.2.3 ask for an output or a monitor profile over grey,
/// RGB or CMYK. §14.11.5 makes an output intent "for informational purposes only" and leaves a
/// processor "free to disregard it", and this reader disregards one whose profile it cannot parse
/// (`pdf_model`'s selection takes the first *usable* entry): such a page was drawn as though the
/// file stated no intent. Writing the profile a file stating none would be given, at the object
/// every entry names, therefore states the page as it was shown — the same statement the
/// `OutputIntent` rewrite makes for a file with no array at all (`doc/adr/0948`, `doc/adr/2007`).
///
/// Only an RGB slot is answered. A `/N 4` profile replaced by a CMYK one would put every
/// `DeviceCMYK` colour through that profile where this reader drew §10.4.2.5's conversion, and no
/// grey profile is shipped; a profile this reader *does* draw through is refused, since any
/// replacement moves the colours drawn through it and restating its class would be a claim about
/// a device nobody made.
#[derive(Debug)]
pub(in crate::archive) struct ReplacedDestination {
    /// The profile stream every entry of the catalog's array names, in the source's numbering.
    pub(in crate::archive) at: ObjectId,
    /// The stream written there.
    pub(in crate::archive) written: Object,
    /// What the report says about the profile written.
    reported: DestinationProfile,
}

impl ReplacedDestination {
    /// The output intent this replacement leaves the document with.
    pub(super) fn intent(&self) -> Intent {
        Intent {
            destination: self.at,
            written: None,
            family: DeviceFamily::Rgb,
            reported: self.reported.clone(),
            assumed_inks: false,
        }
    }
}

/// Why a destination profile this reader draws through is not replaced.
const DESTINATION_DRAWN_THROUGH: &str = "the destination profile this file's own output intent \
     carries is not one ISO 19005 section 6.2.3 admits - an output or a monitor profile over \
     grey, RGB or CMYK - and this reader draws the page's device colour through it all the same. \
     Replacing it would move every colour drawn through it, and restating its class or colour \
     space would be a claim about a device nobody made, so the requirement is refused \
     (doc/adr/2007)";

/// Why an unreadable destination profile outside an RGB slot is not replaced.
const DESTINATION_NOT_RGB: &str = "the destination profile this file's own output intent carries \
     is not one this reader can draw through, so the page was drawn as though no intent were \
     stated - but its stream states no /N of 3, and the profile a file stating no intent would \
     be given for it is not one this program can write without moving a colour: a CMYK profile \
     would put every DeviceCMYK colour through it where this reader drew ISO 32000-2 10.4.2.5's \
     conversion, and no grey profile is shipped (doc/adr/2007)";

/// Why a destination profile not named alike by every entry, or named from a page, is not
/// replaced.
const DESTINATION_NOT_ONE_OBJECT: &str = "the output intents in this file do not name one \
     destination profile object from the document catalog alone - an entry names another, names \
     its profile directly rather than by reference, or a page states an OutputIntents array of \
     its own - so replacing the profile at one object would leave the requirement failing at \
     another (doc/adr/2007)";

/// The one destination profile object every entry of the catalog's `OutputIntents` array names,
/// and its stream.
///
/// A rewrite of the profile at one object answers ISO 19005-2 and ISO 19005-4 section 6.2.3 only
/// where that object is the profile of every entry, which the subclause's own paragraph on several
/// entries requires anyway: an entry naming another object, a profile stated directly rather than
/// by reference, or a page stating an `OutputIntents` array of its own each leaves the requirement
/// failing somewhere the rewrite did not reach (`doc/adr/2007`, `doc/adr/2021`).
pub(in crate::archive) fn the_destination_object(
    document: &Document,
    catalog: &Dictionary,
) -> Result<(ObjectId, std::sync::Arc<Stream>), Because> {
    let not_one = Because::NotBuiltYet(DESTINATION_NOT_ONE_OBJECT);
    let mut named = BTreeSet::new();
    for entry in output_intent_entries(document, catalog) {
        let entry = document.resolve(&entry);
        let Some(dict) = entry.as_dict() else {
            continue;
        };
        match dict.get("DestOutputProfile") {
            None => {}
            Some(Object::Reference(id)) => {
                named.insert(*id);
            }
            Some(_) => return Err(not_one),
        }
    }
    let pages = pdf_model::Pages::new(document);
    if pages
        .every()
        .into_iter()
        .flatten()
        .any(|page| page.dict.get("OutputIntents").is_some())
    {
        return Err(not_one);
    }
    let mut named = named.into_iter();
    let (Some(at), None) = (named.next(), named.next()) else {
        return Err(not_one);
    };
    let Object::Stream(stream) = document.get(at) else {
        return Err(not_one);
    };
    Ok((at, stream))
}

/// The replacement, where every condition [`ReplacedDestination`] states holds.
pub(super) fn replaced_destination(
    plan: &ArchivePlan,
    document: &Document,
    catalog: &Dictionary,
) -> Result<ReplacedDestination, Because> {
    let (at, stream) = the_destination_object(document, catalog)?;
    let drawn_through = document
        .decoded_stream_data(&stream)
        .is_some_and(|data| pdf_model::icc::Profile::parse(&data).is_some());
    if drawn_through {
        return Err(Because::NotBuiltYet(DESTINATION_DRAWN_THROUGH));
    }
    if document.get_key(&stream.dict, "N").as_integer() != Some(3) {
        return Err(Because::NotBuiltYet(DESTINATION_NOT_RGB));
    }
    let (source, bytes) = match &plan.profile {
        Some(supplied) => (ProfileSource::Supplied, supplied.to_vec()),
        None => (ProfileSource::Shipped, SRGB.to_vec()),
    };
    let wrong = Because::NotBuiltYet(NO_USABLE_PROFILE);
    let stated = Identification::read(&bytes).ok_or(wrong)?;
    if family_of(&stated) != Some(DeviceFamily::Rgb) {
        return Err(Because::NotBuiltYet(DESTINATION_NOT_RGB));
    }
    Ok(ReplacedDestination {
        at,
        written: profile_stream(&bytes, DeviceFamily::Rgb),
        reported: DestinationProfile {
            source,
            space: stated.space_name(),
            describes: stated.description,
            copyright: stated.copyright,
        },
    })
}

/// The PDF/A output intents that state no destination profile, and the profile they are given.
///
/// ISO 19005-2 and ISO 19005-4 section 6.2.3 require a PDF/A output intent to state a valid ICC
/// profile stream as its `DestOutputProfile`. §14.11.5's Table 401 lets an output condition that is
/// not a recognised standard be identified as `Custom`, the profile then defining it; an entry
/// stating `Custom` and no profile defines nothing, and this reader, which draws through the first
/// entry whose profile it can read, drew the page as though the file stated no intent. That entry
/// is the one [`intent_dictionary`] writes, `Custom` included, lacking the profile — so it is given
/// the profile that construction would name: one another entry already names, which the same
/// subclause makes every entry's, or the profile a file stating no intent is given. Its `Info` is
/// the producer's and stays (`doc/adr/2035`).
///
/// Refused by name: an entry naming as its profile something that is not a stream, one whose
/// output condition is not `Custom` — a registered condition's profile is the registry's, which
/// this program does not hold — and a page stating an `OutputIntents` array of its own.
#[derive(Debug)]
pub(in crate::archive) struct SuppliedDestination {
    /// The entries written as objects of their own, in the source's numbering.
    pub(in crate::archive) at: BTreeSet<ObjectId>,
    /// Whether an entry is written directly in the catalog's array.
    pub(in crate::archive) in_catalog: bool,
    /// The profile every such entry names.
    pub(in crate::archive) destination: ObjectId,
}

/// Why a PDF/A output intent stating no profile is not given one.
const DESTINATION_NOT_SUPPLIED: &str = "a PDF/A output intent in this file states no destination \
     profile stream, and it is not one this conversion can complete: it names as its profile \
     something that is not a stream, its OutputConditionIdentifier names a condition other than \
     Custom - whose profile is a registry's, which this program does not hold - or a page states \
     an OutputIntents array of its own (doc/adr/2035)";

/// The completion [`SuppliedDestination`] describes, where every condition it states holds.
pub(super) fn supplied_destination(
    document: &Document,
    catalog: &Dictionary,
    intent: &Intent,
) -> Result<SuppliedDestination, Because> {
    let refused = Because::NotBuiltYet(DESTINATION_NOT_SUPPLIED);
    let pages = pdf_model::Pages::new(document);
    if pages
        .every()
        .into_iter()
        .flatten()
        .any(|page| page.dict.get("OutputIntents").is_some())
    {
        return Err(refused);
    }
    let mut at = BTreeSet::new();
    let mut in_catalog = false;
    for entry in output_intent_entries(document, catalog) {
        let resolved = document.resolve(&entry);
        let Some(dict) = resolved.as_dict() else {
            continue;
        };
        if !needs_a_profile(document, dict) {
            continue;
        }
        let custom = document
            .get_key(dict, "OutputConditionIdentifier")
            .as_string()
            == Some(&b"Custom"[..]);
        if dict.get("DestOutputProfile").is_some() || !custom {
            return Err(refused);
        }
        match entry.as_reference() {
            Some(id) => {
                at.insert(id);
            }
            None => in_catalog = true,
        }
    }
    if at.is_empty() && !in_catalog {
        return Err(refused);
    }
    Ok(SuppliedDestination {
        at,
        in_catalog,
        destination: intent.destination,
    })
}

/// Whether an output intent is a PDF/A one stating no destination profile stream: the entry ISO
/// 19005-2 and ISO 19005-4 section 6.2.3 find failing.
pub(in crate::archive) fn needs_a_profile(document: &Document, entry: &Dictionary) -> bool {
    let pdfa = document
        .get_key(entry, "S")
        .as_name()
        .is_some_and(|name| name.as_bytes() == b"GTS_PDFA1");
    pdfa && !matches!(
        document.get_key(entry, "DestOutputProfile"),
        Object::Stream(_)
    )
}

/// The object an entry of the catalog's `OutputIntents` array already names as its destination
/// profile, where one does.
fn held_destination_profile(document: &Document, catalog: &Dictionary) -> Option<ObjectId> {
    output_intent_entries(document, catalog)
        .iter()
        .filter_map(|entry| document.resolve(entry).as_dict().cloned())
        .find_map(|entry| {
            entry
                .get("DestOutputProfile")
                .and_then(Object::as_reference)
        })
}

/// The ICC profile stream an output intent names.
///
/// §14.11.5's Table 401 makes `DestOutputProfile` "\[a]n ICC profile stream defining the
/// transformation from the PDF document's source colours to output device colourants" and says
/// that "\[t]he format of the profile stream is the same as that used in specifying an `ICCBased`
/// colour space", which is §8.6.5.5's — hence the `/N`.
pub(in crate::archive) fn profile_stream(bytes: &[u8], family: DeviceFamily) -> Object {
    let components = match family {
        DeviceFamily::Gray => 1,
        DeviceFamily::Rgb => 3,
        DeviceFamily::Cmyk => 4,
    };
    let mut dict = Dictionary::new();
    dict.insert(Name::new(&b"N"[..]), Object::Integer(components));
    let data = match flate_encode(bytes, COMPRESSION_LEVEL) {
        Some(encoded) => {
            dict.insert(
                Name::new(&b"Filter"[..]),
                Object::Name(Name::new(&b"FlateDecode"[..])),
            );
            encoded
        }
        None => bytes.to_vec(),
    };
    dict.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(data.len()).unwrap_or(i64::MAX)),
    );
    Object::Stream(std::sync::Arc::new(Stream {
        dict,
        data: data.into(),
        decryption_failed: false,
    }))
}

/// The output intent dictionary a conversion adds.
///
/// §14.11.5's Table 401 decides every entry, and one of them is a choice this file
/// records rather than derives. `OutputConditionIdentifier` is required and the table says that
/// "\[i]f the intended production condition is not a recognised standard, the value of this entry
/// may be `Custom` or an application-specific, machine-readable name" — so `Custom` is the
/// standard's own word for exactly this case, and no convention is being copied from anywhere.
/// The same sentence then says that "\[t]he `DestOutputProfile` entry defines the ICC profile, and
/// the `Info` entry shall be used for further human-readable identification", which is why `Info`
/// carries the profile's own `desc` tag: it is read out of the profile rather than written about
/// it. `S` is `GTS_PDFA1`, which both parts' section 6.2.3 requires of a PDF/A output intent.
pub(in crate::archive) fn intent_dictionary(intent: &Intent) -> Dictionary {
    let mut out = Dictionary::new();
    out.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"OutputIntent"[..])),
    );
    out.insert(
        Name::new(&b"S"[..]),
        Object::Name(Name::new(&b"GTS_PDFA1"[..])),
    );
    out.insert(
        Name::new(&b"OutputConditionIdentifier"[..]),
        Object::String(b"Custom".to_vec().into()),
    );
    let info = intent
        .reported
        .describes
        .clone()
        .unwrap_or_else(|| "the ICC profile embedded beside this entry".to_owned());
    out.insert(
        Name::new(&b"Info"[..]),
        Object::String(info.into_bytes().into()),
    );
    out.insert(
        Name::new(&b"DestOutputProfile"[..]),
        Object::Reference(intent.destination),
    );
    out
}

/// The four names §8.6.6.5 reserves for the subtractive process colourants of a CMYK device.
///
/// > The names Cyan , Magenta , Yellow and Black are reserved to name the subtractive process
/// > colourants of a CMYK device.
///
/// So a `DeviceN` over exactly those four names the four components a `DeviceCMYK` value already
/// has, in the order §8.6.4.4 gives them, and none of them is a spot colourant — which is why the
/// space needs no `Colorants` entry under ISO 19005-2 section 6.2.4.4.
const PROCESS_COLOURANTS: [&[u8]; 4] = [b"Cyan", b"Magenta", b"Yellow", b"Black"];

/// A PostScript calculator function, of the kind §7.10.5 defines, stating one clause's arithmetic.
///
/// The clause is §10.4.2.5:
///
/// > The black component shall be added to each of the other components, which shall then be
/// > converted to their complementary colours by subtracting them each from 1.0.
///
/// and its formula is `red = 1.0 - min(1.0, cyan + black)` with green from magenta and blue from
/// yellow. The operator set of §7.10.5.2 has no `min`, so the clamp is the `dup 1 gt { pop 1 } if`
/// each line ends its addition with — the same value by the operators that clause does define.
///
/// The stack begins as `c m y k` and each line rotates one component to the top, adds the `k`
/// that is now one place further down, clamps, complements, and leaves the result behind; the
/// last line rotates the spent `k` to the top and drops it, leaving `red green blue`.
const CMYK_TO_RGB: &[u8] = b"{ 4 3 roll 1 index add dup 1 gt { pop 1 } if 1 exch sub\n\
    4 3 roll 2 index add dup 1 gt { pop 1 } if 1 exch sub\n\
    4 3 roll 3 index add dup 1 gt { pop 1 } if 1 exch sub\n\
    4 3 roll pop }";

/// Prepares the `DeviceN` `/DefaultCMYK`: the tint transform, and the space that names it.
pub(super) fn prepare_default_cmyk(
    document: &Document,
    intent: &Intent,
    spare: &mut Spare,
) -> Result<DefaultCmyk, Because> {
    if intent.family != DeviceFamily::Rgb {
        return Err(Because::NotBuiltYet(NO_RGB_ALTERNATE));
    }
    let missing = Because::NotBuiltYet(NO_SPARE_OBJECT);
    let tint = spare.take(document).ok_or(missing)?;
    let colour_space = spare.take(document).ok_or(missing)?;
    Ok(DefaultCmyk {
        space: colour_space,
        written: [
            (tint, tint_transform()),
            (colour_space, device_n(tint, intent.destination)),
        ],
    })
}

/// The tint transform stream: §7.10.5's type 4 function over §10.4.2.5's arithmetic.
///
/// `/Domain` and `/Range` are §7.10.2's Table 38 entries, four in and three out, each component
/// over the unit interval §8.6.4.4 gives a `DeviceCMYK` component and §8.6.4.3 an additive one.
/// Written unfiltered, for the reason §14.3.2's metadata packet is: it is forty bytes of text,
/// and a reader who opens the file with an editor can see what was asserted about their colours.
fn tint_transform() -> Object {
    let mut dict = Dictionary::new();
    dict.insert(Name::new(&b"FunctionType"[..]), Object::Integer(4));
    let unit_interval = |components: usize| {
        Object::Array(
            std::iter::repeat_with(|| [Object::Integer(0), Object::Integer(1)])
                .take(components)
                .flatten()
                .collect(),
        )
    };
    dict.insert(Name::new(&b"Domain"[..]), unit_interval(4));
    dict.insert(Name::new(&b"Range"[..]), unit_interval(3));
    dict.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(CMYK_TO_RGB.len()).unwrap_or(i64::MAX)),
    );
    Object::Stream(std::sync::Arc::new(Stream {
        dict,
        data: CMYK_TO_RGB.to_vec().into(),
        decryption_failed: false,
    }))
}

/// The colour space array: §8.6.6.5's `DeviceN`, over §8.6.5.5's `ICCBased` alternate.
fn device_n(tint: ObjectId, profile: ObjectId) -> Object {
    let names = PROCESS_COLOURANTS
        .into_iter()
        .map(|name| Object::Name(Name::new(name)))
        .collect();
    Object::Array(vec![
        Object::Name(Name::new(&b"DeviceN"[..])),
        Object::Array(names),
        Object::Array(vec![
            Object::Name(Name::new(&b"ICCBased"[..])),
            Object::Reference(profile),
        ]),
        Object::Reference(tint),
    ])
}

/// Whether the source already states an output intent of its own.
///
/// Asked beside the one this conversion may be adding, because the appended page needs *an* output
/// intent to exist in the output rather than needing this conversion to have written it —
/// [`preserve::NO_OUTPUT_INTENT`](super::super::preserve::NO_OUTPUT_INTENT) is the reading.
pub(super) fn states_an_output_intent(document: &Document, catalog: Option<&Dictionary>) -> bool {
    catalog.is_some_and(|catalog| !output_intent_entries(document, catalog).is_empty())
}
