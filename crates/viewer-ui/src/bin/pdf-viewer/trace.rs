//! `--trace`: which lines are printed, and what each of them says.
//!
//! One module because these are one decision seen from three places. The topics decide *whether*
//! a line is printed, [`Trace`] decides what it looks like and what clock it carries, and the
//! three `describe_*` functions decide what a window event, a command and an event are called —
//! and a name nobody can turn off is as useless as a topic that names nothing. The graphics
//! stack's own voice is here for the same reason: it is silent until something receives it, and
//! `--trace` is what does.

use viewer_core::{Command, Event};
use winit::event::WindowEvent;

/// What a trace line is *about*.
///
/// **The answer to "a verbosity that is not all-or-nothing", and it is a set rather than a
/// level** — ADR 0227 has the argument. The short of it: these seven are not ordered. A person
/// chasing a slow frame wants `frames` and nothing else; a person chasing a window that never
/// appears wants `launch` and `window`; a level would have to decide which of those is "more
/// verbose" than the other, and there is no such fact. 285 of the 1490 lines of the trace that
/// raised that ADR were pointer moves, and `--trace=-pointer` is the whole of what that person
/// needed to type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Topic {
    /// The launch timeline, the graphics device's bring-up, and the line that closes pipeline
    /// compilation.
    Launch,
    /// One line per frame with its stages, and the summary at exit.
    Frames,
    /// Commands handed to `viewer-core` and the events that came back — pointer moves excepted,
    /// because they are the flood.
    Events,
    /// Window events from winit, the pointer's excepted for the same reason.
    Window,
    /// Pointer movement, both the window event and the command it becomes.
    Pointer,
    /// The accessibility bridge and what it publishes.
    Access,
    /// How many shapes a selection is, which is the number `doc/todo/13` turned on.
    Selection,
    /// What a document-wide search cost, and what its readback cache is holding afterwards.
    Search,
    /// What the sidebar's six panels hold, and what §12.3.4's cost to build.
    ///
    /// The two native hosts have had `viewer_host::Topic::Panel` since they had panels; this host
    /// drew six lists and could not say what any of them cost. It is the one topic where the
    /// *number* is a rule rather than a curiosity: `CLAUDE.md` section 2 forbids thumbnail
    /// generation on the launch path, and a panel's build time is where a host finds out it has
    /// moved eager work rather than removed it.
    Panel,
}

impl Topic {
    /// Every topic, in the order `--trace=?` lists them.
    const ALL: [Self; 9] = [
        Self::Launch,
        Self::Frames,
        Self::Events,
        Self::Window,
        Self::Pointer,
        Self::Access,
        Self::Selection,
        Self::Search,
        Self::Panel,
    ];

    /// What a person types after `--trace=`.
    fn name(self) -> &'static str {
        match self {
            Self::Launch => "launch",
            Self::Frames => "frames",
            Self::Events => "events",
            Self::Window => "window",
            Self::Pointer => "pointer",
            Self::Access => "access",
            Self::Selection => "selection",
            Self::Search => "search",
            Self::Panel => "panel",
        }
    }

    /// This topic's place in [`Trace::topics`].
    fn bit(self) -> u16 {
        match self {
            Self::Launch => 1,
            Self::Frames => 1 << 1,
            Self::Events => 1 << 2,
            Self::Window => 1 << 3,
            Self::Pointer => 1 << 4,
            Self::Access => 1 << 5,
            Self::Selection => 1 << 6,
            Self::Search => 1 << 7,
            Self::Panel => 1 << 8,
        }
    }

    /// The topic a word names, or `None` for a word that is not one.
    fn parse(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|topic| topic.name() == word)
    }
}

/// Every topic at once: what a bare `--trace` asks for, and what `all` names.
const EVERY_TOPIC: u16 = 0x1ff;

/// Whether to say what is happening, about what, and since when.
///
/// `Copy`, deliberately: this is read from inside methods that already hold `&mut self`, and a
/// borrowing instrument would have to be argued with at every call site.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Trace {
    /// The set of [`Topic`] bits `--trace` asked for; zero when it was not given at all.
    pub(crate) topics: u16,
    /// `main`'s first `Instant`, so that every line can carry a clock.
    ///
    /// **ADR 0227**: every line of the trace that raised it carried a *duration* and
    /// none carried a time, so the interval a person actually waited could not be recovered and
    /// a gap in the log could not be told from a run of cheap work. One `Instant::elapsed` per
    /// line — tens of nanoseconds against a `println!` that costs microseconds.
    began: std::time::Instant,
}

impl Trace {
    /// Nothing is traced, which is what a run without the flag gets — and what every run starts
    /// as, since the flag may appear anywhere on the command line.
    pub(crate) fn off(began: std::time::Instant) -> Self {
        Self { topics: 0, began }
    }

    /// Whether anything at all is traced, which is what decides the graphics stack's own voice.
    pub(crate) fn any(self) -> bool {
        self.topics != 0
    }

    /// Whether lines about `topic` are wanted.
    pub(crate) fn on(self, topic: Topic) -> bool {
        self.topics & topic.bit() != 0
    }

