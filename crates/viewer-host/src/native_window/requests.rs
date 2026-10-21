//! What a document asks of this machine, carried out under the levels the window holds, and the one
//! question a person answers at `CLAUDE.md`'s *ask* level (ADRs 1155, 1227, 1291, 1331, 1527, 1616,
//! 1738, 2171; written once for the three windows by ADR 2105).
//!
//! **Every decision here was already this crate's** — `may_submit`, `link`, `remote`,
//! `may_open_extracted` and the rest say whether an act happens and in what words. What this module
//! adds is the carrying-out of the answer: the act, the sentence on the status line, and the
//! question put to the person as an [`Asked`]. `viewer-gtk`, `viewer-qt` and `viewer-ffi`'s
//! `quorra_window` each held that carrying-out beside their own dialogue, line for line, and the
//! third copy is where this crate takes a rule (`doc/todo/30`, habit 131).
//!
//! What stays a toolkit's is the dialogue: [`NativeWindow::put_the_question`] puts an [`Asked`] in
//! front of the person however the toolkit does that, and hands the answer to
//! [`NativeWindow::answer_the_question`]. Whether a second question may stand beside the first is
//! the toolkit's to state ([`NativeWindow::question_waiting`]), because GTK puts each in a window of
//! its own and the other two have one dialogue.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use viewer_core::{Command, DocumentId, Purpose};

use super::NativeWindow;

/// What the one outstanding question is about, at `CLAUDE.md`'s *ask* level.
///
/// **Closed**, so that a subject cannot be answered by another's branch (ADR 1155), and carried by
/// value to the dialogue and back, so that the answer acts on exactly what was asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Asked {
    /// An operation the viewer is holding, answered by `viewer_core::Command::Answer`.
    Restricted {
        /// Which document it is holding it for.
        document: DocumentId,
        /// Which operation the document restricts.
        operation: pdf_model::restriction::Operation,
    },
    /// §12.6.4.8's resolved URI, which no core is holding: starting a program is the host's act.
    Link {
        /// The URI as `crate::resolve_uri` left it.
        uri: String,
    },
    /// §12.7.6.2's composed request, answered by sending it or by saying it was not (ADR 1291).
    Submit {
        /// The document whose form it is, which is where an FDF answer goes.
        document: DocumentId,
        /// The request exactly as `pdf_model::submission::compose` made it.
        submission: Box<pdf_model::submission::Submission>,
    },
    /// An import's form data named by an absolute URI, answered by fetching it (ADR 1527).
    Fetch {
        /// The document that asked, which is where the answer is imported.
        document: DocumentId,
        /// The URI as the fragment or the action named it.
        url: String,
    },
    /// A signature policy's published copy, answered by fetching it (ADR 1738).
    Policy {
        /// The document whose signature named it, which a bound copy opens beside.
        document: DocumentId,
        /// The URL, the identifier and the digest the signer committed to.
        policy: Box<crate::PublishedPolicy>,
    },
    /// A file a document named, answered by reading it and supplying it (ADRs 1227, 1239).
    RemoteDocument {
        /// Which of the three purposes asked, so that the answer goes back to the right one.
        purpose: Purpose,
        /// The file as the *document* named it, for the sentence a decline says.
        name: String,
        /// Where `crate::remote` put it, which is what would be read.
        path: PathBuf,
    },
    /// §12.6.4.6's launched document that is not a PDF, answered by handing it to this machine's
    /// own handler (ADR 2171).
    LaunchedDocument {
        /// The file as the *document* named it, for the sentence either answer says.
        name: String,
        /// Where `crate::remote` put it, which is what would be handed over.
        path: PathBuf,
    },
    /// §O.2.1's embedded document, answered by opening it in a tab of its own (ADR 1331).
    Embedded {
        /// The document holding it.
        document: DocumentId,
        /// The name the document filed it under.
        name: String,
        /// Its bytes, out of the holding document.
        bytes: Vec<u8>,
        /// What followed `ef` in the fragment, which applies to it.
        fragment: Option<String>,
    },
    /// RFC 0008 section 6.3's one question per document, answered by
    /// `viewer_core::Command::AnswerScripts` (ADR 1616).
    Scripts {
        /// The document whose scripts are waiting.
        document: DocumentId,
    },
}

