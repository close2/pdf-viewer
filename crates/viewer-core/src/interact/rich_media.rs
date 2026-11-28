//! A rich media annotation activated and deactivated: ISO 32000-2 §13.7.2.2's Tables 335 and 336,
//! for the `Sound` and `Video` configurations this program's players present (ADRs 3854, 3855).
//!
//! **Activation.** Table 335's `XA` is "explicitly activated by a user action": a press, which
//! [`pressed`] answers. `PO` is "as soon as the page that contains the annotation receives focus as
//! the current page" and `PV` "as soon as any part of the page … becomes visible" — the moments
//! Table 197 raises a screen annotation's `/PO` and `/PV` at, so [`raised`] answers those two
//! triggers where they are the condition — from the first frame's presentation on, the page the
//! open shows being answered then by [`presented`], so that page one pays nothing for them. An activation hands the configuration's clip to the
//! players associated with the annotation, as Table 218's `/OP` 4 does — "resume the rendition if
//! it is paused; otherwise, do nothing" where one is already associated — so a press on a playing
//! annotation, or a page shown again while it plays, starts nothing a second time.
//!
//! **Deactivation.** Table 336's `XD` is the reader's: Escape, or a rendition action's stop. `PC`
//! and `PI` are Table 197's `/PC` and `/PI` moments, and [`raised`] answers them with Table 218's
//! `/OP` 1, a stop of whatever plays associated with the annotation, which is "no effect" where
//! nothing does.
//!
//! **The snapshot.** Table 344's `/Snapshot` of the view an activation invokes "shall be displayed
//! in place of the current artwork" (§13.7.2.3.5), so it stands from activation to deactivation.
//! The core learns of every deactivation through the players: a press, a `PO` or a `PV` hands the
//! clip over, and the reader's stop, the clip's end and the stop a `PC` or `PI` asks for each take
//! it out of their hands, which the window says with [`crate::Command::InHand`]. [`in_hand`]
//! displays the image while a player holds such a clip and takes it away once none does, and
//! `pdf_model::annotation::decide` sets the stored appearance aside for it (ADR 3889). A window
//! that hands every clip to this machine's player takes no rich media clip at all, so the
//! annotation stays inactive there and no view is invoked.
//!
//! **What is not played** — the `3D` configuration, whose rendering is excluded, and a
//! configuration the players do not take — keeps ADR 3819's answer on a press: said by name, the
//! annotation in §13.7.2.1's inactive state, and the assets handed over as files to save (ADR
//! 2399), with the view it would have invoked said as `pdf_model` reads it (ADR 3881). A page's
//! condition activates nothing such, and says nothing either: no person asked.

use pdf_model::action::Trigger;
use pdf_model::media::rich_media::{self, Activation, Deactivation};
use pdf_syntax::{Dictionary, ObjectId};

use super::{MediaRequest, Outcome};
use crate::open::Open;

/// Whether a press on `annotation` plays its configuration, so that the press hands its assets
/// over as files only where it does not (ADR 3855).
pub(crate) fn plays(open: &Open, annotation: &Dictionary) -> bool {
    rich_media::activated(
        &open.document,
        annotation,
        super::deprecated_media::processor(open),
    )
    .is_ok()
}

/// A press and release on a rich media annotation: the reader's explicit activation, Table 335's
/// `XA`, whatever condition the annotation states — a press is always a person asking (ADR 3819).
pub(super) fn pressed(open: &Open, annotation: ObjectId, dict: &Dictionary, outcome: &mut Outcome) {
    let subject = "this rich media annotation";
    if let Some(settings) = rich_media::settings(&open.document, dict) {
        outcome.notes.extend(
            settings
                .notes
                .iter()
                .map(|note| format!("{subject} — {note}")),
        );
    }
    match rich_media::activated(
        &open.document,
        dict,
        super::deprecated_media::processor(open),
    ) {
        Ok(activated) => played(
            open,
            activated,
            (annotation, dict),
            &format!("{subject} is activated"),
            outcome,
        ),
        Err(why) => {
            outcome.notes.push(format!(
                "{subject}'s content is not shown — {why} (ISO 32000-2 §13.7), so the annotation \
                 keeps its normal appearance, §13.7.2.1's inactive state; the press hands its \
                 assets over as files to save (ADR 2399)"
            ));
            // The view the activation would have invoked is still read, so that a snapshot or
            // state data the file states is said rather than passed over (ADR 3881).
            if let Some(view) = rich_media::invoked_view(&open.document, dict) {
                outcome
                    .notes
                    .extend(view.notes.iter().map(|note| format!("{subject} — {note}")));
                if view.snapshot.is_some() {
                    outcome.notes.push(format!(
                        "{subject} — the snapshot of the view it would invoke (Table 344's \
                         /Snapshot) is not displayed, since no view is invoked while the \
                         annotation stays inactive (ISO 32000-2 §13.7.2.3.5)"
                    ));
                }
            }
        }
    }
}

