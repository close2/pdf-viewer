//! Commands into `viewer-core`, events back out, and what this window does about each.
//!
//! The whole of the boundary in one place, which is what makes it readable as a boundary: every
//! command this program sends goes through [`App::pump`] and every event it receives is answered
//! in [`App::react`]. A reader asking "what happens when a page turns" finds the answer here and
//! the work itself in the module the arm names.

use std::collections::VecDeque;
use std::path::Path;

use viewer_core::{Answer, Command, Event, Purpose, Query};

use crate::app::App;
use crate::trace::{Topic, describe_command, describe_event};

impl App {
    /// Hands a command to the core and deals with everything that comes back.
    ///
    /// A queue rather than recursion, because reacting to an event may produce a command — a
    /// password supplied, a file read — and a chain of those is a loop rather than a stack.
    pub(crate) fn dispatch(&mut self, command: Command) {
        self.pump(VecDeque::from([command]));
    }

    /// Reacts to events that were produced somewhere other than a [`Self::dispatch`].
    ///
    /// One caller: the thread that opens the document while the window and the graphics device
    /// come up. Its events are a `Vec` rather than an iterator over the viewer, because the
    /// viewer they came from was on another thread — and everything after that is the ordinary
    /// loop, so a `PasswordRequired` from the thread is answered exactly as one from a command.
    pub(crate) fn receive(&mut self, events: Vec<Event>) {
        if self.trace.on(Topic::Events) {
            self.trace.say(
                Topic::Events,
                format_args!("opened on its own thread -> {} event(s)", events.len()),
            );
            for event in &events {
                self.trace
                    .more(Topic::Events, format_args!("{}", describe_event(event)));
            }
        }
        let mut queue = VecDeque::new();
        for event in events {
            self.react(event, &mut queue);
        }
        self.pump(queue);
    }

    /// Runs commands until nothing is left, reacting to what each produces.
    fn pump(&mut self, mut queue: VecDeque<Command>) {
        while let Some(command) = queue.pop_front() {
            // Table 166's `/M` is what §7.5.6's update writes it into, so the clock is read on
            // the way there rather than kept. Which command that is is
            // `viewer_host::modification::before`'s, once for all three windows, because a host
            // reading its clock somewhere else reads it at a different moment (ADR 1160). It is
            // pumped here rather than put back on the queue, which would put the save behind it
            // for ever.
            if let Some(clock) = viewer_host::modification::before(&command) {
                self.pump(VecDeque::from([clock]));
            }
            let started = std::time::Instant::now();
            // The pointer is its own topic and not `events`: 285 of the 1490 lines of the trace
            // that raised ADR 0227 were pointer moves, and they arrive faster than a person
            // can read whatever else is happening.
            let topic = if matches!(command, Command::Pointer { .. }) {
                Topic::Pointer
            } else {
                Topic::Events
            };
            let described = self.trace.on(topic).then(|| describe_command(&command));
            // **A command that changes the document changes what §14.7's tree says**, and
            // nothing republished it until the five-hundred-and-ninetieth session: `App::attend`
            // compares the page and the viewport, and an edit moves neither. Which commands those
            // are is `viewer_accessibility::republishes` since the seven-hundred-and-thirty-first,
            // because the two native hosts publish now too and this was about to be its third copy
            // (ADR 0623).
            if viewer_accessibility::republishes(&command) {
                self.spoken = None;
            }
            let events: Vec<Event> = self.viewer.handle(command).collect();
            if let Some(described) = described {
                self.trace.say(
                    topic,
                    format_args!(
                        "{described} -> {} event(s) in {:?}",
                        events.len(),
                        started.elapsed()
                    ),
                );
                for event in &events {
                    self.trace
                        .more(topic, format_args!("{}", describe_event(event)));
                }
            }
            for event in events {
                self.react(event, &mut queue);
            }
        }
    }

