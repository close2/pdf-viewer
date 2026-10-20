//! What a document asks of this machine, under the levels the window holds, and the one question a
//! person answers at `CLAUDE.md`'s *ask* level (ADRs 1155, 1227, 1291, 1331, 1527, 1616, 1738;
//! ADR 2088 for this window).
//!
//! **Every decision here is `viewer_host`'s**: `may_submit`, `link`, `remote`, `may_open_extracted`
//! and the rest say whether an act happens and in what words, for all five windows. What this
//! module holds is the carrying-out — the act, the sentence on the status line, and the question
//! put to the host as an [`Owed::Question`] — which is the part `viewer-gtk` and `viewer-qt` each
//! hold beside their own dialogue.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use viewer_core::{Command, DocumentId, Purpose};
use viewer_host::native_window::NativeWindow;

use super::Window;
use super::owed::Owed;

/// What the one outstanding question is about.
///
/// **One question at a time**, which is `viewer_core::Command::Answer`'s own rule — a second
/// question replaces the first — and closed, so that a subject cannot be answered by another's
/// branch (ADR 1155).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Pending {
    /// An operation the viewer is holding, answered by `Command::Answer`.
    Restricted {
        /// Which document it is holding it for.
        document: DocumentId,
        /// Which operation the document restricts.
        operation: pdf_model::restriction::Operation,
    },
    /// §12.6.4.8's resolved URI, which no core is holding: starting a program is the host's act.
    Link {
        /// The URI as `viewer_host::resolve_uri` left it.
        uri: String,
    },
    /// §12.7.6.2's composed request (ADR 1291).
    Submit {
        /// The document whose form it is.
        document: DocumentId,
        /// The request exactly as `pdf_model::submission::compose` made it.
        submission: Box<pdf_model::submission::Submission>,
    },
    /// An import's form data named by an absolute URI (ADR 1527).
    Fetch {
        /// The document that asked.
        document: DocumentId,
        /// The URI as the action named it.
        url: String,
    },
    /// A signature policy's published copy (ADR 1738).
    Policy {
        /// The document whose signature named it.
        document: DocumentId,
        /// What the signer committed to.
        policy: Box<viewer_host::PublishedPolicy>,
    },
    /// A file a document named, answered by reading it and supplying it (ADRs 1227, 1239).
    RemoteDocument {
        /// Which purpose asked.
        purpose: Purpose,
        /// The file as the document named it.
        name: String,
        /// Where `viewer_host::remote` put it.
        path: PathBuf,
    },
    /// §O.2.1's embedded document, answered by opening it in a tab of its own (ADR 1331).
    Embedded {
        /// The document holding it.
        document: DocumentId,
        /// The name the document filed it under.
        name: String,
        /// Its bytes.
        bytes: Vec<u8>,
        /// What followed `ef` in the fragment.
        fragment: Option<String>,
    },
    /// RFC 0008 section 6.3's one question per document (ADR 1616).
    Scripts {
        /// The document whose scripts are waiting.
        document: DocumentId,
    },
}

impl Window {
    /// Holds the question and owes the host a dialogue for it, its two paragraphs joined by an
    /// empty line as `viewer-qt` joins them for its one label.
    pub(super) fn put_the_question(&mut self, about: Pending, words: &viewer_host::Question) {
        let text = format!("{}\n\n{}", words.reasons, words.choice);
        self.question = Some(about);
        self.owing.owe(Owed::Question(text));
    }

    /// What the person answered. The decline is said out loud, because `viewer-core` says nothing
    /// on a `no` (ADR 0814) and a person is still owed the fact that what they asked for did not
    /// happen; taking the question is what makes a dialogue closed twice answer once.
    ///
    /// # Errors
    ///
    /// [`crate::Status::NoAnswer`] where no question is waiting.
    pub fn answer(&mut self, proceed: bool) -> Result<(), crate::Status> {
        let about = self.question.take().ok_or(crate::Status::NoAnswer)?;
        let refused = || format!("you answered \"{}\"", viewer_host::restriction::DO_NOT);
        match about {
            Pending::Embedded {
                document,
                name,
                bytes,
                fragment,
            } => {
                if proceed {
                    self.open_embedded(document, &name, bytes, fragment);
                } else {
                    self.say(&viewer_host::embedded_declined(&name));
                }
            }
            Pending::Restricted {
                document,
                operation,
            } => {
                if !proceed {
                    self.say(&viewer_host::declined(operation));
                }
                self.dispatch(Command::Answer { document, proceed });
                // §12.11.6's `no` puts the document down unopened, so one on its way to a tab
                // never arrives and the next may start.
                if !proceed && operation == pdf_model::restriction::Operation::Process {
                    self.given_up(document);
                }
            }
            // The act is the host's own: this window says what the answer was, and starting the
            // program on the URI is the caller's, as it is every window's.
            Pending::Link { uri } => self.say(&viewer_host::answered(&uri, proceed)),
            Pending::Scripts { document } => {
                self.say(&viewer_host::scripts_answered(proceed));
                self.dispatch(Command::AnswerScripts { document, proceed });
            }
            Pending::Submit {
                document,
                submission,
            } => {
                if proceed {
                    self.send_form(document, *submission, None);
                } else {
                    self.say(&viewer_host::policy::submission_note(
                        &submission,
                        Some(&refused()),
                    ));
                }
            }
            Pending::Fetch { document, url } => {
                if proceed {
                    self.start_fetch(document, url, None);
                } else {
                    self.say(&viewer_host::fetch_note(&url, Some(&refused())));
                    self.dispatch(Command::Supply {
                        purpose: Purpose::ImportData,
                        bytes: None,
                    });
                }
            }
            Pending::Policy { document, policy } => {
                if proceed {
                    self.start_policy_fetch(document, *policy, None);
                } else {
                    self.say(&viewer_host::policy::signature_policy_declined(
                        &policy,
                        &refused(),
                    ));
                }
            }
            Pending::RemoteDocument {
                purpose,
                name,
                path,
            } => {
                let mut queue = VecDeque::new();
                let bytes = if proceed {
                    self.read_remote(purpose, &name, &path)
                } else {
                    self.say(&viewer_host::remote_declined(purpose, &name));
                    None
                };
                if let Some(read) = bytes.as_deref() {
                    self.offer_a_name(&path, read, purpose, &mut queue);
                }
                queue.push_back(Command::Supply { purpose, bytes });
                self.drain(queue);
            }
        }
        Ok(())
    }

