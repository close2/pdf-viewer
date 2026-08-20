//! What a document says it needs: ISO 32000-2 §12.11's requirements and §7.12's extensions.
//!
//! Two clauses, one question. §12.11's `/Requirements` is a document naming "feature(s) of PDF
//! beyond those commonly expected … required for correct handling in accordance with this
//! document"; §7.12's `/Extensions` is a document naming the *developer* extensions it was
//! written against. Neither draws anything. Both let a reader say, before a page is on the
//! screen, that this file wants something this program does not do.
//!
//! # Why neither becomes a page report
//!
//! Principle 3's reflex is to report an unmet requirement as unsupported input. §12.11.6 is
//! explicit that it is not one:
//!
//! > If the reader encounters an unsupported feature (whether or not that feature was declared
//! > as a requirement), it shall take the normal fallback actions.
//!
//! and its NOTE 1 adds that "there is no formal connection between the requirement type and the
//! operation of the associated feature(s)". So the declaration is a *statement about the
//! document*, and the place a missing feature is reported is where the feature is used — which
//! is what every other report in this tree already does. A page whose content stream is
//! perfectly drawable does not become incomplete because the catalog said the document wants
//! form interaction.
//!
//! What the declaration is good for is the thing a page report cannot do: telling a person
//! *before* they trust what they are looking at. `viewer-ui` prints it once when the document
//! opens, in the same voice as `--no-sandbox`'s line about what it gave up.
//!
//! # What the clause asks for and this program does not do
//!
//! §12.11.6 says that when requirements cannot be met "then the processing of the document
//! shall not continue", against a penalty computed as §12.11.3 describes. **This paragraph said
//! that §12.11.3 states no threshold, from the day it was written until the
//! six-hundred-and-twenty-sixth session, and it states one** — the
//! clause's last paragraph, quoted verbatim under [`penalty_total`]. So the computation is a
//! comparison the clause *does* complete, and this program performs it: the total is a fact
//! about the document, said out loud beside the requirements it is a total of.
//!
//! What this program then declines to do is obey it, and that is now a departure from a stated
//! `should` rather than an appeal to a silence. Refusing to open a file a person asked for is a
//! worse failure for a viewer than showing it with its limits named — and the clause's own
//! wording is what makes the choice cheap: "should not attempt to display", not `shall`.
//!
//! **The decision is not taken here**, which is `CLAUDE.md` principle 3's shape for a
//! restriction a document asserts over its reader: this crate computes and reports, the host
//! decides, and the four levels — off, on, ask, warn — can be added where the host already is
//! without revisiting anything below it.
//!
//! **0 of the 974 corpus documents state a `/Requirements` array**; 9 state an `/Extensions`
//! dictionary, all of them Adobe's `/ADBE` prefix at extension level 3 or 8 over a base version
//! of 1.7.

use std::collections::BTreeMap;

use pdf_syntax::{Dictionary, Document, Object};

/// Most requirement dictionaries read from one `/Requirements` array.
///
/// Table 275 defines twenty-four types and a document states the ones it uses; an array longer
/// than this is a file making a reader work rather than a document asking for something.
const MAX_REQUIREMENTS: usize = 256;

/// One entry of §12.11's `/Requirements` array. Table 273.
#[derive(Debug, Clone, PartialEq)]
pub struct Requirement {
    /// Table 273's `/S`: which feature the document needs.
    pub kind: Kind,
    /// Table 273's `/V`, the version of that feature, as the name the file states.
    ///
    /// Kept as written. §12.11.4 makes it "a name that specifies a version number, represented
    /// as two or more decimal integers separated by a period", and its NOTE says the name form
    /// exists "in order to avoid any ambiguities caused by inexact internal representation of
    /// decimal fractions" — so parsing it to a float here would reintroduce exactly what the
    /// clause avoided. `None` where the entry is absent, which the clause makes meaningful:
    /// "determining if the requirement is satisfied shall be done without regard to version
    /// number".
    pub version: Option<String>,
    /// Table 273's `/Penalty`, **default 100**, clamped to the range §12.11 states for it.
    ///
    /// > An integer value that shall be between 0 and 100 (inclusive) that represents the
    /// > penalty value to be applied when this requirement cannot be met by a PDF processor.
    ///
    /// §12.11.3 gives the two ends their meanings: 0 says "although the document uses this
    /// feature the need is optional", and 100 that "this document will not produce the author's
    /// intent unless the PDF processor can fully support this feature".
    ///
    /// The clamp costs nothing that §12.11.3's threshold needs, and it is worth saying why
    /// rather than assuming it: the threshold is on a *total* ([`penalty_total`]), and a file
    /// stating an entry above 100 has already broken the "shall" quoted above, so the clamp
    /// bounds a value the standard had bounded first.
    pub penalty: u8,
}