impl Asked {
    /// What the question's window is called by, which is what it is about.
    #[must_use]
    pub const fn subject(&self) -> crate::Subject {
        match self {
            Self::Restricted { .. } => crate::Subject::Restricted,
            Self::Link { .. } => crate::Subject::Link,
            Self::Submit { .. } => crate::Subject::Submission,
            Self::Fetch { .. } => crate::Subject::Fetch,
            Self::Policy { .. } => crate::Subject::Policy,
            Self::RemoteDocument { .. } | Self::LaunchedDocument { .. } => crate::Subject::Document,
            Self::Embedded { .. } => crate::Subject::Embedded,
            Self::Scripts { .. } => crate::Subject::Scripts,
        }
    }
}

/// What the person answered.
///
/// The decline is said out loud, because `viewer-core` says nothing on a `no` (ADR 0814) and a
/// person is still owed the fact that what they asked for did not happen.
pub(super) fn answer<W: NativeWindow + ?Sized>(window: &mut W, about: Asked, proceed: bool) {
    let refused = || format!("you answered \"{}\"", crate::restriction::DO_NOT);
    match about {
        // §O.2.1: the act is the host's own, so the answer decides whether the file opens at all
        // (ADR 1331).
        Asked::Embedded {
            document,
            name,
            bytes,
            fragment,
        } => {
            if proceed {
                window.open_embedded(document, &name, bytes, fragment);
            } else {
                window.say(&crate::embedded_declined(&name));
            }
        }
        Asked::Restricted {
            document,
            operation,
        } => {
            if !proceed {
                window.say(&crate::declined(operation));
            }
            window.dispatch(Command::Answer { document, proceed });
            // §12.11.6's `no` puts the document down unopened, so one on its way to a tab never
            // arrives and the next may start.
            if !proceed && operation == pdf_model::restriction::Operation::Process {
                window.given_up(document);
            }
        }
        // §12.6.4.8: the act is the host's own rather than an edit the core is holding, so what
        // the answer decides is whether the URI reaches another program (ADR 1155).
        Asked::Link { uri } => window.say(&crate::answered(&uri, proceed)),
        // RFC 0008 section 6.3: the viewer holds the document's scripts, so the answer goes to it
        // whichever it is (ADR 1616).
        Asked::Scripts { document } => {
            window.say(&crate::scripts_answered(proceed));
            window.dispatch(Command::AnswerScripts { document, proceed });
        }
        // §12.7.6.2: the answer decides whether the request leaves this machine at all (ADR 1291).
        Asked::Submit {
            document,
            submission,
        } => {
            if proceed {
                window.send_form(document, *submission, None);
            } else {
                window.say(&crate::policy::submission_note(
                    &submission,
                    Some(&refused()),
                ));
            }
        }
        // Table Annex O.4's `fdf` from a server: a `no` fetches nothing, and the core is told so
        // the import says it declined (ADR 1527).
        Asked::Fetch { document, url } => {
            if proceed {
                start_fetch(window, document, url, None);
            } else {
                window.say(&crate::fetch_note(&url, Some(&refused())));
                window.dispatch(Command::Supply {
                    purpose: Purpose::ImportData,
                    bytes: None,
                });
            }
        }
        // A signature policy's copy from a server: a `no` fetches nothing and says so (ADR 1738).
        Asked::Policy { document, policy } => {
            if proceed {
                window.start_policy_fetch(document, *policy, None);
            } else {
                window.say(&crate::policy::signature_policy_declined(
                    &policy,
                    &refused(),
                ));
            }
        }
        // §12.6.4.3: the act is opening a document in place of this one, so a `no` supplies
        // nothing and the core says the link declined (ADR 1227).
        Asked::RemoteDocument {
            purpose,
            name,
            path,
        } => {
            let mut queue = VecDeque::new();
            let bytes = if proceed {
                read_remote(window, purpose, &name, &path)
            } else {
                window.say(&crate::remote_declined(purpose, &name));
                None
            };
            if let Some(read) = bytes.as_deref() {
                window.offer_a_name(&path, read, purpose, &mut queue);
                if purpose == Purpose::LaunchDocument {
                    launched_elsewhere(window, &name, &path, read);
                }
            }
            queue.push_back(Command::Supply { purpose, bytes });
            window.drain(queue);
        }
        // §12.6.4.6: the act is the host's own, handing a file to another program, so the answer
        // decides whether it is handed over at all (ADR 2171).
        Asked::LaunchedDocument { name, path } => {
            if proceed {
                hand_over(window, &name, &path, &crate::policy::handed(&name));
            } else {
                window.say(&crate::remote_declined(Purpose::LaunchDocument, &name));
            }
        }
    }
}

