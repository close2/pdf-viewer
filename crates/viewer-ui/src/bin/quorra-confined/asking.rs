//! `CLAUDE.md`'s *ask* level in this window: §12.6.4.8's link, §12.6.4.3's file a document names,
//! §12.7.6.2's form sent to a server and an import fetched from one, put on a card the window draws
//! and answered by two keys (ADRs 2040, 2076).
//!
//! **The policy is `viewer_host`'s, and nothing here decides it.** [`viewer_host::link`] and
//! [`viewer_host::remote`] take the level and answer with a sentence to say, a file to supply or a
//! question to put, exactly as they answer the three windows that hold the viewer in process; what
//! is this window's is where the question goes — the flagship's [`viewer_ui::chrome::QuestionCard`],
//! over whatever the screen holds — and which key is the answer. Both acts are this side's: the
//! handler a link is given to runs out here, and the file a document named is read out here and
//! crosses to the worker as bytes inside `Command::Supply`, which the confinement then parses as it
//! parses the document it was started with. A form is sent from out here too, and what the server
//! answered crosses inside `Command::Respond` for the same reason (`sending`).

use std::path::{Path, PathBuf};

use pdf_model::submission::Submission;
use viewer_core::{Command, DocumentId, Purpose};
use viewer_ui::chrome::Chrome;
use winit::keyboard::{Key, NamedKey};

use crate::Host;

/// What the card is asking about, held until a key answers it.
#[derive(Debug)]
pub(crate) enum Asked {
    /// §12.6.4.8's URI, resolved against the document's location.
    Link {
        /// The URI as `viewer_host::resolve_uri` left it.
        uri: String,
    },
    /// §12.6.4.3's file, or one of the three other purposes `viewer_host::under_remote_documents`
    /// puts under the same level.
    Remote {
        /// Which purpose asked, so that the answer goes back under it.
        purpose: Purpose,
        /// The file as the document named it, for the sentence a decline prints.
        name: String,
        /// Where `viewer_host::resolve_import` put it, which is what would be read.
        path: PathBuf,
    },
    /// §12.7.6.2's request, composed by the worker, which a `yes` sends (ADR 2076).
    Submit {
        /// The document whose form it is, which the answer is imported into.
        document: DocumentId,
        /// The request, boxed for `Event::Submit`'s reason: the body travels with it.
        submission: Box<Submission>,
    },
    /// §12.6.4.6's launched document that is not a PDF, which a `yes` hands to this machine's own
    /// handler (ADR 2171).
    Launched {
        /// The file as the document named it, for the sentence either answer prints.
        name: String,
        /// Where `viewer_host::resolve_import` put it, which is what would be handed over.
        path: PathBuf,
    },
    /// Table Annex O.4's `fdf`, or §12.7.6.4's file, named by an absolute URI, which a `yes`
    /// fetches (ADR 2076).
    Fetch {
        /// The document that asked, which the data is imported into.
        document: DocumentId,
        /// The URI, as the document named it.
        url: String,
    },
}

/// A file a document named, supplied to the worker, until the worker says what became of it.
///
/// `viewer_host::Arrivals`'s offer, for the one document this window holds: an `Event::Opened`
/// makes it the window's document, an `Event::PasswordRequired` makes it the file the prompt is
/// about, and once the supply's own events have been seen an offer neither took is spent — a
/// declined jump, or a named page drawn into the document that asked (ADRs 1332, 2040).
#[derive(Debug)]
pub(crate) struct Offered {
    /// What the file was supplied for, which is what a cancelled prompt about it declines.
    purpose: Purpose,
    /// The file.
    path: PathBuf,
}

impl Host {
    /// §12.6.4.8's URI, under the level this window was started at (ADRs 1155, 2040).
    pub(crate) fn open_uri(&mut self, uri: &str) {
        let uri = viewer_host::resolve_uri(Some(&self.path), uri);
        match viewer_host::link(&uri, self.links) {
            viewer_host::Link::Say(note) => eprintln!("{note}"),
            viewer_host::Link::Ask(words) => self.put_the_question(Asked::Link { uri }, &words),
        }
    }

    /// A file a document named under [`viewer_host::RemoteDocuments`]' level (ADRs 1227, 2040).
    ///
    /// **Asked as a file that opens in place**, whatever Table 203's `/NewWindow` said: this window
    /// holds one document and reserves no name for a second, so the worker's viewer replaces the one
    /// showing and says so itself (ADR 1263), and the question says the same before it happens.
    pub(crate) fn needs_remote(&mut self, purpose: Purpose, name: &str) {
        match viewer_host::remote(
            self.path.parent(),
            name,
            self.remote_documents,
            purpose,
            false,
        ) {
            viewer_host::Remote::Supply { path, note } => {
                self.supply_remote(purpose, name, path, note.as_deref());
            }
            viewer_host::Remote::Ask { path, question } => self.put_the_question(
                Asked::Remote {
                    purpose,
                    name: name.to_owned(),
                    path,
                },
                &question,
            ),
            viewer_host::Remote::Refuse(why) => {
                eprintln!("note: {why}");
                self.dispatch(Command::Supply {
                    purpose,
                    bytes: None,
                });
            }
        }
    }