    /// §12.7.6.2's composed request, said out loud under the host policy that decides it.
    ///
    /// A function of its own rather than four lines in the arm above, and not only for the
    /// line count: this is the whole of what a *host* contributes to the clause, and a reader
    /// looking for where the decision is made should find one name to follow.
    fn submit(submission: &pdf_model::submission::Submission) {
        println!(
            "{}",
            viewer_host::policy::submission_note(
                submission,
                viewer_host::policy::may_submit().err().as_deref(),
            )
        );
    }

    /// Does what one event asks.
    #[expect(
        clippy::too_many_lines,
        reason = "one arm per `Event` kind, and the kinds are a closed vocabulary a host must \
answer in full — splitting the match would put the question of which events this window \
answers in two places"
    )]
    fn react(&mut self, event: Event, queue: &mut VecDeque<Command>) {
        match event {
            // **Neither of these leaves the process, and both did until the
            // seven-hundred-and-fourth session.** ADR 0545 made that argument for §7.6.4.1's
            // prompt one round earlier and deliberately left these two: a window that exits has
            // told a person who launched it from a desktop nothing at all, and the two native
            // hosts have said the sentence into a status bar and stayed up since their first
            // session. The wording is `viewer_host`'s so that the three say one thing.
            Event::Opened { pages, .. } => {
                println!("{}: {pages} page(s)", self.title);
                if pages == 0 {
                    let said = viewer_host::no_pages(&self.title);
                    eprintln!("{said}");
                    self.refused.say(said);
                    self.redraw();
                    return;
                }
                self.asking.opened();
                // A document opens at the window's levels, so the menu's ticks go back to them
                // too — the core's `Open` forgets the departures (ADR 1145).
                self.restrictions.opened();
                self.report_due.opened();
                self.gather();
            }
            Event::OpenFailed { reason, .. } => {
                let said = viewer_host::cannot_open(&self.title, &reason);
                eprintln!("{said}");
                self.refused.say(said);
                self.redraw();
            }
            Event::PasswordRequired { document } => self.ask_again(document, queue),
            Event::Closed(_) => {}
            Event::PageChanged {
                index,
                label,
                of,
                section,
                ..
            } => self.page_changed(index, label.as_deref(), of, section.as_deref()),
            Event::NeedsRender(request) => {
                // Page one's display list arriving is the launch milestone between
                // `document joined` and the first frame: interpretation, which the trace
                // behind `doc/todo/44` showed as seven unnamed seconds on a 58 009-command
                // page. Every request after the first is the steady state, and the method
                // keeps only the first (ADR 0332).
                self.launch.interpreted(request.list.commands().len());
                // **Held by page rather than replacing what came before**, because Table 29's
                // arrangement asks for one render per page on the screen and they arrive one
                // after another. In page order, which is the order they arrive in and the order
                // `viewer_core::layout` places them; a second request for a page already held is
                // that page at a new placement and replaces it.
                self.unacknowledged.push(request.token);
                match self
                    .requests
                    .binary_search_by_key(&request.page, |held| held.page)
                {
                    Ok(at) => self.requests[at] = request.clone(),
                    Err(at) => self.requests.insert(at, request.clone()),
                }
                // §12.4.4: the page a transition moves *to* is the one whose list has just
                // arrived, so this is where one that was armed can be drawn.
                if let Some(transition) = self.arming.take() {
                    self.begin_transition(&request, transition);
                }
                self.redraw();
            }
            Event::Damage(_) => self.redraw(),
            // §12.6.4.8: resolved against this document's own location where the action left it
            // partial, then declined or opened by the one policy every window shares — so a host
            // that does open links, or `doc/todo/38`'s ask and warn levels, is a change in
            // `viewer_host::policy` and not in four `println!`s (ADR 1079).
            Event::OpenUri { uri, .. } => {
                let uri = viewer_host::resolve_uri(Some(&self.path), &uri);
                match viewer_host::link(&uri, self.links) {
                    viewer_host::Link::Say(note) => println!("{note}"),
                    viewer_host::Link::Ask(words) => {
                        self.put_a_question(crate::app::Pending::Link { uri }, &words);
                    }
                }
            }
            // §12.7.6.2: the policy is `viewer_host::policy::may_submit`'s and not this
            // window's, so that a host with a network — or `doc/todo/38`'s ask and warn levels —
            // is a change in one place (ADR 1062). What this arm owns is saying it out loud.
            Event::Submit { submission, .. } => Self::submit(&submission),
            // §12.6.4.3's file is asked at one of four levels and §12.7.6.4's is not, and the
            // difference is what each does: an import puts another file's *values* into the
            // document being read, and a remote go-to opens another document in place of it —
            // which is the act a person may want to be asked about (ADR 1227).
            Event::NeedsFile { purpose, name, .. } => {
                if purpose == Purpose::RemoteDocument {
                    self.remote(&name, queue);
                    return;
                }
                let bytes = self.supply(purpose, &name);
                queue.push_back(Command::Supply { purpose, bytes });
            }
            // §12.4.4: the frames of it are drawn, since the three-hundred-and-ninety-third
            // session — by this host, because a transition is an animation over wall time and
            // `viewer-core` has no clock (rule 3). What arrives here is the *shape* of what to
            // draw; when to draw it is this window's, and a window that is not presenting shows
            // the page, which is the transition's own end state. ADR 0230.
            Event::Transition { transition, .. } => self.arm_transition(transition),
            // Rule 2 in one arm: the core produced the bytes and the host owns the filesystem.
            // Written beside the document with `.edited.pdf` appended rather than over it,
            // because overwriting somebody's file is a decision this program has not been given.
            Event::Extracted {
                asked,
                name,
                bytes,
                fragment,
                ..
            } => self.extracted(asked, &name, bytes, fragment, queue),
            Event::Saved { bytes, .. } => self.write_saved(&bytes),
            // What a host does with this is mark its window and ask before closing. This one
            // has no dialogue to ask with, so it marks the title and says so on the way past.
            Event::Dirty { dirty, .. } => {
                self.dirty = dirty;
                self.retitle();
                if dirty {
                    println!("note: this document has unsaved changes");
                }
            }
            Event::Reported { page, notes, .. } => {
                for note in &notes {
                    println!("note: {note}");
                }
                if page.is_some() {
                    self.retitle_incomplete();
                }
            }
            Event::Refused { notes, .. } => Self::say_refused(&notes),
            // The other two of `CLAUDE.md`'s four levels, since the eight-hundred-and-eighty-fifth
            // session (ADR 0814). *Warn* is a sentence after an edit that went ahead. *Ask* is the
            // question this window puts, on a card of its own (ADR 1145).
            Event::Warned { notes, .. } => println!("note: {}", viewer_host::warned(&notes)),
            // The grant. What this window then does is show it: §12.5.3's bit 3, §8.11.4.5's
            // `Print` event and §12.5.6.22's sheet are already in force on every page it draws,
            // which is RFC 0004 §6's preview. The spool is ADR 1180's deferral and is named here
            // rather than left as a key that appears to do nothing.
            Event::Printing {
                pages, fidelity, ..
            } => {
                self.printing = true;
                // §7.6.4.2's Table 22 bit 12, said in the window that shows what would print for
                // the same reason the two that print say it: the three are level in what they
                // *show* (ADR 1190), and a reader looking at a preview of a job the document
                // wants degraded should be told so here too (ADR 1203).
                if let Some(note) = viewer_host::printing::destination_refused(fidelity) {
                    println!("note: {note}");
                }
                println!(
                    "note: showing what would print, over {pages} page(s) — this window has no \
                     printer of its own yet (ADR 1180); press the key again to stop"
                );
            }
            Event::Copied {
                logical,
                page_order,
                ..
            } => self.copied(logical, &page_order),
            Event::Asking {
                document,
                operation,
                notes,
            } => {
                let words = viewer_host::asked(operation, &notes);
                self.put_a_question(
                    crate::app::Pending::Restricted {
                        document,
                        operation,
                    },
                    &words,
                );
            }
            // §7.11.4's list moved: the copy `gather` took when the document opened is stale,
            // which is the one way "a property of an immutable document" stopped being true of
            // this list. Read again, and only this list.
            Event::AttachmentsChanged { .. } => {
                if let Answer::Attachments(files) = self.viewer.query(Query::Attachments) {
                    self.attachments = files;
                }
            }
            Event::Searched {
                found,
                remaining,
                wrapped,
                ..
            } => self.searched(found, remaining, wrapped),
        }
    }
}

