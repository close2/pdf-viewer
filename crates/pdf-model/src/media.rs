//! ISO 32000-2 §13.2.2's viability, judged for a processor that plays nothing and hands a clip on.
//!
//! `CLAUDE.md`'s clause 13 entry excludes the *playing* of multimedia and admits what a still page
//! owes a reader about a media object: §13.2.2's viability computed and reported in the clause's
//! own terms, and the media file handed to this machine's own player at the reader's level
//! (`doc/questions/A254`, ADR 1946). This module is the first half and the data for the second. It
//! reads §13.2.3's rendition tree as dictionaries — nothing is decoded — and answers, for each
//! media rendition the walk reaches, whether the clause would call it viable and, if not, every
//! reason it would not.
//!
//! # Whose viability
//!
//! §13.2.2 puts the obligation on the reader in so many words:
//!
//! > In making these decisions, the viewer shall determine the viability of the objects used.
//!
//! Viability is relative to a player and a system, so the question has to name both. **The system
//! is [`Processor`]**, which carries what a host has said about its reader and nothing read off
//! the machine. **The player is the hand-off**: the one player this program has is whichever
//! program this machine starts for the clip's content type, and what this program can pass that
//! player is the clip's bytes or its address and nothing else. So every entry whose honouring
//! needs the player to be *told* something — a volume, a start offset past the clip's beginning, a
//! base URL, a window — is an entry this player cannot honour, and the clause says exactly what
//! that costs: in an `MH` dictionary, "otherwise, the containing object shall be considered
//! non-viable"; in a `BE` dictionary, "the containing object is still considered viable". ADR 1960
//! is the argument.
//!
//! # What is not judged here
//!
//! The playing, which the entry excludes, and with it §13.2.5's and §13.2.6's options beyond the
//! one question viability asks of them. Table 285's form-`XObject` clip is named, not handed: "the
//! associated player is implicitly the interactive PDF processor, and the form `XObject` shall be
//! rendered as if it were any other data type", which is presenting content over time in the
//! screen annotation's rectangle — the playing the entry excludes, whatever the data is (ADR 1960).

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::sync::Arc;

use pdf_syntax::{Dictionary, Document, Object, ObjectId, Stream};

use crate::attachment::Attachment;
use crate::file_spec::FileSpec;

pub mod container;
mod deprecated;
mod wave;

pub use deprecated::{movie, sound};
pub use wave::{Encoding, SampleFormat, Samples, Wave, wave};

/// Most renditions one walk of §13.2.3.3's tree visits.
///
/// A selector's `/R` is a list the file controls, and §13.2.3.3 makes it a tree; a tree with more
/// leaves than this is a file built to make a reader work rather than a document offering a choice.
const MAX_RENDITIONS: usize = 256;

/// Deepest §13.2.3.3 selector nesting, and longest §13.2.4.3 section chain, followed.
///
/// The clause requires the chain of sections to "terminate in a media clip data object"; a chain
/// that does not is a cycle or a file built to exceed any reader, and this bounds both.
const MAX_DEPTH: usize = 32;

/// What this processor knows about itself and its reader when it judges viability.
///
/// Facts a host supplies, never read off the machine — the reason `viewer_host`'s
/// `--interface-language` is the only source of the language (ADR 1106). A fact that is `None` is
/// one nobody stated, and a criterion that needs it cannot be shown to be met.
#[derive(Debug, Clone, Copy, Default)]
pub struct Processor<'a> {
    /// §14.9.2.2's language tag of the application, where a host was told one.
    pub language: Option<&'a str>,
}

/// Table 286's `/TF`: when a temporary file may be written to play the clip.
///
/// A permission a document asserts over its reader, so it is not a verdict here:
/// [`crate::restriction::media`] turns it into the reasons the reader's level decides about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporaryFile {
    /// `(TEMPNEVER)` — "Never allowed", and the default.
    Never,
    /// `(TEMPEXTRACT)` — "Allowed only if the document permissions allow content extraction".
    Extract,
    /// `(TEMPACCESS)` — content extraction, "including for accessibility purposes".
    Access,
    /// `(TEMPALWAYS)` — "Always allowed".
    Always,
}

impl TemporaryFile {
    /// The value as Table 286 spells it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Never => "TEMPNEVER",
            Self::Extract => "TEMPEXTRACT",
            Self::Access => "TEMPACCESS",
            Self::Always => "TEMPALWAYS",
        }
    }

    /// Table 286's reading of the string: "Default value: (TEMPNEVER). An unrecognised value shall
    /// be treated as (TEMPNEVER)."
    fn read(value: &[u8]) -> Self {
        match value {
            b"TEMPEXTRACT" => Self::Extract,
            b"TEMPACCESS" => Self::Access,
            b"TEMPALWAYS" => Self::Always,
            _ => Self::Never,
        }
    }
}

/// Where a clip's media data is, as Table 285's `/D` states it.
#[derive(Debug, Clone)]
pub enum Data {
    /// §7.11.4's embedded file stream, undecoded: the hand-off decodes it, the judgement does not.
    Embedded(Box<Attachment>),
    /// §7.11.5's URL specification.
    Url(String),
    /// A file outside the document, named by its specification — the name, not a path.
    File(String),
    /// A form `XObject`, whose player "is implicitly the interactive PDF processor".
    Form,
    /// §13.3's sound object holding its own samples, which the hand-off decodes and writes as a
    /// RIFF WAVE file ([`wave()`], ADR 2398).
    Samples(Box<Samples>),
}

/// A file the hand-off gives a player, with what it carries: §13.2.4.2's media clip data the clause
/// calls viable, §13.4's movie file, or §13.3's sound file ([`movie`], [`sound`]).
#[derive(Debug, Clone)]
pub struct Clip {
    /// Table 285's `/CT`, which "shall identify the type of data".
    pub content_type: Option<String>,
    /// Table 285's `/D`.
    pub data: Data,
    /// Table 286's `/TF`, and `None` for a movie's or a sound's file, over which no media
    /// permissions dictionary stands: Table 286 belongs to §13.2.4.2's clip alone (ADR 2382).
    pub temporary_file: Option<TemporaryFile>,
    /// Entries the hand-off cannot honour, each named — a clip's `BE` entries, which the clause
    /// keeps viable, and a movie's or a sound's playing parameters — and trap 5 keeps the loss said.
    pub unhonoured: Vec<String>,
}

/// One media rendition the walk reached, and the clause's answer about it.
#[derive(Debug, Clone)]
pub struct Considered {
    /// Table 277's `/N`, or the clip's Table 284 `/N` where the rendition names none.
    pub name: Option<String>,
    /// Table 285's or Table 288's `/Alt`, chosen for the reader's language by §14.9.2.4.
    ///
    /// Read whether or not the clip is viable: the table gives it "in case it cannot be played",
    /// and this program plays nothing.
    pub alternate: Option<String>,
    /// The clip, where the rendition is viable; every reason it is not, otherwise.
    pub verdict: Result<Clip, Vec<String>>,
    /// The media player info dictionaries §13.2.7.2.1 treats "as if it were not present in its
    /// original array", each with the reason, said because a removal can be what makes `/MU` empty.
    pub set_aside: Vec<String>,
}

/// §13.2.3's rendition tree, judged: the renditions considered, in the order §13.2.3.3 evaluates
/// them, ending at the first viable one.
#[derive(Debug, Clone, Default)]
pub struct Report {
    /// Every media rendition evaluated, the chosen one last where there is one.
    pub considered: Vec<Considered>,
    /// What stopped a selector or the rendition itself before any clip was reached.
    pub stopped: Vec<String>,
}

impl Report {
    /// The clip §13.2.3.3 says is played: "the first viable media rendition, if any".
    #[must_use]
    pub fn chosen(&self) -> Option<(&Considered, &Clip)> {
        self.considered
            .iter()
            .find_map(|considered| Some((considered, considered.verdict.as_ref().ok()?)))
    }