/// Table 275's requirement types.
///
/// The table is closed as of PDF 2.0 and open in principle — "[a]dditional requirement types,
/// including ones identifying vendor-specific features, may be registered" — so an unrecognised
/// name is kept rather than dropped: a document that needs something this reader cannot even
/// name is exactly the case a person wants to be told about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    /// `OCInteract`: an optional content panel and set-OCG-state actions.
    OcInteract,
    /// `OCAutoStates`: §8.11.4.4's usage application dictionaries.
    OcAutoStates,
    /// `AcroFormInteract`: interacting with forms, and trigger events.
    AcroFormInteract,
    /// `Navigation`: links, outlines, and the `GoTo`, `GoToR` and `URI` actions.
    Navigation,
    /// `Markup`: creating, modifying and deleting markup annotations.
    Markup,
    /// `3DMarkup`: the same for notes on 3D objects.
    ThreeDMarkup,
    /// `Multimedia`: §13.2's multimedia framework.
    Multimedia,
    /// `U3D`: 3D streams conforming to the U3D specification.
    U3d,
    /// `PRC`: 3D streams conforming to the PRC specification.
    Prc,
    /// `Action`: §12.6's actions in general.
    Action,
    /// `EnableJavaScripts`: executing ECMAScript.
    EnableJavaScripts,
    /// `Attachment`: listing and extracting embedded files.
    Attachment,
    /// `AttachmentEditing`: adding and removing them.
    AttachmentEditing,
    /// `Collection`: §12.3.5's collections.
    Collection,
    /// `CollectionEditing`: editing them.
    CollectionEditing,
    /// `DigSigValidation`: validating signatures.
    DigSigValidation,
    /// `DigSig`: applying one.
    DigSig,
    /// `DigSigMDP`: §12.8.2.2.2's modification detection.
    DigSigMdp,
    /// `RichMedia`: §13.7.2's rich media annotations.
    RichMedia,
    /// `Geospatial2D`: §12.10's geospatial information in page content.
    Geospatial2D,
    /// `Geospatial3D`: the same in 3D annotations.
    Geospatial3D,
    /// `DPartInteract`: navigating §14.12's document part hierarchy.
    DPartInteract,
    /// `SeparationSimulation`: §10.8.3's separation simulation, "sometimes referred to as
    /// 'Overprint Preview'".
    SeparationSimulation,
    /// `Transitions`: §12.4.4's presentations.
    Transitions,
    /// `Encryption`: the specific encryption parameters the requirement's own `/Encrypt` states.
    Encryption,
    /// A registered type this reader does not know, kept by name.
    Other(String),
}