/// §12.7.6.2's composed request, under the level the window holds (ADR 1291).
pub(super) fn submit<W: NativeWindow + ?Sized>(
    window: &mut W,
    document: DocumentId,
    submission: pdf_model::submission::Submission,
) {
    match crate::may_submit(&submission, window.state().restrictions.submissions()) {
        crate::Sending::Send => window.send_form(document, submission, None),
        crate::Sending::Warn(note) => window.send_form(document, submission, Some(note)),
        crate::Sending::Ask(words) => window.put_the_question(
            Asked::Submit {
                document,
                submission: Box::new(submission),
            },
            &words,
        ),
        crate::Sending::Refuse(why) => {
            window.say(&crate::policy::submission_note(&submission, Some(&why)));
        }
    }
}

/// A signature policy's published copy, under the level the window holds (ADR 1738).
///
/// **A policy that would be asked about while another question waits is declined with that
/// reason** rather than put in its place, which would leave the first unanswered and unsaid; a
/// toolkit that puts each question in a window of its own has none waiting.
pub(super) fn fetch_policy<W: NativeWindow + ?Sized>(
    window: &mut W,
    document: DocumentId,
    policy: crate::PublishedPolicy,
) {
    use crate::policy::{
        asked_to_fetch_policy, may_fetch_signature_policy, signature_policy_declined,
    };
    match may_fetch_signature_policy(&policy, window.state().restrictions.submissions()) {
        crate::Sending::Send => window.start_policy_fetch(document, policy, None),
        crate::Sending::Warn(note) => window.start_policy_fetch(document, policy, Some(note)),
        crate::Sending::Ask(_) if window.question_waiting() => {
            window.say(&signature_policy_declined(
                &policy,
                "another question was already waiting for your answer",
            ));
        }
        crate::Sending::Ask(_) => {
            let words = asked_to_fetch_policy(&policy);
            window.put_the_question(
                Asked::Policy {
                    document,
                    policy: Box::new(policy),
                },
                &words,
            );
        }
        crate::Sending::Refuse(why) => window.say(&signature_policy_declined(&policy, &why)),
    }
}

/// §12.6.4.8's URI, resolved against this document's location and then said, or put to the
/// person, at the level the window holds (ADR 1155).
pub(super) fn link<W: NativeWindow + ?Sized>(window: &mut W, uri: &str) {
    let uri = crate::resolve_uri(Some(&window.state().showing.path), uri);
    match crate::link(&uri, window.state().links) {
        crate::Link::Say(note) => window.say(&note),
        crate::Link::Ask(words) => window.put_the_question(Asked::Link { uri }, &words),
    }
}