    /// The report as sentences for a person, the alternate text first where the file gives one.
    #[must_use]
    pub fn notes(&self) -> Vec<String> {
        let mut out = Vec::new();
        for considered in &self.considered {
            let name = considered.name.as_deref().map_or_else(
                || "a rendition".to_owned(),
                |name| format!("rendition {name:?}"),
            );
            if let Some(alternate) = &considered.alternate {
                out.push(format!(
                    "{name} describes itself as {alternate:?} (§14.9.2.4)"
                ));
            }
            match &considered.verdict {
                Ok(clip) => {
                    out.push(format!("{name} is viable (§13.2.2): {}", clip.describe()));
                    out.extend(
                        clip.unhonoured
                            .iter()
                            .map(|entry| format!("{name} plays without {entry}")),
                    );
                }
                Err(reasons) => {
                    for reason in reasons {
                        out.push(format!("{name} is not viable (§13.2.2): {reason}"));
                    }
                }
            }
            out.extend(
                considered
                    .set_aside
                    .iter()
                    .map(|entry| format!("{name} reads {entry}")),
            );
        }
        out.extend(self.stopped.iter().cloned());
        if self.chosen().is_none() {
            out.push(
                "no rendition this action names is viable, so §13.2.2 says the media should not be \
                 played"
                    .to_owned(),
            );
        }
        out
    }
}

impl Clip {
    /// What the clip is, in a phrase.
    #[must_use]
    pub fn describe(&self) -> String {
        let kind = self
            .content_type
            .as_deref()
            .map_or_else(String::new, |kind| format!("{kind}, "));
        match &self.data {
            Data::Embedded(file) => {
                let name = file.file_name.as_deref().unwrap_or(&file.name);
                match file.size {
                    Some(size) => format!("{kind}{name:?} embedded, {size} bytes"),
                    None => format!("{kind}{name:?} embedded"),
                }
            }
            Data::Url(url) => format!("{kind}at {url}"),
            Data::File(name) => format!("{kind}the file {name:?} outside this document"),
            Data::Form => "a form XObject, which only this processor could play".to_owned(),
            Data::Samples(samples) => format!(
                "{kind}{}, written as {:?} for the player",
                samples.format.describe(),
                samples.name
            ),
        }
    }
}

/// Judges the rendition a rendition action's `/R` names, as §13.2.3.3's depth-first search does.
#[must_use]
pub fn judge(document: &Document, rendition: &Object, processor: Processor<'_>) -> Report {
    let mut walk = Walk {
        document,
        processor,
        seen: BTreeSet::new(),
        visited: 0,
        bounded: false,
        report: Report::default(),
        timelines: RefCell::new(Vec::new()),
    };
    walk.rendition(rendition, 0);
    walk.report
}

/// What the container of the media clip data `data` states about its timeline, read the way the
/// judgement reads it for an offset (§13.2.4.3, ADR 2452).
///
/// This is the census's door onto the same code path: it decodes the embedded file under the
/// document's own limits and reads the header of the container the clip's Table 285 `/CT` names,
/// so a count of clips this returns `Ok` for is a count of clips whose offsets the judgement can
/// decide.
///
/// # Errors
///
/// The sentence the report would give: a data dictionary without a `/CT`, a type whose container is
/// not read, data named by URL or by a file outside the document, a file that decodes only in
/// part, and a container that is cut short or past one of the reader's bounds.
pub fn clip_timeline(
    document: &Document,
    data: &Dictionary,
) -> Result<container::Timeline, String> {
    let walk = Walk {
        document,
        processor: Processor::default(),
        seen: BTreeSet::new(),
        visited: 0,
        bounded: false,
        report: Report::default(),
        timelines: RefCell::new(Vec::new()),
    };
    walk.timeline(Some(data))
}

/// The state of one walk: the bounds and what has been found.
struct Walk<'a> {
    document: &'a Document,
    processor: Processor<'a>,
    seen: BTreeSet<ObjectId>,
    visited: usize,
    /// Whether the bound below has been said, so that a tree past it is said once rather than once
    /// per rendition it still names.
    bounded: bool,
    report: Report,
    /// Each embedded clip's timeline once read, by its stream, so that a chain of sections reads
    /// its data once. The stream is held, not its address, so no entry can name another stream.
    timelines: RefCell<Vec<(Arc<Stream>, TimelineRead)>>,
}

/// What reading one clip's container gave: its timeline, or the sentence a person is told.
type TimelineRead = Result<container::Timeline, String>;

/// What §13.2.4.3's rules make of a section's next-level clip under the hand-off, which plays its
/// file from the beginning to the end: whether the clip begins where the file does, and whether it
/// ends where the file does.
#[derive(Debug, Clone, Copy)]
struct Span {
    starts: bool,
    ends: bool,
}

/// A §13.2.6.2 media offset, as Tables 296 to 300 read it.
enum Offset {
    /// Table 297's time, in seconds, over a Table 300 simple timespan.
    Time(f64),
    /// Table 298's frame.
    Frame(i64),
    /// Table 299's marker name.
    Marker(String),
    /// A Table 296 or Table 300 `/S` stated with a value this processor does not recognise, of
    /// which each table says "The rendition is non-viable if the interactive PDF processor does
    /// not recognise the value of this entry" — the rendition, in `MH` or `BE` (ADR 2447); the
    /// table it is.
    Unrecognised(&'static str),
    /// One no player is told: a negative timespan ("\[n\]egative timespans are prohibited in this
    /// context"), a negative frame ("out of range"), or a required entry missing.
    Unviable,
}

impl Walk<'_> {
    /// One rendition; `true` once a viable media rendition is found, which ends the search.
    fn rendition(&mut self, entry: &Object, depth: usize) -> bool {
        if depth > MAX_DEPTH || self.visited >= MAX_RENDITIONS {
            if !self.bounded {
                self.bounded = true;
                self.report.stopped.push(format!(
                    "the rendition tree is deeper than {MAX_DEPTH} or wider than {MAX_RENDITIONS} \
                     renditions, and the rest was not read"
                ));
            }
            return false;
        }
        self.visited = self.visited.saturating_add(1);
        if let Object::Reference(id) = entry
            && !self.seen.insert(*id)
        {
            return false;
        }
        let document = self.document;
        let resolved = document.resolve(entry);
        let Some(dict) = resolved.as_dict() else {
            self.report
                .stopped
                .push("Table 218's /R is not a rendition dictionary".to_owned());
            return false;
        };
        let name = text(document, dict, "N");
        let mut reasons = Vec::new();
        // Table 277's `/S`: "The rendition is non-viable if the interactive PDF processor does not
        // recognise the value of this entry."
        let kind = document.get_key(dict, "S");
        let kind = kind.as_name().map(|name| name.as_bytes().to_vec());
        // Table 278: the rendition's `MH` and `BE` hold one entry, `/C`, a media criteria
        // dictionary, which §13.2.3.1 has evaluated "regardless of whether they are in an MH or a
        // BE dictionary".
        self.criteria(dict, &mut reasons);
        match kind.as_deref() {
            Some(b"MR") => {
                let mut set_aside = Vec::new();
                let verdict = if reasons.is_empty() {
                    self.media_rendition(dict, &mut set_aside)
                } else {
                    Err(reasons)
                };
                let alternate = self.alternate_of(dict);
                let clip_name = || {
                    let clip = document.get_key(dict, "C");
                    clip.as_dict().and_then(|clip| text(document, clip, "N"))
                };
                let viable = verdict.is_ok();
                self.report.considered.push(Considered {
                    name: name.or_else(clip_name),
                    alternate,
                    verdict,
                    set_aside,
                });
                viable
            }
            Some(b"SR") => {
                // §13.2.3.3: "A selector rendition itself may be non-viable; in this case, none of
                // its associated media renditions shall be evaluated".
                if !reasons.is_empty() {
                    let name = name.map_or_else(
                        || "a selector rendition".to_owned(),
                        |name| format!("selector {name:?}"),
                    );
                    self.report.stopped.extend(
                        reasons
                            .into_iter()
                            .map(|reason| format!("{name} is not viable (§13.2.2): {reason}")),
                    );
                    return false;
                }
                let items = document.get_key(dict, "R");
                let items = items.as_array().map(<[Object]>::to_vec).unwrap_or_default();
                items
                    .iter()
                    .any(|item| self.rendition(item, depth.saturating_add(1)))
            }
            _ => {
                self.report.stopped.push(
                    "a rendition whose Table 277 /S is neither MR nor SR is not viable (§13.2.2)"
                        .to_owned(),
                );
                false
            }
        }
    }