impl Kind {
    /// The name Table 275 gives this type.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::OcInteract => "OCInteract",
            Self::OcAutoStates => "OCAutoStates",
            Self::AcroFormInteract => "AcroFormInteract",
            Self::Navigation => "Navigation",
            Self::Markup => "Markup",
            Self::ThreeDMarkup => "3DMarkup",
            Self::Multimedia => "Multimedia",
            Self::U3d => "U3D",
            Self::Prc => "PRC",
            Self::Action => "Action",
            Self::EnableJavaScripts => "EnableJavaScripts",
            Self::Attachment => "Attachment",
            Self::AttachmentEditing => "AttachmentEditing",
            Self::Collection => "Collection",
            Self::CollectionEditing => "CollectionEditing",
            Self::DigSigValidation => "DigSigValidation",
            Self::DigSig => "DigSig",
            Self::DigSigMdp => "DigSigMDP",
            Self::RichMedia => "RichMedia",
            Self::Geospatial2D => "Geospatial2D",
            Self::Geospatial3D => "Geospatial3D",
            Self::DPartInteract => "DPartInteract",
            Self::SeparationSimulation => "SeparationSimulation",
            Self::Transitions => "Transitions",
            Self::Encryption => "Encryption",
            Self::Other(name) => name,
        }
    }

    /// Reads one of Table 275's names.
    fn read(name: &[u8]) -> Self {
        match name {
            b"OCInteract" => Self::OcInteract,
            b"OCAutoStates" => Self::OcAutoStates,
            b"AcroFormInteract" => Self::AcroFormInteract,
            b"Navigation" => Self::Navigation,
            b"Markup" => Self::Markup,
            b"3DMarkup" => Self::ThreeDMarkup,
            b"Multimedia" => Self::Multimedia,
            b"U3D" => Self::U3d,
            b"PRC" => Self::Prc,
            b"Action" => Self::Action,
            b"EnableJavaScripts" => Self::EnableJavaScripts,
            b"Attachment" => Self::Attachment,
            b"AttachmentEditing" => Self::AttachmentEditing,
            b"Collection" => Self::Collection,
            b"CollectionEditing" => Self::CollectionEditing,
            b"DigSigValidation" => Self::DigSigValidation,
            b"DigSig" => Self::DigSig,
            b"DigSigMDP" => Self::DigSigMdp,
            b"RichMedia" => Self::RichMedia,
            b"Geospatial2D" => Self::Geospatial2D,
            b"Geospatial3D" => Self::Geospatial3D,
            b"DPartInteract" => Self::DPartInteract,
            b"SeparationSimulation" => Self::SeparationSimulation,
            b"Transitions" => Self::Transitions,
            b"Encryption" => Self::Encryption,
            other => Self::Other(String::from_utf8_lossy(other).into_owned()),
        }
    }

    /// Whether this program meets the requirement, and what it can say about it if not.
    ///
    /// **A claim about this tree rather than about the standard**, which is why every arm names
    /// its reason and why the answer is a sentence rather than a boolean. It decays exactly as a
    /// ledger row does: a session that builds a layer panel has to come back and change
    /// `OCInteract`. **It has decayed twice** — three arms in the two-hundred-and-twenty-first
    /// session and nine more in the three-hundred-and-seventy-fifth — so the warning is a record
    /// of what happens rather than a caution that works.
    ///
    /// `None` means met. The three shapes of "not met" are all here on purpose — a feature this
    /// project *excludes* (`CLAUDE.md` principle 5), a feature that is a viewer's rather than a
    /// renderer's and is not built, and a feature nobody has written yet.
    ///
    /// **A reason names what is missing, never a clause as unread.** Every one of the nine arms
    /// corrected in the three-hundred-and-seventy-fifth session was of that second shape — "§12.8
    /// is unread", "§12.10 is unread", "§14.12 is unread" — and a clause this tree reads is
    /// exactly what a later session makes false without noticing. Table 275's own wording is the
    /// discipline: several of its types say "in addition to the requirements of" another, so a
    /// reason has to name the *increment* rather than the whole.
    #[must_use]
    pub fn unmet(&self) -> Option<&'static str> {
        Some(match self {
            // Met: the pieces each of these three clauses names are all here — §8.11.4.4's
            // usage application dictionaries for the `View` event (ADR 0044), §12.5.6.5's
            // links with §12.3.3's outline and the three actions §12.11's own table lists
            // (ADR 0070), and §7.6's encryption at every revision and method (ADR 0031).
            //
            // `Encryption` is answered *by type* while Table 274's `/Encrypt` states "the
            // specific set of encryption parameters" the document needs, and that is sound
            // rather than lucky: §7.6.4.2 implements every `/V` and `/R` the standard defines,
            // and the one value refused is `/R` 5, which Table 21 itself calls "Shall not be
            // used". A parameter set outside that does not exist to be asked about.
            //
            // **Met since the sessions named, and this arm said otherwise for between forty and
            // eighty-six of them.** The doc comment above anticipated exactly that — "a session
            // that builds a layer panel has to come back and change `OCInteract`" — and nothing
            // fires when one does, which is `doc/todo/01`'s sweeps pointed at the source rather
            // than at the ledger (the two-hundred-and-twenty-first session).
            //
            // `OCInteract`: `Query::Layers` answers with §8.11.4.3's list and `Command::SetGroup`
            // switches one unless Table 99's `/Locked` forbids it, drawn in `viewer_ui::chrome`
            // since the hundred-and-sixty-seventh session. `AcroFormInteract`:
            // `ViewState::set_field` since the hundred-and-thirty-fifth, saved by §7.5.6's
            // incremental update since the hundred-and-thirty-sixth. `Attachment`:
            // `Command::Extract` writes an embedded file's decoded bytes with Table 45's
            // checksum checked against them, and the sidebar lists them, since the
            // hundred-and-sixty-seventh (ADR 0145).
            //
            // **The claim is about the program this crate is part of and not about the crate**,
            // which is what makes it decay: the capability is two crates away in every one of
            // these three cases, and no compiler notices when it arrives.
            //
            // **`Collection` joined them in the three-hundred-and-seventy-fifth**, on the same
            // sweep and for the same reason — ADR 0202's collection view is two crates away.
            // Table 275 asks for two things and both are here: "displaying the embedded files
            // referenced from the document's collection dictionary (12.3.5, "Collections") along
            // with any associated metadata", which is `Query::Collection` and the columns
            // `viewer_ui::chrome` builds from Table 153's `/Schema` and each file's Table 46
            // collection item dictionary — both numbers were wrong until the
            // four-hundred-and-thirteenth session, 43 being the file specification dictionary
            // and 44 the additional entries in an embedded file stream, which is
            // `doc/todo/01`'s ninth sweep finding two in one sentence; and "that the user can extract or otherwise view the
            // contents of each item in the collection", which is `Command::Extract` — a
            // collection's items *are* the `/EmbeddedFiles` tree's entries, which is the key that
            // command takes.
            Self::OcAutoStates
            | Self::Navigation
            | Self::Encryption
            | Self::OcInteract
            | Self::AcroFormInteract
            | Self::Attachment
            | Self::Collection => return None,
            // **Eight reasons below were false or expired when the three-hundred-and-seventy-fifth
            // session read them against the code, between six and about a hundred and eighty
            // sessions after each stopped being true.** Every one named a clause as *unread* that
            // this tree reads, which is the cheapest of `doc/todo/01`'s shapes to check and the
            // one nothing fires on. What each says now is the part that is genuinely missing.
            Self::Markup => {
                // Table 275 asks for "the creation, modification and deletion of markup
                // annotations". `Edit::Markup` creates one and writes its appearance (ADR 0196);
                // the other two verbs have no command.
                "markup annotations are created and saved, but not modified or deleted"
            }
            Self::AttachmentEditing => {
                // "In addition to the requirements of the Attachment value" — which is met — so
                // the list is not what is missing.
                "no attachment editing: attachments are listed and extracted, never added, \
                 deleted or renamed"
            }
            Self::CollectionEditing => {
                // Likewise "in addition to the requirements of the Collection value".
                "no collection editing: §12.3.5's collection is displayed, never added to or \
                 changed"
            }
            Self::Action => {
                // Table 275 subsumes `GoTo` and `URI` under `Navigation` and `Attachment`,
                // `SetOCGState` under `OCInteract`, and `JavaScript` under `EnableJavaScripts`,
                // leaving 16 of Table 201's 20 types. Eight are performed — `GoToE`, `GoToDp`,
                // `Thread`, `Hide`, `Named`, `ResetForm`, `ImportData` and `Trans` — and eight
                // are refused by name in `action::refused`.
                "eight of the sixteen §12.6.4 action types this requirement covers are performed \
                 and eight are refused by name"
            }
            Self::Transitions => {
                // **This arm's third decay, and the second one it predicted about itself.** It
                // read "no transition player: §12.4.4's timing is obeyed and the animation
                // between two pages is not drawn", and the animation has been drawn since the
                // three-hundred-and-ninety-third session (ADR 0230) — `viewer_core::transition`
                // shapes the frame at a fraction of the way through and both backends draw it,
                // which §12.4.4's own ledger row has said since that round while this sentence
                // went on saying the opposite for over two hundred more. The doc comment above
                // names exactly this shape; nothing fires when a capability two crates away
                // arrives, which is why the sweep is over the source and not only the ledger.
                //
                // What is genuinely missing is the five of Table 164's twelve styles that state
                // no quantity a frame could be shaped from — how many lines a `Blinds` has, how
                // wide a `Glitter`'s band is, what a `Dissolve` does to a pixel, what a `Fly`'s
                // "changes" are — which `transition::note` reports by name rather than drawing
                // as a cut. `R` is the cut by definition and is not among them. Table 275 also
                // asks for transition actions, and §12.6.4.15's `Trans` is read and performed.
                "five of Table 164's twelve transition styles are reported by name rather than \
                 drawn, because the clause states no quantity to shape their frames from"
            }
            Self::DPartInteract => {
                // Table 275 asks for two things. §12.6.4.5's `GoToDp` navigates to a part
                // (`document_part::first_page`, §14.12.4.1); the hierarchy is not displayed.
                "no document part panel: a GoToDp action navigates §14.12's parts, and the DPart \
                 hierarchy is not displayed"
            }
            Self::DigSigValidation | Self::DigSig | Self::DigSigMdp => {
                // §12.8 is read since the ninety-eighth session — the signature dictionary,
                // Table 257's `/P` level, Table 258's usage rights — and what a validator
                // needs is a trust store, which is the decision `doc/todo/51` holds.
                "no signature validation or signing: §12.8 is read and reported, and verifying a \
                 signature needs a certificate store"
            }
            Self::Geospatial2D | Self::Geospatial3D => {
                // §12.10's dictionaries are all read (§12.10.2, §12.10.3); the projection needs
                // the EPSG registry and ISO 19162's WKT, both outside this standard.
                "no geospatial projection: §12.10's dictionaries are read, and turning a page \
                 point into a coordinate needs the EPSG registry"
            }
            Self::SeparationSimulation => {
                // §10.8.2 is implemented — the alternate space and its tint transform — and
                // §10.8.3's simulation is what is absent. The reason this arm used to give was
                // "§10.8 describes a marking device", the phrase ADR 0204 retired: ISO 32000-2
                // does not contain it, and §10.8.3's own condition is a user's request rather
                // than a device's nature.
                "no separation simulation: §10.8.3 is performed when a user asks for it and this \
                 program offers no such control"
            }
            // Excluded by CLAUDE.md principle 5's closed list.
            Self::EnableJavaScripts => "ECMAScript is excluded by this project's principle 5",
            Self::Multimedia | Self::RichMedia | Self::ThreeDMarkup | Self::U3d | Self::Prc => {
                "clause 13's multimedia and 3D are excluded by this project's principle 5"
            }
            Self::Other(_) => "a requirement type this reader does not recognise",
        })
    }
}

