//! How a sound is to be played, in ISO 32000-2 §13.2.5's terms: Table 291's volume, duration,
//! repeat count and auto-play, and Table 212's synchronous and mix flags beside them (ADR 3341).
//!
//! **One vocabulary for every source.** §13.2.5's media play parameters are the framework PDF 2.0
//! keeps, and §12.6.4.9's sound action is one of the deprecated objects it supersedes, so Table
//! 212's entries are read *into* Table 291's: `/Volume` becomes a level, `/Repeat` a repeat count of
//! zero ("Zero means repeat forever"). A player then honours one set of parameters whichever
//! object stated them. Three sources reach this program's own player: Table 212's action and
//! §12.5.6.16's annotation, which carry §13.3's sound objects, and a media rendition whose clip is
//! an embedded RIFF WAVE file, whose Table 291 `MH` and `BE` dictionaries
//! [`Playing::of_play_parameters`] reads and judges (ADR 3350).

use pdf_syntax::{Dictionary, Document, Object};

/// Table 292's simple duration: what one iteration of a sound lasts.
///
/// §13.2.5: "The duration may be a specific amount of time, it may be infinity, or it may be the
/// media's intrinsic duration."
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SimpleDuration {
    /// Table 292's `/S /I`, and the default: the sound's own length.
    Intrinsic,
    /// Table 292's `/S /F`.
    Infinite,
    /// Table 292's `/S /T`: Table 300's `/V`, in seconds, never negative.
    Seconds(f64),
}

/// The part of a clip a chain of §13.2.4.3 media clip sections defines, in seconds from the start
/// of the media clip data it ends in (ADR 3398).
///
/// §13.2.4.3: "If the absolute offset derived from the values of all B entries in a media clip
/// section chain is greater than or equal to the absolute offset derived from the values of all E
/// entries, an empty range shall be defined. An empty range is valid."
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Section {
    /// Where the section begins, never negative; zero is the clip's start.
    pub begin: f64,
    /// Where it ends, never negative, or `None` for the clip's own end.
    pub end: Option<f64>,
}

impl Section {
    /// The whole clip: Table 289's defaults, "the start of the next-level media object" and "the
    /// end of the next-level media object".
    pub const WHOLE: Self = Self {
        begin: 0.0,
        end: None,
    };
}

/// Table 291's `/F`: "The manner in which the player shall treat a visual media type that does
/// not exactly fit the rectangle in which it plays" — each value named by the SMIL `fit` value
/// the table's notes give it.
///
/// Where a picture stands inside the rectangle, where it does not fill it or is cut by it, the
/// table does not say; the windows place it at the rectangle's top-left corner, SMIL's default
/// registration, a choice (ADR 3553).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Fit {
    /// `0`, SMIL's `meet`: "The media's width and height shall be scaled while preserving the
    /// aspect ratio so that the media and play rectangles have the greatest possible intersection
    /// while still displaying all media content."
    Meet,
    /// `1`, SMIL's `slice`: "The media's width and height shall be scaled while preserving the
    /// aspect ratio so that the play rectangle is entirely filled, and the amount of media content
    /// that does not fit within the play rectangle shall be minimised."
    Slice,
    /// `2`, SMIL's `fill`: "The media's width and height shall be scaled independently so that the
    /// media and play rectangles are the same; the aspect ratio shall not be preserved."
    Fill,
    /// `3`, SMIL's `scroll`: "The media shall not be scaled. A scrolling user interface shall be
    /// provided if the media rectangle is wider or taller than the play rectangle."
    Scroll,
    /// `4`, SMIL's `hidden`: "The media shall not be scaled. Only the portions of the media
    /// rectangle that intersect the play rectangle shall be displayed."
    Hidden,
    /// `5`, and the default: "Use the player's default setting (author has no preference)." This
    /// player's own setting is [`Fit::Meet`], a choice (ADR 3553).
    #[default]
    PlayersDefault,
}

impl Fit {
    /// Table 291's `/F` integer, or `None` for a value the table does not define.
    #[must_use]
    pub fn of_value(value: i64) -> Option<Self> {
        Some(match value {
            0 => Self::Meet,
            1 => Self::Slice,
            2 => Self::Fill,
            3 => Self::Scroll,
            4 => Self::Hidden,
            5 => Self::PlayersDefault,
            _ => return None,
        })
    }

