//! What a document asks of this machine: §12.7.6.2's submission, Annex O's and a signature
//! policy's fetches, §12.7.6.4's import, §12.6.4.3's remote file and §12.6.4.8's link — each
//! under the level the window holds, each answer looked for on Qt's timer (ADRs 1227, 1291, 1527,
//! 1738; filed here by ADR 1908).

use std::collections::VecDeque;
use std::path::Path;

use viewer_core::{Command, DocumentId, Purpose};
use viewer_host::native_window::NativeWindow;

use super::Host;
use super::questions::Pending;

impl Host {
    /// §12.7.6.2's composed request, under the level the menu holds (ADR 1291).
    pub(super) fn submit(
        &mut self,
        document: DocumentId,
        submission: pdf_model::submission::Submission,
    ) {
        match viewer_host::may_submit(&submission, self.restrictions.submissions()) {
            viewer_host::Sending::Send => self.send_form(document, submission, None),
            viewer_host::Sending::Warn(note) => self.send_form(document, submission, Some(note)),
            viewer_host::Sending::Ask(words) => self.put_the_question(
                Pending::Submit {
                    document,
                    submission: Box::new(submission),
                },
                &words,
            ),
            viewer_host::Sending::Refuse(why) => self.say(&viewer_host::policy::submission_note(
                &submission,
                Some(&why),
            )),
        }
    }

    /// Table Annex O.4's `fdf` naming an absolute URI, under the level the menu holds — the one
    /// answer `viewer_host::may_fetch_import` gives (ADR 1527). A refusal is said and supplied as
    /// `None`, so the core says the import declined; a fetch arrives as `Command::Respond` by
    /// document, whichever is in front by then.
    pub(super) fn fetch_import(
        &mut self,
        document: DocumentId,
        url: String,
        queue: &mut VecDeque<Command>,
    ) {
        match viewer_host::may_fetch_import(&url, self.restrictions.submissions()) {
            viewer_host::Sending::Send => self.start_fetch(document, url, None),
            viewer_host::Sending::Warn(note) => self.start_fetch(document, url, Some(note)),
            viewer_host::Sending::Ask(words) => {
                self.put_the_question(Pending::Fetch { document, url }, &words);
            }
            viewer_host::Sending::Refuse(why) => {
                self.say(&viewer_host::fetch_note(&url, Some(&why)));
                queue.push_back(Command::Supply {
                    purpose: Purpose::ImportData,
                    bytes: None,
                });
            }
        }
    }

    /// A signature policy's published copy, under the level the menu holds — the one answer
    /// `viewer_host::policy::may_fetch_signature_policy` gives (ADR 1738).
    ///
    /// **One question at a time**, because this window has one dialogue: a policy that would be
    /// asked about while another question waits is declined with that reason rather than put in
    /// its place, which would leave the first unanswered and unsaid.
    pub(super) fn fetch_policy(
        &mut self,
        document: DocumentId,
        policy: viewer_host::PublishedPolicy,
    ) {
        use viewer_host::policy::{
            asked_to_fetch_policy, may_fetch_signature_policy, signature_policy_declined,
        };
        match may_fetch_signature_policy(&policy, self.restrictions.submissions()) {
            viewer_host::Sending::Send => self.start_policy_fetch(document, policy, None),
            viewer_host::Sending::Warn(note) => {
                self.start_policy_fetch(document, policy, Some(note));
            }
            viewer_host::Sending::Ask(_) if self.question.is_some() => {
                self.say(&signature_policy_declined(
                    &policy,
                    "another question was already waiting for your answer",
                ));
            }
            viewer_host::Sending::Ask(_) => {
                let words = asked_to_fetch_policy(&policy);
                self.put_the_question(
                    Pending::Policy {
                        document,
                        policy: Box::new(policy),
                    },
                    &words,
                );
            }
            viewer_host::Sending::Refuse(why) => {
                self.say(&signature_policy_declined(&policy, &why));
            }
        }
    }

    /// Puts the GET on a `fetch-import` thread of its own; the answer is looked for on the drawing
    /// timer, as a submission's is.
    pub(super) fn start_fetch(
        &mut self,
        document: DocumentId,
        url: String,
        warned: Option<String>,
    ) {
        self.say(&viewer_host::fetch_note(&url, None));
        if let Err(sentence) = self.state.submitter.fetch(document, url, warned, None) {
            self.say(&sentence);
        }
    }