    /// §13.2.3.2: "A media rendition object shall be viable if and only if the objects referenced
    /// by its C , P , and SP entries are viable."
    fn media_rendition(
        &self,
        dict: &Dictionary,
        set_aside: &mut Vec<String>,
    ) -> Result<Clip, Vec<String>> {
        let document = self.document;
        let mut reasons = Vec::new();
        let parameters = document.get_key(dict, "P");
        let parameters = parameters.as_dict();
        // Table 290's `/PL` takes part in §13.2.7.2.2's algorithm beside the clip's own.
        let play_players = parameters.map(|parameters| document.get_key(parameters, "PL"));
        if let Some(parameters) = parameters {
            unhonourable(document, parameters, "Table 291", &mut reasons);
            unrecognised_duration(document, parameters, &mut reasons);
        }
        let screen = document.get_key(dict, "SP");
        if let Some(screen) = screen.as_dict() {
            unhonourable(document, screen, "Table 294", &mut reasons);
        }
        let clip = document.get_key(dict, "C");
        let Some(clip) = clip.as_dict() else {
            // §13.2.3.2: "C may be omitted only in cases where a referenced player takes no
            // meaningful input", which names a player by its software identifier; the hand-off is
            // no such player.
            reasons.push(
                "it names no media clip (Table 282's /C), only a player that takes no input, and \
                 this program hands a clip to a player rather than being one"
                    .to_owned(),
            );
            return Err(reasons);
        };
        let clip = self.clip(clip, play_players.as_ref(), 0, &mut reasons, set_aside);
        match clip {
            Some(clip) if reasons.is_empty() => Ok(clip),
            _ => Err(reasons),
        }
    }

    /// Table 284's two subtypes: §13.2.4.2's data and §13.2.4.3's section.
    fn clip(
        &self,
        dict: &Dictionary,
        play_players: Option<&Object>,
        depth: usize,
        reasons: &mut Vec<String>,
        set_aside: &mut Vec<String>,
    ) -> Option<Clip> {
        let document = self.document;
        if depth > MAX_DEPTH {
            reasons.push(format!(
                "its media clip sections do not end in media clip data within {MAX_DEPTH} levels \
                 (§13.2.4.3)"
            ));
            return None;
        }
        let kind = document.get_key(dict, "S");
        match kind.as_name().map(pdf_syntax::Name::as_bytes) {
            Some(b"MCD") => self.clip_data(dict, play_players, reasons, set_aside),
            Some(b"MCS") => {
                // The next level is judged first because its content type decides whether this
                // section's offsets count at all; its reasons are said after this section's own.
                let next = document.get_key(dict, "D");
                let mut below = Vec::new();
                let clip = next.as_dict().and_then(|next| {
                    self.clip(
                        next,
                        play_players,
                        depth.saturating_add(1),
                        &mut below,
                        set_aside,
                    )
                });
                // §13.2.4.3: "For media types where an offset makes no sense (such as JPEG
                // images), B and E shall be ignored, with no effect on viability."
                let ignored = clip
                    .as_ref()
                    .and_then(|clip| clip.content_type.as_deref())
                    .is_some_and(timeless);
                // Otherwise Table 289's `/B` and `/E` in an `MH` "shall be honoured at play-time in
                // order for the media clip section to be considered viable", and a player handed a
                // file is started at its beginning and stopped at its end. So a `/B` naming that
                // beginning is honoured by the hand-off, and only one: a time or frame `/B` is
                // "relative to the start of the next-level media clip", which is the file's
                // beginning only where every `/B` below it names its own clip's start too.
                let next_span = next
                    .as_dict()
                    .map(|next| self.span(next, depth.saturating_add(1)));
                let mut unhonoured = Vec::new();
                for (key, entry) in [("MH", true), ("BE", false)] {
                    let options = document.get_key(dict, key);
                    let Some(options) = options.as_dict() else {
                        continue;
                    };
                    for (name, _) in options.iter() {
                        let name = String::from_utf8_lossy(name.as_bytes()).into_owned();
                        let known = matches!(name.as_str(), "B" | "E");
                        if known && ignored {
                            continue;
                        }
                        if known
                            && let Offset::Unrecognised(table) =
                                read_offset(document, &document.get_key(options, &name))
                        {
                            // Tables 296 and 300 name the rendition, so a `BE` holding it is no
                            // refuge (ADR 2447).
                            reasons.push(unrecognised(key, &name, table));
                            continue;
                        }
                        let held = match (known, next_span, next.as_dict()) {
                            (true, Some(next_span), Some(next)) => self.bound(
                                &name,
                                &document.get_key(options, &name),
                                next,
                                next_span,
                                depth.saturating_add(1),
                            ),
                            _ => Err(String::new()),
                        };
                        let why = match held {
                            Ok(()) => continue,
                            Err(why) if why.is_empty() => String::new(),
                            Err(why) => format!(": {why}"),
                        };
                        match (entry, known) {
                            (true, true) => reasons.push(format!(
                                "its media clip section must start or end at Table 289's /{name}, \
                                 and a player handed the file plays all of it{why}"
                            )),
                            (true, false) => reasons.push(format!(
                                "its media clip section's MH holds /{name}, which Table 289 does \
                                 not define"
                            )),
                            (false, true) => unhonoured
                                .push(format!("the section's best-effort /{name} (Table 289)")),
                            (false, false) => {}
                        }
                    }
                }
                if next.as_dict().is_none() {
                    reasons.push(
                        "its media clip section names no next-level clip (Table 288's /D)"
                            .to_owned(),
                    );
                    return None;
                }
                // "If the next-level media object is non-viable, the media clip section shall be
                // also non-viable."
                reasons.extend(below);
                let mut clip = clip?;
                clip.unhonoured.extend(unhonoured);
                Some(clip)
            }
            _ => {
                // Table 284's `/S`: "The media clip is non-viable if the interactive PDF processor
                // does not recognise the value of this entry."
                reasons.push("its media clip's Table 284 /S is neither MCD nor MCS".to_owned());
                None
            }
        }
    }

    /// What the next-level clip `dict` spans of its file (§13.2.4.3): media clip data spans the
    /// whole of it, and a section begins where its file does when its own next-level clip does and
    /// each `/B` it states, in `MH` or `BE`, holds at the file's start ([`Self::bound`]), and ends
    /// where its file does on the same terms for each `/E`. Best-effort entries count here because
    /// they move the clip whatever this player does with them, and §13.2.4.3 binds them as must-honour
    /// where a section above states the same entry in its `MH`. A chain past [`MAX_DEPTH`] spans
    /// nothing this judges, and the clip above it is non-viable on its own account.
    fn span(&self, dict: &Dictionary, depth: usize) -> Span {
        let document = self.document;
        if depth > MAX_DEPTH {
            return Span {
                starts: false,
                ends: false,
            };
        }
        match document
            .get_key(dict, "S")
            .as_name()
            .map(pdf_syntax::Name::as_bytes)
        {
            Some(b"MCS") => {
                let next = document.get_key(dict, "D");
                let Some(next) = next.as_dict() else {
                    return Span {
                        starts: false,
                        ends: false,
                    };
                };
                let below = self.span(next, depth.saturating_add(1));
                let holds = |key: &str| {
                    ["MH", "BE"].into_iter().all(|options| {
                        document
                            .get_key(dict, options)
                            .as_dict()
                            .is_none_or(|options| {
                                options.get(key).is_none()
                                    || self
                                        .bound(
                                            key,
                                            &document.get_key(options, key),
                                            next,
                                            below,
                                            depth.saturating_add(1),
                                        )
                                        .is_ok()
                            })
                    })
                };
                Span {
                    starts: below.starts && holds("B"),
                    ends: below.ends && holds("E"),
                }
            }
            _ => Span {
                starts: true,
                ends: true,
            },
        }
    }