    /// Puts the question on the card, which then has the keyboard.
    ///
    /// The faces are loaded here the first time a card needs them, as the password card's are, so a
    /// document that asks nothing costs the launch no chrome (principle 2). **One question stands at
    /// a time**: a second that arrives while one is up is declined out loud rather than put over it,
    /// because a person answering a card is answering the words they read.
    pub(crate) fn put_the_question(&mut self, asked: Asked, words: &viewer_host::Question) {
        if self.chrome.is_none() {
            match Chrome::new() {
                Ok(chrome) => self.chrome = Some(chrome),
                Err(said) => {
                    // A build whose compiled-in faces will not parse cannot draw the card;
                    // declining by name is what is left (trap 5).
                    self.decline(
                        asked,
                        &format!("this build cannot draw the question ({said})"),
                    );
                    return;
                }
            }
        }
        if self.asked.is_some() {
            self.decline(asked, "another question is already on the window");
            return;
        }
        self.asked = Some(asked);
        self.question.ask(words);
        self.card_changed();
    }

    /// A question that was never put, said with the reason and answered as a `no`.
    fn decline(&mut self, asked: Asked, why: &str) {
        match asked {
            Asked::Link { uri } => eprintln!("{}", viewer_host::uri_note(&uri, Some(why))),
            Asked::Remote { purpose, name, .. } => {
                eprintln!(
                    "note: {}",
                    viewer_host::remote_note(purpose, &name, Some(why))
                );
                self.dispatch(Command::Supply {
                    purpose,
                    bytes: None,
                });
            }
            Asked::Submit { submission, .. } => eprintln!(
                "{}",
                viewer_host::policy::submission_note(&submission, Some(why))
            ),
            Asked::Fetch { url, .. } => {
                eprintln!("{}", viewer_host::fetch_note(&url, Some(why)));
                self.dispatch(Command::Supply {
                    purpose: Purpose::ImportData,
                    bytes: None,
                });
            }
            Asked::Launched { name, .. } => eprintln!(
                "note: {}",
                viewer_host::remote_note(Purpose::LaunchDocument, &name, Some(why))
            ),
        }
    }

    /// A key press while the question has the keyboard: Enter goes ahead, Escape does not.
    ///
    /// Every key is taken, for the password card's reason: the card is modal, and Escape is the
    /// question's `no` rather than [`Host::abort`], because what the person is declining is the act
    /// and not the worker. The two keys are named on the card itself (ADR 1145).
    pub(crate) fn question_key(&mut self, key: &Key<&str>) {
        match key {
            Key::Named(NamedKey::Enter) => self.question_answered(true),
            Key::Named(NamedKey::Escape) => self.question_answered(false),
            _ => {}
        }
    }

    /// The card was answered: the act carried out, or the sentence saying it was not.
    fn question_answered(&mut self, proceed: bool) {
        self.question.answered();
        self.card_changed();
        let Some(asked) = self.asked.take() else {
            return;
        };
        match asked {
            Asked::Link { uri } => eprintln!("{}", viewer_host::answered(&uri, proceed)),
            Asked::Remote {
                purpose,
                name,
                path,
            } if proceed => self.supply_remote(purpose, &name, path, None),
            Asked::Remote { purpose, name, .. } => {
                eprintln!("note: {}", viewer_host::remote_declined(purpose, &name));
                self.dispatch(Command::Supply {
                    purpose,
                    bytes: None,
                });
            }
            Asked::Submit {
                document,
                submission,
            } if proceed => self.send_form(document, *submission, None),
            Asked::Submit { submission, .. } => eprintln!(
                "{}",
                viewer_host::policy::submission_note(&submission, Some(&you_did_not()))
            ),
            Asked::Fetch { document, url } if proceed => self.start_fetch(document, url, None),
            Asked::Fetch { url, .. } => {
                eprintln!("{}", viewer_host::fetch_note(&url, Some(&you_did_not())));
                self.dispatch(Command::Supply {
                    purpose: Purpose::ImportData,
                    bytes: None,
                });
            }
            Asked::Launched { name, path } if proceed => {
                hand_over(&name, &path, &viewer_host::policy::handed(&name));
            }
            Asked::Launched { name, .. } => eprintln!(
                "note: {}",
                viewer_host::remote_declined(Purpose::LaunchDocument, &name)
            ),
        }
    }

