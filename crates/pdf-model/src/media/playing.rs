//! How a sound is to be played, in ISO 32000-2 §13.2.5's terms: Table 291's volume, duration,
//! repeat count and auto-play, and Table 212's synchronous and mix flags beside them (ADR 3341).
//!
//! **One vocabulary for every source.** §13.2.5's media play parameters are the framework PDF 2.0
//! keeps, and §12.6.4.9's sound action is one of the deprecated objects it supersedes, so Table
//! 212's entries are read *into* Table 291's: `/Volume` becomes a level, `/Repeat` a repeat count of
//! zero ("Zero means repeat forever"). A player then honours one set of parameters whichever
//! object stated them. Today the only sounds this program plays itself are §13.3's sound objects,
//! which Table 212's action and §12.5.6.16's annotation reach and no rendition does — Table 285's
//! `/D` is "\[a] full file specification or form `XObject`", never a sound object — so Table 291's
//! own dictionaries are not yet read into this type (ADR 3341 says what does it).

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

/// How a sound is played: §13.2.5's play parameters, with each default the tables state.
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
}

impl Default for Playing {
    /// Each table's default: `/V` 100, `/D` intrinsic, `/RC` 1.0, `/A` true, `/Synchronous` and
    /// `/Mix` false.
    fn default() -> Self {
        Self {
            level: 1.0,
            duration: SimpleDuration::Intrinsic,
            repeat: 1.0,
            auto_play: true,
            synchronous: false,
            mix: false,
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
            }
            None => playing.synchronous = synchronous.unwrap_or(false),
        }
        playing.mix = flag("Mix", &mut notes).unwrap_or(false);
        (playing, notes)
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
