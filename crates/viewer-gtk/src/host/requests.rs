//! What a document asks to send or to fetch: a form's submission, an import, a signature policy's copy, a remote file.

use std::collections::VecDeque;
use std::path::Path;
use std::rc::Rc;

use gtk4::glib;
use viewer_core::{Command, DocumentId, Purpose};
use viewer_host::native_window::NativeWindow;

use super::{Host, with};

impl Host {
    /// §12.6.4.8's URI, resolved against this document's location and then opened, declined or put
    /// to the person at the level `--links=` set; the question is this window's dialogue.
    pub(super) fn follow_the_link(&mut self, uri: &str) {
        let uri = viewer_host::resolve_uri(Some(&self.state.showing.path), uri);
        match viewer_host::link(&uri, self.links) {
            viewer_host::Link::Say(note) => self.say(&note),
            viewer_host::Link::Ask(words) => self.ask_whether_to_open(uri, &words),
        }
    }

    /// A file a *document* named, routed by which policy decides it.
    ///
    /// Three of the five purposes are asked at one of four levels and §12.7.6.4's own file is
    /// not, and the difference is what each does: an import puts another file's *values* into
    /// the document being read, while a remote go-to, a thread in another file and a named page
    /// each make this program parse a second PDF — which is the act a person may want to be
    /// asked about. `viewer_host::under_remote_documents` is the division, written once rather
    /// than in each of three windows (ADRs 1227, 1239).
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
            return;
        }
        // An import named by an absolute URI is fetched rather than read, under the level a
        // submission is sent at (ADR 1527).
        if purpose == Purpose::ImportData && viewer_host::import_is_fetched(&name) {
            self.fetch_import(document, name, queue);
            return;
        }
        let bytes = match viewer_host::policy::read_import(
            self.state.showing.directory.as_deref(),
            &name,
        ) {
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
                if bytes.is_some() {
                    if let Some(note) = note {
                        self.say(&note);
                    }
                    self.offer_a_name(&path, bytes.as_deref().unwrap_or_default(), purpose, queue);
                }
                queue.push_back(Command::Supply { purpose, bytes });
            }
            viewer_host::Remote::Ask { path, question } => {
                let named = name.to_owned();
                self.put_a_question(
                    viewer_host::Subject::Document.title(),
                    &question,
                    Rc::new(move |host: &mut Self, proceed| {
                        let mut queue = VecDeque::new();
                        let bytes = if proceed {
                            host.read_remote(purpose, &named, &path)
                        } else {
                            host.say(&viewer_host::remote_declined(purpose, &named));
                            None
                        };
                        if bytes.is_some() {
                            host.offer_a_name(
                                &path,
                                bytes.as_deref().unwrap_or_default(),
                                purpose,
                                &mut queue,
                            );
                        }
                        queue.push_back(Command::Supply { purpose, bytes });
                        host.pump(queue);
                    }),
                );
            }
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
    fn read_remote(&self, purpose: Purpose, name: &str, path: &Path) -> Option<Vec<u8>> {
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

    /// §12.7.6.2's composed request, under the level the menu holds (ADR 1291).
    pub(super) fn submit(
        &mut self,
        document: DocumentId,
        submission: pdf_model::submission::Submission,
    ) {
        match viewer_host::may_submit(&submission, self.restrictions.submissions()) {
            viewer_host::Sending::Send => self.send_form(document, submission, None),
            viewer_host::Sending::Warn(note) => self.send_form(document, submission, Some(note)),
            viewer_host::Sending::Ask(words) => self.put_a_question(
                viewer_host::Subject::Submission.title(),
                &words,
                Rc::new(move |host: &mut Self, proceed| {
                    if proceed {
                        host.send_form(document, submission.clone(), None);
                    } else {
                        host.say(&viewer_host::policy::submission_note(
                            &submission,
                            Some(&format!(
                                "you answered \"{}\"",
                                viewer_host::restriction::DO_NOT
                            )),
                        ));
                    }
                }),
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
    /// document, whichever tab is in front by then.
    fn fetch_import(&mut self, document: DocumentId, url: String, queue: &mut VecDeque<Command>) {
        match viewer_host::may_fetch_import(&url, self.restrictions.submissions()) {
            viewer_host::Sending::Send => self.start_fetch(document, url, None),
            viewer_host::Sending::Warn(note) => self.start_fetch(document, url, Some(note)),
            viewer_host::Sending::Ask(words) => self.put_a_question(
                viewer_host::Subject::Fetch.title(),
                &words,
                Rc::new(move |host: &mut Self, proceed| {
                    if proceed {
                        host.start_fetch(document, url.clone(), None);
                    } else {
                        host.say(&viewer_host::fetch_note(
                            &url,
                            Some(&format!(
                                "you answered \"{}\"",
                                viewer_host::restriction::DO_NOT
                            )),
                        ));
                        host.dispatch(Command::Supply {
                            purpose: Purpose::ImportData,
                            bytes: None,
                        });
                    }
                }),
            ),
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
    /// `viewer_host::policy::may_fetch_signature_policy` gives (ADR 1738). Declined, it is said;
    /// fetched, its answer is looked for as a submission's is and a bound PDF opens beside.
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
            viewer_host::Sending::Ask(_) => {
                let words = asked_to_fetch_policy(&policy);
                self.put_a_question(
                    viewer_host::Subject::Policy.title(),
                    &words,
                    Rc::new(move |host: &mut Self, proceed| {
                        if proceed {
                            host.start_policy_fetch(document, policy.clone(), None);
                        } else {
                            host.say(&signature_policy_declined(
                                &policy,
                                &format!("you answered \"{}\"", viewer_host::restriction::DO_NOT),
                            ));
                        }
                    }),
                );
            }
            viewer_host::Sending::Refuse(why) => {
                self.say(&signature_policy_declined(&policy, &why));
            }
        }
    }

    /// Puts the GET on a `fetch-import` thread of its own, looked for as a submission's answer is.
    fn start_fetch(&mut self, document: DocumentId, url: String, warned: Option<String>) {
        self.say(&viewer_host::fetch_note(&url, None));
        let first = !self.state.looking();
        if let Err(sentence) = self.state.submitter.fetch(document, url, warned, None) {
            self.say(&sentence);
            return;
        }
        if first {
            self.look_for_answers();
        }
    }

    /// Looks for the answers on GTK's main loop, at `viewer_host::submit::LOOK`, until none is out.
    pub(super) fn look_for_answers(&self) {
        let me = self.me.clone();
        glib::timeout_add_local(viewer_host::submit::LOOK, move || {
            let mut more = false;
            with(&me, |host| {
                // A conversion's report is collected inside it too (ADR 1954).
                host.take_the_answers();
                more = host.state.looking();
            });
            if more {
                glib::ControlFlow::Continue
            } else {
                glib::ControlFlow::Break
            }
        });
    }
}