/// A file a *document* named, routed by which policy decides it: a second PDF to parse under the
/// remote-documents level, an import named by an absolute URI fetched under the level a submission
/// is sent at (ADR 1527), and any other import read (ADRs 1227, 1239).
pub(super) fn needs_file<W: NativeWindow + ?Sized>(
    window: &mut W,
    document: DocumentId,
    purpose: Purpose,
    name: &str,
    beside: bool,
    queue: &mut VecDeque<Command>,
) {
    if crate::under_remote_documents(purpose) {
        remote(window, purpose, name, beside, queue);
    } else if purpose == Purpose::ImportData && crate::import_is_fetched(name) {
        fetch_import(window, document, name.to_owned(), queue);
    } else {
        import(window, purpose, name, queue);
    }
}

/// §7.11.4's bytes came out: a document §O.2.1's `ef` asked to be opened, or a file to write.
///
/// ISO 32000-2 §O.2.1, Table Annex O.3's `ef` row:
///
/// > When used as part of a PDF open parameter, the PDF processor shall open the embedded file
/// > contained within the EmbeddedFiles name tree identified by name .
///
/// The embedded document opens in a tab of its own and the document holding it keeps its tab; what
/// follows `ef` in the fragment goes with it as its own fragment. It waits in `crate::Arrivals` like
/// any document a reader named, so a password it asks for is asked about it, and it starts once
/// the loop that extracted it has finished (ADR 1275). Whether it opens at all is
/// `crate::may_open_extracted`'s answer at the level the window holds (ADR 1331).
pub(super) fn extracted<W: NativeWindow + ?Sized>(
    window: &mut W,
    document: DocumentId,
    asked: viewer_core::Extraction,
    name: &str,
    bytes: Vec<u8>,
    fragment: Option<String>,
) {
    if !crate::opens_as_document(asked, &bytes) {
        window.write_extracted(asked, name, &bytes);
        return;
    }
    match crate::may_open_extracted(
        asked,
        name,
        window.state().restrictions.embedded_documents(),
    ) {
        crate::Unpacking::Open => window.open_embedded(document, name, bytes, fragment),
        crate::Unpacking::Warn(note) => {
            window.say(&note);
            window.open_embedded(document, name, bytes, fragment);
        }
        crate::Unpacking::Refuse(why) => window.say(&why),
        crate::Unpacking::Ask(question) => window.put_the_question(
            Asked::Embedded {
                document,
                name: name.to_owned(),
                bytes,
                fragment,
            },
            &question,
        ),
    }
}

/// Table Annex O.4's `fdf` naming an absolute URI, under the level a submission is sent at (ADR
/// 1527). A refusal is said and supplied as `None`, so the core says the import declined; a fetch
/// arrives as `Command::Respond` by document, whichever is in front by then.
fn fetch_import<W: NativeWindow + ?Sized>(
    window: &mut W,
    document: DocumentId,
    url: String,
    queue: &mut VecDeque<Command>,
) {
    match crate::may_fetch_import(&url, window.state().restrictions.submissions()) {
        crate::Sending::Send => start_fetch(window, document, url, None),
        crate::Sending::Warn(note) => start_fetch(window, document, url, Some(note)),
        crate::Sending::Ask(words) => {
            window.put_the_question(Asked::Fetch { document, url }, &words);
        }
        crate::Sending::Refuse(why) => {
            window.say(&crate::fetch_note(&url, Some(&why)));
            queue.push_back(Command::Supply {
                purpose: Purpose::ImportData,
                bytes: None,
            });
        }
    }
}

/// Puts the GET on a `fetch-import` thread of its own, looked for as a submission's answer is.
fn start_fetch<W: NativeWindow + ?Sized>(
    window: &mut W,
    document: DocumentId,
    url: String,
    warned: Option<String>,
) {
    window.say(&crate::fetch_note(&url, None));
    let first = !window.state().looking();
    if let Err(sentence) = window
        .state_mut()
        .submitter
        .fetch(document, url, warned, None)
    {
        window.say(&sentence);
        return;
    }
    if first {
        window.look_for_answers();
    }
}

