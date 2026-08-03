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
//! shall not continue", against a penalty computed as §12.11.3 describes. **§12.11.3 states no
//! threshold** — it says 100 "indicates that this document will not produce the author's
//! intent" and that intermediate values "are available to weight the value of this feature
//! among other features" — so the computation is a comparison the clause never completes. This
//! program therefore draws the document and says what it could not promise, which is a
//! documented choice and not a reading: refusing to open a file a person asked for is a worse
//! failure than showing it with its limits named.
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
    /// `OCInteract`.
    ///
    /// `None` means met. The three shapes of "not met" are all here on purpose — a feature this
    /// project *excludes* (`CLAUDE.md` principle 5), a feature that is a viewer's rather than a
    /// renderer's and is not built, and a feature nobody has written yet.
    #[must_use]
    pub fn unmet(&self) -> Option<&'static str> {
        Some(match self {
            // Met: the pieces each of these three clauses names are all here — §8.11.4.4's
            // usage application dictionaries for the `View` event (ADR 0044), §12.5.6.5's
            // links with §12.3.3's outline and the three actions §12.11's own table lists
            // (ADR 0070), and §7.6's encryption at every revision and method (ADR 0031).
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
            Self::OcAutoStates
            | Self::Navigation
            | Self::Encryption
            | Self::OcInteract
            | Self::AcroFormInteract
            | Self::Attachment => return None,
            Self::Markup => "no annotation editing: markup annotations are drawn, not created",
            Self::AttachmentEditing => "no attachment editing, and no attachment list",
            Self::Collection => "no collection view: §12.3.5's collections are unread",
            Self::CollectionEditing => "no collection editing, and no collection view",
            Self::Action => "some §12.6 actions are performed and most are refused by name",
            Self::Transitions => {
                "no presentation mode: §12.4.4 is read as data and nothing plays it"
            }
            Self::DPartInteract => "no document part navigation: §14.12 is unread",
            Self::DigSigValidation | Self::DigSig | Self::DigSigMdp => {
                "no signature handling: §12.8 is unread"
            }
            Self::Geospatial2D | Self::Geospatial3D => "no geospatial support: §12.10 is unread",
            Self::SeparationSimulation => {
                "no separation simulation: §10.8 describes a marking device and this is a screen"
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
/// whether it was met. Weighing them is the caller's, because the clause states no threshold.
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
    use super::{Extensions, Kind, read, unmet};
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
