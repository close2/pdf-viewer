//! §12.6.4.9's sound action, §12.6.4.10's movie action and the activation of §12.5.6.16's and
//! §12.5.6.17's annotations, as far as a processor that plays nothing performs them.
//!
//! All four clauses open by handing themselves to 13.2 — "superseded by the general multimedia
//! framework" — and what `CLAUDE.md`'s clause 13 entry admits there is the media file handed to this
//! machine's own player at the reader's level. So each ends where [`super::rendered`] ends: the
//! file `pdf_model::media` reads becomes a clip in [`Outcome::media`], and the reader's level and
//! the host decide the rest exactly as they do for a rendition's (`doc/questions/A631`, ADR 2382).
//! Every arm says something, because each is a click a person made (trap 5).
//!
//! **A movie this program shows itself** — for a host whose player shows video, an embedded MP4
//! movie of `video/` type (ADR 3594) — is played in the movie annotation's own rectangle: the clip
//! goes over associated with the annotation, as a rendition action's goes over associated with its
//! screen annotation, so the window draws its pictures there and §12.6.4.10's `Stop`, `Pause` and
//! `Resume` reach it as Table 214's operations do.

use pdf_model::action::{MovieAction, MovieOperation, MovieTarget, SoundAction};
use pdf_syntax::{Dictionary, Document, ObjectId};

use super::Outcome;
use crate::open::Open;

/// What activating a movie or a sound annotation hands over, and `None` for every other subtype.
///
/// §12.5.6.17: "When the annotation is activated, the movie shall be played." And §12.5.6.16, of a
/// sound annotation: "When the annotation is activated, the sound shall be played."
pub(crate) fn exhibited(open: &Open, annotation: ObjectId) -> Option<Outcome> {
    let object = open.document.get(annotation);
    let dict = object.as_dict()?;
    let subtype = open.document.get_key(dict, "Subtype");
    let mut outcome = Outcome::default();
    match subtype.as_name()?.as_bytes() {
        b"Movie" => handed(
            pdf_model::media::movie(&open.document, dict, None, processor(open)),
            &format!("this {}", movie_name(&open.document, dict)),
            &mut outcome,
            Some(annotation),
        ),
        b"Sound" => handed(
            pdf_model::media::sound(
                &open.document,
                &open.document.get_key(dict, "Sound"),
                "Table 188",
                &[],
            ),
            "this sound annotation",
            &mut outcome,
            None,
        ),
        _ => return None,
    }
    Some(outcome)
}

/// §12.6.4.10's action: the movie annotation found, and its movie handed over where the action
/// asks for it to be played.
///
/// `page` is where a `/T` is looked for: the clause says the movie annotation "shall be associated
/// with the page that is the destination of the link annotation or outline item containing the
/// movie action, or with the page object with which the action is associated", which is the page a
/// chain's jump reached where it reached one and the page shown otherwise.
pub(super) fn movie_requested(
    open: &Open,
    movie: &MovieAction,
    page: usize,
    subject: &str,
    outcome: &mut Outcome,
) {
    let note = |outcome: &mut Outcome, why: &str| outcome.notes.push(format!("{subject} — {why}"));
    // Table 213's operations on a movie this program shows: the clip associated with the movie
    // annotation, which a movie it handed on is never (`VideoPlayer::operate`'s "no effect").
    let operation = match &movie.operation {
        MovieOperation::Play => None,
        MovieOperation::Stop => Some(crate::MediaOperation::Stop),
        MovieOperation::Pause => Some(crate::MediaOperation::Pause),
        MovieOperation::Resume => Some(crate::MediaOperation::Resume),
        MovieOperation::Unrecognised(name) => {
            return note(
                outcome,
                &format!(
                    "Movie: Table 213 defines no /Operation /{name}, so nothing was performed"
                ),
            );
        }
    };
    let document = &open.document;
    let found = match &movie.target {
        MovieTarget::Invalid(why) => return note(outcome, why),
        MovieTarget::Annotation(id) => document
            .get(*id)
            .as_dict()
            .cloned()
            .map(|dict| (Some(*id), dict)),
        MovieTarget::Title(title) => titled(open, page, title),
    };
    let Some((id, annotation)) = found.filter(|(_, dict)| {
        document
            .get_key(dict, "Subtype")
            .as_name()
            .is_some_and(|name| name.as_bytes() == b"Movie")
    }) else {
        return note(
            outcome,
            match movie.target {
                MovieTarget::Title(_) => {
                    "Movie: no movie annotation on the page this action is associated with has the \
                     title Table 213's /T names"
                }
                _ => "Movie: Table 213's /Annotation is not a movie annotation",
            },
        );
    };
    if let Some(operation) = operation {
        // A movie named by a direct dictionary in `/Annots` has no object to be associated with,
        // so nothing this program plays can be it.
        match id {
            Some(id) => outcome
                .media
                .push(super::MediaRequest::Operate(id, operation)),
            None => note(
                outcome,
                "Movie: the movie annotation is not an indirect object, so no movie this program \
                 plays is associated with it and there is nothing to stop, pause or resume",
            ),
        }
        return;
    }
    handed(
        pdf_model::media::movie(
            document,
            &annotation,
            Some(&movie.overrides),
            processor(open),
        ),
        &format!("{subject} plays {}", movie_name(document, &annotation)),
        outcome,
        id,
    );
}