    /// Whether a section's `/B` or `/E` (`key`), with the value `offset`, holds for a player that
    /// plays the file from its beginning to its end, where `next` is the section's next-level clip
    /// and `below` what that clip spans; the error says why not, or is empty where the sentence
    /// that follows it in the report is the whole reason.
    ///
    /// §13.2.4.3's rules, in order:
    ///
    /// - "For media types where an offset makes no sense (such as JPEG images), B and E shall be
    ///   ignored, with no effect on viability" ([`timeless`]).
    /// - "When B or E are specified by time or frames, their value shall be considered to be
    ///   relative to the start of the next-level media clip", so a `/B` of zero seconds or frame 0
    ///   is the file's start where the next-level clip begins there. "However, if E specifies an
    ///   offset beyond the end of the next-level media clip, the end value shall be used instead,
    ///   and there is no effect on viability", so an `/E` at or past the next-level clip's length
    ///   is the file's end where that clip is the whole file — its length the container's
    ///   ([`container::timeline`]), in seconds or in frames as the offset is.
    /// - "When B or E are specified by markers, there shall be a corresponding absolute offset into
    ///   the underlying media clip data object. If this offset is not within the range defined by
    ///   the next-level media clip (if any), or if the marker is not present in the underlying
    ///   media clip, the existence of the entry shall be ignored, and there is no effect on
    ///   viability." A marker the container's table does not name holds; one it names holds where
    ///   it falls at the start (a `/B`) or at or past the end (an `/E`) of a next-level clip that
    ///   is the whole file, or past that file's end, outside its range.
    ///
    /// Anything else is a place inside the clip the player would have to be told, and §13.2.4.3's
    /// NOTE 2 says what that is: "The entry cannot be honoured if its value was not viable or if
    /// the player did not support its value". A container this program cannot read is said as the
    /// reason, because whether an end or a marker holds is then a fact nobody here knows (ADR 2452).
    fn bound(
        &self,
        key: &str,
        offset: &Object,
        next: &Dictionary,
        below: Span,
        depth: usize,
    ) -> Result<(), String> {
        let document = self.document;
        let terminal = self.terminal(next, depth);
        if terminal
            .as_ref()
            .and_then(|data| content_type(document, data))
            .is_some_and(|content_type| timeless(&content_type))
        {
            return Ok(());
        }
        let whole = below.starts && below.ends;
        let begin = key == "B";
        match read_offset(document, offset) {
            Offset::Unrecognised(table) => Err(format!(
                "its /{key} states a {table} /S this program does not recognise"
            )),
            Offset::Unviable => Err(format!(
                "its /{key} is not an offset Tables 296 to 300 recognise, which they make \
                 non-viable"
            )),
            Offset::Time(seconds) if begin => (seconds == 0.0 && below.starts)
                .then_some(())
                .ok_or_else(String::new),
            Offset::Frame(frame) if begin => (frame == 0 && below.starts)
                .then_some(())
                .ok_or_else(String::new),
            Offset::Time(seconds) => {
                if !whole {
                    return Err(String::new());
                }
                let timeline = self.timeline(terminal.as_ref())?;
                let length = timeline.seconds.ok_or_else(|| {
                    "its container does not state how long the clip runs".to_owned()
                })?;
                (seconds >= length)
                    .then_some(())
                    .ok_or_else(|| format!("the clip runs {length} s, past its /E"))
            }
            Offset::Frame(frame) => {
                if !whole {
                    return Err(String::new());
                }
                let timeline = self.timeline(terminal.as_ref())?;
                let frames = timeline.frames.ok_or_else(|| {
                    "its container states no frames for a frame offset to count".to_owned()
                })?;
                (u64::try_from(frame).is_ok_and(|frame| frame >= frames))
                    .then_some(())
                    .ok_or_else(|| format!("the clip holds {frames} frames, past its /E"))
            }
            Offset::Marker(name) => {
                let timeline = self.timeline(terminal.as_ref())?;
                let Some(marker) = timeline.markers.iter().find(|marker| marker.name == name)
                else {
                    return Ok(());
                };
                let length = timeline.seconds;
                let past_the_file = length.is_some_and(|length| marker.seconds > length);
                let at_the_bound = if begin {
                    marker.seconds == 0.0
                } else {
                    length.is_some_and(|length| marker.seconds >= length)
                };
                (whole && (past_the_file || at_the_bound))
                    .then_some(())
                    .ok_or_else(|| {
                        format!(
                            "its container places the marker {name:?} at {} s",
                            marker.seconds
                        )
                    })
            }
        }
    }

    /// The media clip data a chain of sections from `dict` ends in, within [`MAX_DEPTH`] levels.
    fn terminal(&self, dict: &Dictionary, depth: usize) -> Option<Dictionary> {
        let document = self.document;
        let mut current = dict.clone();
        for _ in depth..=MAX_DEPTH {
            match document
                .get_key(&current, "S")
                .as_name()
                .map(pdf_syntax::Name::as_bytes)
            {
                Some(b"MCD") => return Some(current),
                Some(b"MCS") => current = document.get_key(&current, "D").as_dict()?.clone(),
                _ => return None,
            }
        }
        None
    }

    /// The timeline of the media clip data `data`, read once per stream.
    ///
    /// Only data the document embeds is read: a URL or a file named outside the document is a
    /// resource this program does not fetch to judge viability (the renderer opens no path and no
    /// connection, `CLAUDE.md` principle 3), and the error says so. The stream is decoded under the
    /// document's own limits, and only once a container this program reads is named, so a clip of
    /// any other type costs nothing.
    fn timeline(&self, data: Option<&Dictionary>) -> Result<container::Timeline, String> {
        let document = self.document;
        let data = data.ok_or_else(|| {
            "its sections do not end in media clip data this program could read".to_owned()
        })?;
        let content_type = content_type(document, data)
            .ok_or_else(|| "its media clip data states no Table 285 /CT".to_owned())?;
        if !container::is_read(&content_type) {
            return Err(container::unread(&content_type));
        }
        let stream = match self.data(data, &mut Vec::new()) {
            Some(Data::Embedded(file)) => file.stream,
            Some(Data::Url(_)) => {
                return Err(
                    "its media data is a URL, which this program does not fetch to judge \
                            it"
                    .to_owned(),
                );
            }
            Some(Data::File(_)) => {
                return Err(
                    "its media data is a file outside the document, which this program does not \
                     open to judge it"
                        .to_owned(),
                );
            }
            _ => return Err("its media data is not a file this program could read".to_owned()),
        };
        if let Some((_, read)) = self
            .timelines
            .borrow()
            .iter()
            .find(|(held, _)| Arc::ptr_eq(held, &stream))
        {
            return read.clone();
        }
        // A decode that kept what it read before damage is not the file, and a container cut short
        // would state a shorter clip than the producer embedded (trap 5).
        let read = match document.decoded_stream_data_reported(&stream) {
            Ok(decoded) if decoded.damage.is_none() => {
                container::timeline(&content_type, &decoded.data)
            }
            Ok(_) => Err("its embedded file decoded only in part".to_owned()),
            Err(pdf_syntax::StreamRefusal::Filter { name, .. }) => Err(format!(
                "its embedded file's /{name} filter would not decode it"
            )),
            Err(_) => Err("its embedded file could not be decrypted or read".to_owned()),
        };
        self.timelines
            .borrow_mut()
            .push((Arc::clone(&stream), read.clone()));
        read
    }

    /// §13.2.4.2's media clip data, against Tables 285 to 287 and §13.2.7.2.2's player algorithm.
    fn clip_data(
        &self,
        dict: &Dictionary,
        play_players: Option<&Object>,
        reasons: &mut Vec<String>,
        set_aside: &mut Vec<String>,
    ) -> Option<Clip> {
        let document = self.document;
        let content_type = content_type(document, dict);
        let data = self.data(dict, reasons);

        // Table 285's `/P`, with the clause's own rule: "If the dictionary contains unrecognised
        // entries or entries with unrecognised values, it is considered non-viable". Table 286's
        // `/TF` row states the one exception — "An unrecognised value shall be treated as
        // (TEMPNEVER)" — and the row is the more specific statement, so an unknown `/TF` is read as
        // `TEMPNEVER` rather than made non-viable (ADR 1960).
        let mut temporary_file = TemporaryFile::Never;
        let permissions = document.get_key(dict, "P");
        if let Some(permissions) = permissions.as_dict() {
            for (name, value) in permissions.iter() {
                match name.as_bytes() {
                    b"TF" => match document.resolve(value) {
                        Object::String(bytes) => temporary_file = TemporaryFile::read(&bytes),
                        _ => reasons.push(
                            "its media permissions state a /TF that is not a string (Table 286)"
                                .to_owned(),
                        ),
                    },
                    b"Type" => {}
                    other => reasons.push(format!(
                        "its media permissions hold /{}, which Table 286 does not define, and the \
                         clause says such a dictionary is non-viable",
                        String::from_utf8_lossy(other)
                    )),
                }
            }
        }

        // Table 287's `/BU` in an `MH`: "If BU is in the MH dictionary and the base URL is not
        // honoured the media clip data shall be nonviable", and a player handed only the file is
        // told no base URL.
        let mut unhonoured = Vec::new();
        for (key, must) in [("MH", true), ("BE", false)] {
            let options = document.get_key(dict, key);
            let Some(options) = options.as_dict() else {
                continue;
            };
            for (name, _) in options.iter() {
                let name = String::from_utf8_lossy(name.as_bytes()).into_owned();
                match (must, name == "BU") {
                    (true, true) => reasons.push(
                        "it must be played against Table 287's base URL /BU, and a player handed \
                         the file is told none"
                            .to_owned(),
                    ),
                    (true, false) => reasons.push(format!(
                        "its media clip data's MH holds /{name}, which Table 287 does not define"
                    )),
                    (false, true) => unhonoured.push("the best-effort base URL /BU".to_owned()),
                    (false, false) => {}
                }
            }
        }

        // §13.2.4.2: "For media other than form XObjects, the media clip object shall provide
        // enough information to allow an interactive PDF processor to locate an appropriate
        // player", and §13.2.7.2.2 decides whether the hand-off may be that player.
        if !matches!(data, Some(Data::Form)) {
            let clip_players = document.get_key(dict, "PL");
            players(
                document,
                content_type.as_deref(),
                [Some(&clip_players), play_players],
                reasons,
                set_aside,
            );
        }
        Some(Clip {
            content_type,
            data: data?,
            temporary_file: Some(temporary_file),
            unhonoured,
        })
    }

