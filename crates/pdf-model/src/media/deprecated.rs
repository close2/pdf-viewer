//! §13.3's sounds and §13.4's movies, the two media objects PDF 2.0 deprecates, as more sources for
//! the hand-off a rendition's clip uses.
//!
//! Both clauses open with the sentence that places them:
//!
//! > The features described in this subclause are deprecated with PDF 2.0. They are superseded by
//! > the general multimedia framework described in 13.2, "Multimedia".
//!
//! and `CLAUDE.md`'s clause 13 entry admits what that framework's processor owes a reader without a
//! media engine: the media file handed to this machine's own player at the reader's level, nothing
//! played in-process. A movie's Table 306 `/F` and a sound's Table 5 `/F` each name such a file — a
//! "self-describing" one, in both clauses' words — so each becomes a [`Clip`] the way a viable
//! §13.2.4.2 clip does, and crosses through the same event (`doc/questions/A631`, ADR 2382).
//!
//! **A movie this program shows itself** — an embedded MP4 file of `video/` type, for a host whose
//! player shows video (ADR 3594) — is played in the movie annotation's rectangle, or in a floating
//! window where Table 307's `/FWScale` asks for one, and is told the activation entries this
//! player honours: `/Start` and `/Duration` as §13.2.4.3's section, `/Volume`, `/ShowControls`,
//! `/Mode`'s `Once`, `Open`, `Repeat` and `Palindrome` (ADR 3842), `/FWScale` and `/FWPosition` —
//! and Table 306's `/Rotate`, the pictures and the poster turned clockwise by it (ADR 3817), and a
//! negative `/Rate`'s pictures and sound played backward (ADRs 3837, 3841). Those it cannot honour
//! are said, as for the hand-off.
//!
//! **What the hand-off cannot carry is said, never dropped** (trap 5): Table 307's activation
//! entries and Table 212's playing parameters are each something a player would have to be *told*,
//! and a player handed only the file is told nothing, so every one the document states becomes a
//! sentence in [`Clip::unhonoured`]. §13.2.2's viability is not asked of either — it is a property
//! of §13.2's renditions, and neither object is one.

use pdf_syntax::{Dictionary, Document, Object};

use super::{
    Background, Clip, Data, Floating, MonitorSpecifier, Offscreen, Players, Playing, Position,
    Processor, Relative, Resize, Section, Window, WindowKind,
};
use crate::file_spec::FileSpec;

/// Table 307's entries, in the table's order: how a movie is to be presented, which a player
/// handed only the file is told nothing of.
const ACTIVATION: [&str; 9] = [
    "Start",
    "Duration",
    "Rate",
    "Volume",
    "ShowControls",
    "Mode",
    "Synchronous",
    "FWScale",
    "FWPosition",
];

/// The clip a movie annotation's movie is handed over as, or the sentence saying why there is none.
///
/// `action` is the movie action where one asked for the movie, `None` where §12.5.6.17's annotation
/// was activated itself. ISO 32000-2 §12.6.4.10 makes the action's entries the stronger statement:
///
/// > The contents of the activation dictionary associated with the movie annotation provide the
/// > default values. Any information specified in the movie action dictionary overrides these
/// > values.
///
/// and Table 189's `/A` `false` — "the movie shall not be played" — is about the annotation's own
/// activation, so it is read only where no action asked. Where the movie's file is embedded, Table
/// 44's `/Subtype` is the content type, because Table 306 states none: "\[t]he format of a
/// self-describing movie file shall be left unspecified".
///
/// # Errors
///
/// The sentence a person is told: an annotation whose `/A` is `false`, one with no movie
/// dictionary, and a movie whose `/F` is not a file specification.
pub fn movie(
    document: &Document,
    annotation: &Dictionary,
    action: Option<&Dictionary>,
    processor: Processor<'_>,
) -> Result<Clip, String> {
    let activation = document.get_key(annotation, "A");
    if action.is_none() && matches!(activation, Object::Boolean(false)) {
        return Err(
            "Table 189's /A is false, so §12.5.6.17 says the movie is not played when the \
             annotation is activated"
                .to_owned(),
        );
    }
    movie_clip(document, annotation, action, processor)
}

/// The movie a movie annotation's Table 306 `/Poster` `true` retrieves its poster from — "the
/// poster image shall be retrieved from the movie file" — as [`movie`] would hand it over: `None`
/// where `/Poster` is not `true`, the clip otherwise, whose [`Clip::played_here`] says whether a
/// window that shows video decodes its first picture (ADR 3609). Table 189's `/A` is about the
/// annotation's activation and not about what its page shows, so it is not read.
///
/// # Errors
///
/// [`movie`]'s sentences for a movie dictionary or a file specification that is not there.
pub fn poster_movie(
    document: &Document,
    annotation: &Dictionary,
    processor: Processor<'_>,
) -> Option<Result<Clip, String>> {
    let movie = document.get_key(annotation, "Movie");
    let poster = document.get_key(movie.as_dict()?, "Poster");
    matches!(poster, Object::Boolean(true))
        .then(|| movie_clip(document, annotation, None, processor))
}

