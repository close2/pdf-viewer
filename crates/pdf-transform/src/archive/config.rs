//! A remedy configuration: a refusal answered in advance, read by the caller and handed in as data.
//!
//! # What this is, and the seam it keeps
//!
//! `doc/rfc/0007` is the design. Its sentence is the whole of the motivation — *a refusal would
//! mean that a human has to intervene* — and the reframe is that a refusal is a question the
//! converter asks, so the answer can be given in advance in a file. This module is the file's
//! reader and nothing more: it turns TOML ([`super::toml`]) into a [`Configuration`], a data value
//! the caller builds and hands to [`ArchivePlan`]. **Nothing here opens a path, reads a clock or
//! spawns a process** — RFC 0002 section 5's rule that `apply` is a pure function of its inputs is
//! why the configuration is *read by the caller*, exactly as `Policy` and `Budget` are.
//!
//! [`ArchivePlan`]: super::ArchivePlan
//!
//! # What it does today, and what it is shaped for
//!
//! The owner's answers `A54`–`A60` (2026-09-12) draw the line this round builds to. Two remedies
//! reach a real conversion:
//!
//! - **`discard`**, where the site is one of the seven `doc/pdf-a-conversion-limits.md` section 3
//!   losses — it becomes the [`Authorisations`] entry a caller would otherwise type on the command
//!   line, so a configuration authorising `image-smoothing` and `--authorise image-smoothing`
//!   convert the same document identically.
//! - **A departure** (section 4.7), the first of which accepts XML and only XML attachments when
//!   the target is `PDF/A-2` — `A60`'s case, `ZUGFeRD` and `Factur-X`'s, and the only route an operator
//!   has now that part 3 is not a target.
//!
//! Every other remedy the catalogue names — `preserve`, `derive`, `supply`, and `discard` on a
//! site whose lossless rewrite is unbuilt — is **recognised, validated and enumerated but not yet
//! applied**: naming one leaves that site's requirement refused with the sentence it already
//! carries, so the configuration promises nothing it cannot keep (`CLAUDE.md` principle 1, and the
//! round's own rule — *a promise nothing will keep is worse than a refusal with a sentence*). The
//! next converter round builds `derive` and the external-tool executor (`A54`, `A56`); appending
//! pages waits on session 994's authoring amendment (`A58`); `supply` waits on its own machinery
//! (catalogue section 0.2). The format is built so each slots in without a reader change.
//!
//! Installing a configuration therefore changes no pipeline unless it authorises a built loss or
//! names a departure — which is `doc/adr/0954`'s requirement that `stop` stay every site's default.

use std::collections::{BTreeMap, BTreeSet};

use pdf_archive::{Target, table};
use pdf_syntax::Document;
use pdf_syntax::object::ObjectId;

use super::decision::{Answer, Loss, REMEDIES};
use super::toml::{self, Value};

/// `doc/rfc/0007` section 2's remedy vocabulary, closed and small so a configuration is legible.
///
/// The order is what happens to the document's content, which is the sort the RFC's section 2
/// argues is load-bearing: nothing, a loss, a move, a new representation — and section 0.2 of
/// `doc/pdf-a-mitigations.md` adds the fifth, where the operator rather than the document is the
/// source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The conversion refuses, as today. Every site's default.
    Stop,
    /// The information is lost, with the operator's authorisation — section 3's authorised loss.
    Discard,
    /// The information survives, somewhere the target admits.
    Preserve,
    /// A new representation is made from content the document already has.
    Derive,
    /// The operator states a fact the document does not — section 0.2's fifth kind.
    Supply,
}

impl Kind {
    /// The word a configuration names this kind by.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Stop => "stop",
            Self::Discard => "discard",
            Self::Preserve => "preserve",
            Self::Derive => "derive",
            Self::Supply => "supply",
        }
    }

    /// The kind a configuration's word names.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        [
            Self::Stop,
            Self::Discard,
            Self::Preserve,
            Self::Derive,
            Self::Supply,
        ]
        .into_iter()
        .find(|kind| kind.word() == word)
    }

    /// Whether this round can carry out a remedy of this kind at a site whose loss is built.
    ///
    /// Only `stop` (a no-op) and `discard` (an authorised loss) reach a real conversion today; the
    /// module comment says why the other three are recognised but not applied.
    #[must_use]
    pub const fn is_built(self) -> bool {
        matches!(self, Self::Stop | Self::Discard)
    }
}