impl App {
    /// §12.4.2's caption for the page now showing, which is the window's title bar.
    ///
    /// "Page labels and page indices need not coincide". Where the document states a label it is
    /// what a reader is meant to see — a page of front matter is *iv*, not page four — so the
    /// index is shown beside it rather than instead of it, because a title saying only `iv`
    /// cannot say `of 320`. §12.3.3's outline is a table of contents, so the item covering this
    /// page is the section a reader is in: after the page number rather than before it, because
    /// it is context for a position rather than the position itself.
    fn page_changed(
        &mut self,
        index: usize,
        label: Option<&str>,
        of: usize,
        section: Option<&str>,
    ) {
        let page = match label {
            Some(label) => format!("{label} — page {} of {of}", index.saturating_add(1)),
            None => format!("page {} of {of}", index.saturating_add(1)),
        };
        self.caption = match section {
            Some(section) if !section.is_empty() => format!("{page} — {section}"),
            _ => page,
        };
        self.retitle();
    }

    /// §7.6.4.1: a processor tries the default user password and then prompts.
    ///
    /// **This used to write to `stderr`, read `stdin` and call `std::process::exit(1)` when there
    /// was no terminal**, which is the one place in this program that answered a document on a
    /// file descriptor and the only one that left the process for want of one. §7.6.4.1's NOTE 2
    /// describes the processor that genuinely cannot ask — "non-interactive PDF readers that do
    /// not have a person running them such as printing off-line or on a server" — and a window on
    /// a screen is not one of them whatever it was launched from, so a desktop launcher could not
    /// open an encrypted document at all. What replaces it is a modal card this host draws for
    /// itself, `viewer_ui::chrome::PasswordCard`, which is the tier-2 counterpart of
    /// `viewer-gtk`'s `gtk4::PasswordEntry` and `viewer-qt`'s `QLineEdit`.
    ///
    /// The *policy* is [`viewer_host::password`]'s and is shared with those two: how many attempts,
    /// what to say when they are used up, and that an empty entry is a decline.
    fn ask_again(&mut self, document: viewer_core::DocumentId, _queue: &mut VecDeque<Command>) {
        self.locked = Some(document);
        // Exhaustive over `Ask` on purpose: a case added to `viewer-host` fails to compile in all
        // three hosts, which is what holds the level-hosts decision up (ADR 0526's shape).
        match self.asking.required() {
            viewer_host::Ask::Prompt { attempt, of } => {
                self.password
                    .ask(viewer_host::password::prompt(&self.title, attempt, of));
                self.redraw();
            }
            viewer_host::Ask::Exhausted => {
                println!("note: {}", viewer_host::password::EXHAUSTED);
                self.redraw();
            }
        }
    }