    /// The integer Table 291 gives this fit.
    #[must_use]
    pub fn value(self) -> u8 {
        match self {
            Self::Meet => 0,
            Self::Slice => 1,
            Self::Fill => 2,
            Self::Scroll => 3,
            Self::Hidden => 4,
            Self::PlayersDefault => 5,
        }
    }
}

/// How a sound is played: §13.2.5's play parameters, with each default the tables state.
///
/// One field per table entry: Tables 291, 212 and 307 state `/A`, `/Synchronous`, `/Mix` and
/// `/ShowControls` as four independent flags with four independent defaults.
#[expect(
    clippy::struct_excessive_bools,
    reason = "one field per table entry; the tables state four independent flags"
)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Playing {
    /// Table 291's `/V` as a fraction of the recorded level: "the desired volume level as a
    /// percentage of recorded volume level. A zero value shall be equivalent to mute". Never
    /// negative; above 1 where a document asks for more than was recorded.
    pub level: f64,
    /// Table 291's `/D`.
    pub duration: SimpleDuration,
    /// Table 291's `/RC`: "the number of iterations of the duration D to repeat. … Zero means
    /// repeat forever." Never negative; non-integral values are permitted.
    pub repeat: f64,
    /// Table 291's `/A`: "If false , the media shall be initially paused when activated."
    pub auto_play: bool,
    /// Table 212's `/Synchronous`: "If this flag is true , the interactive PDF processor retains
    /// control, allowing no further user interaction other than cancelling the sound, until the
    /// sound has been completely played."
    pub synchronous: bool,
    /// Table 212's `/Mix`: "A flag specifying whether to mix this sound with any other sound
    /// already playing."
    pub mix: bool,
    /// The part of the clip a chain of media clip sections defines (§13.2.4.3); the whole of it
    /// for a sound object and for media clip data.
    pub section: Section,
    /// Table 291's `/F`, which only a clip with pictures is fitted by.
    pub fit: Fit,
    /// Table 307's `/ShowControls`, "whether to display a movie controller bar while playing the
    /// movie", and Table 291's `/C`, "whether to display a player-specific controller user
    /// interface when playing": a bar this program draws over the clip's pictures (ADRs 3594,
    /// 3610).
    pub controls: bool,
}

impl Default for Playing {
    /// Each table's default: `/V` 100, `/D` intrinsic, `/RC` 1.0, `/A` true, `/F` 5,
    /// `/Synchronous`, `/Mix` and `/ShowControls` false.
    fn default() -> Self {
        Self {
            level: 1.0,
            duration: SimpleDuration::Intrinsic,
            repeat: 1.0,
            auto_play: true,
            synchronous: false,
            mix: false,
            section: Section::WHOLE,
            fit: Fit::PlayersDefault,
            controls: false,
        }
    }
}

impl Playing {
    /// Whether the sound never ends by itself: a repeat count of zero, or an infinite duration.
    #[must_use]
    pub fn endless(&self) -> bool {
        self.repeat == 0.0 || matches!(self.duration, SimpleDuration::Infinite)
    }