/// Reads §12.11's `/Requirements` array from the document catalog.
///
/// Empty for a document that states none, which is every one of the 974 corpus documents.
#[must_use]
pub fn read(document: &Document) -> Vec<Requirement> {
    let Ok(catalog) = document.catalog() else {
        return Vec::new();
    };
    let array = document.get_key(&catalog, "Requirements");
    let Some(items) = array.as_array() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in items.iter().take(MAX_REQUIREMENTS) {
        let resolved = document.resolve(item);
        let Some(dict) = resolved.as_dict() else {
            continue;
        };
        // "(Required) The type of requirement that this dictionary describes": a dictionary
        // with no `/S` has not stated a requirement at all.
        let Some(kind) = document
            .get_key(dict, "S")
            .as_name()
            .map(|name| Kind::read(name.as_bytes()))
        else {
            continue;
        };
        out.push(Requirement {
            kind,
            version: version(document, dict),
            penalty: penalty(document, dict),
        });
    }
    out
}

/// Every requirement this program cannot meet, with the reason, in the document's own order.
///
/// A penalty of 0 is still listed: §12.11.3 says it means "although the document uses this
/// feature the need is optional", which is a statement about how much it matters and not about
/// whether it was met. [`penalty_total`] is what weighs them.
#[must_use]
pub fn unmet(document: &Document) -> Vec<(Requirement, &'static str)> {
    read(document)
        .into_iter()
        .filter_map(|requirement| {
            requirement
                .kind
                .unmet()
                .map(|reason| (requirement.clone(), reason))
        })
        .collect()
}