    /// §12.7.6.2's composed request, under the level the window holds (ADR 1291).
    pub(super) fn submit_under_the_level(
        &mut self,
        document: DocumentId,
        submission: pdf_model::submission::Submission,
    ) {
        match viewer_host::may_submit(&submission, self.state.restrictions.submissions()) {
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

    /// A signature policy's published copy, under the level the window holds (ADR 1738). A policy
    /// that would be asked about while another question waits is declined with that reason rather
    /// than put in its place, which would leave the first unanswered and unsaid.
    pub(super) fn fetch_policy_under_the_level(
        &mut self,
        document: DocumentId,
        policy: viewer_host::PublishedPolicy,
    ) {
        use viewer_host::policy::{
            asked_to_fetch_policy, may_fetch_signature_policy, signature_policy_declined,
        };
        match may_fetch_signature_policy(&policy, self.state.restrictions.submissions()) {
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

    /// §12.6.4.8's URI, resolved against this document's location and then said, or put to the
    /// person, at the level the window holds (ADR 1155).
    pub(super) fn link_under_the_level(&mut self, uri: &str) {
        let uri = viewer_host::resolve_uri(Some(&self.state.showing.path), uri);
        match viewer_host::link(&uri, self.links) {
            viewer_host::Link::Say(note) => self.say(&note),
            viewer_host::Link::Ask(words) => self.put_the_question(Pending::Link { uri }, &words),
        }
    }

    /// A file a document named, routed by which policy decides it (ADRs 1227, 1239, 1527).
    pub(super) fn file_under_the_level(
        &mut self,
        document: DocumentId,
        purpose: Purpose,
        name: &str,
        beside: bool,
        queue: &mut VecDeque<Command>,
    ) {
        if viewer_host::under_remote_documents(purpose) {
            self.remote(purpose, name, beside, queue);
        } else if purpose == Purpose::ImportData && viewer_host::import_is_fetched(name) {
            self.fetch_import(document, name.to_owned(), queue);
        } else {
            self.import(purpose, name, queue);
        }
    }

    /// §7.11.4's file or §O.2.1's, taken out of the document: written beside it, or opened in a
    /// tab of its own under the level the window holds (ADR 1331).
    pub(super) fn extracted_under_the_level(
        &mut self,
        document: DocumentId,
        asked: viewer_core::Extraction,
        name: &str,
        bytes: Vec<u8>,
        fragment: Option<String>,
    ) {
        if !viewer_host::opens_as_document(asked, &bytes) {
            self.write_extracted(asked, name, &bytes);
            return;
        }
        match viewer_host::may_open_extracted(
            asked,
            name,
            self.state.restrictions.embedded_documents(),
        ) {
            viewer_host::Unpacking::Open => self.open_embedded(document, name, bytes, fragment),
            viewer_host::Unpacking::Warn(note) => {
                self.say(&note);
                self.open_embedded(document, name, bytes, fragment);
            }
            viewer_host::Unpacking::Refuse(why) => self.say(&why),
            viewer_host::Unpacking::Ask(question) => self.put_the_question(
                Pending::Embedded {
                    document,
                    name: name.to_owned(),
                    bytes,
                    fragment,
                },
                &question,
            ),
        }
    }

    /// Table Annex O.4's `fdf` naming an absolute URI, under the level a submission is sent at
    /// (ADR 1527). A refusal is said and supplied as `None`, so the core says the import declined.
    fn fetch_import(&mut self, document: DocumentId, url: String, queue: &mut VecDeque<Command>) {
        match viewer_host::may_fetch_import(&url, self.state.restrictions.submissions()) {
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

    /// Puts the GET on a thread of its own; the answer is looked for on the window's tick.
    fn start_fetch(&mut self, document: DocumentId, url: String, warned: Option<String>) {
        self.say(&viewer_host::fetch_note(&url, None));
        if let Err(sentence) = self.state.submitter.fetch(document, url, warned, None) {
            self.say(&sentence);
        }
    }

    /// §12.7.6.4's file, under the narrowest policy that performs the action: no level, and the
    /// one rule `viewer_host::policy::read_import` states. A refusal is said (trap 5).
    fn import(&mut self, purpose: Purpose, name: &str, queue: &mut VecDeque<Command>) {
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

    /// §12.6.4.3's file, under the level the window was opened at (ADR 1227).
    fn remote(
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
                if let Some(read) = bytes.as_deref() {
                    if let Some(note) = note {
                        self.say(&note);
                    }
                    self.offer_a_name(&path, read, purpose, queue);
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
    fn read_remote(&mut self, purpose: Purpose, name: &str, path: &Path) -> Option<Vec<u8>> {
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