/// [`movie`] after its activation's `/A`.
fn movie_clip(
    document: &Document,
    annotation: &Dictionary,
    action: Option<&Dictionary>,
    processor: Processor<'_>,
) -> Result<Clip, String> {
    let activation = document.get_key(annotation, "A");
    let movie = document.get_key(annotation, "Movie");
    let Some(movie) = movie.as_dict() else {
        return Err(
            "Table 189 makes /Movie required and this annotation states no movie dictionary"
                .to_owned(),
        );
    };
    let Some(data) = file(document, &document.get_key(movie, "F"), "movie") else {
        return Err(
            "Table 306's required /F is not a file specification this reader can read (§7.11)"
                .to_owned(),
        );
    };
    let mut unhonoured: Vec<String> = ACTIVATION
        .iter()
        .filter(|key| {
            activation
                .as_dict()
                .is_some_and(|dict| !matches!(document.get_key(dict, key), Object::Null))
                || action.is_some_and(|dict| !matches!(document.get_key(dict, key), Object::Null))
        })
        .map(|key| format!("Table 307's /{key}"))
        .collect();
    // Table 306's /Aspect describes the movie and asks nothing of a player; /Rotate asks for the
    // picture to be turned, which a player handed the file is not told, and which this program's
    // own player is (below).
    let rotate = document
        .get_key(movie, "Rotate")
        .as_integer()
        .filter(|degrees| *degrees != 0);
    if let Some(degrees) = rotate {
        unhonoured.push(format!("Table 306's /Rotate {degrees}"));
    }
    let mut clip = Clip {
        content_type: content_type(&data),
        data,
        temporary_file: None,
        unhonoured,
        playing: Playing::default(),
        played_here: false,
        background: None,
        window: None,
    };
    let shows_it = processor.players == Players::AlsoVideoHere
        && matches!(clip.data, Data::Embedded(_))
        && clip
            .content_type
            .as_deref()
            .is_some_and(super::container::is_mp4_video);
    if shows_it {
        let stated = |key: &str| {
            action
                .map(|action| document.get_key(action, key))
                .filter(|value| !matches!(value, Object::Null))
                .or_else(|| {
                    activation
                        .as_dict()
                        .map(|activation| document.get_key(activation, key))
                })
                // not a repair: an entry neither dictionary states is absent, and the reading below
                // applies Table 307's default for it.
                .unwrap_or(Object::Null)
        };
        let told = told(document, movie, &clip.data, &stated, processor);
        // This player is told Table 307; what it sets aside, each entry says itself.
        clip.unhonoured
            .retain(|entry| !entry.starts_with("Table 307's /"));
        clip.unhonoured.extend(told.set_aside);
        clip.playing = told.playing;
        if let Some(degrees) = rotate {
            clip.unhonoured
                .retain(|entry| !entry.starts_with("Table 306's /Rotate"));
            match quarter_turns(degrees) {
                Some(turns) => clip.playing.quarter_turns = turns,
                None => clip.unhonoured.push(format!(
                    "Table 306's /Rotate {degrees}, because the table says the value shall be a \
                     multiple of 90, so the pictures stand at the page's orientation"
                )),
            }
        }
        clip.window = told.window;
        clip.background = Some(Background::default());
        clip.played_here = true;
    }
    Ok(clip)
}

/// Table 306's `/Rotate` as clockwise quarter turns from 0 to 3: "The number of degrees by which
/// the movie shall be rotated clockwise relative to the page. The value shall be a multiple of 90."
/// A negative multiple turns anticlockwise, which is the same as the clockwise turn that completes
/// it to a whole turn. `None` for a value that is no multiple of 90.
fn quarter_turns(degrees: i64) -> Option<u8> {
    if degrees % 90 != 0 {
        return None;
    }
    Some(match (degrees / 90).rem_euclid(4) {
        0 => 0,
        1 => 1,
        2 => 2,
        _ => 3,
    })
}

/// What this program's own player is told of a movie it shows: the play parameters, the floating
/// window `/FWScale` asks for, and a sentence for each of Table 307's entries it does not honour,
/// or honours only in part (ADR 3594).
struct Told {
    playing: Playing,
    window: Option<Window>,
    set_aside: Vec<String>,
}