/// Table 197's page triggers on a rich media annotation, where Table 335 or Table 336 makes one
/// of them its condition: `PO` or `PV` activates it, `PC` or `PI` deactivates it. Nothing for
/// every other trigger, annotation and condition.
///
/// **Nothing before the document's first frame is presented**, not even the look at the
/// annotation's subtype: the page moments the open raises are on page one's critical path, and
/// [`presented`] raises the presented page's rich media `PO` and `PV` once the host says its first
/// frame is on the screen, as the open sequence's scripts wait for it (ADRs 1602, 3854). Nothing
/// plays before then, so a `PC` or `PI` before then has nothing to stop.
pub(super) fn raised(
    open: &Open,
    annotation: ObjectId,
    dict: &Dictionary,
    event: Trigger,
    outcome: &mut Outcome,
) {
    if !open.presented {
        return;
    }
    if !matches!(
        event,
        Trigger::PageOpen | Trigger::PageVisible | Trigger::PageClose | Trigger::PageInvisible
    ) {
        return;
    }
    let Some(settings) = rich_media::settings(&open.document, dict) else {
        return;
    };
    let activates = matches!(
        (settings.activation, event),
        (Activation::PageOpen, Trigger::PageOpen) | (Activation::PageVisible, Trigger::PageVisible)
    );
    let deactivates = matches!(
        (settings.deactivation, event),
        (Deactivation::PageClose, Trigger::PageClose)
            | (Deactivation::PageInvisible, Trigger::PageInvisible)
    );
    if deactivates {
        outcome.media.push(MediaRequest::Operate(
            annotation,
            crate::MediaOperation::Stop,
        ));
    }
    if activates
        && let Ok(activated) = rich_media::activated(
            &open.document,
            dict,
            super::deprecated_media::processor(open),
        )
    {
        let condition = if event == Trigger::PageOpen {
            "its page became the current page (Table 335's /Condition PO)"
        } else {
            "its page became visible (Table 335's /Condition PV)"
        };
        played(
            open,
            activated,
            (annotation, dict),
            &format!("this rich media annotation is activated because {condition}"),
            outcome,
        );
    }
}

/// The presented page's rich media `PO` — `current`, the annotations of the current page — and
/// `PV` — `visible`, those of the pages on the screen — raised once, at `Command::Presented`, for
/// the moments the open raised before the first frame and [`raised`] passed over.
pub(crate) fn presented(open: &Open, current: &[ObjectId], visible: &[ObjectId]) -> Outcome {
    let mut outcome = Outcome::default();
    let moments = current
        .iter()
        .map(|annotation| (*annotation, Trigger::PageOpen))
        .chain(
            visible
                .iter()
                .map(|annotation| (*annotation, Trigger::PageVisible)),
        );
    for (annotation, event) in moments {
        let object = open.document.get(annotation);
        if let Some(dict) = object.as_dict() {
            raised(open, annotation, dict, event, &mut outcome);
        }
    }
    outcome
}

/// The configuration's clip handed to the players associated with `annotation`, as Table 218's
/// `/OP` 4 associates one, presented as Table 338 states, with what it reads differently and what
/// it sets aside said (ADRs 3855, 3865).
fn played(
    open: &Open,
    activated: rich_media::Activated,
    (annotation, dict): (ObjectId, &Dictionary),
    subject: &str,
    outcome: &mut Outcome,
) {
    let rich_media::Activated { clip, notes } = activated;
    let snapshot = rich_media::invoked_view(&open.document, dict).and_then(|view| view.snapshot);
    let presented = if clip.window.is_some() {
        "in a floating window of its own (Table 338's Windowed)"
    } else if clip.background.is_some() {
        "in the annotation's rectangle (Table 338's Embedded)"
    } else {
        "as a sound"
    };
    outcome.notes.push(format!(
        "{subject} — this program's player plays {} {presented}",
        clip.describe()
    ));
    outcome
        .notes
        .extend(notes.iter().map(|note| format!("{subject} — {note}")));
    // §13.7.2.3.5's image, displayed while the window's players hold the clip; the players take
    // every clip `activated` answers, and only they tell the page when it leaves their hands
    // (ADR 3889).
    if snapshot.is_some() {
        outcome.notes.push(format!(
            "{subject} — the snapshot of the view it invokes (Table 344's /Snapshot) is displayed \
             in place of its appearance while this window's player holds the clip, fitted in \
             /Rect at a placement this program chose (ISO 32000-2 §13.7.2.3.5)"
        ));
    }
    outcome.notes.extend(
        clip.unhonoured
            .iter()
            .map(|entry| format!("{subject} — this program's player sets aside {entry}")),
    );
    outcome.media.push(MediaRequest::Play(
        Box::new(clip),
        Some(crate::Associated {
            annotation,
            or_resume: true,
        }),
    ));
}

/// [`crate::Command::InHand`]: one of the window's players has taken a clip associated with
/// `annotation`, or the clip has left its hands. Answers whether the page changed.
///
/// The annotation is active while any player holds such a clip, and the snapshot of the view its
/// activation invokes is displayed for that long — ISO 32000-2 §13.7.2.3.5's "displayed in place
/// of the current artwork" — and taken away once no player does, Table 336's deactivation having
/// reached the page by the only route that sees each one (ADR 3889). An annotation that is not a
/// rich media annotation, or whose view names no image, changes nothing on the page.
pub(crate) fn in_hand(open: &mut Open, annotation: ObjectId, in_hand: bool) -> bool {
    let holders = open.in_hand.entry(annotation).or_insert(0);
    *holders = if in_hand {
        holders.saturating_add(1)
    } else {
        holders.saturating_sub(1)
    };
    let held = *holders > 0;
    if !held {
        open.in_hand.remove(&annotation);
    }
    if held == open.view.shows_snapshot(annotation) {
        return false;
    }
    let object = open.document.get(annotation);
    let names_an_image = object.as_dict().is_some_and(|dict| {
        rich_media::invoked_view(&open.document, dict).is_some_and(|view| view.snapshot.is_some())
    });
    (names_an_image || !held) && open.view.show_snapshot(annotation, held)
}
