//! Commands into `viewer-core`, events back out, and what this window does about each.
//!
//! The whole of the boundary in one place, which is what makes it readable as a boundary: every
//! command this program sends goes through [`App::pump`] and every event it receives is answered
//! in [`App::react`]. A reader asking "what happens when a page turns" finds the answer here and
//! the work itself in the module the arm names.

use std::collections::VecDeque;
use std::io::Write as _;

use viewer_core::{Command, Event};

use crate::app::App;
use crate::trace::{Topic, describe_command, describe_event};

/// How many passwords a person is asked for before the program gives up.
///
/// §7.6.4.1 states no limit — it says a processor tries the empty password and then prompts —
/// so this is a choice about a terminal rather than about the clause, and an empty line cancels
/// before it is reached.
const PASSWORD_ATTEMPTS: usize = 3;

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
            // **A command that changes the document changes what §14.7's tree says**, and nothing
            // republished it until the five-hundred-and-ninetieth session: `App::attend` compares
            // the page and the viewport, and an edit moves neither — so a check box a person
            // ticked went on being announced as unticked, and after ADR 0425 that included one an
            // assistive technology had clicked itself. Forgetting what was last published is what
            // makes the next frame say it again. `Event::Dirty` looked like the condition and is
            // not: it fires when the flag *changes*, so only the first edit of a session raises it.
            if matches!(
                command,
                Command::Edit(_) | Command::Undo | Command::Redo | Command::SetGroup { .. }
            ) {
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

    /// Does what one event asks.
    fn react(&mut self, event: Event, queue: &mut VecDeque<Command>) {
        match event {
            Event::Opened { pages, .. } => {
                println!("{}: {pages} page(s)", self.title);
                if pages == 0 {
                    eprintln!("the document has no pages");
                    std::process::exit(1);
                }
                self.attempts = 0;
                self.gather();
            }
            Event::OpenFailed { reason, .. } => {
                eprintln!("cannot open {}: {reason}", self.title);
                std::process::exit(1);
            }
            Event::PasswordRequired { document } => self.ask_again(document, queue),
            Event::Closed(_) => {}
            Event::PageChanged {
                index,
                label,
                of,
                section,
                ..
            } => {
                // ISO 32000-2 §12.4.2: "Page labels and page indices need not coincide". Where
                // the document states a label it is what a reader is meant to see — a page of
                // front matter is *iv*, not page four — so the index is shown beside it rather
                // than instead of it, because a title saying only `iv` cannot say `of 320`.
                let page = match label {
                    Some(label) => format!("{label} — page {} of {of}", index.saturating_add(1)),
                    None => format!("page {} of {of}", index.saturating_add(1)),
                };
                // §12.3.3's outline is a table of contents, so the item covering this page is
                // the section a reader is in. After the page number rather than before it,
                // because it is context for a position rather than the position itself.
                self.caption = match section {
                    Some(section) if !section.is_empty() => format!("{page} — {section}"),
                    _ => page,
                };
                self.retitle();
            }
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
            // §12.6.4.8: printed rather than opened. What this program will not do is hand a
            // string a document controls to a browser, because that is a decision about this
            // machine and not about the document.
            Event::OpenUri { uri, .. } => println!("link: {uri}"),
            Event::NeedsFile { purpose, name, .. } => {
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
    /// §7.6.4.1: a processor tries the default user password and then prompts.
    ///
    /// This is the prompt, and it is the whole of what this program owed the clause. Split out of
    /// [`Self::react`] rather than written there because the *file* it re-opens has two
    /// provenances since Annex O's `ef` opened one.
    fn ask_again(&mut self, document: viewer_core::DocumentId, queue: &mut VecDeque<Command>) {
        self.attempts = self.attempts.saturating_add(1);
        if self.attempts > PASSWORD_ATTEMPTS {
            eprintln!("{}: too many attempts", self.title);
            std::process::exit(1);
        }
        let Some(password) = ask_password(&self.title) else {
            eprintln!("{}: needs a password", self.title);
            std::process::exit(1);
        };
        // The file again — off the disk, or out of the document it was embedded in, which is where
        // Annex O's `ef` left it: §7.11.4 puts an embedded file inside another document, so there
        // is no path to re-read for one (§O.2.1, ADR 0431).
        let bytes = if let Some(bytes) = self.embedded.clone() {
            bytes
        } else {
            let Ok(bytes) = std::fs::read(&self.path) else {
                eprintln!("cannot re-read {}", self.title);
                std::process::exit(1);
            };
            bytes
        };
        queue.push_back(Command::Open {
            id: document,
            bytes,
            password: Some(password),
            fragment: self.fragment.clone(),
        });
    }
}

/// Reads a password from the terminal, or `None` if the person cancelled with an empty line.
fn ask_password(name: &str) -> Option<String> {
    eprint!("{name} needs a password (empty line to give up): ");
    std::io::stderr().flush().ok()?;
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).ok()?;
    let password = line.trim_end_matches(['\r', '\n']).to_owned();
    (!password.is_empty()).then_some(password)
}