/// Table 307's entries read for this program's player, each looked up by `stated` — the movie
/// action's value where it states one, the activation dictionary's otherwise (§12.6.4.10).
///
/// - `/Start` and `/Duration`, "the starting time" and "the duration of the movie segment to be
///   played", become §13.2.4.3's section, counted in the movie's own time scale (its `mvhd` box)
///   unless an array names another.
/// - `/Volume` is "in the range -1.0 to 1.0 … negative values shall mute the sound": the level is
///   the number from zero up, and silence below it — the reading Table 212's `/Volume` has (ADR
///   3341).
/// - `/Mode`: `Once` plays one pass; `Repeat` plays "repeatedly from beginning to end until
///   stopped", a repeat count of zero; `Open` plays "and leave\[s] the movie controller bar open",
///   which this player reads as the last picture held, with the bar where one is shown, until the
///   movie is stopped — an infinite duration. `Palindrome` is "Play continuously forward and
///   backward until stopped": a repeat count of zero, each pass in the direction opposite to the
///   one before it, the first in `/Rate`'s (ADR 3842).
/// - `/Rate` negative — "the movie shall be played backward with respect to Start and Duration" —
///   plays the pictures backward, from the section's end down to its start (ADR 3837), and the
///   sound backward with them on the same clock (ADR 3841).
/// - `/Rate` of a size other than 1 — "the initial speed" — is said and the movie played at its own
///   speed: the pictures follow the sound's device clock (ADR 3555), which runs at the recording's
///   rate.
/// - `/Synchronous` true — "the movie player shall retain control until the movie is completed or
///   dismissed by the user" — is [`Playing::synchronous`]: the window's players hold the reader
///   while the movie plays, taking no key or click but Escape, which dismisses it (ADR 3853).
/// - `/FWScale` "implies that the movie shall be played in a floating window" of `(numerator ÷
///   denominator) × Aspect` pixels, placed by `/FWPosition` on the screen.
fn told(
    document: &Document,
    movie: &Dictionary,
    data: &Data,
    stated: &dyn Fn(&str) -> Object,
    processor: Processor<'_>,
) -> Told {
    let mut told = Told {
        playing: Playing::default(),
        window: None,
        set_aside: Vec::new(),
    };
    let start = stated("Start");
    let duration = stated("Duration");
    if !matches!((&start, &duration), (Object::Null, Object::Null)) {
        match section(document, data, &start, &duration) {
            Ok(section) => told.playing.section = section,
            Err(why) => told
                .set_aside
                .push(format!("Table 307's /Start and /Duration, because {why}")),
        }
    }
    match stated("Volume") {
        Object::Null => {}
        value => match value.as_number().filter(|volume| volume.is_finite()) {
            // not a repair: Table 307's /Volume is "in the range -1.0 to 1.0" and "negative values
            // shall mute the sound"; a number past the range is held to it, the reading Table 212's
            // /Volume has (ADR 3341).
            Some(volume) => told.playing.level = volume.clamp(0.0, 1.0),
            None => told
                .set_aside
                .push("Table 307's /Volume, because its value is not a number".to_owned()),
        },
    }
    match stated("ShowControls") {
        Object::Boolean(flag) => told.playing.controls = flag,
        Object::Null => {}
        _ => told
            .set_aside
            .push("Table 307's /ShowControls, because its value is not a boolean".to_owned()),
    }
    let mode = stated("Mode");
    match mode.as_name().map(pdf_syntax::Name::as_bytes) {
        None if matches!(mode, Object::Null) => {}
        Some(b"Once") => {}
        Some(b"Repeat") => told.playing.repeat = 0.0,
        Some(b"Open") => told.playing.duration = super::SimpleDuration::Infinite,
        Some(b"Palindrome") => {
            told.playing.repeat = 0.0;
            told.playing.palindrome = true;
        }
        _ => told.set_aside.push(
            "Table 307's /Mode, because it is none of Once, Open, Repeat and Palindrome, so the \
             movie plays once"
                .to_owned(),
        ),
    }
    if let Some(rate) = stated("Rate").as_number().filter(|rate| rate.is_finite()) {
        // A negative rate is "the movie shall be played backward with respect to Start and
        // Duration": the worker decodes each group of pictures from its sync sample and lets it
        // out last picture first (ADR 3837), and the sound player plays the section's sound
        // reversed on the same clock (ADR 3841).
        if rate < 0.0 {
            told.playing.backward = true;
        }
        if (rate.abs() - 1.0).abs() >= f64::EPSILON {
            told.set_aside.push(format!(
                "Table 307's /Rate {rate}, because this program's player plays a movie at its \
                 recorded speed, its pictures following the sound device's clock"
            ));
        }
    }
    match stated("Synchronous") {
        Object::Boolean(flag) => told.playing.synchronous = flag,
        Object::Null => {}
        _ => told
            .set_aside
            .push("Table 307's /Synchronous, because its value is not a boolean".to_owned()),
    }
    match floating(document, movie, stated, processor) {
        Ok((window, nearest)) => {
            told.window = window;
            told.set_aside.extend(nearest);
        }
        Err(why) => told.set_aside.push(why),
    }
    told
}

/// Table 307's `/Start` and `/Duration` as §13.2.4.3's section, in seconds.
fn section(
    document: &Document,
    data: &Data,
    start: &Object,
    duration: &Object,
) -> Result<Section, String> {
    let mut scale = None;
    let mut seconds = |value: &Object, what: &str| -> Result<Option<f64>, String> {
        if matches!(value, Object::Null) {
            return Ok(None);
        }
        let (time, stated_scale) = time_and_scale(document, value)
            .ok_or_else(|| format!("its {what} is not a time in any of the table's three forms"))?;
        let units = if let Some(units) = stated_scale {
            units
        } else {
            if scale.is_none() {
                scale = Some(movie_scale(document, data)?);
            }
            // not a repair: `scale` was set on the line above.
            scale.unwrap_or(1)
        };
        #[expect(
            clippy::cast_precision_loss,
            reason = "a time past 2⁵³ units is past any movie a person watches; the seconds it \
                      rounds to are as near as an f64 holds"
        )]
        let at = time as f64 / f64::from(units);
        Ok(Some(at))
    };
    // not a repair: "If this entry is omitted, the movie shall be played from the beginning" (Table
    // 307's /Start).
    let begin = seconds(start, "/Start")?.unwrap_or(0.0);
    let length = seconds(duration, "/Duration")?;
    Ok(Section {
        begin,
        end: length.map(|length| begin + length),
    })
}

/// One of Table 307's time forms: an integer; an eight-byte string, "a 64-bit twos-complement
/// integer, most significant byte first"; or an array of either and "an integer specifying the
/// time scale in units per second". A negative time is none: "nominally a non-negative 64-bit
/// integer".
fn time_and_scale(document: &Document, value: &Object) -> Option<(u64, Option<u32>)> {
    let time = |value: &Object| -> Option<u64> {
        match value {
            // not a repair: a negative time or a string not of eight bytes is none of Table 307's
            // forms; the caller sets /Start and /Duration aside and says so.
            Object::Integer(time) => u64::try_from(*time).ok(),
            Object::String(bytes) => {
                let bytes: [u8; 8] = (**bytes).try_into().ok()?;
                u64::try_from(i64::from_be_bytes(bytes)).ok()
            }
            _ => None,
        }
    };
    match value {
        Object::Array(items) => match items.as_slice() {
            [stated, units] => {
                let units = document
                    .resolve(units)
                    .as_integer()
                    // not a repair: a scale that is not a positive integer counts no time; the
                    // caller sets the entry aside and says so.
                    .and_then(|units| u32::try_from(units).ok())
                    .filter(|units| *units > 0)?;
                Some((time(&document.resolve(stated))?, Some(units)))
            }
            _ => None,
        },
        other => Some((time(other)?, None)),
    }
}