/// One thing a configuration got wrong, named so a person can fix it.
///
/// **An error naming both**, which is `doc/rfc/0007` section 4.6's rule and `doc/adr/0954`'s: a
/// configuration that silently did less than it said is the failure mode the whole feature exists
/// to remove, so an unknown site, an unknown remedy, or a remedy a target cannot admit is stated
/// rather than ignored.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfigError {
    /// The file is not the subset of TOML a configuration is written in.
    #[error("the configuration is not readable: {0}")]
    Toml(String),
    /// A `[site."…"]` names something no requirement of ISO 19005 is called.
    #[error(
        "line {line}: the site {site:?} is no requirement this converter knows; \
         `--remedy-sites --to <target>` lists every one"
    )]
    UnknownSite {
        /// The 1-based line.
        line: usize,
        /// The site the configuration named.
        site: String,
    },
    /// A `remedy` names a word outside the closed vocabulary.
    #[error(
        "line {line}: the site {site:?} states the remedy {remedy:?}, which is none of \
         stop, discard, preserve, derive or supply"
    )]
    UnknownRemedy {
        /// The 1-based line.
        line: usize,
        /// The site.
        site: String,
        /// The word it named.
        remedy: String,
    },
    /// A key held the wrong kind of value.
    #[error("line {line}: {key} in {site:?} wants {wanted}, and this is {found}")]
    WrongValue {
        /// The 1-based line.
        line: usize,
        /// The table the key is in.
        site: String,
        /// The key.
        key: String,
        /// What the key wanted.
        wanted: &'static str,
        /// What was there.
        found: &'static str,
    },
    /// `on-failure` was a list, which `A57` rules out.
    #[error(
        "line {line}: on-failure in {site:?} takes one remedy and not a chain — the owner's \
         answer A57 — so it is a string, not a list"
    )]
    FallbackChain {
        /// The 1-based line.
        line: usize,
        /// The site.
        site: String,
    },
    /// A target qualifier names something that is not a target.
    #[error(
        "line {line}: {qualifier:?} in {site:?} is no target; the targets are 2b, 2u, 2a, 4, 4f, 4e"
    )]
    UnknownTargetQualifier {
        /// The 1-based line.
        line: usize,
        /// The site.
        site: String,
        /// The qualifier.
        qualifier: String,
    },
    /// A `[depart."…"]` names a requirement this round's departure cannot carry.
    #[error(
        "line {line}: a departure from {requirement:?} is not built — the only departure this \
         version carries is from embedded-files/embedded-file-is-itself-pdfa (and its plain-profile \
         sibling), which is A60's XML-attachment case"
    )]
    UnsupportedDeparture {
        /// The 1-based line.
        line: usize,
        /// The requirement named.
        requirement: String,
    },
    /// A departure states no `reason`, which section 4.7.2 requires.
    #[error(
        "line {line}: the departure from {requirement:?} states no reason, and one is required — \
         a departure nobody wrote a reason for is one nobody will be able to explain in two years"
    )]
    DepartureWithoutReason {
        /// The 1-based line.
        line: usize,
        /// The requirement.
        requirement: String,
    },
}

/// A departure from one requirement, narrowed by a declarative predicate.
///
/// `doc/rfc/0007` section 4.7. **Not a remedy**: every remedy produces a file that conforms, and a
/// departure produces one that does not — so it is named per requirement, carries a narrowing
/// predicate (the media-type form is enough to start), states a `reason`, and by default leaves the
/// PDF/A identification off the output so the file does not claim what it has not earned (`A59`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Departure {
    /// The requirement identifier departed from.
    pub requirement: String,
    /// The media types the departure admits — section 4.7.2's `(and only xml)`. Empty admits any.
    pub media_types: Vec<String>,
    /// The `/AFRelationship` values the departure admits, narrowing further. Empty admits any.
    pub relationships: Vec<String>,
    /// Why, copied verbatim into the report and the file's `xmpMM:History`.
    pub reason: String,
}