    /// Table 285's `/D`, with the clause's condition on it.
    fn data(&self, dict: &Dictionary, reasons: &mut Vec<String>) -> Option<Data> {
        let document = self.document;
        let target = document.get_key(dict, "D");
        // "The media clip data object is non-viable if the object referenced by the D entry does
        // not contain a Type entry, the Type entry is unrecognised, or the referenced object is not
        // a dictionary or stream. This shall effectively exclude the use of simple file
        // specifications".
        let (target_dict, is_stream) = match &target {
            Object::Dictionary(dict) => (dict.clone(), false),
            Object::Stream(stream) => (stream.dict.clone(), true),
            _ => {
                reasons.push(
                    "its Table 285 /D is not a dictionary or a stream, which §13.2.4.2 makes \
                     non-viable"
                        .to_owned(),
                );
                return None;
            }
        };
        let kind = document.get_key(&target_dict, "Type");
        match (kind.as_name().map(pdf_syntax::Name::as_bytes), is_stream) {
            (Some(b"Filespec"), false) => {
                let specification = FileSpec::from_dictionary(document, &target_dict);
                let name = specification
                    .display_name()
                    .unwrap_or_else(|| "media".to_owned());
                if let Some(file) = crate::attachment::read(document, &target_dict, name.clone()) {
                    return Some(Data::Embedded(Box::new(file)));
                }
                if let Some(url) = specification.url() {
                    return Some(Data::Url(url));
                }
                Some(Data::File(name))
            }
            (Some(b"XObject"), true)
                if document
                    .get_key(&target_dict, "Subtype")
                    .as_name()
                    .is_some_and(|name| name.as_bytes() == b"Form") =>
            {
                Some(Data::Form)
            }
            (None, _) => {
                reasons.push(
                    "its Table 285 /D states no /Type, which §13.2.4.2 makes non-viable".to_owned(),
                );
                None
            }
            _ => {
                reasons.push(
                    "its Table 285 /D has a /Type that is neither a file specification nor a form \
                     XObject, which §13.2.4.2 makes non-viable"
                        .to_owned(),
                );
                None
            }
        }
    }

    /// Table 279's media criteria, from the rendition's `MH` and `BE` (§13.2.3.1).
    ///
    /// "The criteria specified by all of its entries shall be met regardless of whether they are in
    /// an MH or a BE dictionary. The only exception is that if an entry in a BE dictionary is
    /// unrecognised by the interactive PDF processor, it shall not affect the viability of the
    /// object." And where both state one entry, "MH taking precedence (corresponding BE entries
    /// shall be ignored)".
    fn criteria(&self, rendition: &Dictionary, reasons: &mut Vec<String>) {
        let document = self.document;
        let mut decided = BTreeSet::new();
        for (key, must) in [("MH", true), ("BE", false)] {
            let options = document.get_key(rendition, key);
            let Some(options) = options.as_dict() else {
                continue;
            };
            for (name, value) in options.iter() {
                if name.as_bytes() != b"C" {
                    if must {
                        reasons.push(format!(
                            "its rendition MH holds /{}, which Table 278 does not define",
                            String::from_utf8_lossy(name.as_bytes())
                        ));
                    }
                    continue;
                }
                let criteria = document.resolve(value);
                let Some(criteria) = criteria.as_dict() else {
                    continue;
                };
                for (entry, value) in criteria.iter() {
                    let entry = String::from_utf8_lossy(entry.as_bytes()).into_owned();
                    if entry == "Type" || !decided.insert(entry.clone()) {
                        continue;
                    }
                    if let Some(reason) = self.criterion(&entry, &document.resolve(value), must) {
                        reasons.push(reason);
                    }
                }
            }
        }
    }

    /// One Table 279 entry: `None` where it is met, the reason where it is not.
    fn criterion(&self, entry: &str, value: &Object, must: bool) -> Option<String> {
        Some(match entry {
            "A" | "C" | "O" | "S" => {
                let preference = match entry {
                    "A" => "hear audio descriptions",
                    "C" => "see text captions",
                    "O" => "hear audio overdubs",
                    _ => "see subtitles",
                };
                format!(
                    "Table 279's /{entry} asks whether the reader prefers to {preference}, a \
                     preference this program does not hold"
                )
            }
            "R" => "Table 279's /R needs the system's bandwidth, which this program does not \
                    measure"
                .to_owned(),
            "D" | "Z" => format!(
                "Table 279's /{entry} needs a monitor's {}, which a host has not stated",
                if entry == "D" { "depth" } else { "size" }
            ),
            // Table 279's /V and /P each bind only "[i]f this entry is present and non-empty".
            "V" | "P" if value.as_array().is_none_or(<[Object]>::is_empty) => return None,
            // "If this entry is present and non-empty, the interactive PDF processor shall be
            // identified by one or more of the objects in the array". §13.2.7.4.3's names are
            // registered second-class names, and this program has none.
            "V" => "Table 279's /V names the processors that may play it, and this program is \
                    identified by no software name"
                .to_owned(),
            // "the version of multimedia constructs fully supported by the interactive PDF
            // processor shall be within the specified range", and this program plays none of them.
            "P" => "Table 279's /P asks for a processor that fully supports a version of the \
                    multimedia constructs, and this program plays none"
                .to_owned(),
            "L" => return self.language_criterion(value),
            _ if must => {
                format!("its media criteria's MH holds /{entry}, which Table 279 does not define")
            }
            _ => return None,
        })
    }

    /// Table 279's `/L`: "the language in which the interactive PDF processor is running shall
    /// exactly match a language identifier, or consist only of a primary code that matches the
    /// primary code of an identifier".
    fn language_criterion(&self, value: &Object) -> Option<String> {
        let tags: Vec<String> = value
            .as_array()
            .unwrap_or_default()
            .iter()
            .filter_map(|item| match self.document.resolve(item) {
                Object::String(bytes) => Some(pdf_syntax::text_string(&bytes)),
                _ => None,
            })
            .collect();
        if tags.is_empty() {
            return None;
        }
        let Some(running) = self.processor.language else {
            return Some(format!(
                "Table 279's /L asks for a processor running in {}, and no language was stated \
                 for this one",
                tags.join(" or ")
            ));
        };
        let matches = tags.iter().any(|tag| {
            tag.eq_ignore_ascii_case(running)
                || (!running.contains('-')
                    && tag
                        .split('-')
                        .next()
                        .is_some_and(|primary| primary.eq_ignore_ascii_case(running)))
        });
        (!matches).then(|| {
            format!(
                "Table 279's /L asks for a processor running in {}, and this one runs in {running}",
                tags.join(" or ")
            )
        })
    }

    /// The `/Alt` of the clip a media rendition names, through any sections to its data.
    fn alternate_of(&self, rendition: &Dictionary) -> Option<String> {
        let document = self.document;
        let mut clip = document.get_key(rendition, "C");
        for _ in 0..=MAX_DEPTH {
            let dict = clip.as_dict()?.clone();
            let alt = document.get_key(&dict, "Alt");
            if let Some(items) = alt.as_array()
                && let Some(text) = alternate(document, items, self.processor.language)
            {
                return Some(text);
            }
            clip = document.get_key(&dict, "D");
        }
        None
    }
}