    /// A key press while §7.6.4.1's card has the keyboard.
    ///
    /// Every key is taken, which is what *modal* means here and is stronger than the find bar's
    /// version of the same rule: the document behind this card is **not open**, so a key that
    /// turned a page would be turning a page that does not exist. Escape and an empty Enter are
    /// the same fact about the person, and [`viewer_host::password::supplied`] is where that is
    /// decided rather than here.
    pub(crate) fn password_key(&mut self, key: &winit::keyboard::Key<&str>) {
        use winit::keyboard::{Key, NamedKey};
        match key {
            Key::Named(NamedKey::Escape) => {
                // Discards what was typed and then answers, so that Escape and an empty Enter
                // reach `viewer_host::password::supplied` by the same route and one place decides
                // what a decline means. The same answer a `QDialog` dismissed with Escape gives in
                // `viewer-qt`.
                self.password.clear();
                self.password_answered();
            }
            Key::Named(NamedKey::Enter) => self.password_answered(),
            Key::Named(NamedKey::Backspace) => {
                self.password.backspace();
                self.redraw();
            }
            Key::Named(NamedKey::Space) => {
                self.password.typed(" ");
                self.redraw();
            }
            Key::Character(text) if !text.is_empty() => {
                self.password.typed(text);
                self.redraw();
            }
            // A key with no character and no meaning here. Taken anyway, for the reason above.
            _ => {}
        }
    }