/// Whether a departure's predicate covers a document, and why not where it does not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Coverage {
    /// Every embedded file the requirement is about matches the predicate; the departure applies.
    Covers,
    /// An embedded file the predicate does not admit, so the requirement stays refused.
    Leaves {
        /// The file's own name.
        file: String,
        /// The media type it stated, or `None` where it stated none.
        media_type: Option<String>,
    },
}

impl Departure {
    /// The two requirement identifiers a built departure may name.
    ///
    /// `embedded-files/embedded-file-is-itself-pdfa` binds PDF/A-2's three levels;
    /// `…-in-the-plain-profile` binds PDF/A-4 plain. Both are the embedding rule the owner's XML
    /// case departs from; the 4f and 4e flavours lift the rule outright and need no departure.
    const SUPPORTED: [&'static str; 2] = [
        "embedded-files/embedded-file-is-itself-pdfa",
        "embedded-files/embedded-file-is-itself-pdfa-in-the-plain-profile",
    ];

    /// Whether this round can carry this departure out.
    #[must_use]
    pub fn is_built(&self) -> bool {
        Self::SUPPORTED.contains(&self.requirement.as_str())
    }

    /// Whether the departure's predicate covers this document.
    ///
    /// The predicate is `(and only xml)`, so the question is asked of **every** embedded file the
    /// requirement is about — the same population ISO 19005-2 section 6.8 binds: every file
    /// specification carrying an `/EF`. The departure covers the document only where every one of
    /// them matches, which is the difference between *accept XML attachments* and *accept XML and
    /// only XML attachments* — the second is what makes a departure narrower than any target.
    #[must_use]
    pub fn covers(&self, document: &Document) -> Coverage {
        for (name, media_type, relationship) in embedded_files(document) {
            let type_ok = self.media_types.is_empty()
                || media_type
                    .as_deref()
                    .is_some_and(|stated| self.media_types.iter().any(|want| want == stated));
            let relationship_ok =
                self.relationships.is_empty() || self.relationships.contains(&relationship);
            if !type_ok || !relationship_ok {
                return Coverage::Leaves {
                    file: name,
                    media_type,
                };
            }
        }
        Coverage::Covers
    }
}

/// A site the configuration named whose remedy this round does not carry out.
///
/// Reported so the operator sees their intent was read, not ignored — the requirement itself is
/// refused with the sentence it already carries, which names what the remedy waits on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unbuilt {
    /// The site.
    pub site: String,
    /// The remedy the configuration named.
    pub remedy: Kind,
}

/// One remedy configuration, read from a file and validated against a target.
///
/// The two things a conversion consumes are [`Self::authorisations`] and [`Self::departures`]; the
/// rest is what the report tells the operator about answers this round could not yet act on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Configuration {
    /// The profile's own name, where the file is a shipped profile (`[profile] name`).
    pub name: Option<String>,
    /// Every site the file named, in file order, with the remedy it chose.
    sites: Vec<(String, Kind)>,
    /// The departures the file states.
    pub departures: Vec<Departure>,
}