/// The total §12.11.3 states a threshold on, over the requirements this program cannot meet.
///
/// # The computation, and the two clauses it is split across
///
/// The consequence is §12.11.6's, and it defines no arithmetic of its own:
///
/// > If requirements cannot be met, as determined by the computation of the penalty value as
/// > described in 12.11.3, "Requirement penalty values", then the processing of the document
/// > shall not continue.
///
/// The arithmetic it points at is one sentence, the last of §12.11.3:
///
/// > In the situation where the penalty values are being used to evaluate the presentation of
/// > the base PDF document, and there exist no other alternates, if the penalty value exceeds
/// > 100 then the PDF processor should not attempt to display or process the document.
///
/// # Why the sum, and why over the unmet ones only
///
/// Two sentences fix both halves, and neither is an inference this crate made up.
///
/// A *sum* rather than any single entry, because Table 273 bounds each entry at "between 0 and
/// 100 (inclusive)" — so a threshold on one entry could never fire and the sentence above would
/// be dead text — while the paragraph before it says the values contribute "to the total penalty
/// points". The threshold sentence is the case that paragraph leaves over: no alternates, so the
/// total has nothing to be weighed against except the number 100.
///
/// Over the *unmet* ones, because Table 273 says what a penalty is the penalty for: "the penalty
/// value to be applied when this requirement cannot be met by a PDF processor". A requirement
/// this program meets costs nothing, whatever the file priced it at.
///
/// # What is done with it
///
/// Nothing, here. It is reported beside the requirements it totals and the document is drawn —
/// see this module's own header for why that is a choice rather than a reading, and why the
/// choice belongs to a host rather than to `pdf-model`.
///
/// Zero for the 974 corpus documents, every one of which states no `/Requirements` array at all.
#[must_use]
pub fn penalty_total(document: &Document) -> u32 {
    // Each penalty is a `u8` and [`MAX_REQUIREMENTS`] bounds the count, so the sum is under
    // 26 000 and the widening is what makes the addition exact rather than saturating.
    unmet(document)
        .iter()
        .map(|(requirement, _)| u32::from(requirement.penalty))
        .sum()
}

/// The number §12.11.3's threshold is stated against: "if the penalty value exceeds 100".
pub const PENALTY_LIMIT: u32 = 100;

/// Table 273's `/V`, in the name form §12.11.4 defines.
fn version(document: &Document, dict: &Dictionary) -> Option<String> {
    match document.get_key(dict, "V") {
        Object::Name(name) => Some(String::from_utf8_lossy(name.as_bytes()).into_owned()),
        // "An extensions dictionary that specifies a vendor-specific extension to a PDF
        // version": a version this reader cannot compare against anything, since the extension
        // is the developer's. The prefix it names is the useful part.
        Object::Dictionary(inner) => inner
            .iter()
            .find(|(key, _)| key.as_bytes() != b"Type")
            .map(|(key, _)| String::from_utf8_lossy(key.as_bytes()).into_owned()),
        _ => None,
    }
}

/// Table 273's `/Penalty`, defaulting to 100 and clamped to the clause's range.
fn penalty(document: &Document, dict: &Dictionary) -> u8 {
    document
        .get_key(dict, "Penalty")
        .as_integer()
        .map_or(100, |value| {
            u8::try_from(value.clamp(0, 100)).unwrap_or(100)
        })
}

/// §7.12's extensions dictionary: the developer extensions a document was written against.
///
/// Keyed by the registered prefix name — `/ADBE` for Adobe's, `/ISO_` for ISO's — because that
/// is what §7.12.1 makes the key: "[t]he remaining keys shall be names consisting of registered
/// prefix names of the developer extensions used". A prefix may name several extensions, which
/// is PDF 2.0's array form and is why the value is a list.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Extensions {
    /// The developer extensions, by registered prefix.
    pub by_prefix: BTreeMap<String, Vec<DeveloperExtension>>,
}

/// One developer extensions dictionary. Table 49.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeveloperExtension {
    /// `/BaseVersion`: "[t]he name of the PDF version to which this extension applies".
    ///
    /// A name, and compared as one: §7.12.4's NOTE says "[t]he value of `BaseVersion` is not to
    /// be interpreted as a real number but as two integers with a PERIOD (2Eh) between them".
    pub base_version: String,
    /// `/ExtensionLevel`: "[a]n integer defined by the developer to denote the extension being
    /// used", which §7.12.5 says increases over time within one base version.
    pub level: i64,
    /// `/URL`: where the developer documents the extension.
    pub url: Option<String>,
    /// `/ExtensionRevision`: PDF 2.0's optional further revision information.
    pub revision: Option<String>,
}