    /// `CLAUDE.md`'s *ask* level: the two keys that answer the question, and nothing else.
    ///
    /// Every key is taken, which is what *modal* means here and is [`App::password_key`]'s rule
    /// one clause over: the core is holding an operation until this is answered, so a key that
    /// turned a page would leave a person reading somewhere else with an unanswered question
    /// behind them. Escape is the decline, which is what a dismissed dialogue means in the other
    /// two windows and in `pdf-transform`'s pipe.
    pub(crate) fn question_key(&mut self, key: &winit::keyboard::Key<&str>) {
        use winit::keyboard::{Key, NamedKey};
        match key {
            Key::Named(NamedKey::Enter) => self.question_answered(true),
            Key::Named(NamedKey::Escape) => self.question_answered(false),
            // A key with no meaning here. Taken anyway, for the reason above.
            _ => {}
        }
    }

    /// What the person answered, on its way to the viewer that is holding the operation.
    ///
    /// The decline is said out loud because `viewer-core` says nothing at all on a `no` —
    /// deliberately (ADR 0814) — and a person is still owed the fact that what they asked for did
    /// not happen. Taking the question rather than reading it is what makes a card answered twice
    /// answer once.
    pub(crate) fn question_answered(&mut self, proceed: bool) {
        self.question.answered();
        self.redraw();
        let Some(about) = self.asked.take() else {
            return;
        };
        match about {
            crate::app::Pending::Restricted {
                document,
                operation,
            } => {
                if !proceed {
                    println!("note: {}", viewer_host::declined(operation));
                }
                self.dispatch(Command::Answer { document, proceed });
            }
            // §12.6.4.8: the act is this host's own rather than an edit the core is holding, so
            // what the answer decides is whether the URI reaches `xdg-open` (ADR 1155).
            crate::app::Pending::Link { uri } => {
                println!("{}", viewer_host::answered(&uri, proceed));
            }
            // §12.6.4.3: the act is opening a document in place of this one, so a `no` supplies
            // nothing and the core says the link declined (ADR 1227).
            crate::app::Pending::RemoteDocument { name, path } => {
                if proceed {
                    self.supply_remote(&name, &path);
                } else {
                    println!("note: {}", viewer_host::remote_declined(&name));
                    self.dispatch(Command::Supply {
                        purpose: Purpose::RemoteDocument,
                        bytes: None,
                    });
                }
            }
        }
    }

    /// §12.6.4.3's file, under the level this window was started at.
    ///
    /// The policy is `viewer_host::remote`'s and not this window's, so a level a reader sets is
    /// a value there rather than three windows' worth of editing — which is ADR 1079's shape and
    /// ADR 1155's, one clause along. What is this window's is where the question goes and where
    /// the sentence is printed (ADR 1227).
    fn remote(&mut self, name: &str, queue: &mut VecDeque<Command>) {
        match viewer_host::remote(self.directory.as_deref(), name, self.remote_documents) {
            viewer_host::Remote::Supply { path, note } => {
                let bytes = App::read_remote(name, &path);
                if bytes.is_some()
                    && let Some(note) = note
                {
                    println!("note: {note}");
                }
                queue.push_back(Command::Supply {
                    purpose: Purpose::RemoteDocument,
                    bytes,
                });
            }
            viewer_host::Remote::Ask { path, question } => {
                self.put_a_question(
                    crate::app::Pending::RemoteDocument {
                        name: name.to_owned(),
                        path,
                    },
                    &question,
                );
            }
            viewer_host::Remote::Refuse(why) => {
                println!("note: {why}");
                queue.push_back(Command::Supply {
                    purpose: Purpose::RemoteDocument,
                    bytes: None,
                });
            }
        }
    }