impl Configuration {
    /// Reads and validates a configuration against the target it will be used for.
    ///
    /// # Errors
    ///
    /// [`ConfigError`], naming the line and what was wrong. A site the target does not bind is not
    /// an error — the refusal never arises, so the answer is never consulted (a profile works with
    /// all six targets for exactly this reason) — but a site no requirement is called, a remedy
    /// outside the vocabulary, and an `on-failure` chain are all stated rather than ignored.
    pub fn read(text: &str, target: Target) -> Result<Self, ConfigError> {
        let tables = toml::parse(text).map_err(|error| ConfigError::Toml(error.to_string()))?;
        let requirements: BTreeSet<&'static str> = table::requirements()
            .map(|requirement| requirement.id)
            .collect();
        let mut name = None;
        let mut sites: Vec<(String, Kind)> = Vec::new();
        let mut seen_sites: BTreeSet<String> = BTreeSet::new();
        let mut departures = Vec::new();
        for tbl in &tables {
            match tbl.path.first().map(String::as_str) {
                Some("profile") => {
                    name = tbl.get("name").and_then(Value::as_text).map(str::to_owned);
                    // `doc/pdf-a-mitigations.md` section 14's ninth finding: a `default` may only be
                    // `stop` — today's behaviour, stated deliberately so `refuse-any-loss` is not
                    // the empty file. A default of anything else would authorise a loss for every
                    // site at once, which is exactly what a per-site vocabulary exists to prevent.
                    if let Some(default) = tbl.get("default")
                        && default.as_text() != Some("stop")
                    {
                        return Err(ConfigError::WrongValue {
                            line: tbl.line,
                            site: "profile".to_owned(),
                            key: "default".to_owned(),
                            wanted: "the string \"stop\" (the only default a profile may state)",
                            found: default.kind(),
                        });
                    }
                }
                Some("site") => {
                    let site = site_identifier(tbl, &requirements)?;
                    let remedy = site_remedy(tbl, &site)?;
                    check_fallback(tbl, &site)?;
                    check_target_qualifier(tbl, &site)?;
                    check_key_shapes(tbl, &site)?;
                    // The target qualifier decides *whether this row applies* to the conversion:
                    // an unqualified row is the default, a qualified one wins for its target. A
                    // row for another target is read (and validated) but does not answer here.
                    if applies_to(tbl, target) && seen_sites.insert(site.clone()) {
                        sites.push((site, remedy));
                    }
                }
                Some("depart") => {
                    if let Some(departure) = departure(tbl, target)? {
                        departures.push(departure);
                    }
                }
                // `[tool.…]` is read by the next converter round's executor (`A54`); this round
                // parses past it so a configuration shipped with a tool declaration still loads.
                _ => {}
            }
        }
        Ok(Self {
            name,
            sites,
            departures,
        })
    }

    /// The command-line authorisations the built `discard` remedies stand for.
    ///
    /// A `discard` at a site that is one of the seven section 3 losses is exactly
    /// `--authorise <that loss>`, so a configuration and the flags reach the same conversion. A
    /// `discard` at any other site authorises nothing: its lossless-or-loss rewrite is unbuilt, so
    /// the requirement stays refused with the sentence it carries, and [`Self::unbuilt`] names it.
    #[must_use]
    pub fn authorisations(&self, target: Target) -> Authorisations {
        let losses = loss_sites();
        let mut authorised = Authorisations::default();
        for (site, remedy) in &self.sites {
            if *remedy == Kind::Discard
                && let Some(loss) = losses.get(site.as_str())
                && requirement_binds(site, target)
            {
                authorised.authorise(*loss);
            }
        }
        authorised
    }

    /// The sites whose remedy this round recognised but cannot carry out.
    #[must_use]
    pub fn unbuilt(&self, target: Target) -> Vec<Unbuilt> {
        let losses = loss_sites();
        let mut out = Vec::new();
        for (site, remedy) in &self.sites {
            if *remedy == Kind::Stop || !requirement_binds(site, target) {
                continue;
            }
            let built = *remedy == Kind::Discard && losses.contains_key(site.as_str());
            if !built {
                out.push(Unbuilt {
                    site: site.clone(),
                    remedy: *remedy,
                });
            }
        }
        out
    }

    /// The departures that bind the target and this round can carry out.
    #[must_use]
    pub fn built_departures(&self, target: Target) -> Vec<Departure> {
        self.departures
            .iter()
            .filter(|departure| {
                departure.is_built() && requirement_binds(&departure.requirement, target)
            })
            .cloned()
            .collect()
    }
}

/// Reads the `[site."<id>"]` identifier, or the error that names the unknown one.
fn site_identifier(
    tbl: &toml::Table,
    requirements: &BTreeSet<&'static str>,
) -> Result<String, ConfigError> {
    let site = tbl.path.get(1).cloned().unwrap_or_default();
    if !requirements.contains(site.as_str()) {
        return Err(ConfigError::UnknownSite {
            line: tbl.line,
            site,
        });
    }
    Ok(site)
}

/// Reads a site's `remedy`, or the error naming the unknown word.
fn site_remedy(tbl: &toml::Table, site: &str) -> Result<Kind, ConfigError> {
    let Some(value) = tbl.get("remedy") else {
        // A site table stating no remedy at all defaults to `stop`, like an absent site.
        return Ok(Kind::Stop);
    };
    let Some(word) = value.as_text() else {
        return Err(ConfigError::WrongValue {
            line: tbl.line,
            site: site.to_owned(),
            key: "remedy".to_owned(),
            wanted: "a string",
            found: value.kind(),
        });
    };
    Kind::parse(word).ok_or_else(|| ConfigError::UnknownRemedy {
        line: tbl.line,
        site: site.to_owned(),
        remedy: word.to_owned(),
    })
}