    /// ISO 32000-2 §12.6.4.9's Table 212, read into §13.2.5's terms, with a sentence for each entry
    /// that could not be read as the table states it (trap 5).
    ///
    /// `/Volume` is "in the range -1.0 to 1.0. Default value: 1.0", and the table says no more
    /// about what a number in that range means. **This reading is a choice** (ADR 3341): the level
    /// is the number itself, as Table 291's `/V` is a fraction of the recorded level, and the half
    /// below zero — which no sentence gives a meaning, and which as a scale on samples would only
    /// invert their sign — is silence, said in a note. A number outside the range is held to it.
    ///
    /// `/Repeat` decides `/Synchronous` by its presence, not its value: "If this entry is present,
    /// the Synchronous entry shall be ignored."
    #[must_use]
    pub fn of_sound_action(document: &Document, action: &Dictionary) -> (Self, Vec<String>) {
        let mut playing = Self::default();
        let mut notes = Vec::new();
        match document.get_key(action, "Volume") {
            Object::Null => {}
            stated => match stated.as_number() {
                Some(volume) if volume.is_finite() => {
                    let held = volume.clamp(-1.0, 1.0);
                    if !(-1.0..=1.0).contains(&volume) {
                        notes.push(format!(
                            "Table 212's /Volume {volume} is outside the range -1.0 to 1.0, so it \
                             is played at {held}"
                        ));
                    }
                    if held < 0.0 {
                        notes.push(format!(
                            "Table 212's /Volume {held} is below zero, which the table gives no \
                             meaning, so the sound is played silent"
                        ));
                    }
                    playing.level = held.max(0.0);
                }
                _ => notes.push(
                    "Table 212's /Volume is not a number, so its default 1.0 stands".to_owned(),
                ),
            },
        }
        let flag = |key: &str, notes: &mut Vec<String>| match document.get_key(action, key) {
            Object::Null => None,
            Object::Boolean(value) => Some(value),
            _ => {
                notes.push(format!(
                    "Table 212's /{key} is not a boolean, so its default false stands"
                ));
                Some(false)
            }
        };
        let repeat = flag("Repeat", &mut notes);
        let synchronous = flag("Synchronous", &mut notes);
        match repeat {
            Some(repeat) => {
                if repeat {
                    playing.repeat = 0.0;
                }
                if synchronous.is_some() {
                    notes.push(
                        "Table 212's /Synchronous is ignored, because \"If this entry is present, \
                         the Synchronous entry shall be ignored\" of its /Repeat"
                            .to_owned(),
                    );
                }
                // not a repair: ISO 32000-2 §12.6.4.9 (Table 212) defaults an absent /Synchronous and
                // /Mix to false, and a wrong type is noted above.
            }
            None => playing.synchronous = synchronous.unwrap_or(false),
        }
        playing.mix = flag("Mix", &mut notes).unwrap_or(false);
        (playing, notes)
    }
}

/// Table 291's `MH` and `BE`, read for this program's own player: how it plays, and what it
/// cannot honour (ADR 3350).
#[derive(Debug, Clone, PartialEq)]
pub struct Honoured {
    /// The parameters, `MH`'s instance of each entry over `BE`'s.
    pub playing: Playing,
    /// Each `MH` entry this player cannot honour, which §13.2.2 makes the rendition non-viable for:
    /// "The options specified by this entry shall be honoured; otherwise, the containing object
    /// shall be considered non-viable."
    pub must: Vec<String>,
    /// Each `BE` entry set aside, said (trap 5): "if they cannot be honoured, the containing object
    /// is still considered viable".
    pub set_aside: Vec<String>,
    /// Whether `/F` was read from `MH`, which decides whether a fit the windows cannot honour
    /// makes a clip with pictures non-viable or is set aside (ADR 3553).
    pub fit_must: bool,
    /// Whether `/C` was read from `MH`, which decides whether a controller asked of a clip with no
    /// pictures to draw it over makes the clip non-viable or is set aside (ADR 3610).
    pub controls_must: bool,
}