/// The movie's own time scale, read from its file's `mvhd` box.
fn movie_scale(document: &Document, data: &Data) -> Result<u32, String> {
    let Data::Embedded(file) = data else {
        return Err(
            "the movie's file is not embedded, so its time scale cannot be read".to_owned(),
        );
    };
    let bytes = document.decoded_stream_data(&file.stream).ok_or_else(|| {
        "the movie's file states a filter or an encryption this program cannot undo".to_owned()
    })?;
    super::container::movie_time_scale(&bytes)
}

/// Table 307's floating window: `None` where `/FWScale` is absent — "the movie shall be played in
/// the annotation rectangle" — and the window otherwise, `(numerator ÷ denominator) × Aspect`
/// pixels, centred where `/FWPosition` places it on the screen holding the document window.
///
/// `/FWPosition` is "relative" in each axis from 0.0 to 1.0, and Table 295's `/P` — which the
/// host's windows are placed by (ADR 3586) — names the nine points where each is 0, 0.5 or 1; a
/// position between them is placed at the nearest and said.
///
/// Answers the sentence for a position placed at the nearest point beside the window.
///
/// # Errors
///
/// The sentence for a window this host does not open, an `/FWScale` that is not two positive
/// integers, and a movie that states no `/Aspect` to scale.
fn floating(
    document: &Document,
    movie: &Dictionary,
    stated: &dyn Fn(&str) -> Object,
    processor: Processor<'_>,
) -> Result<(Option<Window>, Option<String>), String> {
    let scale = stated("FWScale");
    if matches!(scale, Object::Null) {
        return Ok((None, None));
    }
    if !processor.screens.windows {
        return Err(
            "Table 307's /FWScale, because this window opens no floating window of its own, so \
             the movie plays in the annotation's rectangle"
                .to_owned(),
        );
    }
    let pair = |object: &Object| -> Option<(i64, i64)> {
        match object.as_array()? {
            [first, second] => Some((
                document.resolve(first).as_integer()?,
                document.resolve(second).as_integer()?,
            )),
            _ => None,
        }
    };
    let Some((numerator, denominator)) =
        pair(&scale).filter(|(numerator, denominator)| *numerator > 0 && *denominator > 0)
    else {
        return Err(
            "Table 307's /FWScale, because it is not two positive integers, so the movie plays \
             in the annotation's rectangle"
                .to_owned(),
        );
    };
    let Some((width, height)) = pair(&document.get_key(movie, "Aspect"))
        .filter(|(width, height)| *width > 0 && *height > 0)
    else {
        return Err(
            "Table 307's /FWScale, because Table 306's /Aspect, which it scales, is not stated as \
             two positive integers, so the movie plays in the annotation's rectangle"
                .to_owned(),
        );
    };
    let scaled = |side: i64| -> u32 {
        // Each factor is a positive i64, so the product fits an i128 and the divisor is not zero.
        let pixels = i128::from(side)
            .saturating_mul(i128::from(numerator))
            .checked_div(i128::from(denominator))
            // not a repair: the product of two positive integers past an i128 is past every window
            // this reader opens, which the next line holds to u32.
            .unwrap_or(i128::MAX);
        // not a repair: a window this reader opens is at least one pixel and at most what a u32
        // counts, its own bound on what it opens.
        u32::try_from(pixels.max(1)).unwrap_or(u32::MAX)
    };
    let (position, nearest_said) = position(document, stated)?;
    let window = Window {
        kind: WindowKind::Floating(Floating {
            size: (scaled(width), scaled(height)),
            // "with respect to the screen": the monitor Table 304's default finds, the one holding
            // the largest part of the document window.
            relative: Relative::Monitor,
            position,
            offscreen: Offscreen::OnScreen,
            // Table 307 asks for no title bar, close control or resizing; the window keeps the
            // first two so that a person can see what it is and close it (ADR 3587).
            title_bar: true,
            user_close: true,
            resize: Resize::Fixed,
            title: None,
            held_within: None,
        }),
        monitor: MonitorSpecifier::Largest,
        colour: Background::DEFAULT_COLOUR,
    };
    Ok((Some(window), nearest_said))
}

/// Table 295's `/P`, numbered row by row from the upper left.
const GRID: [[i64; 3]; 3] = [[0, 1, 2], [3, 4, 5], [6, 7, 8]];