/// `A57`: `on-failure` picks one alternative, not a chain.
fn check_fallback(tbl: &toml::Table, site: &str) -> Result<(), ConfigError> {
    match tbl.get("on-failure") {
        None | Some(Value::Text(_)) => Ok(()),
        Some(Value::List(_)) => Err(ConfigError::FallbackChain {
            line: tbl.line,
            site: site.to_owned(),
        }),
        Some(other) => Err(ConfigError::WrongValue {
            line: tbl.line,
            site: site.to_owned(),
            key: "on-failure".to_owned(),
            wanted: "a remedy word",
            found: other.kind(),
        }),
    }
}

/// Well-known site keys whose value is a boolean, checked so a typo is caught rather than ignored.
///
/// A site documents its own keys (`doc/rfc/0007` section 3), and the reader does not know most of
/// them — but the ones the shipped profiles use most are flags, and `fresh-packet = "yes"` is a
/// mistake worth naming rather than passing over. The keys a remedy this round cannot yet carry
/// out are still validated for shape, so a configuration written against the format is well formed
/// before the remedy that reads it exists.
const BOOLEAN_KEYS: &[&str] = &[
    "keep-attachment",
    "construct",
    "fresh-packet",
    "attach-source",
];

/// Well-known site keys whose value is an inline table (the `supply` kind's, section 0.2).
const MAP_KEYS: &[&str] = &["media-types", "role-map", "colourants", "schemas"];

/// Checks the well-known keys hold the shape they are documented to.
fn check_key_shapes(tbl: &toml::Table, site: &str) -> Result<(), ConfigError> {
    for entry in &tbl.entries {
        let wanted = if BOOLEAN_KEYS.contains(&entry.key.as_str()) {
            Some(("a boolean", entry.value.as_bool().is_some()))
        } else if MAP_KEYS.contains(&entry.key.as_str()) {
            Some(("an inline table", entry.value.as_map().is_some()))
        } else {
            None
        };
        if let Some((wanted, ok)) = wanted
            && !ok
        {
            return Err(ConfigError::WrongValue {
                line: entry.line,
                site: site.to_owned(),
                key: entry.key.clone(),
                wanted,
                found: entry.value.kind(),
            });
        }
    }
    Ok(())
}

/// What narrows a site row beyond its identifier.
///
/// Two qualifiers, both `doc/rfc/0007` and the mitigations catalogue argue the site key needs:
/// `doc/rfc/0007` section 4.6's **target** (a remedy differs in kind by target) and section 14's
/// first finding's **shape** (one requirement splits into two shapes with different answers — a
/// blend mode written as an array against a bare name, an inline image's `LZWDecode` against its
/// `Crypt` filter). The target qualifier is applied today; the shape qualifier is parsed and
/// validated so a configuration can be written against it, and a shape-qualified row is inert
/// until the shape-aware remedies exist, exactly as a row for another target is.
enum Qualifier<'a> {
    /// `[site."x"]` — applies to every target.
    None,
    /// `[site."x".target."<t>"]`.
    Target(&'a str),
    /// `[site."x".shape."<s>"]`. The shape string is parsed and discarded until a shape-aware
    /// remedy reads it; the row is inert meanwhile.
    Shape,
}

/// The qualifier a site header carries, or the error naming a malformed one.
fn qualifier<'a>(tbl: &'a toml::Table, site: &str) -> Result<Qualifier<'a>, ConfigError> {
    match tbl.path.as_slice() {
        [_, _] => Ok(Qualifier::None),
        [_, _, marker, value] if marker == "target" => Ok(Qualifier::Target(value.as_str())),
        [_, _, marker, _] if marker == "shape" => Ok(Qualifier::Shape),
        _ => Err(ConfigError::UnknownTargetQualifier {
            line: tbl.line,
            site: site.to_owned(),
            qualifier: tbl.path.get(2).cloned().unwrap_or_default(),
        }),
    }
}