    /// §12.6.4.8's URI, resolved against this document's location and then opened, declined or put
    /// to the person at the level `--links=` set.
    pub(super) fn follow_the_link(&mut self, uri: &str) {
        let uri = viewer_host::resolve_uri(Some(&self.state.showing.path), uri);
        match viewer_host::link(&uri, self.links) {
            viewer_host::Link::Say(note) => self.say(&note),
            viewer_host::Link::Ask(words) => self.put_the_question(Pending::Link { uri }, &words),
        }
    }

    /// A file a *document* named, routed by which policy decides it: a second PDF to parse under
    /// the remote-documents level, an import named by an absolute URI fetched under the level a
    /// submission is sent at (ADR 1527), and any other import read (ADRs 1227, 1239).
    pub(super) fn needs_file(
        &mut self,
        document: DocumentId,
        purpose: Purpose,
        name: String,
        beside: bool,
        queue: &mut VecDeque<Command>,
    ) {
        if viewer_host::under_remote_documents(purpose) {
            self.remote(purpose, &name, beside, queue);
        } else if purpose == Purpose::ImportData && viewer_host::import_is_fetched(&name) {
            self.fetch_import(document, name, queue);
        } else {
            self.import(purpose, &name, queue);
        }
    }

    /// §12.7.6.4's file, under the narrowest policy that performs the action.
    ///
    /// No level: the clause makes performing an import a `shall` and what is admitted is the one
    /// rule `viewer_host::resolve_import` states — a relative path to a file in or below the open
    /// document's directory. A refusal is said rather than swallowed (trap 5).
    pub(super) fn import(&mut self, purpose: Purpose, name: &str, queue: &mut VecDeque<Command>) {
        let bytes =
            match viewer_host::policy::read_import(self.state.showing.directory.as_deref(), name) {
                Ok(bytes) => Some(bytes),
                Err(refusal) => {
                    self.say(&viewer_host::policy::supply_note(purpose, &refusal));
                    None
                }
            };
        queue.push_back(Command::Supply { purpose, bytes });
    }

    /// §12.6.4.3's file, under the level this window was started at.
    ///
    /// The policy is `viewer_host::remote`'s and not this window's, so a level a reader sets is a
    /// value there rather than three windows' worth of editing — ADR 1079's shape and ADR 1155's,
    /// one clause along. What is this window's is the dialogue and the status line (ADR 1227).
    pub(super) fn remote(
        &mut self,
        purpose: Purpose,
        name: &str,
        beside: bool,
        queue: &mut VecDeque<Command>,
    ) {
        match viewer_host::remote(
            self.state.showing.directory.as_deref(),
            name,
            self.remote_documents,
            purpose,
            beside,
        ) {
            viewer_host::Remote::Supply { path, note } => {
                let bytes = self.read_remote(purpose, name, &path);
                if bytes.is_some() {
                    if let Some(note) = note {
                        self.say(&note);
                    }
                    self.offer_a_name(&path, bytes.as_deref().unwrap_or_default(), purpose, queue);
                }
                queue.push_back(Command::Supply { purpose, bytes });
            }
            viewer_host::Remote::Ask { path, question } => self.put_the_question(
                Pending::RemoteDocument {
                    purpose,
                    name: name.to_owned(),
                    path,
                },
                &question,
            ),
            viewer_host::Remote::Refuse(why) => {
                self.say(&why);
                queue.push_back(Command::Supply {
                    purpose,
                    bytes: None,
                });
            }
        }
    }

    /// The bytes of a file `viewer_host::remote` has already decided on, or the sentence saying
    /// why there are none.
    pub(super) fn read_remote(
        &mut self,
        purpose: Purpose,
        name: &str,
        path: &Path,
    ) -> Option<Vec<u8>> {
        match std::fs::read(path) {
            Ok(bytes) => Some(bytes),
            Err(error) => {
                self.say(&viewer_host::remote_note(
                    purpose,
                    name,
                    Some(&format!("cannot read {}: {error}", path.display())),
                ));
                None
            }
        }
    }
}