    /// The bytes of a file this window has just been given leave to open.
    fn supply_remote(&mut self, name: &str, path: &Path) {
        let bytes = App::read_remote(name, path);
        self.dispatch(Command::Supply {
            purpose: Purpose::RemoteDocument,
            bytes,
        });
    }

    /// Puts one question on the card, whatever it is about.
    ///
    /// One place rather than two, so that a subject added here cannot arrive without the card
    /// going up or without the repaint that shows it.
    fn put_a_question(&mut self, about: crate::app::Pending, words: &viewer_host::Question) {
        self.asked = Some(about);
        self.question.ask(words);
        self.redraw();
    }

    /// `CLAUDE.md`'s four levels: moving through the menu, choosing one, and closing it.
    ///
    /// The arrows and Enter rather than a click, which is what every other card in this window is
    /// answered with — there is no window manager behind them and no pointer affordance drawn on
    /// them. It is a documented choice rather than a limit of the toolkit (trap 17): a click model
    /// exists here, `viewer_ui::chrome::ChoiceList` has it, and it is there because §12.7.5.4's
    /// list is a control the *document* placed at a point on a page. A menu is not.
    pub(crate) fn menu_key(&mut self, key: &winit::keyboard::Key<&str>) {
        use winit::keyboard::{Key, NamedKey};
        match key {
            Key::Named(NamedKey::Escape) => {
                self.menu.hide();
                self.redraw();
            }
            Key::Named(NamedKey::ArrowUp) => {
                if self.menu.move_by(-1) {
                    self.redraw();
                }
            }
            Key::Named(NamedKey::ArrowDown) => {
                if self.menu.move_by(1) {
                    self.redraw();
                }
            }
            Key::Named(NamedKey::Enter) => self.chose_restriction(),
            // A key with no meaning here. Taken anyway, because the menu is modal.
            _ => {}
        }
    }

    /// A level a person picked out of the menu, sent to the viewer and kept for the next opening.
    ///
    /// The menu stays up, which is what a person setting three levels at once needs, and its rows
    /// are taken again so that the tick follows the choice.
    pub(crate) fn chose_restriction(&mut self) {
        let Some(chose) = self.menu.chosen() else {
            return;
        };
        let command = self.restrictions.chose(chose);
        self.dispatch(command);
        println!("note: {}", viewer_host::chosen(chose));
        let rows = self.restrictions.rows();
        self.menu.refill(rows);
        self.redraw();
    }

    /// The card was answered: open again with what was typed, or say why nothing opened.
    ///
    /// Called from the window, which is where the keyboard is. The [`viewer_core::Secret`] moves
    /// from the card into [`Command::Open`] and is dropped with the command — no copy of it exists
    /// in this host at any point, and none of it reaches a trace.
    pub(crate) fn password_answered(&mut self) {
        let typed = self.password.take();
        let Some(document) = self.locked else {
            return;
        };
        self.redraw();
        // Exhaustive over `Supplied` on purpose, for `ask_again`'s reason.
        let secret = match viewer_host::password::supplied(typed) {
            viewer_host::Supplied::Open(secret) => secret,
            viewer_host::Supplied::Cancelled => {
                println!("note: {}", viewer_host::password::CANCELLED);
                return;
            }
        };
        // The file again — off the disk, or out of the document it was embedded in, which is where
        // Annex O's `ef` left it: §7.11.4 puts an embedded file inside another document, so there
        // is no path to re-read for one (§O.2.1, ADR 0431).
        let bytes = if let Some(bytes) = self.embedded.clone() {
            bytes.into()
        } else {
            // Trap 5, and the second `exit` this method used to carry: a file that has gone away
            // between the first open and the second is a fact about this machine, and a window that
            // vanished rather than saying so would be answering nothing.
            match pdf_syntax::FileBytes::on_disk(&self.path) {
                Ok(bytes) => bytes,
                Err(error) => {
                    println!("note: cannot re-open {}: {error}", self.title);
                    return;
                }
            }
        };
        self.dispatch(Command::Open {
            id: document,
            bytes,
            password: Some(secret),
            fragment: self.fragment.clone(),
        });
    }
}
