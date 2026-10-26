//! What the worker says: a command sent, the events it answers with, and what each one does to this window.
//!
//! Methods of [`crate::Host`], which this file continues (ADR 2352).

use std::time::Instant;

use viewer_core::{Command, Event};
use viewer_host::Ask;
use viewer_host::trace::Topic;
use viewer_ui::chrome::Chrome;

use crate::arguments::{brief, file_name};
use crate::exchange::Held;
use crate::{DOCUMENT, Host};

impl Host {
    /// Sends one command, hands its events on, and pulls the frame the view now shows — or, while
    /// the worker is busy, holds it to be sent once the worker answers (ADR 1827).
    pub(crate) fn dispatch(&mut self, command: Command) {
        if self.stopped.is_some() {
            return;
        }
        if self.must_hold() {
            self.hold(Held::Command {
                command,
                at: Instant::now(),
            });
            return;
        }
        self.send(command);
    }

    /// What this window does about each event the confined viewer sent back.
    ///
    /// Exhaustive on purpose: a message added to the boundary must fail to compile here rather
    /// than fall into a catch-all arm (`doc/ui-boundary.md`'s rule).
    #[expect(
        clippy::too_many_lines,
        reason = "one arm per variant of `viewer_core::Event`, and the count is that enum's. \
                  Splitting it would lose the property the match rests on: a message added to \
                  the core is named here rather than swallowed by a catch-all"
    )]
    pub(crate) fn event(&mut self, event: Event) {
        if self.trace.on(Topic::Events) {
            self.trace
                .say(Topic::Events, format_args!("<- {}", brief(&event)));
        }
        match event {
            Event::Opened { pages, .. } => {
                // A file a document named and this window supplied is the window's document now.
                self.opened_an_offer();
                // The next document's attempts start from nothing (`Asking::opened`'s rule).
                self.asking.opened();
                self.report_due.opened();
                // A document opens where its own view says, which the viewer states as a
                // `PageChanged` when it is not the first page. Until it does, this is page one.
                self.page = 0;
                self.heading = format!("{} — {pages} page(s)", self.name());
                self.retitle();
            }
            // This window sends no `Command::Print`, so the grant never arrives — and it is
            // answered all the same, in the direction that says so, for the reason the two
            // levels below are: an operation added to this window later may not behave like
            // another one in silence. What the arm would mean if it did arrive is RFC 0004 §6's
            // preview: the worker interprets for paper and this window shows what it sends
            // (ADR 1180).
            Event::Printing { pages, .. } => {
                eprintln!(
                    "note: the confined viewer granted a print operation over {pages} page(s), \
                     which this window never asked for"
                );
            }
            // The same for a conversion's grant: `Command::Archive` never crosses (ADR 1954).
            Event::Archiving { .. } => {
                eprintln!(
                    "note: the confined viewer granted a conversion this window never asked for"
                );
            }
            Event::OpenFailed { reason, .. } => {
                // The worker survives an open it refused; only this document is over.
                self.stop(format!("cannot open {}: {reason}", self.path.display()));
            }
            Event::PasswordRequired { .. } => {
                // The file a document named, where it is the one that asks (ADR 1332).
                self.locked_an_offer();
                self.password_required();
            }
            // Five arms with nothing to do, and each for its own reason, kept in one place so
            // that a message added to the boundary still breaks this build: a `Closed` follows
            // this window's own `Close` and nothing else; a `Transition` never fires because this
            // window sends no `Command::Tick` and starts no presentation; a `Searched` answers a
            // `Find` this window never sends; a `Dirty` marks a title this window does not mark,
            // because its title is the confinement's sentence; and an `AttachmentsChanged` names a
            // list this window shows no panel for (ADR 0713).
            Event::Closed(_)
            | Event::Transition { .. }
            | Event::Dirty { .. }
            | Event::Searched { .. }
            | Event::AttachmentsChanged { .. } => {}
            Event::PageChanged {
                index,
                label,
                of,
                section,
                ..
            } => {
                self.page = index;
                let name = self.name();
                let page = match label.as_deref().map(viewer_host::documents::one_line) {
                    Some(label) => format!("page {label} ({} of {of})", index.saturating_add(1)),
                    None => format!("page {} of {of}", index.saturating_add(1)),
                };
                // §12.3.3's section, which the worker reads beside page one and says again once
                // it has (ADR 1553): the caption the other three windows show, on one line as
                // theirs is (ADR 1973).
                self.heading = match section.as_deref().map(viewer_host::documents::one_line) {
                    Some(section) if !section.is_empty() => format!("{name} — {page} — {section}"),
                    _ => format!("{name} — {page}"),
                };
                self.retitle();
            }
            // The render round trip is the worker's own; this event never crosses the boundary
            // (`viewer_confined`'s module documentation). Arriving here would mean the protocol
            // changed under this window, which is worth a sentence and not a crash.
            Event::NeedsRender(_) => {
                eprintln!("note: a render request crossed the confined boundary; ignored");
            }
            Event::Damage(_) => {
                self.ask_frame();
                self.redraw();
            }
            // §12.6.4.8: the same policy the other three windows ask, at the level this window was
            // started at, and the *ask* level's question on the card (ADRs 1079, 1155, 2040).
            Event::OpenUri { uri, .. } => self.open_uri(&uri),
            // §12.7.6.2: composed by the confined process and sent from this one, at the level
            // `--submissions=` set and on the card at `ask` (ADRs 1291, 2040, 2076).
            Event::Submit {
                document,
                submission,
            } => self.submit(document, *submission),
            // A file the document asks for is a question about *this* machine's filesystem
            // (rule 2), and this window supplies none: said, not swallowed — and **answered**,
            // which is the half that was missing. `Command::Supply` with no bytes is what tells
            // the confined viewer that nobody will supply it, so the action it is holding is
            // dropped and the person is told the link declined; without it the worker waits for
            // an answer that never comes and the click reports nothing (trap 5).
            //
            // §12.6.4.3's file and the three purposes beside it are asked at the level
            // `--remote-documents=` set, as in the other three windows (ADRs 1227, 1239, 2040);
            // §12.7.6.4's own file is the rest of this arm.
            //
            // An import named by an absolute URI is fetched from this side, under the level a
            // submission is sent at, as in the other three windows (ADRs 1527, 2076).
            Event::NeedsFile { purpose, name, .. }
                if viewer_host::under_remote_documents(purpose) =>
            {
                self.needs_remote(purpose, &name);
            }
            Event::NeedsFile {
                document,
                purpose: viewer_core::Purpose::ImportData,
                name,
                ..
            } if viewer_host::import_is_fetched(&name) => self.fetch_import(document, name),
            Event::NeedsFile { purpose, name, .. } => {
                let why = format!(
                    "this window supplies no file a document names, and has no dialogue to ask \
                     you about one. The document asked for {name}"
                );
                eprintln!("{}", viewer_host::policy::supply_note(purpose, &why));
                self.dispatch(Command::Supply {
                    purpose,
                    bytes: None,
                });
            }
            // §12.8.3.4.4's policy, whose published copy is a server's: declined at every level
            // here, because what a fetched copy is for is a document opened beside the one that
            // named it, and this window holds one (ADRs 1738, 2076).
            Event::SignaturePoliciesPublished { policies, .. } => {
                for policy in &policies {
                    eprintln!(
                        "{}",
                        viewer_host::policy::signature_policy_declined(
                            policy,
                            "this window holds one document and opens no copy beside it, so it \
                             fetches none, at any level"
                        )
                    );
                }
            }
            // §O.2.1's `ef` at `EmbeddedDocuments::Refuse`, because this window holds one document
            // and opens none beside it (ADR 1331). It passes no fragment, so the arm is the policy's answer said
            // out loud should one ever arrive, rather than a window that opens it unasked.
            Event::Extracted {
                asked, name, bytes, ..
            } if viewer_host::opens_as_document(asked, &bytes) => {
                let why = match viewer_host::may_open_extracted(
                    asked,
                    &name,
                    viewer_host::EmbeddedDocuments::Refuse,
                ) {
                    viewer_host::Unpacking::Refuse(why) => why,
                    // Unreachable at this level and said rather than ignored (trap 5).
                    viewer_host::Unpacking::Ask(words) => {
                        viewer_host::unanswerable(&[words.reasons])
                    }
                    viewer_host::Unpacking::Open | viewer_host::Unpacking::Warn(_) => {
                        viewer_host::embedded_note(
                            &name,
                            Some("this window holds one document and opens none beside it"),
                        )
                    }
                };
                eprintln!("note: {why}");
            }
            Event::Saved { bytes, .. } => self.write_saved(&bytes),
            Event::Extracted { .. } => {
                eprintln!("note: the confined viewer sent bytes this window never asked for");
            }
            Event::Refused { notes, .. } => {
                for note in notes {
                    eprintln!("refused by the document's restrictions: {note}");
                }
            }
            // The other two of `CLAUDE.md`'s four levels (ADR 0814). This window sets no level,
            // so each operation stands at the default, `off`, and neither arrives; both are
            // answered all the same, in the direction that obeys, so that a level added to this
            // window later cannot behave like another one in silence.
            Event::Warned { notes, .. } => {
                eprintln!("note: {}", viewer_host::warned(&notes));
            }
            // This window has no copy gesture, so nothing here sends `Command::Copy` and this
            // cannot arrive; the arm exists because `doc/ui-boundary.md`'s rule is that a message
            // added to the core is named here rather than swallowed by a catch-all.
            Event::Copied {
                logical,
                page_order,
                ..
            } => {
                if let Some(copied) = viewer_host::copied(logical, &page_order) {
                    eprintln!(
                        "note: copied {} characters in {}",
                        copied.text.chars().count(),
                        copied.order
                    );
                }
            }
            // And the third, for the reason the arms above give. Two of this window's operations
            // can be restricted — a click that toggles a check box fills a field, and one on a
            // rendition hands its clip over (ADR 1987) — but it sets no level, so both stand at
            // `off` and no question arrives. The other three windows put the question on a
            // dialogue of their own (ADR 1145); what this one owes is that a level never behaves
            // like another in silence, which `viewer_host::unanswerable` is the sentence for.
            Event::Asking {
                document, notes, ..
            } => {
                eprintln!("note: {}", viewer_host::unanswerable(&notes));
                self.dispatch(Command::Answer {
                    document,
                    proceed: false,
                });
            }
            // RFC 0008 section 6.3's question cannot arrive: this window is pinned to `off` and
            // hands its worker no runner. Answered `no` where it does, for the arm above's reason
            // (ADR 1616).
            Event::AskingToRunScripts { document, .. } => {
                eprintln!(
                    "note: {SCRIPTS_PINNED}",
                    SCRIPTS_PINNED = viewer_host::SCRIPTS_PINNED
                );
                self.dispatch(Command::AnswerScripts {
                    document,
                    proceed: false,
                });
            }
            // RFC 0008 section 4.2's `app.alert` and `app.response` cannot arrive either, for the
            // arm above's reason; this window has no dialogue for a script, so it refuses by name
            // and answers as a closed dialogue does, rather than hold the script until its wait
            // runs out (ADR 1628).
            Event::ScriptAsking { document, .. } => {
                eprintln!("note: {}", viewer_host::script_asks::UNANSWERABLE);
                self.dispatch(Command::AnswerScript {
                    document,
                    answer: viewer_core::ScriptAnswer::Unanswerable,
                });
            }
            // A question is answered the moment it arrives, so none stands for a withdrawal to
            // take down; the arm says so rather than receiving a newer worker's event in silence
            // (ADR 1643).
            // Pinned to `off`, this window's worker runs no script that could ask for a sound; the
            // arm says so rather than receiving a newer worker's event in silence (ADR 1702).
            Event::Beep { document, sound } => {
                eprintln!(
                    "note: {}",
                    viewer_host::script_timers::unplayed(
                        &format!("document {}", document.0),
                        sound,
                        "this window has no sound for a document's script"
                    )
                );
            }
            // §13.2.4.2's clip, which the worker judged and this window was handed: no player is
            // started yet, said (ADR 1961).
            Event::Media {
                document,
                content_type,
                media,
            } => {
                eprintln!(
                    "note: {}",
                    viewer_host::handed_media(
                        &format!("document {}", document.0),
                        content_type.as_deref(),
                        &media
                    )
                );
            }
            // Pinned to `off` as for a sound; the arm says so rather than receiving a newer
            // worker's event in silence (ADR 1771).
            Event::Console { document, request } => {
                eprintln!(
                    "note: {}",
                    viewer_host::script_timers::console(
                        &format!("document {}", document.0),
                        request
                    )
                );
            }
            Event::ScriptQuestionWithdrawn { .. } => {
                eprintln!(
                    "note: a script's question was withdrawn, and this window had answered it \
                     already: it has no dialogue for a document's script"
                );
            }
            Event::Reported { page, notes, .. } => {
                for note in notes {
                    match page {
                        Some(page) => {
                            eprintln!("page {}: {note}", page.saturating_add(1));
                        }
                        None => eprintln!("document: {note}"),
                    }
                }
            }
        }
    }

    /// What a draw here of a page whose marks crossed declined, told to the confined viewer —
    /// which says each condition once, as an event — and named in the title (ADR 2290).
    pub(crate) fn declined_in(&mut self, page: usize, declined: Vec<viewer_core::Declined>) {
        if declined.is_empty() {
            return;
        }
        self.dispatch(Command::Declined {
            document: DOCUMENT,
            page,
            declined,
        });
        if self.declined.insert(page) {
            self.retitle();
        }
    }

    /// §7.6.4.1's prompt, which is the one arm of [`Self::event`] long enough to be its own.
    ///
    /// "an interactive PDF processor *should* prompt", and this window is one (ADR 0718; the
    /// refusal it replaces was ADR 0713's scope decision). The policy — how many attempts, what
    /// exhaustion means — is `viewer_host::password`'s, shared with the three established
    /// windows.
    fn password_required(&mut self) {
        if self.chrome.is_none() {
            match Chrome::new() {
                Ok(chrome) => self.chrome = Some(chrome),
                Err(said) => {
                    // A build whose compiled-in faces will not parse cannot draw the card;
                    // refusing by name is what is left (trap 5).
                    self.stop(format!(
                        "{} is encrypted and this build cannot draw the prompt ({said}); open \
                         it in quorra, quorra-gtk or quorra-qt",
                        self.path.display()
                    ));
                    return;
                }
            }
        }
        match self.asking.required() {
            Ask::Prompt { attempt, of } => {
                self.password.ask(viewer_host::password::prompt(
                    &file_name(self.prompted_for()),
                    attempt,
                    of,
                ));
                self.card_changed();
            }
            Ask::Exhausted => {
                // Stop asking and leave the window open — `viewer_host::password`'s rule: no
                // host may make `Exhausted` mean the window closes. A file a document named is
                // declined to the worker that holds it, and the window's own stays.
                eprintln!("note: {}", viewer_host::password::EXHAUSTED);
                if self.decline_the_prompted() {
                    return;
                }
                self.heading = format!("{} — {}", self.name(), viewer_host::password::EXHAUSTED);
                self.retitle();
            }
        }
    }
}
