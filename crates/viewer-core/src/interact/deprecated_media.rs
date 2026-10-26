//! §12.6.4.9's sound action, §12.6.4.10's movie action and the activation of §12.5.6.16's and
//! §12.5.6.17's annotations, as far as a processor that plays nothing performs them.
//!
//! All four clauses open by handing themselves to 13.2 — "superseded by the general multimedia
//! framework" — and what `CLAUDE.md`'s clause 13 entry admits there is the media file handed to this
//! machine's own player at the reader's level. So each ends where [`super::rendered`] ends: the
//! file `pdf_model::media` reads becomes a clip in [`Outcome::media`], and the reader's level and
//! the host decide the rest exactly as they do for a rendition's (`doc/questions/A631`, ADR 2382).
//! Every arm says something, because each is a click a person made (trap 5).

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
            pdf_model::media::movie(&open.document, dict, None),
            &format!("this {}", movie_name(&open.document, dict)),
            &mut outcome,
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
    match &movie.operation {
        MovieOperation::Play => {}
        MovieOperation::Stop => {
            return note(
                outcome,
                "Movie: stop, and nothing plays in this program, so there is nothing to stop \
                 (§12.6.4.10)",
            );
        }
        MovieOperation::Pause | MovieOperation::Resume => {
            return note(
                outcome,
                "Movie: pause or resume, and nothing plays in this program to pause or resume \
                 (§12.6.4.10)",
            );
        }
        MovieOperation::Unrecognised(name) => {
            return note(
                outcome,
                &format!(
                    "Movie: Table 213 defines no /Operation /{name}, so nothing was performed"
                ),
            );
        }
    }
    let document = &open.document;
    let found = match &movie.target {
        MovieTarget::Invalid(why) => return note(outcome, why),
        MovieTarget::Annotation(id) => document.get(*id).as_dict().cloned(),
        MovieTarget::Title(title) => titled(open, page, title),
    };
    let Some(annotation) = found.filter(|dict| {
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
    handed(
        pdf_model::media::movie(document, &annotation, Some(&movie.overrides)),
        &format!("{subject} plays {}", movie_name(document, &annotation)),
        outcome,
    );
}

/// §12.6.4.9's action: the sound object's file handed over.
pub(super) fn sound_requested(
    open: &Open,
    sound: &SoundAction,
    subject: &str,
    outcome: &mut Outcome,
) {
    handed(
        pdf_model::media::sound(&open.document, &sound.sound, "Table 212", &sound.stated),
        &format!("{subject} plays a sound"),
        outcome,
    );
}

/// The clip kept for the hand-off and said, or the reason there is none said.
fn handed(clip: Result<pdf_model::media::Clip, String>, subject: &str, outcome: &mut Outcome) {
    match clip {
        Ok(clip) => {
            outcome
                .notes
                .push(format!("{subject} — the file is {}", clip.describe()));
            outcome.notes.extend(
                clip.unhonoured.iter().map(|entry| {
                    format!("{subject} — a player handed the file is not told {entry}")
                }),
            );
            outcome.media.push(clip);
        }
        Err(why) => outcome.notes.push(format!("{subject} — {why}")),
    }
}

/// The movie annotation on `page` whose Table 189 `/T` is `title`.
fn titled(open: &Open, page: usize, title: &str) -> Option<Dictionary> {
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
        (named && movie).then(|| dict.clone())
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