/// §12.7.6.4's file, under the narrowest policy that performs the action.
///
/// No level: the clause makes performing an import a `shall` and what is admitted is the one rule
/// `crate::policy::read_import` states — a relative path to a file in or below the open document's
/// directory. A refusal is said rather than swallowed (trap 5).
fn import<W: NativeWindow + ?Sized>(
    window: &mut W,
    purpose: Purpose,
    name: &str,
    queue: &mut VecDeque<Command>,
) {
    let read = crate::policy::read_import(window.state().showing.directory.as_deref(), name);
    let bytes = match read {
        Ok(bytes) => Some(bytes),
        Err(refusal) => {
            window.say(&crate::policy::supply_note(purpose, &refusal));
            None
        }
    };
    queue.push_back(Command::Supply { purpose, bytes });
}

/// §12.6.4.3's file, under the level the window was started at.
///
/// The policy is `crate::remote`'s, so a level a reader sets is a value there rather than three
/// windows' worth of editing — ADR 1079's shape and ADR 1155's, one clause along (ADR 1227).
fn remote<W: NativeWindow + ?Sized>(
    window: &mut W,
    purpose: Purpose,
    name: &str,
    beside: bool,
    queue: &mut VecDeque<Command>,
) {
    let decided = crate::remote(
        window.state().showing.directory.as_deref(),
        name,
        window.state().remote_documents,
        purpose,
        beside,
    );
    match decided {
        crate::Remote::Supply { path, note } => {
            let bytes = read_remote(window, purpose, name, &path);
            if let Some(read) = bytes.as_deref() {
                if let Some(note) = note {
                    window.say(&note);
                }
                window.offer_a_name(&path, read, purpose, queue);
                if purpose == Purpose::LaunchDocument {
                    launched_elsewhere(window, name, &path, read);
                }
            }
            queue.push_back(Command::Supply { purpose, bytes });
        }
        crate::Remote::Ask { path, question } => window.put_the_question(
            Asked::RemoteDocument {
                purpose,
                name: name.to_owned(),
                path,
            },
            &question,
        ),
        crate::Remote::Refuse(why) => {
            window.say(&why);
            queue.push_back(Command::Supply {
                purpose,
                bytes: None,
            });
        }
    }
}

/// §12.6.4.6's launched file once read: where it is not a PDF, what the reader's level does with it
/// — a program started never, a document handed to this machine's own handler or not (ADR 2171).
///
/// The bytes still go to the core, whose sentence says the link opened nothing in this reader; what
/// this adds is what became of the file outside it.
fn launched_elsewhere<W: NativeWindow + ?Sized>(
    window: &mut W,
    name: &str,
    path: &Path,
    read: &[u8],
) {
    let level = window.state().restrictions.launched_documents();
    match crate::policy::launched(path, name, read, level) {
        None => {}
        Some(crate::Launched::Program(said) | crate::Launched::Refuse(said)) => window.say(&said),
        Some(crate::Launched::Hand(said)) => hand_over(window, name, path, &said),
        Some(crate::Launched::Ask(question)) => window.put_the_question(
            Asked::LaunchedDocument {
                name: name.to_owned(),
                path: path.to_owned(),
            },
            &question,
        ),
    }
}

/// Hands `path` to this machine's own handler, saying `said` once it is started and the reason
/// where it could not be.
fn hand_over<W: NativeWindow + ?Sized>(window: &mut W, name: &str, path: &Path, said: &str) {
    match crate::policy::hand_over(path) {
        Ok(()) => window.say(said),
        Err(error) => window.say(&format!(
            "Launch: {name} could not be handed over — {error}"
        )),
    }
}

/// The bytes of a file `crate::remote` has already decided on, or the sentence saying why there
/// are none.
fn read_remote<W: NativeWindow + ?Sized>(
    window: &mut W,
    purpose: Purpose,
    name: &str,
    path: &Path,
) -> Option<Vec<u8>> {
    match std::fs::read(path) {
        Ok(bytes) => Some(bytes),
        Err(error) => {
            window.say(&crate::remote_note(
                purpose,
                name,
                Some(&format!("cannot read {}: {error}", path.display())),
            ));
            None
        }
    }
}