impl Playing {
    /// ISO 32000-2 §13.2.5's media play parameters `parameters`, read for this program's own
    /// player — the one that plays an embedded RIFF WAVE clip in the reader's window.
    ///
    /// §13.2.2's rules for the two dictionaries: "If an instance of the same entry is present in
    /// both MH and BE, the instance in the BE dictionary shall be ignored", an entry in neither is
    /// "present with its default value … in the BE dictionary", and "If an entry that is
    /// unrecognised by the viewer is in the MH dictionary, the object shall be considered
    /// non-viable. If an unrecognised entry is in a BE dictionary, the entry shall be ignored".
    ///
    /// What this player honours of Table 291, each argued in ADR 3350: `/V` as a level, `/D`'s
    /// three durations, `/RC` and `/A` (held at its start, and started by the window's resume
    /// control); `/F`, read into [`Playing::fit`] — every value for an aural clip, because the
    /// entry is about "a visual media type that does not exactly fit the rectangle in which it
    /// plays" and a sound is fitted to nothing, and every value but 3 for a clip with pictures
    /// (ADR 3553); and `/C`, "whether to display a player-specific controller user interface when
    /// playing", read into [`Playing::controls`] — the controller bar a window draws over a clip's
    /// pictures, which a clip without pictures has nowhere to show (ADR 3610).
    #[must_use]
    pub fn of_play_parameters(document: &Document, parameters: &Dictionary) -> Honoured {
        let mut honoured = Honoured {
            playing: Self::default(),
            must: Vec::new(),
            set_aside: Vec::new(),
            fit_must: false,
            controls_must: false,
        };
        let must = document.get_key(parameters, "MH");
        let must = must.as_dict();
        let best = document.get_key(parameters, "BE");
        for (options, is_must) in [(must, true), (best.as_dict(), false)] {
            let Some(options) = options else {
                continue;
            };
            for (name, value) in options.iter() {
                let key = String::from_utf8_lossy(name.as_bytes()).into_owned();
                if !is_must && must.is_some_and(|must| must.get(&key).is_some()) {
                    continue;
                }
                let value = document.resolve(value);
                let answer = match key.as_str() {
                    "V" | "D" | "RC" | "A" | "C" | "F" => {
                        honoured.playing.honour(document, &key, &value)
                    }
                    _ if is_must => Err(format!(
                        "Table 291 does not define /{key}, and §13.2.2 makes an MH holding an \
                         unrecognised entry non-viable"
                    )),
                    _ => Ok(()),
                };
                if is_must && key == "F" && answer.is_ok() {
                    honoured.fit_must = true;
                }
                if is_must && key == "C" && answer.is_ok() {
                    honoured.controls_must = true;
                }
                if let Err(why) = answer {
                    if is_must {
                        honoured.must.push(format!(
                            "it must be played with Table 291's /{key} honoured, and this \
                             program's player cannot: {why}"
                        ));
                    } else {
                        honoured
                            .set_aside
                            .push(format!("Table 291's best-effort /{key}, because {why}"));
                    }
                }
            }
        }
        honoured
    }

    /// One Table 291 entry honoured, or why this player cannot.
    fn honour(&mut self, document: &Document, key: &str, value: &Object) -> Result<(), String> {
        let number = || value.as_number().filter(|number| number.is_finite());
        let flag = || match value {
            Object::Boolean(flag) => Ok(*flag),
            _ => Err("its value is not a boolean".to_owned()),
        };
        match key {
            "V" => match number() {
                // "A zero value shall be equivalent to mute; negative values shall not be
                // permitted."
                Some(percent) if percent >= 0.0 => self.level = percent / 100.0,
                _ => return Err("its value is not a volume of zero or more".to_owned()),
            },
            "RC" => match number() {
                // "Negative values shall be invalid; non-integral values shall be permitted."
                Some(count) if count >= 0.0 => self.repeat = count,
                _ => return Err("its value is not a repeat count of zero or more".to_owned()),
            },
            "A" => self.auto_play = flag()?,
            // Table 291's `/C`, "whether to display a player-specific controller user interface
            // when playing": the controller bar over a clip's pictures (ADR 3610), which a clip
            // without pictures is judged against where that is known (`uncontrolled`).
            "C" => self.controls = flag()?,
            "F" => match value {
                // Table 291's six fits, each of "a visual media type": a sound has no rectangle to
                // fit and honours every one by playing; a clip's pictures are fitted by the window,
                // and the one fit its windows cannot honour, `/F 3`'s scrolling interface, is
                // judged where the clip is known to have pictures (`Fit::Scroll`, ADR 3553).
                Object::Integer(stated) => {
                    self.fit = Fit::of_value(*stated).ok_or_else(|| {
                        format!(
                            "its value {stated} is none of the fits 0 to 5, which a BE \
                             dictionary treats as the default and an MH dictionary as \
                             non-viable"
                        )
                    })?;
                }
                _ => return Err("its value is not an integer".to_owned()),
            },
            "D" => self.duration = duration(document, value)?,
            _ => return Err("Table 291 does not define it".to_owned()),
        }
        Ok(())
    }
}