    /// §12.6.4.6's launched file once read: where it is not a PDF, what `--launched-documents=`
    /// does with it — a program started never, a document handed over or not (ADR 2171). The bytes
    /// still cross to the worker, whose sentence says the link opened nothing in this reader.
    fn launched_elsewhere(&mut self, name: &str, path: &Path, read: &[u8]) {
        match viewer_host::policy::launched(path, name, read, self.launched_documents) {
            None => {}
            Some(viewer_host::Launched::Program(said) | viewer_host::Launched::Refuse(said)) => {
                eprintln!("note: {said}");
            }
            Some(viewer_host::Launched::Hand(said)) => hand_over(name, path, &said),
            Some(viewer_host::Launched::Ask(question)) => self.put_the_question(
                Asked::Launched {
                    name: name.to_owned(),
                    path: path.to_owned(),
                },
                &question,
            ),
        }
    }

    /// Reads a file a document named and hands it to the worker, or says why it could not.
    ///
    /// Read rather than opened as a descriptor because `Command::Supply` carries bytes; this side
    /// reads them and never parses one, and the worker opens them under its own limits (ADR 1358).
    /// `note` is what [`viewer_host::RemoteDocuments::Warn`] adds, said once the file was read.
    fn supply_remote(&mut self, purpose: Purpose, name: &str, path: PathBuf, note: Option<&str>) {
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => {
                if let Some(note) = note {
                    eprintln!("note: {note}");
                }
                if purpose == Purpose::LaunchDocument {
                    self.launched_elsewhere(name, &path, &bytes);
                }
                self.offered = Some(Offered { purpose, path });
                Some(bytes)
            }
            Err(error) => {
                eprintln!(
                    "note: {}",
                    viewer_host::remote_note(
                        purpose,
                        name,
                        Some(&format!("cannot read {}: {error}", path.display()))
                    )
                );
                None
            }
        };
        self.dispatch(Command::Supply { purpose, bytes });
    }

    /// `Event::Opened`: where it answers a file this window supplied, that file is the window's
    /// document now — the one its title names, its save is written beside, its next relative link
    /// resolves against and a resume opens again.
    pub(crate) fn opened_an_offer(&mut self) {
        if let Some(offered) = self.offered.take() {
            self.path = offered.path;
        }
    }

    /// `Event::PasswordRequired`: where it is about the file this window just supplied, the prompt
    /// is about that file, and its second attempt opens it.
    pub(crate) fn locked_an_offer(&mut self) {
        if let Some(offered) = self.offered.take() {
            self.prompting = Some(offered);
        }
    }

    /// §7.6.4.1's second attempt at a file a document named goes out: it is offered again, for
    /// the one command that carries it, so its answer is read as the supply's was.
    pub(crate) fn offer_the_prompted(&mut self) {
        if let Some(offered) = self.prompting.take() {
            self.offered = Some(offered);
        }
    }

    /// The command carrying an offer has been answered and its events all seen: an offer nothing
    /// took is spent, so that no later open is mistaken for it.
    pub(crate) fn supplied(&mut self) {
        self.offered = None;
    }

    /// The file §7.6.4.1's card is asking about: one a document named, or the window's own.
    pub(crate) fn prompted_for(&self) -> &Path {
        self.prompting
            .as_ref()
            .map_or(self.path.as_path(), |offered| offered.path.as_path())
    }

    /// The prompt about a file a document named was cancelled or ran out of attempts: the worker
    /// is still holding that file for the password, and `Command::Supply` with no bytes under the
    /// purpose it was supplied for is what ends the hold (ADR 1345).
    pub(crate) fn decline_the_prompted(&mut self) -> bool {
        let Some(offered) = self.prompting.take() else {
            return false;
        };
        self.dispatch(Command::Supply {
            purpose: offered.purpose,
            bytes: None,
        });
        true
    }

    /// A new worker holds the window's document and nothing it was offered.
    pub(crate) fn forget_the_offers(&mut self) {
        self.offered = None;
        self.prompting = None;
    }
}

/// The reason a `no` on the card gives, in the words the other three windows say it in.
fn you_did_not() -> String {
    format!("you answered \"{}\"", viewer_host::restriction::DO_NOT)
}

/// Hands `path` to this machine's own handler, printing `said` once it is started and the reason
/// where it could not be (ADR 2171).
fn hand_over(name: &str, path: &Path, said: &str) {
    match viewer_host::policy::hand_over(path) {
        Ok(()) => eprintln!("note: {said}"),
        Err(error) => eprintln!("note: Launch: {name} could not be handed over — {error}"),
    }
}