/// A dictionary of options the hand-off cannot pass to a player, judged by §13.2.2's rules.
///
/// Tables 291 and 294 define what may be asked — volume, controls, fit, duration, a window, a
/// monitor — and every one of them is something a player would have to be *told*. In `MH` each is
/// a reason the object is non-viable; in `BE` each is ignored, which §13.2.2 permits.
fn unhonourable(document: &Document, dict: &Dictionary, table: &str, reasons: &mut Vec<String>) {
    let options = document.get_key(dict, "MH");
    let Some(options) = options.as_dict() else {
        return;
    };
    for (name, _) in options.iter() {
        reasons.push(format!(
            "it must be played with {table}'s /{} honoured, and a player handed only the file is \
             told nothing about how to play it",
            String::from_utf8_lossy(name.as_bytes())
        ));
    }
}

/// ISO 32000-2 §13.2.5's Table 292 media duration `/D` under the play parameters' `BE`, whose
/// explicit duration `/T` is a §13.2.6.3 Table 300 timespan.
///
/// Table 291's `BE` entries are honoured in a best-effort sense only, so a duration this program
/// cannot be told leaves the rendition chosen. One sentence is not about the entry but about the
/// rendition, and a `BE` does not set it aside (ADR 2447, ADR 2472): Table 300's, of a timespan's
/// `/S`,
///
/// > The rendition is non-viable if the interactive PDF processor does not recognise the value of
/// > this entry.
///
/// Table 292 reaches a timespan only when its own `/S` is `T`; otherwise `/T` is ignored. A duration
/// whose own `/S` is not recognised is a non-viable duration dictionary, and so is a timespan with
/// a negative or a missing `/V`; a `BE` sets those aside. An `MH` duration is read by
/// [`unhonourable`] already, as every `MH` entry is.
fn unrecognised_duration(document: &Document, parameters: &Dictionary, reasons: &mut Vec<String>) {
    let best_effort = document.get_key(parameters, "BE");
    let Some(best_effort) = best_effort.as_dict() else {
        return;
    };
    let duration = document.get_key(best_effort, "D");
    let Some(duration) = duration.as_dict() else {
        return;
    };
    let explicit = document
        .get_key(duration, "S")
        .as_name()
        .is_some_and(|subtype| subtype.as_bytes() == b"T");
    if explicit
        && matches!(
            read_timespan(document, &document.get_key(duration, "T")),
            Timespan::Unrecognised
        )
    {
        reasons.push(
            "its media play parameters' BE /D states a Table 300 /S this program does not \
             recognise, and \"The rendition is non-viable if the interactive PDF processor does \
             not recognise the value of this entry\""
                .to_owned(),
        );
    }
}

/// §13.2.7.2.2's algorithm, for the one player this program has: whichever program this machine
/// starts for the content type, which no software identifier names.
///
/// Of the four conditions under which "\[t]he player may not be used", (a) cannot hold — that
/// player is chosen by the type — and (b) cannot either, because a player is "found" in an array
/// only by §13.2.7.4's software identifier. So (c) and (d) decide it: a non-empty `/MU` in either
/// dictionary names the players "one of which shall be used", and with no `/MU` and no content type
/// the player must be found in an `/A`, which it cannot be.
///
/// An array's emptiness is read after its entries' own viability, as §13.2.7.2.1 orders:
///
/// > If a non-viable media player info dictionary is referenced by MU , NU , or A , it shall be
/// > treated as if it were not present in its original array
///
/// The same sentence goes on to place a dictionary holding the same software identifier "present
/// in NU", which decides nothing here, since condition (b) needs the player found by that
/// identifier and no software identifier names this one.
fn players(
    document: &Document,
    content_type: Option<&str>,
    dictionaries: [Option<&Object>; 2],
    reasons: &mut Vec<String>,
    set_aside: &mut Vec<String>,
) {
    let mut must_use = false;
    for players in dictionaries.iter().flatten() {
        let Some(players) = players.as_dict() else {
            continue;
        };
        for key in ["MU", "A", "NU"] {
            let items = document.get_key(players, key);
            let Some(items) = items.as_array() else {
                continue;
            };
            for (index, item) in items.iter().enumerate() {
                match player_info_set_aside(document, item) {
                    Some(reason) => set_aside.push(format!(
                        "a media players dictionary's /{key} entry {} as not present \
                         (§13.2.7.2.1): {reason}",
                        index.saturating_add(1)
                    )),
                    None => must_use |= key == "MU",
                }
            }
        }
    }
    if must_use {
        reasons.push(
            "a media players dictionary's /MU names the players that must play it (§13.2.7.2.2 \
             condition c), and the program this machine would start is identified by none of them"
                .to_owned(),
        );
    } else if content_type.is_none() {
        reasons.push(
            "it states no content type (Table 285's /CT) and names no player this program could \
             start (§13.2.7.2.2 condition d)"
                .to_owned(),
        );
    }
}

/// Why §13.2.7.2.1 treats a media players array's entry as not present, or `None` where it stays.
///
/// §13.2.7.3's own rules come first: Table 302's `/PID` is required, and of its `MH`, "Currently,
/// there are no defined entries for this dictionary", so by §13.2.2 — "If an entry that is
/// unrecognised by the viewer is in the MH dictionary, the object shall be considered non-viable"
/// — any entry there makes it non-viable. Then its software identifier, which §13.2.7.2.1 binds by
/// the same rule: "The same rule shall apply to a media player info dictionary that contains a
/// partially unrecognised software identifier dictionary." A required entry's absence is read as
/// non-viability, the reading this module gives Table 288's `/D` (ADR 2392).
fn player_info_set_aside(document: &Document, entry: &Object) -> Option<String> {
    let entry = document.resolve(entry);
    let Some(info) = entry.as_dict() else {
        return Some("it is not a media player info dictionary (Table 302)".to_owned());
    };
    let must = document.get_key(info, "MH");
    if let Some((name, _)) = must.as_dict().and_then(|must| must.iter().next()) {
        return Some(format!(
            "its MH holds /{}, and Table 302 defines no entry for that dictionary",
            String::from_utf8_lossy(name.as_bytes())
        ));
    }
    let identifier = document.get_key(info, "PID");
    let Some(identifier) = identifier.as_dict() else {
        return Some("it has no software identifier, which Table 302's /PID requires".to_owned());
    };
    software_identifier_set_aside(document, identifier)
}

/// Why a software identifier makes the media player info dictionary holding it non-viable, by
/// §13.2.7.4.1 and §13.2.7.4.4, or `None`.
///
/// §13.2.7.4.1: "If the dictionary contains keys that are unrecognised by the interactive PDF
/// processor, it shall be considered to be partially recognised", and §13.2.7.2.1 decides for this
/// context what the general clause leaves open. §13.2.7.4.4: "If a version array contains negative
/// numbers, it is non-viable, as is the enclosing software identifier." The comparison the arrays
/// feed is never reached here, because §13.2.7.4.2's first step needs a software name and the
/// hand-off has none (ADR 2376), so a negative number is the one thing in them that decides.
fn software_identifier_set_aside(document: &Document, identifier: &Dictionary) -> Option<String> {
    if let Some((name, _)) = identifier.iter().find(|(name, _)| {
        !matches!(
            name.as_bytes(),
            b"Type" | b"U" | b"L" | b"LI" | b"H" | b"HI" | b"OS"
        )
    }) {
        return Some(format!(
            "its software identifier holds /{}, which Table 303 does not define, so it is \
             partially recognised (§13.2.7.4.1)",
            String::from_utf8_lossy(name.as_bytes())
        ));
    }
    if identifier.get("U").is_none() {
        return Some("its software identifier has no /U, which Table 303 requires".to_owned());
    }
    for key in ["L", "H"] {
        let versions = document.get_key(identifier, key);
        let negative = versions.as_array().is_some_and(|versions| {
            versions.iter().any(|version| {
                document
                    .resolve(version)
                    .as_number()
                    .is_some_and(|version| version < 0.0)
            })
        });
        if negative {
            return Some(format!(
                "its software identifier's /{key} version array holds a negative number, which \
                 makes it non-viable (§13.2.7.4.4)"
            ));
        }
    }
    None
}