impl Extensions {
    /// Reads the catalog's `/Extensions`.
    ///
    /// Every part of this is a direct object by §7.12.1 — "this information shall be nested
    /// directly within the catalog dictionary with no indirect objects used" — and each is
    /// resolved anyway, because a file that writes an indirect reference has broken a rule that
    /// costs a reader nothing to survive.
    ///
    /// 9 of the 974 corpus documents state one, all of them `/ADBE`.
    #[must_use]
    pub fn read(document: &Document) -> Self {
        let Ok(catalog) = document.catalog() else {
            return Self::default();
        };
        let extensions = document.get_key(&catalog, "Extensions");
        let Some(dict) = extensions.as_dict() else {
            return Self::default();
        };
        let mut by_prefix: BTreeMap<String, Vec<DeveloperExtension>> = BTreeMap::new();
        for (key, value) in dict.iter() {
            if key.as_bytes() == b"Type" {
                continue;
            }
            let prefix = String::from_utf8_lossy(key.as_bytes()).into_owned();
            let mut read_one = |object: &Object| {
                if let Some(inner) = document.resolve(object).as_dict()
                    && let Some(extension) = DeveloperExtension::read(document, inner)
                {
                    by_prefix.entry(prefix.clone()).or_default().push(extension);
                }
            };
            match document.resolve(value) {
                // PDF 2.0's array form: several extensions under one prefix.
                Object::Array(items) => {
                    for item in items.iter().take(MAX_REQUIREMENTS) {
                        read_one(item);
                    }
                }
                other => read_one(&other),
            }
        }
        Self { by_prefix }
    }

    /// Whether the document states any extension at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_prefix.is_empty()
    }
}