/// Table 307's `/FWPosition` as the nearest of Table 295's nine positions, with the sentence for a
/// position between them, which is placed at the nearest.
///
/// # Errors
///
/// The sentence for a value that is not two numbers.
fn position(
    document: &Document,
    stated: &dyn Fn(&str) -> Object,
) -> Result<(Position, Option<String>), String> {
    let mut nearest_said = None;
    let position = match stated("FWPosition").as_array().map(|items| {
        items
            .iter()
            .map(|item| document.resolve(item).as_number())
            .collect::<Vec<_>>()
    }) {
        None => Position::CENTRE,
        Some(numbers) => match numbers.as_slice() {
            [Some(horizontal), Some(vertical)] => {
                let step = |fraction: f64| -> u8 {
                    // not a repair: /FWPosition's numbers are each "in the range 0.0 to 1.0", and
                    // one past it is placed at the nearest edge, which is the window's own
                    // placement rule (Table 295's /O).
                    let fraction = fraction.clamp(0.0, 1.0);
                    if fraction < 0.25 {
                        0
                    } else if fraction <= 0.75 {
                        1
                    } else {
                        2
                    }
                };
                let on_grid = |fraction: f64| [0.0, 0.5, 1.0].contains(&fraction);
                if !(on_grid(*horizontal) && on_grid(*vertical)) {
                    nearest_said = Some(format!(
                        "Table 307's /FWPosition [{horizontal} {vertical}] in part, because a \
                         window is placed at one of Table 295's nine positions, so it stands at \
                         the nearest"
                    ));
                }
                GRID.get(usize::from(step(*vertical)))
                    .and_then(|row| row.get(usize::from(step(*horizontal))))
                    .and_then(|value| Position::of_value(*value))
                    // not a repair: every value of GRID is one of Table 295's nine, so `of_value`
                    // answers each.
                    .unwrap_or(Position::CENTRE)
            }
            _ => {
                return Err(
                    "Table 307's /FWPosition, because it is not two numbers, so the movie plays \
                     in the annotation's rectangle"
                        .to_owned(),
                );
            }
        },
    };
    Ok((position, nearest_said))
}

/// The clip a sound object is handed over as, or the sentence saying why there is none.
///
/// `sound` is Table 212's or Table 188's `/Sound`, `table` the name of the one it came from, and
/// `stated` the entries of the action's that a player would have to be told. §13.3 says what makes
/// a sound a file a player can be handed:
///
/// > In particular, if it contains an F (file specification) entry, the sound shall be defined in
/// > an external file. This sound file shall be selfdescribing, containing all information needed
/// > to render the sound; no additional information need be present in the PDF file.
///
/// # Errors
///
/// The sentence a person is told: a `/Sound` that is not a stream, one whose `/F` is not a file
/// specification, and one holding its own samples that Table 305 does not describe in a form a
/// RIFF WAVE file can carry ([`super::SampleFormat::read`]).
///
/// # The samples a sound holds itself
///
/// > If no F entry is present, the sound object itself shall contain the sample data and all other
/// > information needed to define the sound.
///
/// Such a sound becomes [`Data::Samples`]: the hand-off decodes the stream and writes it as a RIFF
/// WAVE file, the form §13.3's NOTE names as self-describing, so a player is handed a file as it is
/// for an `/F` (`doc/questions/A631`, ADR 2398). The judgement reads Table 305 and decodes nothing.
pub fn sound(
    document: &Document,
    sound: &Object,
    table: &str,
    stated: &[&str],
) -> Result<Clip, String> {
    let resolved = document.resolve(sound);
    let Some(stream) = resolved.as_stream() else {
        return Err(format!(
            "{table}'s required /Sound is not a stream, which §13.3 says a sound object shall be"
        ));
    };
    let unhonoured = stated.iter().map(|key| format!("{table}'s /{key}"));
    let specification = document.get_key(&stream.dict, "F");
    if matches!(specification, Object::Null) {
        let (samples, rounded) = super::Samples::read(document, stream)
            .map_err(|why| format!("the sound object holds its own samples (§13.3), and {why}"))?;
        return Ok(Clip {
            content_type: Some(samples.format.content_type()),
            data: Data::Samples(Box::new(samples)),
            temporary_file: None,
            unhonoured: unhonoured.chain(rounded).collect(),
            playing: Playing::default(),
            played_here: false,
            background: None,
            window: None,
        });
    }
    let Some(data) = file(document, &specification, "sound") else {
        return Err(
            "the sound object's /F is not a file specification this reader can read (§7.11, \
             Table 5)"
                .to_owned(),
        );
    };
    Ok(Clip {
        content_type: content_type(&data),
        data,
        temporary_file: None,
        unhonoured: unhonoured.collect(),
        playing: Playing::default(),
        played_here: false,
        background: None,
        window: None,
    })
}

/// A file specification of either form (§7.11.1) as the data a player is handed: its embedded file
/// where §7.11.4 gives it one, its URL where §7.11.5 names one, and its name otherwise.
///
/// Both forms, unlike §13.2.4.2's `/D`: Table 306 and Table 5 each say "file specification", and the
/// sentence that excludes "the use of simple file specifications" is §13.2.4.2's own.
pub(super) fn file(document: &Document, specification: &Object, fallback: &str) -> Option<Data> {
    let resolved = document.resolve(specification);
    let parsed = FileSpec::parse(document, &resolved)?;
    // not a repair: the caller's own name for a file specification that names none, shown to a
    // person only.
    let name = parsed.display_name().unwrap_or_else(|| fallback.to_owned());
    if let Some(dict) = resolved.as_dict()
        && let Some(file) = crate::attachment::read(document, dict, name.clone())
    {
        return Some(Data::Embedded(Box::new(file)));
    }
    if let Some(url) = parsed.url() {
        return Some(Data::Url(url));
    }
    Some(Data::File(name))
}