/// §13.2.4.3's "media types where an offset makes no sense (such as JPEG images)", by Table 285's
/// `/CT`: the registered still-image types whose format holds one picture and no timeline. A type
/// whose format can animate — `image/gif`, `image/webp`, `image/svg+xml` — is not one, and nor is
/// a type this list does not name, whose offsets keep their effect on viability (ADR 2392).
fn timeless(content_type: &str) -> bool {
    const STILL: [&str; 4] = ["image/jpeg", "image/png", "image/bmp", "image/jp2"];
    // RFC 2045 section 5.1: a type and subtype are matched case-insensitively, and parameters
    // follow `;`.
    let essence = content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim();
    STILL
        .iter()
        .any(|still| essence.eq_ignore_ascii_case(still))
}

/// The reason a section's `/B` or `/E` stating an unrecognised Table 296 or Table 300 `/S` gives,
/// from the `options` dictionary (`MH` or `BE`) that holds it.
fn unrecognised(options: &str, key: &str, table: &str) -> String {
    format!(
        "its media clip section's {options} /{key} states a {table} /S this program does not \
         recognise, and \"The rendition is non-viable if the interactive PDF processor does not \
         recognise the value of this entry\""
    )
}

/// What Table 300 makes of a §13.2.6.3 timespan dictionary.
enum Timespan {
    /// A simple timespan, in seconds: `/S` is `S` and `/V` a number that is not negative.
    Seconds(f64),
    /// A `/S` stated with a value this processor does not recognise, of which Table 300 says "The
    /// rendition is non-viable if the interactive PDF processor does not recognise the value of
    /// this entry".
    Unrecognised,
    /// A timespan no player is told: no dictionary, no `/S`, a negative `/V` ("\[n\]egative values
    /// shall not be used") or none at all.
    Unviable,
}

/// §13.2.6.3's timespan dictionary `timespan`, read by Table 300. It is the one reading a media
/// offset's `/T` (Table 297) and a media duration's `/T` (Table 292) share, so that the two give the
/// same answer for the same dictionary (ADR 2472).
///
/// > The value shall be S (simple timespan).
///
/// And of `/V`, "Non-integral values shall be allowed."
fn read_timespan(document: &Document, timespan: &Object) -> Timespan {
    let Some(timespan) = timespan.as_dict() else {
        return Timespan::Unviable;
    };
    let simple = match document.get_key(timespan, "S") {
        Object::Null => false,
        kind if kind.as_name().is_some_and(|kind| kind.as_bytes() == b"S") => true,
        _ => return Timespan::Unrecognised,
    };
    match document.get_key(timespan, "V").as_number() {
        Some(seconds) if simple && seconds.is_finite() && seconds >= 0.0 => {
            Timespan::Seconds(seconds)
        }
        _ => Timespan::Unviable,
    }
}

/// §13.2.6.2's media offset dictionary `offset`, read by Tables 296 to 300.
///
/// Of an unrecognised `/S` in either table, "\[t\]he rendition is non-viable if the interactive PDF
/// processor does not recognise the value of this entry"; Table 297 says "Negative timespans are
/// prohibited in this context", and Table 298 "\[f\]rame numbers begin at 0; negative frame numbers
/// are out of range".
fn read_offset(document: &Document, offset: &Object) -> Offset {
    let Some(offset) = offset.as_dict() else {
        return Offset::Unviable;
    };
    match document
        .get_key(offset, "S")
        .as_name()
        .map(pdf_syntax::Name::as_bytes)
    {
        Some(b"T") => match read_timespan(document, &document.get_key(offset, "T")) {
            Timespan::Seconds(seconds) => Offset::Time(seconds),
            Timespan::Unrecognised => Offset::Unrecognised("Table 300"),
            Timespan::Unviable => Offset::Unviable,
        },
        Some(b"F") => match document.get_key(offset, "F").as_integer() {
            Some(frame) if frame >= 0 => Offset::Frame(frame),
            _ => Offset::Unviable,
        },
        Some(b"M") => match document.get_key(offset, "M") {
            Object::String(bytes) => Offset::Marker(pdf_syntax::text_string(&bytes)),
            _ => Offset::Unviable,
        },
        // An absent `/S` is not a value the processor fails to recognise (ADR 2447).
        None if matches!(document.get_key(offset, "S"), Object::Null) => Offset::Unviable,
        _ => Offset::Unrecognised("Table 296"),
    }
}

/// Table 285's `/CT` of the media clip data `data`.
fn content_type(document: &Document, data: &Dictionary) -> Option<String> {
    match document.get_key(data, "CT") {
        Object::String(bytes) => Some(String::from_utf8_lossy(&bytes).into_owned()),
        _ => None,
    }
}

/// §14.9.2.4's search of a multi-language text array for the reader's language.
///
/// > When a PDF processor searches a multi-language text array to find text for a given language,
/// > it shall look for an exact (though caseinsensitive) match between the given language's
/// > identifier and the language identifiers in the array.
///
/// Then prefix matching "in increasing array order: a match shall be declared if the given
/// identifier is a leading, case-insensitive, substring of an identifier in the array, and the
/// first post-substring character in the array identifier is a hyphen", and last the default text,
/// the pair whose identifier is the empty string. With no language stated only the default answers.
#[must_use]
pub fn alternate(document: &Document, items: &[Object], language: Option<&str>) -> Option<String> {
    let pairs: Vec<(String, String)> = items
        .chunks_exact(2)
        .filter_map(|pair| {
            let [tag, text] = pair else { return None };
            match (document.resolve(tag), document.resolve(text)) {
                (Object::String(tag), Object::String(text)) => Some((
                    pdf_syntax::text_string(&tag),
                    pdf_syntax::text_string(&text),
                )),
                _ => None,
            }
        })
        .collect();
    if let Some(language) = language.filter(|language| !language.is_empty()) {
        if let Some((_, text)) = pairs
            .iter()
            .find(|(tag, _)| tag.eq_ignore_ascii_case(language))
        {
            return Some(text.clone());
        }
        let prefix = |tag: &str| {
            tag.len() > language.len()
                && tag.is_char_boundary(language.len())
                && tag[..language.len()].eq_ignore_ascii_case(language)
                && tag.as_bytes().get(language.len()) == Some(&b'-')
        };
        if let Some((_, text)) = pairs.iter().find(|(tag, _)| prefix(tag)) {
            return Some(text.clone());
        }
    }
    pairs
        .into_iter()
        .find(|(tag, _)| tag.is_empty())
        .map(|(_, text)| text)
}