/// Table 292's media duration `value`, as a simple duration, or why it is none this player knows.
fn duration(document: &Document, value: &Object) -> Result<SimpleDuration, String> {
    let Some(duration) = value.as_dict() else {
        return Err("its value is not a media duration dictionary (Table 292)".to_owned());
    };
    // Table 292's `/S`: "The media duration dictionary is non-viable if the interactive PDF
    // processor does not recognise the value of this entry."
    match document
        .get_key(duration, "S")
        .as_name()
        .map(pdf_syntax::Name::as_bytes)
    {
        Some(b"I") => Ok(SimpleDuration::Intrinsic),
        Some(b"F") => Ok(SimpleDuration::Infinite),
        Some(b"T") => match super::read_timespan(document, &document.get_key(duration, "T")) {
            super::Timespan::Seconds(seconds) => Ok(SimpleDuration::Seconds(seconds)),
            super::Timespan::Unrecognised | super::Timespan::Unviable => Err(
                "its explicit duration is not a Table 300 timespan of zero seconds or more"
                    .to_owned(),
            ),
        },
        _ => Err("its Table 292 /S is not I, F or T".to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Table 212's action holding `entries`, read, from a file whose object 2 it is.
    fn read(entries: &str) -> (Playing, Vec<String>) {
        use std::fmt::Write as _;
        let mut source = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for (number, body) in [
            "<< /Type /Catalog >>".to_owned(),
            format!("<< /S /Sound {entries} >>"),
        ]
        .iter()
        .enumerate()
        {
            offsets.push(source.len());
            let _ = write!(
                source,
                "{} 0 obj\n{body}\nendobj\n",
                number.saturating_add(1)
            );
        }
        let xref_at = source.len();
        source.push_str("xref\n0 3\n0000000000 65535 f \n");
        for offset in offsets {
            let _ = writeln!(source, "{offset:010} 00000 n ");
        }
        let _ = write!(
            source,
            "trailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
        );
        let document = Document::open(source.into_bytes()).expect("a valid file");
        let dict = document
            .get(pdf_syntax::ObjectId::new(2, 0))
            .as_dict()
            .cloned()
            .expect("object 2 is the action");
        Playing::of_sound_action(&document, &dict)
    }

    /// Table 212's defaults are Table 291's: level 1.0, played once, asynchronously, unmixed.
    #[test]
    fn an_action_stating_nothing_plays_at_every_default() {
        let (playing, notes) = read("");
        assert_eq!(playing, Playing::default());
        assert!(notes.is_empty(), "{notes:?}");
    }

    /// `/Volume 0.5` is half the recorded level; `/Repeat true` is a repeat count of zero, which
    /// Table 291 calls forever; and `/Repeat`'s presence sets `/Synchronous` aside, with a note.
    #[test]
    fn volume_is_a_level_and_repeat_is_forever_over_synchronous() {
        let (playing, notes) = read("/Volume 0.5 /Repeat true /Synchronous true /Mix true");
        assert!((playing.level - 0.5).abs() < f64::EPSILON);
        assert!(playing.repeat == 0.0 && playing.endless());
        assert!(!playing.synchronous);
        assert!(playing.mix);
        assert_eq!(notes.len(), 1, "{notes:?}");
        assert!(notes[0].contains("/Synchronous is ignored"), "{notes:?}");
    }

    /// A `/Repeat false` is present too, so it sets `/Synchronous` aside as `true` would.
    #[test]
    fn a_false_repeat_still_sets_synchronous_aside() {
        let (playing, _) = read("/Repeat false /Synchronous true");
        assert!(!playing.synchronous);
        assert!((playing.repeat - 1.0).abs() < f64::EPSILON);
        let (playing, notes) = read("/Synchronous true");
        assert!(playing.synchronous && notes.is_empty(), "{notes:?}");
    }

    /// Below zero is silence and said; past the range is held to it and said; a name is not a
    /// number and leaves the default.
    #[test]
    fn a_volume_outside_its_sense_is_said() {
        let (playing, notes) = read("/Volume -0.5");
        assert!(playing.level == 0.0 && notes.len() == 1, "{notes:?}");
        let (playing, notes) = read("/Volume 3");
        assert!((playing.level - 1.0).abs() < f64::EPSILON && notes.len() == 1);
        let (playing, notes) = read("/Volume /Loud /Mix 1");
        assert!((playing.level - 1.0).abs() < f64::EPSILON);
        assert_eq!(notes.len(), 2, "{notes:?}");
    }
}