/// Table 44's `/Subtype` of an embedded file, the one content type a self-describing file states.
pub(super) fn content_type(data: &Data) -> Option<String> {
    match data {
        Data::Embedded(file) => file.media_type.clone(),
        Data::Url(_) | Data::File(_) | Data::Form | Data::Samples(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{movie, poster_movie, sound};
    use crate::media::Data;
    use pdf_syntax::{Document, Object, ObjectId};

    /// Builds a document from object bodies numbered from 1.
    fn document(objects: &[&str]) -> Document {
        use std::fmt::Write as _;
        let mut out = String::from("%PDF-1.7\n");
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

    /// The dictionary of object `number`.
    fn dict(doc: &Document, number: u32) -> pdf_syntax::Dictionary {
        doc.get(ObjectId::new(number, 0))
            .as_dict()
            .cloned()
            .expect("the object is a dictionary")
    }

    /// Table 306's `/F` embedded, with Table 44's `/Subtype` as the type, and every Table 307 entry
    /// the annotation's activation states said rather than dropped.
    #[test]
    fn an_embedded_movie_is_handed_over_with_its_activation_entries_said() {
        let doc = document(&[
            "<< /Type /Catalog >>",
            "<< /Type /Annot /Subtype /Movie /Rect [0 0 10 10] /Movie << /F 3 0 R /Rotate 90 >> \
             /A << /Mode /Repeat /Volume 0.5 >> >>",
            "<< /Type /Filespec /F (clip.mov) /EF << /F 4 0 R >> >>",
            "<< /Type /EmbeddedFile /Subtype /video#2Fquicktime /Length 4 >>\nstream\nmoov\nendstream",
        ]);
        let clip = movie(
            &doc,
            &dict(&doc, 2),
            None,
            crate::media::Processor::default(),
        )
        .expect("the movie is handed over");
        assert_eq!(clip.content_type.as_deref(), Some("video/quicktime"));
        assert!(matches!(clip.data, Data::Embedded(_)), "{:?}", clip.data);
        assert_eq!(clip.temporary_file, None, "Table 286 stands over no movie");
        assert_eq!(
            clip.unhonoured,
            [
                "Table 307's /Volume",
                "Table 307's /Mode",
                "Table 306's /Rotate 90"
            ]
        );
    }

    /// A host whose player shows video (ADR 3594).
    fn shows_video(windows: bool) -> crate::media::Processor<'static> {
        crate::media::Processor {
            players: crate::media::Players::AlsoVideoHere,
            screens: crate::media::Screens {
                windows,
                placed: true,
                monitors: [true; 7],
            },
            ..crate::media::Processor::default()
        }
    }

    /// Table 306's `/Rotate` "shall be a multiple of 90": a clockwise multiple turns the pictures
    /// clockwise, a negative one the other way, and a value that is no multiple is said.
    #[test]
    fn a_rotation_is_read_as_clockwise_quarter_turns() {
        assert_eq!(super::quarter_turns(90), Some(1));
        assert_eq!(super::quarter_turns(180), Some(2));
        assert_eq!(super::quarter_turns(-90), Some(3));
        assert_eq!(super::quarter_turns(450), Some(1));
        assert_eq!(super::quarter_turns(45), None);
        let doc = document(&[
            "<< /Type /Catalog >>",
            "<< /Type /Annot /Subtype /Movie /Rect [0 0 10 10] /Movie << /F 3 0 R /Rotate 45 >> \
             >>",
            "<< /Type /Filespec /F (clip.mp4) /EF << /F 4 0 R >> >>",
            "<< /Type /EmbeddedFile /Subtype /video#2Fmp4 /Length 4 >>\nstream\nmoov\nendstream",
        ]);
        let clip = movie(&doc, &dict(&doc, 2), None, shows_video(false)).expect("shown");
        assert_eq!(clip.playing.quarter_turns, 0);
        assert!(
            clip.unhonoured
                .iter()
                .any(|said| said.contains("/Rotate 45") && said.contains("multiple of 90")),
            "{:?}",
            clip.unhonoured
        );
    }

    /// An embedded MP4 movie shown here is told Table 307's entries (ADR 3594): `/Start` 600 and
    /// `/Duration` 1200 in a stated scale of 600 a second are the section from 1 s to 3 s;
    /// `/Volume` 0.5 the level; `/ShowControls` the bar; `/Mode /Repeat` a count of zero; `/FWScale`
    /// [1 2] over an `/Aspect` of [320 240] a floating window of 160 by 120 pixels, `/FWPosition`
    /// [0 1] Table 295's lower-left. The action's `/Volume` overrides the activation's (§12.6.4.10).
    /// Table 306's `/Rotate` 90 is one clockwise quarter turn (ADR 3817); what it cannot honour —
    /// `/Rate` — is still said.
    #[test]
    fn a_movie_shown_here_is_told_table_307() {
        let doc = document(&[
            "<< /Type /Catalog >>",
            "<< /Type /Annot /Subtype /Movie /Rect [0 0 10 10] /Movie << /F 3 0 R /Rotate 90 \
             /Aspect [320 240] >> /A << /Start [600 600] /Duration [1200 600] /Volume 0.25 \
             /ShowControls true /Mode /Repeat /Rate 2 /FWScale [1 2] /FWPosition [0 1] >> >>",
            "<< /Type /Filespec /F (clip.mp4) /EF << /F 4 0 R >> >>",
            "<< /Type /EmbeddedFile /Subtype /video#2Fmp4 /Length 4 >>\nstream\nmoov\nendstream",
            "<< /S /Movie /Volume 0.5 >>",
        ]);
        let action = dict(&doc, 5);
        let clip = movie(&doc, &dict(&doc, 2), Some(&action), shows_video(true))
            .expect("the movie is shown");
        assert!(clip.played_here);
        assert_eq!(clip.playing.section.begin, 1.0);
        assert_eq!(clip.playing.section.end, Some(3.0));
        assert_eq!(clip.playing.level, 0.5);
        assert!(clip.playing.controls);
        assert_eq!(clip.playing.repeat, 0.0);
        let window = clip.window.as_ref().expect("a floating window");
        match &window.kind {
            crate::media::WindowKind::Floating(floating) => {
                assert_eq!(floating.size, (160, 120));
                assert_eq!(floating.position.value(), 6);
            }
            crate::media::WindowKind::FullScreen => panic!("not full screen"),
        }
        assert_eq!(clip.playing.quarter_turns, 1, "Table 306's /Rotate 90");
        assert_eq!(clip.unhonoured.len(), 1, "{:?}", clip.unhonoured);
        assert!(clip.unhonoured[0].starts_with("Table 307's /Rate 2"));
        // The same movie for a host that opens no window plays in the annotation's rectangle, and
        // for one that shows no video is handed on as before.
        let in_place = movie(&doc, &dict(&doc, 2), None, shows_video(false)).expect("shown");
        assert!(in_place.window.is_none());
        assert!(
            in_place
                .unhonoured
                .iter()
                .any(|said| said.contains("/FWScale"))
        );
        let handed = movie(
            &doc,
            &dict(&doc, 2),
            None,
            crate::media::Processor::default(),
        )
        .expect("handed on");
        assert!(!handed.played_here);
    }

    /// Table 307's `/Synchronous` true, "the movie player shall retain control until the movie is
    /// completed or dismissed by the user", reaches the player as [`Playing::synchronous`] and is
    /// no longer said as unhonoured (ADR 3853); the action's `false` overrides the activation's
    /// `true` (§12.6.4.10), and a value that is no boolean is said and plays asynchronously.
    #[test]
    fn a_synchronous_movie_tells_the_player_to_hold() {
        let doc = document(&[
            "<< /Type /Catalog >>",
            "<< /Type /Annot /Subtype /Movie /Rect [0 0 10 10] /Movie << /F 3 0 R >> \
             /A << /Synchronous true >> >>",
            "<< /Type /Filespec /F (clip.mp4) /EF << /F 4 0 R >> >>",
            "<< /Type /EmbeddedFile /Subtype /video#2Fmp4 /Length 4 >>\nstream\nmoov\nendstream",
            "<< /S /Movie /Synchronous false >>",
            "<< /S /Movie /Synchronous /Yes >>",
        ]);
        let held = movie(&doc, &dict(&doc, 2), None, shows_video(false)).expect("shown");
        assert!(held.playing.synchronous);
        assert!(held.unhonoured.is_empty(), "{:?}", held.unhonoured);
        let released = movie(
            &doc,
            &dict(&doc, 2),
            Some(&dict(&doc, 5)),
            shows_video(false),
        )
        .expect("shown");
        assert!(
            !released.playing.synchronous,
            "the action's false overrides"
        );
        let odd = movie(
            &doc,
            &dict(&doc, 2),
            Some(&dict(&doc, 6)),
            shows_video(false),
        )
        .expect("shown");
        assert!(!odd.playing.synchronous);
        assert_eq!(
            odd.unhonoured,
            ["Table 307's /Synchronous, because its value is not a boolean"]
        );
    }

    /// Table 306's `/Poster` `true`, "the poster image shall be retrieved from the movie file",
    /// names the movie a window that shows video takes its first picture from, whatever the
    /// annotation's `/A` says of a click (ADR 3609); `false`, a stream and an absent entry name none.
    #[test]
    fn a_poster_retrieved_from_the_movie_names_the_movie() {
        let with = |poster: &str| {
            document(&[
                "<< /Type /Catalog >>",
                &format!(
                    "<< /Type /Annot /Subtype /Movie /Rect [0 0 10 10] /A false \
                     /Movie << /F 3 0 R {poster} >> >>"
                ),
                "<< /Type /Filespec /F (clip.mp4) /EF << /F 4 0 R >> >>",
                "<< /Type /EmbeddedFile /Subtype /video#2Fmp4 /Length 4 >>\nstream\nmoov\nendstream",
            ])
        };
        let doc = with("/Poster true");
        let clip = poster_movie(&doc, &dict(&doc, 2), shows_video(false))
            .expect("a poster from the movie")
            .expect("the movie is read");
        assert!(clip.played_here, "an embedded MP4 movie this host shows");
        let handed = poster_movie(&doc, &dict(&doc, 2), crate::media::Processor::default())
            .expect("a poster from the movie")
            .expect("the movie is read");
        assert!(
            !handed.played_here,
            "a host that shows no video decodes none"
        );
        for poster in ["/Poster false", "", "/Poster 4 0 R"] {
            let doc = with(poster);
            assert!(
                poster_movie(&doc, &dict(&doc, 2), shows_video(false)).is_none(),
                "{poster:?}"
            );
        }
    }

    /// `/Mode /Open` holds the last picture until the movie is stopped; `/Palindrome` — "Play
    /// continuously forward and backward until stopped" — repeats for ever and alternates, with
    /// nothing set aside (ADRs 3594, 3842).
    #[test]
    fn open_holds_and_palindrome_alternates() {
        let movie_with = |mode: &str| {
            let doc = document(&[
                "<< /Type /Catalog >>",
                &format!(
                    "<< /Type /Annot /Subtype /Movie /Rect [0 0 10 10] /Movie << /F 3 0 R >> \
                     /A << /Mode /{mode} >> >>"
                ),
                "<< /Type /Filespec /F (clip.mp4) /EF << /F 4 0 R >> >>",
                "<< /Type /EmbeddedFile /Subtype /video#2Fmp4 /Length 4 >>\nstream\nmoov\nendstream",
            ]);
            movie(&doc, &dict(&doc, 2), None, shows_video(false)).expect("shown")
        };
        let open = movie_with("Open");
        assert_eq!(
            open.playing.duration,
            crate::media::SimpleDuration::Infinite
        );
        assert!(open.unhonoured.is_empty(), "{:?}", open.unhonoured);
        let palindrome = movie_with("Palindrome");
        assert_eq!(palindrome.playing.repeat, 0.0);
        assert!(palindrome.playing.palindrome);
        assert!(!palindrome.playing.backward);
        assert!(
            palindrome.unhonoured.is_empty(),
            "{:?}",
            palindrome.unhonoured
        );
        assert!(!open.playing.palindrome);
    }

    /// Table 307's negative `/Rate` — "the movie shall be played backward with respect to Start
    /// and Duration" — reaches the player as `backward`, pictures and sound together, with nothing
    /// set aside; a rate of -2 also says its speed, and a positive one plays forward (ADRs 3837,
    /// 3841).
    #[test]
    fn a_negative_rate_plays_the_movie_backward() {
        let movie_at = |rate: &str| {
            let doc = document(&[
                "<< /Type /Catalog >>",
                &format!(
                    "<< /Type /Annot /Subtype /Movie /Rect [0 0 10 10] /Movie << /F 3 0 R >> \
                     /A << /Rate {rate} >> >>"
                ),
                "<< /Type /Filespec /F (clip.mp4) /EF << /F 4 0 R >> >>",
                "<< /Type /EmbeddedFile /Subtype /video#2Fmp4 /Length 4 >>\nstream\nmoov\nendstream",
            ]);
            movie(&doc, &dict(&doc, 2), None, shows_video(false)).expect("shown")
        };
        let backward = movie_at("-1");
        assert!(backward.playing.backward);
        assert!(backward.unhonoured.is_empty(), "{:?}", backward.unhonoured);
        let faster = movie_at("-2");
        assert!(faster.playing.backward);
        assert!(
            faster
                .unhonoured
                .iter()
                .any(|said| said.starts_with("Table 307's /Rate -2"))
        );
        let forward = movie_at("1");
        assert!(!forward.playing.backward);
        assert!(forward.unhonoured.is_empty(), "{:?}", forward.unhonoured);
    }

    /// Table 189's `/A false` withholds the annotation's own activation and not a movie action's,
    /// whose dictionary the clause makes the stronger statement.
    #[test]
    fn an_activation_of_false_withholds_the_click_and_not_the_action() {
        let doc = document(&[
            "<< /Type /Catalog >>",
            "<< /Type /Annot /Subtype /Movie /Rect [0 0 10 10] /Movie << /F (clip.mov) >> /A false >>",
            "<< /S /Movie /Annotation 2 0 R /Rate 2 >>",
        ]);
        let clicked = movie(
            &doc,
            &dict(&doc, 2),
            None,
            crate::media::Processor::default(),
        );
        assert!(
            clicked
                .as_ref()
                .is_err_and(|why| why.contains("/A is false")),
            "{clicked:?}"
        );
        let action = dict(&doc, 3);
        let played = movie(
            &doc,
            &dict(&doc, 2),
            Some(&action),
            crate::media::Processor::default(),
        )
        .expect("the action plays it");
        assert!(
            matches!(&played.data, Data::File(name) if name == "clip.mov"),
            "a simple file specification is Table 306's too: {:?}",
            played.data
        );
        assert_eq!(played.unhonoured, ["Table 307's /Rate"]);
    }

    /// Table 189's required `/Movie`, absent, is said.
    #[test]
    fn a_movie_annotation_with_no_movie_dictionary_hands_nothing_over() {
        let doc = document(&[
            "<< /Type /Catalog >>",
            "<< /Type /Annot /Subtype /Movie /Rect [0 0 10 10] >>",
        ]);
        let why = movie(
            &doc,
            &dict(&doc, 2),
            None,
            crate::media::Processor::default(),
        )
        .expect_err("no movie, no hand-off");
        assert!(why.contains("/Movie required"), "{why}");
    }

    /// §13.3's `/F` names a self-describing file, which is handed over with the action's playing
    /// parameters said; a stream holding its own samples becomes samples for the WAVE writer, and
    /// one compressed by a `/CO` no standard value names is said and handed nowhere.
    #[test]
    fn a_sound_is_handed_over_by_its_file_or_its_own_samples() {
        let doc = document(&[
            "<< /Type /Catalog >>",
            "<< /Type /Sound /R 22050 /F << /FS /URL /F (https://example.org/a.wav) >> /Length 0 \
             >>\nstream\n\nendstream",
            "<< /Type /Sound /R 8000 /E /muLaw /Length 2 >>\nstream\nab\nendstream",
            "<< /Type /Sound /R 8000 /CO /Private /Length 2 >>\nstream\nab\nendstream",
        ]);
        let reference = |number| Object::Reference(ObjectId::new(number, 0));
        let clip = sound(&doc, &reference(2), "Table 212", &["Volume", "Repeat"])
            .expect("a sound naming its file is handed over");
        assert!(
            matches!(&clip.data, Data::Url(url) if url == "https://example.org/a.wav"),
            "{:?}",
            clip.data
        );
        assert_eq!(
            clip.unhonoured,
            ["Table 212's /Volume", "Table 212's /Repeat"]
        );
        let held = sound(&doc, &reference(3), "Table 212", &[]).expect("samples are written");
        assert_eq!(
            held.content_type.as_deref(),
            Some("audio/vnd.wave; codec=7")
        );
        assert!(
            matches!(&held.data, Data::Samples(samples) if samples.format.rate == 8000),
            "{:?}",
            held.data
        );
        let why = sound(&doc, &reference(4), "Table 212", &[]).expect_err("compressed");
        assert!(why.contains("/CO /Private"), "{why}");
        let why = sound(&doc, &Object::Null, "Table 188", &[]).expect_err("not a stream");
        assert!(why.starts_with("Table 188's required /Sound"), "{why}");
    }
}