/// Validates a site header's qualifier: a target one must name a target.
fn check_target_qualifier(tbl: &toml::Table, site: &str) -> Result<(), ConfigError> {
    if let Qualifier::Target(value) = qualifier(tbl, site)?
        && Target::parse(value).is_none()
    {
        return Err(ConfigError::UnknownTargetQualifier {
            line: tbl.line,
            site: site.to_owned(),
            qualifier: value.to_owned(),
        });
    }
    Ok(())
}

/// Whether a site row applies to this conversion's target.
///
/// An unqualified row applies to every target; a target-qualified one only to the target it names
/// (`doc/rfc/0007` section 4.6, why the same profile works with all six targets — the 4f-only answer
/// sits behind a qualifier the other five skip); a shape-qualified row is inert until a shape-aware
/// remedy reads it, because no conversion carries a shape yet.
fn applies_to(tbl: &toml::Table, target: Target) -> bool {
    match qualifier(tbl, "") {
        Ok(Qualifier::None) => true,
        Ok(Qualifier::Target(value)) => Target::parse(value) == Some(target),
        Ok(Qualifier::Shape) | Err(_) => false,
    }
}

/// Reads a `[depart."<id>"]` table into a [`Departure`], where the target binds the requirement.
fn departure(tbl: &toml::Table, target: Target) -> Result<Option<Departure>, ConfigError> {
    let requirement = tbl.path.get(1).cloned().unwrap_or_default();
    let media_types = string_list(tbl, "media-type");
    let relationships = string_list(tbl, "relationship");
    let Some(reason) = tbl.get("reason").and_then(Value::as_text) else {
        return Err(ConfigError::DepartureWithoutReason {
            line: tbl.line,
            requirement,
        });
    };
    let departure = Departure {
        requirement: requirement.clone(),
        media_types,
        relationships,
        reason: reason.to_owned(),
    };
    if !departure.is_built() {
        return Err(ConfigError::UnsupportedDeparture {
            line: tbl.line,
            requirement,
        });
    }
    // A departure from a requirement the target does not bind is inert — the rule is not in force,
    // so there is nothing to depart from. It is read and validated, and simply does not apply.
    Ok(requirement_binds(&requirement, target).then_some(departure))
}

/// A key's value read as a list of strings, empty where the key is absent.
fn string_list(tbl: &toml::Table, key: &str) -> Vec<String> {
    tbl.get(key)
        .and_then(Value::as_list)
        .map(|items| items.into_iter().map(str::to_owned).collect())
        .unwrap_or_default()
}

/// The requirement identifier → [`Loss`] map, read off the decision table.
///
/// The one place a `discard` becomes a built authorisation. Read off [`REMEDIES`] so it cannot
/// disagree with what the converter actually authorises — a site is a built `discard` exactly
/// where the decision table answers its requirement with `Answer::Loses`.
fn loss_sites() -> BTreeMap<&'static str, Loss> {
    REMEDIES
        .iter()
        .filter_map(|remedy| match remedy.answer {
            Answer::Loses(loss, _) => Some((remedy.requirement, loss)),
            _ => None,
        })
        .collect()
}

/// Whether a requirement identifier names a requirement the target binds.
fn requirement_binds(id: &str, target: Target) -> bool {
    table::requirements().any(|requirement| requirement.id == id && requirement.binds(target))
}

/// Every embedded file the departure predicate is asked about: its name, media type and
/// relationship.
///
/// The population is ISO 19005-2 section 6.8's — every file specification dictionary carrying an
/// `/EF`. The media type is the embedded stream's own `/Subtype`, which §7.11.4.2 makes a MIME
/// media type; the relationship is §7.11.3's Table 43 `/AFRelationship`, defaulting to
/// `Unspecified`. Walking every object with an `/EF` mirrors what the validator's predicate does,
/// so *only XML* means only XML by the same reckoning the requirement was failed on.
fn embedded_files(document: &Document) -> Vec<(String, Option<String>, String)> {
    let mut out = Vec::new();
    for number in document.xref().object_numbers() {
        let object = document.get(ObjectId::new(number, 0));
        let Some(specification) = object.as_dict() else {
            continue;
        };
        let files = document.get_key(specification, "EF");
        let Some(files) = files.as_dict() else {
            continue;
        };
        let name = specification_name(document, specification);
        let media_type = ["F", "UF", "DOS", "Mac", "Unix"]
            .into_iter()
            .find_map(|key| {
                let stream = document.get_key(files, key);
                let stream = stream.as_stream()?;
                let subtype = document.get_key(&stream.dict, "Subtype");
                subtype.as_name()?.as_str().map(str::to_owned)
            });
        let relationship = document
            .get_key(specification, "AFRelationship")
            .as_name()
            .and_then(|name| name.as_str().map(str::to_owned))
            .unwrap_or_else(|| "Unspecified".to_owned());
        out.push((name, media_type, relationship));
    }
    out
}