/// A text string entry, decoded, where it is one and not empty.
fn text(document: &Document, dict: &Dictionary, key: &str) -> Option<String> {
    match document.get_key(dict, key) {
        Object::String(bytes) => {
            let text = pdf_syntax::text_string(&bytes);
            (!text.is_empty()).then_some(text)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{Data, Processor, TemporaryFile, alternate, judge};
    use pdf_syntax::{Document, Object, ObjectId};

    /// Builds a document from object bodies numbered from 1.
    fn document(objects: &[&str]) -> Document {
        use std::fmt::Write as _;
        let mut out = String::from("%PDF-2.0\n");
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
        }
        let xref_at = out.len();
        let _ = write!(
            out,
            "xref\n0 {}\n0000000000 65535 f \n",
            objects.len().saturating_add(1)
        );
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
            objects.len().saturating_add(1)
        );
        Document::open(out.into_bytes()).expect("a valid file")
    }

    fn rendition(number: u32) -> Object {
        Object::Reference(ObjectId {
            number,
            generation: 0,
        })
    }

    const EMBEDDED: &str = "<< /Type /Filespec /F (clip.mp4) /UF (clip.mp4) /EF << /F 4 0 R >> >>";
    const FILE: &str = "<< /Length 4 /Type /EmbeddedFile >>\nstream\nmoov\nendstream";

    /// §14.9.2.4's EXAMPLE, searched as the clause states: `en` finds `en-US` by prefix, `en-US`
    /// does not find `en`, and a language the array lacks gets the default text.
    #[test]
    fn a_multi_language_text_array_is_searched_as_the_clause_states() {
        let doc = document(&[
            "<< /Type /Catalog >>",
            "[(en-US) (My vacation) (fr) (mes vacances) () (default text)]",
        ]);
        let items = doc.get(ObjectId {
            number: 2,
            generation: 0,
        });
        let items = items.as_array().expect("an array");
        assert_eq!(
            alternate(&doc, items, Some("EN-us")).as_deref(),
            Some("My vacation")
        );
        assert_eq!(
            alternate(&doc, items, Some("en")).as_deref(),
            Some("My vacation")
        );
        assert_eq!(
            alternate(&doc, items, Some("fr")).as_deref(),
            Some("mes vacances")
        );
        // "given identifier en-US matches neither en nor en-GB": only the default is left.
        assert_eq!(
            alternate(&doc, items, Some("fr-CA")).as_deref(),
            Some("default text")
        );
        assert_eq!(
            alternate(&doc, items, None).as_deref(),
            Some("default text")
        );
    }

    /// An embedded clip with a content type, no options and no players is viable for the hand-off,
    /// and Table 286's `/TF` defaults to `TEMPNEVER`.
    #[test]
    fn an_embedded_clip_with_a_content_type_is_viable_and_never_permits_a_temporary_file() {
        let doc = document(&[
            "<< /Type /Catalog >>",
            "<< /S /MR /C 3 0 R >>",
            &format!("<< /S /MCD /CT (video/mp4) /D {EMBEDDED} /Alt [() (A clip)] >>"),
            FILE,
        ]);
        let report = judge(&doc, &rendition(2), Processor::default());
        let (considered, clip) = report.chosen().expect("viable");
        assert_eq!(considered.alternate.as_deref(), Some("A clip"));
        assert_eq!(clip.content_type.as_deref(), Some("video/mp4"));
        assert_eq!(clip.temporary_file, Some(TemporaryFile::Never));
        assert!(matches!(clip.data, Data::Embedded(_)), "{:?}", clip.data);
    }

    /// §13.2.3.3's depth-first search: the first viable media rendition is played, and a
    /// non-viable one before it is reported with its reasons.
    #[test]
    fn a_selector_plays_the_first_viable_rendition_and_says_why_the_others_were_passed() {
        let doc = document(&[
            "<< /Type /Catalog >>",
            "<< /S /SR /R [5 0 R 3 0 R] >>",
            &format!("<< /S /MR /N (small) /C << /S /MCD /CT (video/mp4) /D {EMBEDDED} >> >>"),
            FILE,
            // Table 279's /R needs a bandwidth, which this program does not measure.
            &format!(
                "<< /S /MR /N (large) /MH << /C << /R 1000000 >> >> /C << /S /MCD /CT (video/mp4) \
                 /D {EMBEDDED} >> >>"
            ),
        ]);
        let report = judge(&doc, &rendition(2), Processor::default());
        assert_eq!(report.considered.len(), 2, "{report:?}");
        assert_eq!(report.considered[0].name.as_deref(), Some("large"));
        assert!(report.considered[0].verdict.is_err());
        let (chosen, _) = report.chosen().expect("the second is viable");
        assert_eq!(chosen.name.as_deref(), Some("small"));
    }

    /// §13.2.2's rule on the two dictionaries, as its EXAMPLE states it for Table 291's `/V`: in
    /// `MH` a volume the player cannot be told makes the rendition non-viable; in `BE` it does not.
    #[test]
    fn an_option_must_honour_makes_the_rendition_non_viable_and_best_effort_does_not() {
        let clip = format!("<< /S /MCD /CT (audio/mpeg) /D {EMBEDDED} >>");
        let must = document(&[
            "<< /Type /Catalog >>",
            &format!("<< /S /MR /C {clip} /P << /MH << /V 50 >> >> >>"),
            "null",
            FILE,
        ]);
        let report = judge(&must, &rendition(2), Processor::default());
        assert!(report.chosen().is_none(), "{report:?}");
        let best = document(&[
            "<< /Type /Catalog >>",
            &format!("<< /S /MR /C {clip} /P << /BE << /V 50 >> >> >>"),
            "null",
            FILE,
        ]);
        assert!(
            judge(&best, &rendition(2), Processor::default())
                .chosen()
                .is_some()
        );
    }

    /// Table 286's rule for an unrecognised `/P` entry, and its own row's rule for an unrecognised
    /// `/TF` value.
    #[test]
    fn media_permissions_are_read_as_table_286_states() {
        let with = |permissions: &str| {
            let doc = document(&[
                "<< /Type /Catalog >>",
                &format!(
                    "<< /S /MR /C << /S /MCD /CT (video/mp4) /D {EMBEDDED} /P {permissions} >> >>"
                ),
                "null",
                FILE,
            ]);
            judge(&doc, &rendition(2), Processor::default())
        };
        let report = with("<< /TF (TEMPACCESS) >>");
        let (_, clip) = report.chosen().expect("viable");
        assert_eq!(clip.temporary_file, Some(TemporaryFile::Access));
        let report = with("<< /TF (TEMPSOMETIMES) >>");
        let (_, clip) = report.chosen().expect("viable");
        assert_eq!(clip.temporary_file, Some(TemporaryFile::Never));
        assert!(with("<< /Copy true >>").chosen().is_none());
    }

    /// §13.2.4.2: "The media clip data object is non-viable if the object referenced by the D
    /// entry does not contain a Type entry", which "shall effectively exclude the use of simple
    /// file specifications".
    #[test]
    fn a_simple_file_specification_makes_the_clip_non_viable() {
        let doc = document(&[
            "<< /Type /Catalog >>",
            "<< /S /MR /C << /S /MCD /CT (video/mp4) /D (clip.mp4) >> >>",
        ]);
        assert!(
            judge(&doc, &rendition(2), Processor::default())
                .chosen()
                .is_none()
        );
    }

    /// §13.2.7.2.2's condition (d): no `/MU`, no content type, and a player this program starts is
    /// found in no `/A`.
    #[test]
    fn a_clip_with_no_content_type_names_no_player_this_program_could_start() {
        let doc = document(&[
            "<< /Type /Catalog >>",
            &format!("<< /S /MR /C << /S /MCD /D {EMBEDDED} >> >>"),
            "null",
            FILE,
        ]);
        assert!(
            judge(&doc, &rendition(2), Processor::default())
                .chosen()
                .is_none()
        );
    }

    /// Table 279's `/L`, matched against the language a host stated, by its primary code too.
    #[test]
    fn a_language_criterion_is_met_by_the_stated_language_or_its_primary_code() {
        let doc = document(&[
            "<< /Type /Catalog >>",
            &format!(
                "<< /S /MR /BE << /C << /L [(de-AT)] >> >> /C << /S /MCD /CT (video/mp4) /D \
                 {EMBEDDED} >> >>"
            ),
            "null",
            FILE,
        ]);
        let judged = |language| {
            judge(&doc, &rendition(2), Processor { language })
                .chosen()
                .is_some()
        };
        assert!(judged(Some("de")));
        assert!(judged(Some("de-AT")));
        assert!(!judged(Some("fr")));
        assert!(!judged(None));
    }

    /// Table 285's form `XObject` clip is named rather than handed: its player is this processor.
    #[test]
    fn a_form_xobject_clip_is_reported_as_this_processors_to_play() {
        let doc = document(&[
            "<< /Type /Catalog >>",
            "<< /S /MR /C << /S /MCD /D 3 0 R >> >>",
            "<< /Type /XObject /Subtype /Form /BBox [0 0 1 1] /Length 0 >>\nstream\n\nendstream",
        ]);
        let report = judge(&doc, &rendition(2), Processor::default());
        let (_, clip) = report.chosen().expect("viable");
        assert!(matches!(clip.data, Data::Form));
    }

    /// A selector naming more renditions than the walk visits is said once, not once per rendition.
    #[test]
    fn a_tree_past_the_bound_is_said_once() {
        let items = "<< /S /MR >> ".repeat(super::MAX_RENDITIONS.saturating_add(10));
        let doc = document(&[
            "<< /Type /Catalog >>",
            &format!("<< /S /SR /R [{items}] >>"),
        ]);
        let report = judge(&doc, &rendition(2), Processor::default());
        let said = report
            .stopped
            .iter()
            .filter(|line| line.contains("the rest was not read"))
            .count();
        assert_eq!(said, 1, "{:?}", report.stopped);
    }

    /// A selector whose `/R` refers back to itself is visited once.
    #[test]
    fn a_selector_that_names_itself_ends() {
        let doc = document(&["<< /Type /Catalog >>", "<< /S /SR /R [2 0 R] >>"]);
        let report = judge(&doc, &rendition(2), Processor::default());
        assert!(report.chosen().is_none());
        assert!(!report.notes().is_empty());
    }
}