    /// One line, stamped with the seconds since this process started.
    ///
    /// The check is repeated here even where the caller has already made it, because the caller
    /// makes it to avoid *formatting* the arguments and this makes it to avoid printing them —
    /// a call site that forgot the first is quiet rather than wrong.
    pub(crate) fn say(self, topic: Topic, what: std::fmt::Arguments<'_>) {
        if !self.on(topic) {
            return;
        }
        println!("trace: {:9.3} {what}", self.began.elapsed().as_secs_f64());
    }

    /// A continuation of the line above: indented under the message column, and carrying no
    /// clock of its own because it happened at the same moment.
    pub(crate) fn more(self, topic: Topic, what: std::fmt::Arguments<'_>) {
        if !self.on(topic) {
            return;
        }
        println!("trace: {:9}   {what}", "");
    }
}

/// The set of topics a `--trace` argument asks for, or the word that named none of them.
///
/// Empty asks for everything, which is what a bare `--trace` has always meant and what the
/// project owner's own invocation types. A list that *starts* with a subtraction is read as
/// "everything except", because `--trace=-pointer` can mean nothing else and demanding
/// `--trace=all,-pointer` would be a rule with no purpose but to be remembered.
pub(crate) fn parse_topics(list: &str) -> Result<u16, String> {
    if list.is_empty() {
        return Ok(EVERY_TOPIC);
    }
    let mut topics = if list.starts_with('-') {
        EVERY_TOPIC
    } else {
        0
    };
    for word in list.split(',') {
        let (word, remove) = match word.strip_prefix('-') {
            Some(rest) => (rest, true),
            None => (word, false),
        };
        if word == "all" {
            topics = if remove { 0 } else { EVERY_TOPIC };
            continue;
        }
        let Some(topic) = Topic::parse(word) else {
            return Err(word.to_owned());
        };
        if remove {
            topics &= !topic.bit();
        } else {
            topics |= topic.bit();
        }
    }
    Ok(topics)
}

/// The topics `--trace=` accepts, for a message that has to list them.
pub(crate) fn topic_names() -> String {
    Topic::ALL
        .iter()
        .map(|topic| topic.name())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Receives what `wgpu`, `vello` and `naga` say about themselves.
///
/// Those three write to the `log` facade, and a facade with nothing behind it drops every record
/// — which is why a page that would not draw produced no output at all (ADR 0126). Twenty lines
/// rather than a logging framework: there is one destination, one format, and one filter, and a
/// configuration language for those would be longer than this.
struct Speak {
    /// The most detailed level to print, from `PDFVIEWER_LOG`.
    level: log::LevelFilter,
}

impl log::Log for Speak {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &log::Record<'_>) {
        if self.enabled(record.metadata()) {
            eprintln!("{}: {}: {}", record.level(), record.target(), record.args());
        }
    }

    fn flush(&self) {}
}

/// Installs [`Speak`], at `PDFVIEWER_LOG`'s level or `warn`.
///
/// `warn` by default because that is the level at which a graphics driver says something is
/// wrong; `PDFVIEWER_LOG=debug` is what to set when *nothing* is wrong and the question is what
/// the device is doing.
pub(crate) fn speak_up() {
    let level = match std::env::var("PDFVIEWER_LOG").unwrap_or_default().as_str() {
        "error" => log::LevelFilter::Error,
        "info" => log::LevelFilter::Info,
        "debug" => log::LevelFilter::Debug,
        "trace" => log::LevelFilter::Trace,
        _ => log::LevelFilter::Warn,
    };
    // A failure here means a logger is already installed, which nothing else in this program
    // does; there is no second thing to try and nothing is lost by carrying on quietly.
    if log::set_boxed_logger(Box::new(Speak { level })).is_ok() {
        log::set_max_level(level);
    }
}

/// One line naming a window event, for `--trace`.
pub(crate) fn describe_window_event(event: &WindowEvent) -> String {
    match event {
        WindowEvent::RedrawRequested => "redraw requested".to_owned(),
        WindowEvent::Resized(size) => format!("resized to {}x{}", size.width, size.height),
        WindowEvent::KeyboardInput { event, .. } => {
            format!("key {:?} {:?}", event.logical_key, event.state)
        }
        WindowEvent::MouseInput { state, button, .. } => format!("mouse {button:?} {state:?}"),
        WindowEvent::CloseRequested => "close requested".to_owned(),
        other => format!("{other:?}"),
    }
}