/// A file specification's own name, for a report a person reads.
fn specification_name(
    document: &Document,
    specification: &pdf_syntax::object::Dictionary,
) -> String {
    for key in ["UF", "F"] {
        if let Some(bytes) = document.get_key(specification, key).as_string() {
            return pdf_syntax::text_string(bytes);
        }
    }
    "an unnamed embedded file".to_owned()
}

use super::decision::Authorisations;

/// One refusal site a configuration may answer, for `--remedy-sites --to <target>`.
///
/// `doc/rfc/0007` section 3.1: every site is enumerable, printed *from the same table the converter
/// decides from* so a site cannot exist undocumented and a configuration naming one that does not
/// exist is an error rather than an ignored line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Site {
    /// The requirement identifier — the site's key in a configuration.
    pub requirement: &'static str,
    /// Its clause, as cited for the target asked for.
    pub citation: String,
    /// The remedy a built `discard` at this site authorises, where one is built.
    pub built_discard: Option<Loss>,
    /// Whether this site is one of the two the XML departure may name.
    pub departable: bool,
}

/// Every refusal site the target binds, in clause order, for the enumeration.
///
/// A refusal site is a requirement the converter answers with a refusal or an authorised loss —
/// the ones a configuration has anything to say about. A requirement the writer meets by
/// construction, or that is already answered without a choice, is not a site: there is no refusal
/// to configure. The list is generated from [`super::census`], so it is exactly the table the
/// converter decides from.
#[must_use]
pub fn sites(target: Target) -> Vec<Site> {
    use super::census::{Kind as Standing, Standing as Row};
    let losses = loss_sites();
    super::census::census(target)
        .filter_map(|(requirement, standing)| {
            let is_site = matches!(standing, Row::Refused(_) | Row::Remedy(Standing::Loses));
            if !is_site {
                return None;
            }
            Some(Site {
                requirement: requirement.id,
                citation: requirement
                    .clauses
                    .citation(target)
                    .unwrap_or_else(|| requirement.id.to_owned()),
                built_discard: losses.get(requirement.id).copied(),
                departable: Departure::SUPPORTED.contains(&requirement.id),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdf_archive::{Flavour, Level};

    const TWO_B: Target = Target::Two(Level::B);

    #[test]
    fn a_discard_at_a_loss_site_becomes_the_authorisation_the_flag_would() {
        let text = "\
[site.\"graphics/image-interpolation-is-off\"]
remedy = \"discard\"
";
        let config = Configuration::read(text, TWO_B).expect("it reads");
        let authorised = config.authorisations(TWO_B);
        assert!(authorised.grants(Loss::ImageSmoothing));
        assert!(config.unbuilt(TWO_B).is_empty());
    }

    #[test]
    fn a_discard_at_an_unbuilt_site_authorises_nothing_and_is_named() {
        let text = "\
[site.\"file-structure/no-encryption\"]
remedy = \"discard\"
";
        let config = Configuration::read(text, TWO_B).expect("it reads");
        assert_eq!(config.authorisations(TWO_B), Authorisations::default());
        let unbuilt = config.unbuilt(TWO_B);
        assert_eq!(unbuilt.len(), 1);
        assert_eq!(unbuilt[0].site, "file-structure/no-encryption");
    }

    #[test]
    fn an_unknown_site_is_an_error_naming_it() {
        let text = "[site.\"graphics/not-a-requirement\"]\nremedy = \"discard\"\n";
        let error = Configuration::read(text, TWO_B).expect_err("unknown site");
        assert!(matches!(error, ConfigError::UnknownSite { .. }), "{error}");
    }

    #[test]
    fn an_unknown_remedy_is_an_error_naming_it() {
        let text = "[site.\"file-structure/no-encryption\"]\nremedy = \"ignore\"\n";
        let error = Configuration::read(text, TWO_B).expect_err("unknown remedy");
        assert!(
            matches!(error, ConfigError::UnknownRemedy { .. }),
            "{error}"
        );
    }

    #[test]
    fn an_on_failure_chain_is_refused_per_a57() {
        let text = "\
[site.\"annotations/three-dimensional-stream-format\"]
remedy = \"derive\"
on-failure = [\"preserve\", \"stop\"]
";
        let error = Configuration::read(text, Target::Four(Flavour::E)).expect_err("a chain");
        assert!(
            matches!(error, ConfigError::FallbackChain { .. }),
            "{error}"
        );
    }

    #[test]
    fn a_target_qualifier_decides_which_row_applies() {
        let text = "\
[site.\"embedded-files/embedded-file-is-itself-pdfa-in-the-plain-profile\"]
remedy = \"discard\"

[site.\"embedded-files/embedded-file-is-itself-pdfa-in-the-plain-profile\".target.\"4f\"]
remedy = \"preserve\"
";
        // At PDF/A-4 the unqualified `discard` row applies; the 4f row is read but does not.
        let four = Configuration::read(text, Target::Four(Flavour::Plain)).expect("reads");
        assert_eq!(four.unbuilt(Target::Four(Flavour::Plain)).len(), 1);
    }

    #[test]
    fn a_shape_qualifier_reads_and_is_inert_until_a_shape_aware_remedy_exists() {
        // `doc/pdf-a-mitigations.md` section 14's first finding: a site is finer than a requirement.
        // The format carries the shape qualifier so a configuration can be written against it; a
        // shape-qualified row is inert this round, like a row for another target.
        let text = "\
[site.\"graphics/graphics-state-blend-modes-are-defined\".shape.\"array\"]
remedy = \"discard\"
";
        let config = Configuration::read(text, TWO_B).expect("a shape qualifier reads");
        assert!(
            config.unbuilt(TWO_B).is_empty(),
            "a shape-qualified row does not apply yet"
        );
    }

    #[test]
    fn an_unknown_header_marker_is_an_error() {
        let text = "[site.\"file-structure/no-encryption\".flavour.\"x\"]\nremedy = \"discard\"\n";
        let error = Configuration::read(text, TWO_B).expect_err("unknown marker");
        assert!(
            matches!(error, ConfigError::UnknownTargetQualifier { .. }),
            "{error}"
        );
    }

    #[test]
    fn a_departure_without_a_reason_is_refused() {
        let text = "\
[depart.\"embedded-files/embedded-file-is-itself-pdfa\"]
media-type = [\"application/xml\"]
";
        let error = Configuration::read(text, TWO_B).expect_err("no reason");
        assert!(
            matches!(error, ConfigError::DepartureWithoutReason { .. }),
            "{error}"
        );
    }

    #[test]
    fn the_xml_departure_reads_and_binds_part_two() {
        let text = "\
[depart.\"embedded-files/embedded-file-is-itself-pdfa\"]
media-type = [\"application/xml\", \"text/xml\"]
relationship = [\"Alternative\"]
reason = \"Factur-X invoices; our archive accepts them\"
";
        let config = Configuration::read(text, TWO_B).expect("reads");
        let built = config.built_departures(TWO_B);
        assert_eq!(built.len(), 1);
        assert_eq!(built[0].media_types, vec!["application/xml", "text/xml"]);
        // At PDF/A-4f the embedding rule is lifted, so the departure does not apply.
        assert!(config.built_departures(Target::Four(Flavour::F)).is_empty());
    }

    #[test]
    fn a_departure_this_round_cannot_carry_is_an_error() {
        let text = "\
[depart.\"file-structure/no-encryption\"]
reason = \"we accept it\"
";
        let error = Configuration::read(text, TWO_B).expect_err("unsupported departure");
        assert!(
            matches!(error, ConfigError::UnsupportedDeparture { .. }),
            "{error}"
        );
    }
}