/// What this processor tells `pdf_model::media` about itself, as a rendition action's judgement is
/// told.
fn processor(open: &Open) -> pdf_model::media::Processor<'_> {
    pdf_model::media::Processor {
        language: open.view.audience().language.as_deref(),
        players: open.view.players(),
        screens: open.view.screens(),
    }
}

/// §12.6.4.9's action: the sound object's file handed over.
pub(super) fn sound_requested(
    open: &Open,
    sound: &SoundAction,
    subject: &str,
    outcome: &mut Outcome,
) {
    let subject = format!("{subject} plays a sound");
    outcome.notes.extend(
        sound
            .misread
            .iter()
            .map(|misread| format!("{subject} — {misread}")),
    );
    // Table 212's entries travel with the clip, for a window that plays the sound itself and so is
    // told them (ADR 3341); a player handed the file is still told none, which `handed` says.
    let clip = pdf_model::media::sound(&open.document, &sound.sound, "Table 212", &sound.stated)
        .map(|clip| pdf_model::media::Clip {
            playing: sound.playing,
            ..clip
        });
    handed(clip, &subject, outcome, None);
}

/// The clip kept for the hand-off and said, or the reason there is none said. A movie this
/// program's player shows goes over associated with `annotation`, the movie annotation whose
/// rectangle it plays in (ADR 3594).
fn handed(
    clip: Result<pdf_model::media::Clip, String>,
    subject: &str,
    outcome: &mut Outcome,
    annotation: Option<ObjectId>,
) {
    match clip {
        Ok(clip) if clip.played_here => {
            outcome.notes.push(format!(
                "{subject} — this program's player shows {}",
                clip.describe()
            ));
            outcome.notes.extend(
                clip.unhonoured
                    .iter()
                    .map(|entry| format!("{subject} — this program's player sets aside {entry}")),
            );
            let associated = annotation.map(|annotation| crate::Associated {
                annotation,
                or_resume: false,
            });
            outcome
                .media
                .push(super::MediaRequest::Play(Box::new(clip), associated));
        }
        Ok(clip) => {
            outcome
                .notes
                .push(format!("{subject} — the file is {}", clip.describe()));
            outcome.notes.extend(
                clip.unhonoured.iter().map(|entry| {
                    format!("{subject} — a player handed the file is not told {entry}")
                }),
            );
            outcome
                .media
                .push(super::MediaRequest::Play(Box::new(clip), None));
        }
        Err(why) => outcome.notes.push(format!("{subject} — {why}")),
    }
}

/// The movie annotation on `page` whose Table 189 `/T` is `title`, with its object number where it
/// is an indirect object.
fn titled(open: &Open, page: usize, title: &str) -> Option<(Option<ObjectId>, Dictionary)> {
    let document = &open.document;
    let page = open.placed_page(page)?;
    let annotations = document.get_key(&page.dict, "Annots");
    annotations.as_array()?.iter().find_map(|item| {
        let resolved = document.resolve(item);
        let dict = resolved.as_dict()?;
        let named = matches!(
            document.get_key(dict, "T"),
            pdf_syntax::Object::String(bytes) if pdf_syntax::text_string(&bytes) == title
        );
        let movie = document
            .get_key(dict, "Subtype")
            .as_name()
            .is_some_and(|name| name.as_bytes() == b"Movie");
        let id = match item {
            pdf_syntax::Object::Reference(id) => Some(*id),
            _ => None,
        };
        (named && movie).then(|| (id, dict.clone()))
    })
}

/// "movie annotation" with its Table 189 `/T` where it states one, for the sentence.
fn movie_name(document: &Document, annotation: &Dictionary) -> String {
    match document.get_key(annotation, "T") {
        pdf_syntax::Object::String(bytes) => {
            format!("movie annotation {:?}", pdf_syntax::text_string(&bytes))
        }
        _ => "movie annotation".to_owned(),
    }
}