/// One line naming a command, for `--trace`.
///
/// The command's own `Debug` would print a document's bytes and a raster's pixels, which is not a
/// line. This is what a person following a page turn needs to see.
pub(crate) fn describe_command(command: &Command) -> String {
    match command {
        Command::Open { id, bytes, .. } => format!("open {:?}, {} bytes", id, bytes.len()),
        Command::Close(id) => format!("close {id:?}"),
        Command::Restrict(level) => format!("restrictions {level:?}"),
        Command::Answer { proceed, .. } => format!("answer {proceed}"),
        Command::Delegate(appearances) => format!("widget appearances {appearances:?}"),
        Command::Tick { millis } => format!("tick {millis} ms"),
        Command::Present(mode) => format!("presentation {mode:?}"),
        Command::Layout(layout) => format!("page layout {layout:?}"),
        Command::Focus(id) => format!("focus {id:?}"),
        Command::Resize {
            width,
            height,
            scale,
        } => format!("resize {width}x{height} at {scale}"),
        Command::GoTo(target) => format!("go to {target:?}"),
        Command::Zoom { zoom, at } => format!("zoom {zoom:?} at {at:?}"),
        Command::Scroll { dx, dy } => format!("scroll {dx} {dy}"),
        Command::View(view) => format!(
            "view page {} zoom {:?} scroll {} {}",
            view.page, view.zoom, view.scroll.0, view.scroll.1
        ),
        Command::SetGroup { group, on } => format!("layer {group:?} {on}"),
        Command::Activate(object) => format!("activate {object:?}"),
        Command::Extract { name } => format!("extract {name:?}"),
        Command::Pointer { at, action } => format!("pointer {action:?} at {at:?}"),
        Command::Select(what) => format!("select {what:?}"),
        Command::Focused(move_to) => format!("focus {move_to:?} annotation"),
        Command::Edit(edit) => format!("edit {edit:?}"),
        Command::Undo => "undo".to_owned(),
        Command::Redo => "redo".to_owned(),
        Command::Save => "save".to_owned(),
        Command::Supply { purpose, bytes } => format!(
            "supply {purpose:?}, {}",
            bytes
                .as_ref()
                .map_or_else(|| "declined".to_owned(), |b| format!("{} bytes", b.len()))
        ),
        Command::Find(find) => format!("find {find:?}"),
        Command::RenderReady { token, .. } => format!("render ready {token:?}"),
    }
}

/// One line naming an event, for `--trace`.
pub(crate) fn describe_event(event: &Event) -> String {
    match event {
        Event::Opened { pages, .. } => format!("opened, {pages} page(s)"),
        Event::OpenFailed { reason, .. } => format!("open failed: {reason}"),
        Event::PasswordRequired { .. } => "a password is required".to_owned(),
        Event::Closed(_) => "closed".to_owned(),
        Event::PageChanged { index, of, .. } => format!("page {} of {of}", index.saturating_add(1)),
        Event::NeedsRender(request) => format!(
            "needs render: page {}, {}x{}, {} command(s), {:?}",
            request.page.saturating_add(1),
            request.target.width,
            request.target.height,
            request.list.command_count(),
            request.token
        ),
        Event::Damage(_) => "damage".to_owned(),
        Event::OpenUri { uri, .. } => format!("open uri {uri}"),
        Event::NeedsFile { name, .. } => format!("needs file {name}"),
        Event::Transition { .. } => "a transition".to_owned(),
        Event::Dirty { dirty, .. } => format!("dirty {dirty}"),
        Event::Saved { bytes, .. } => format!("saved, {} bytes", bytes.len()),
        Event::Extracted { name, bytes, .. } => {
            format!("extracted {name:?}, {} bytes", bytes.len())
        }
        Event::Reported { page, notes, .. } => {
            format!("reported about page {page:?}: {}", notes.join("; "))
        }
        Event::Refused {
            operation, notes, ..
        } => format!("refused {}: {}", operation.as_str(), notes.join("; ")),
        Event::Asking {
            operation, notes, ..
        } => format!("asking about {}: {}", operation.as_str(), notes.join("; ")),
        Event::Warned {
            operation, notes, ..
        } => format!("warned about {}: {}", operation.as_str(), notes.join("; ")),
        Event::AttachmentsChanged { .. } => "attachments changed".to_owned(),
        Event::Searched {
            found, remaining, ..
        } => match found {
            Some(found) => format!("searched: page {}, {:?}", found.page, found.range),
            None => format!("searched: nothing yet, {remaining} page(s) left"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{EVERY_TOPIC, Topic, parse_topics};

    /// `--trace` with no list means every topic, and "every" is the enum's own answer.
    ///
    /// [`EVERY_TOPIC`] is a hand-written bit pattern beside a list a session may extend, and the
    /// four-hundred-and-twentieth extended it: `search` is the eighth, and a mask left at `0x7f`
    /// would have made a bare `--trace` silently the seven it used to be. The gate is arithmetic
    /// rather than a number written twice.
    #[test]
    fn a_bare_trace_asks_for_every_topic_there_is() {
        let every = Topic::ALL
            .into_iter()
            .fold(0_u16, |bits, topic| bits | topic.bit());
        assert_eq!(every, EVERY_TOPIC, "the mask and the list are one thing");
        assert_eq!(parse_topics(""), Ok(EVERY_TOPIC), "a bare --trace");
        assert_eq!(parse_topics("all"), Ok(EVERY_TOPIC));
        for topic in Topic::ALL {
            assert_eq!(
                parse_topics(&format!("-{}", topic.name())),
                Ok(EVERY_TOPIC & !topic.bit()),
                "everything except {}",
                topic.name()
            );
        }
        assert_eq!(
            Topic::ALL.map(Topic::bit).len(),
            Topic::ALL.len(),
            "no two topics share a bit"
        );
    }
}