impl DeveloperExtension {
    /// Reads Table 49, or `None` where either required entry is missing.
    ///
    /// Both `/BaseVersion` and `/ExtensionLevel` are required, and together they are the whole
    /// statement: an extension dictionary without them names no extension, and a reader that
    /// invented a level would be claiming to know which of a developer's extensions this is.
    ///
    /// **`/URL` is required too since Errata Collection 3** — Issue #732, `/State` `Review`
    /// `Accepted`, which replaces Table 49's "Optional; PDF 2.0; shall be a direct object if
    /// present" with "Required" and adds that the URL "should be unique for each extension".
    /// `doc/md/` still shows it optional (ADR 0252, ADR 0253). It is **deliberately not** made a
    /// third condition here, and the asymmetry with the other two is the argument: the first two
    /// are the extension's identity, so a dictionary missing one has said nothing this reader can
    /// report; the URL is where a human goes to read about it, so a dictionary missing it has
    /// still said which extension is in the file. Refusing it would lose that for a `shall` whose
    /// whole force is on the producer. The fixture at the foot of this module writes an extension
    /// with no `/URL` for exactly this case, and it is now a malformed file rather than a sparse
    /// one.
    fn read(document: &Document, dict: &Dictionary) -> Option<Self> {
        let base = document.get_key(dict, "BaseVersion");
        let base_version = String::from_utf8_lossy(base.as_name()?.as_bytes()).into_owned();
        let level = document.get_key(dict, "ExtensionLevel").as_integer()?;
        Some(Self {
            base_version,
            level,
            url: match document.get_key(dict, "URL") {
                Object::String(bytes) => Some(String::from_utf8_lossy(&bytes).into_owned()),
                _ => None,
            },
            revision: match document.get_key(dict, "ExtensionRevision") {
                Object::String(bytes) => Some(pdf_syntax::text_string(&bytes)),
                _ => None,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{Extensions, Kind, penalty_total, read, unmet};
    use pdf_syntax::Document;

    fn document(catalog: &str) -> Document {
        use std::fmt::Write as _;
        let mut out = String::from("%PDF-2.0\n");
        let mut offsets = Vec::new();
        for (index, body) in [catalog, "<< /Type /Pages /Count 0 /Kids [] >>"]
            .iter()
            .enumerate()
        {
            offsets.push(out.len());
            let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
        }
        let xref_at = out.len();
        let _ = write!(out, "xref\n0 3\n0000000000 65535 f \n");
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
        );
        Document::open(out.into_bytes()).expect("a valid file")
    }

    /// Table 273's entries, with the defaults the clause states.
    #[test]
    fn a_requirement_states_a_type_a_version_and_a_penalty() {
        let doc = document(
            "<< /Type /Catalog /Pages 2 0 R /Requirements [ \
             << /Type /Requirement /S /EnableJavaScripts /Penalty 100 >> \
             << /S /U3D /V /1.0.0 >> \
             << /S /Navigation /Penalty 0 >> \
             << /S /Reticulation >> \
             << /Type /Requirement >> ] >>",
        );
        let requirements = read(&doc);
        assert_eq!(
            requirements.len(),
            4,
            "the dictionary with no /S states no requirement: {requirements:?}"
        );
        assert_eq!(
            requirements.first().map(|r| r.kind.clone()),
            Some(Kind::EnableJavaScripts)
        );
        assert_eq!(
            requirements.get(1).and_then(|r| r.version.clone()),
            Some("1.0.0".to_owned()),
            "kept as the name the file wrote, per §12.11.4's NOTE"
        );
        assert_eq!(
            requirements.get(1).map(|r| r.penalty),
            Some(100),
            "Table 273's default"
        );
        assert_eq!(requirements.get(2).map(|r| r.penalty), Some(0));
        assert_eq!(
            requirements.get(3).map(|r| r.kind.clone()),
            Some(Kind::Other("Reticulation".to_owned())),
            "a type outside Table 275 is kept by name, because the table is open"
        );
    }

    /// What this program cannot meet is named, and what it can is not.
    ///
    /// `Navigation` and `AcroFormInteract` are met — links, outlines and the three actions the
    /// clause names, and a field a person can type into and save — and the other two are not,
    /// for two different reasons the strings say. **The second of those moved in the
    /// two-hundred-and-twenty-first session**: the arm had said "nothing edits the value" since
    /// before `ViewState::set_field` existed, and this test had asserted the stale answer.
    #[test]
    fn the_requirements_this_program_cannot_meet_are_named() {
        let doc = document(
            "<< /Type /Catalog /Pages 2 0 R /Requirements [ \
             << /S /Navigation >> << /S /EnableJavaScripts >> << /S /AcroFormInteract >> \
             << /S /Markup >> ] >>",
        );
        let unmet = unmet(&doc);
        let kinds: Vec<&str> = unmet.iter().map(|(r, _)| r.kind.as_str()).collect();
        assert_eq!(kinds, vec!["EnableJavaScripts", "Markup"]);
        assert!(
            unmet
                .first()
                .is_some_and(|(_, reason)| reason.contains("principle 5")),
            "an exclusion says so: {unmet:?}"
        );
    }

    /// §12.11.3's threshold, over the total §12.11.6 asks a processor to compute.
    ///
    /// The fixture prices four requirements and this program meets two of them, so the two that
    /// decide the total are `EnableJavaScripts` at 60 and `Markup` at 55. Three things are
    /// pinned at once and each is a sentence of the clause rather than a convenience:
    ///
    /// - the total is a **sum**, so 115 rather than 60 — Table 273 bounds one entry at 100 and
    ///   §12.11.3 speaks of "the total penalty points";
    /// - it is over the **unmet** ones, so `Navigation`'s 100 and `Collection`'s 100 are not in
    ///   it even though the file priced them highest — Table 273 makes a penalty "the penalty
    ///   value to be applied when this requirement cannot be met";
    /// - and 115 **exceeds** [`PENALTY_LIMIT`], which is the one condition §12.11.3 states.
    ///
    /// The second document is the same file with the two unpriced, which takes Table 273's
    /// default of 100 each and lands on 200 — the case a producer creates by saying nothing.
    #[test]
    fn the_penalty_total_sums_the_requirements_this_program_cannot_meet() {
        let doc = document(
            "<< /Type /Catalog /Pages 2 0 R /Requirements [ \
             << /S /Navigation /Penalty 100 >> << /S /EnableJavaScripts /Penalty 60 >> \
             << /S /Collection /Penalty 100 >> << /S /Markup /Penalty 55 >> ] >>",
        );
        assert_eq!(penalty_total(&doc), 115);
        assert!(penalty_total(&doc) > super::PENALTY_LIMIT);

        let defaulted = document(
            "<< /Type /Catalog /Pages 2 0 R /Requirements [ \
             << /S /Navigation /Penalty 100 >> << /S /EnableJavaScripts >> \
             << /S /Collection /Penalty 100 >> << /S /Markup >> ] >>",
        );
        assert_eq!(
            penalty_total(&defaulted),
            200,
            "Table 273's default is 100, so an unpriced requirement is priced at the maximum"
        );

        // The other side of the threshold, so that "exceeds" is not asserted by a test that
        // could not have seen a document under it: one unmet requirement at 100 is *not* over.
        let met = document(
            "<< /Type /Catalog /Pages 2 0 R /Requirements [ \
             << /S /Navigation /Penalty 100 >> << /S /Markup /Penalty 100 >> ] >>",
        );
        assert_eq!(penalty_total(&met), 100);
        assert!(penalty_total(&met) <= super::PENALTY_LIMIT);

        // And a document that states no requirements at all owes nothing, which is every one of
        // the 974.
        let none = document("<< /Type /Catalog /Pages 2 0 R >>");
        assert_eq!(penalty_total(&none), 0);
    }

    /// A collection this program displays and can extract from is a requirement it meets.
    ///
    /// Table 275's `Collection` states two conditions and both are answered two crates away:
    /// "displaying the embedded files referenced from the document's collection dictionary
    /// (12.3.5, "Collections") along with any associated metadata" is `Query::Collection` drawn
    /// by `viewer_ui::chrome` (ADR 0202), and "that the user can extract or otherwise view the
    /// contents of each item in the collection" is `Command::Extract`, whose key is the
    /// `/EmbeddedFiles` name a collection's items are filed under.
    ///
    /// `CollectionEditing` is *not* met, and the pair is what this test is for: Table 275 words
    /// the second as "[i]n addition to the requirements of the Collection value", so the two
    /// answers must differ and the second's reason must name the increment rather than repeat
    /// the first. It said "no collection view" until the three-hundred-and-seventy-fifth session,
    /// forty-odd after the view arrived.
    #[test]
    fn a_collection_is_met_and_editing_one_is_not() {
        let doc = document(
            "<< /Type /Catalog /Pages 2 0 R /Requirements [ \
             << /S /Collection >> << /S /CollectionEditing >> ] >>",
        );
        let unmet = unmet(&doc);
        let kinds: Vec<&str> = unmet.iter().map(|(r, _)| r.kind.as_str()).collect();
        assert_eq!(kinds, vec!["CollectionEditing"]);
        assert!(
            unmet
                .first()
                .is_some_and(|(_, reason)| reason.contains("never added to or changed")),
            "the reason names the increment the type asks for: {unmet:?}"
        );
    }

    /// No reason says a clause is unread, because that is the claim that decays.
    ///
    /// Nine of these strings were wrong in the three-hundred-and-seventy-fifth session and every
    /// one was of this shape: `DigSig` said "§12.8 is unread" with §12.8 read since the
    /// ninety-eighth, `Geospatial2D` said "§12.10 is unread" with every dictionary of it read,
    /// `DPartInteract` said "§14.12 is unread" with `GoToDp` navigating parts. A reason that
    /// names a *clause* as absent is a claim about the tree that no compiler and no gate watches;
    /// a reason that names the missing *capability* stays true until that capability arrives.
    /// This is the gate `doc/todo/01`'s fifth sweep had to be run by hand to be.
    #[test]
    fn no_reason_claims_a_clause_is_unread() {
        for kind in [
            Kind::Markup,
            Kind::AttachmentEditing,
            Kind::Collection,
            Kind::CollectionEditing,
            Kind::Action,
            Kind::Transitions,
            Kind::DPartInteract,
            Kind::DigSigValidation,
            Kind::DigSig,
            Kind::DigSigMdp,
            Kind::Geospatial2D,
            Kind::Geospatial3D,
            Kind::SeparationSimulation,
            Kind::EnableJavaScripts,
            Kind::Multimedia,
            Kind::RichMedia,
            Kind::ThreeDMarkup,
            Kind::U3d,
            Kind::Prc,
            Kind::Other("Reticulation".to_owned()),
        ] {
            let Some(reason) = kind.unmet() else { continue };
            assert!(
                !reason.contains("is unread") && !reason.contains("are unread"),
                "{}: a reason names what is missing, not a clause as unread — {reason}",
                kind.as_str()
            );
        }
    }

    /// §7.12's own EXAMPLE 3: both the dictionary form and PDF 2.0's array form.
    ///
    /// > /Extensions << /Type /Extensions /ISO_ [ … ] /GLGR << … >> >>
    ///
    /// Two prefixes, one of which states two extensions, and `/Type` is not a prefix.
    #[test]
    fn extensions_are_read_under_their_registered_prefixes() {
        let doc = document(
            "<< /Type /Catalog /Pages 2 0 R /Extensions << /Type /Extensions \
             /ISO_ [ << /Type /DeveloperExtensions /BaseVersion /2.0 /ExtensionLevel 24064 \
             /URL (https://www.iso.org/standard/77686.html) >> \
             << /Type /DeveloperExtensions /BaseVersion /2.0 /ExtensionLevel 24654 >> ] \
             /GLGR << /BaseVersion /1.7 /ExtensionLevel 1002 >> >> >>",
        );
        let extensions = Extensions::read(&doc);
        assert_eq!(extensions.by_prefix.len(), 2, "{extensions:?}");
        let iso = extensions.by_prefix.get("ISO_").expect("the ISO_ prefix");
        assert_eq!(iso.len(), 2);
        assert_eq!(iso.first().map(|e| e.level), Some(24064));
        assert_eq!(
            iso.first().and_then(|e| e.url.clone()),
            Some("https://www.iso.org/standard/77686.html".to_owned())
        );
        assert_eq!(
            iso.get(1).map(|e| e.base_version.clone()),
            Some("2.0".to_owned())
        );
        assert_eq!(
            extensions
                .by_prefix
                .get("GLGR")
                .and_then(|entries| entries.first())
                .map(|e| e.level),
            Some(1002)
        );
    }

    /// An extension missing either required entry names no extension.
    #[test]
    fn an_extension_without_a_level_is_not_read() {
        let doc = document(
            "<< /Type /Catalog /Pages 2 0 R /Extensions << /ADBE << /BaseVersion /1.7 >> >> >>",
        );
        assert!(Extensions::read(&doc).is_empty());
    }
}
