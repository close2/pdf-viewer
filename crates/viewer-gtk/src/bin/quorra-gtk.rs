//! The GTK4 viewer: `quorra-gtk [--trace[=topics]] <file.pdf[#fragment]>...`.
//!
//! A second program beside `quorra`, and deliberately not a flag on it: the two differ in
//! their *toolkit* and in nothing else, which is the claim `viewer-core` exists to make and which
//! a single binary linking both would stop making.
//!
//! `CLAUDE.md` makes the launch path a measured thing, so `--trace=launch` prints the same shape
//! of timeline `quorra` does: arguments read, window built, first allocation, document
//! opened, first frame on the screen.

#![forbid(unsafe_code)]

use std::time::Instant;

use pdf_model::view::WidgetAppearances;

use gtk4::prelude::*;
use gtk4::{gio, glib};
use viewer_core::{RestrictionLevel, RestrictionPolicy};
use viewer_gtk::Host;
use viewer_host::{IGNORE_RESTRICTIONS, Topic, Trace, parse_topics};

/// What the command line asked for.
#[derive(Debug)]
struct Arguments {
    /// The document, with Annex O's fragment where the word carried one.
    document: viewer_host::Named,
    /// Every document named after the first, each to open as a tab of its own behind it once the
    /// first page is on the screen (ADR 1275).
    also: Vec<viewer_host::Named>,
    /// The topics `--trace` asked for, zero for a run without it.
    topics: u8,
    /// Who draws §12.7's widgets, per `--draw-widget-appearances`.
    widget_appearances: WidgetAppearances,
    /// What this reader does with the restrictions a document asserts, per
    /// [`viewer_host::RESTRICTIONS`] and [`viewer_host::IGNORE_RESTRICTIONS`].
    ///
    /// One level per operation, all six [`RestrictionLevel::Off`] unless a person said otherwise:
    /// `CLAUDE.md` is why — "it shall always be possible to turn them off" — and until ADR 0604
    /// this program printed the word and then refused it. A command line is not the menu
    /// `doc/todo/38` still wants, and it is the channel a window has today (ADR 1144).
    restrictions: RestrictionPolicy,
    /// §12.6.4.8: what this window does when a link asks for a URI, per [`viewer_host::LINKS`].
    ///
    /// The other direction from the entry above: what a *document* asserts over its reader there,
    /// and what a document asks this machine to start here. `viewer_host::Links::Ask` unless a
    /// person said otherwise, which is the level that hands nothing over without a press and still
    /// performs the act the clause describes (ADR 1155).
    links: viewer_host::Links,
    /// §12.6.4.3: what this window does when an action names another file, per
    /// [`viewer_host::REMOTE_DOCUMENTS`].
    ///
    /// The same four levels as the entry above and a value of its own: one starts another program
    /// on a URL, the other opens a PDF beside this one in this reader (ADR 1227).
    remote_documents: viewer_host::RemoteDocuments,
    /// §10.8.3: whether this window asks for the separation simulation, per
    /// [`viewer_host::SEPARATIONS`].
    ///
    /// A preference rather than a level, and off unless a person said otherwise (ADR 1228).
    separations: bool,
}

/// Reads the command line, or says what is wrong with it.
fn arguments(words: impl Iterator<Item = String>) -> Result<Arguments, String> {
    let mut documents: Vec<viewer_host::Named> = Vec::new();
    let mut topics = 0;
    let mut widget_appearances = WidgetAppearances::Delegated;
    let mut restrictions = RestrictionPolicy::default();
    let mut links = viewer_host::Links::default();
    let mut remote_documents = viewer_host::RemoteDocuments::default();
    let mut separations = false;
    for word in words {
        if word == "--draw-widget-appearances" {
            widget_appearances = WidgetAppearances::Drawn;
        } else if word == IGNORE_RESTRICTIONS {
            restrictions = RestrictionPolicy::uniform(RestrictionLevel::Off);
        } else if let Some(list) = word.strip_prefix(viewer_host::RESTRICTIONS) {
            // `CLAUDE.md`'s four levels, one operation at a time (ADR 1144).
            restrictions = viewer_host::restrictions(list, restrictions)?;
        } else if let Some(level) = word.strip_prefix(viewer_host::LINKS) {
            // §12.6.4.8's act, at one of the same four levels (ADR 1155).
            links = viewer_host::links(level)?;
        } else if let Some(level) = word.strip_prefix(viewer_host::REMOTE_DOCUMENTS) {
            // §12.6.4.3's act, at four levels of its own (ADR 1227).
            remote_documents = viewer_host::remote_documents(level)?;
        } else if let Some(setting) = word.strip_prefix(viewer_host::SEPARATIONS) {
            // §10.8.3's simulation, which is a preference and has two words (ADR 1228).
            separations = viewer_host::separations(setting)?;
        } else if word == "--trace" {
            topics = parse_topics("")?;
        } else if let Some(list) = word.strip_prefix("--trace=") {
            topics = parse_topics(list)
                .map_err(|unknown| format!("--trace: {unknown} names no topic"))?;
        } else if word.starts_with("--") {
            return Err(format!("{word} is not an option this program has"));
        } else {
            // `viewer_host::Named` is the one reading of a document word for three windows,
            // Annex O's fragment and a file with a `#` in its name included.
            documents.push(viewer_host::Named::from_argument(std::ffi::OsStr::new(
                &word,
            )));
        }
    }
    let mut documents = documents.into_iter();
    let document = documents.next().ok_or_else(|| {
        format!(
            "usage: quorra-gtk [--trace[=topics]] [--draw-widget-appearances] \
             [{IGNORE_RESTRICTIONS}] [--links=refuse|ask|warn|open] \
             [--remote-documents=refuse|ask|warn|open] [--separations=on|off] <file.pdf>..."
        )
    })?;
    Ok(Arguments {
        document,
        also: documents.collect(),
        topics,
        widget_appearances,
        restrictions,
        links,
        remote_documents,
        separations,
    })
}

fn main() -> glib::ExitCode {
    let began = Instant::now();
    let arguments = match arguments(std::env::args().skip(1)) {
        Ok(arguments) => arguments,
        Err(complaint) => {
            eprintln!("{complaint}");
            return glib::ExitCode::FAILURE;
        }
    };
    let trace = if arguments.topics == 0 {
        Trace::off(began)
    } else {
        Trace::of(arguments.topics, began)
    };
    trace.say(Topic::Launch, format_args!("arguments read"));

    // `NON_UNIQUE` because this is a document viewer and two documents are two windows; without
    // it a second invocation would hand its file to the first process and exit, which is a
    // decision about how a desktop works rather than about how a PDF is read.
    let app = gtk4::Application::new(Some("org.pdfviewer.gtk"), gio::ApplicationFlags::NON_UNIQUE);
    let failed = std::rc::Rc::new(std::cell::Cell::new(false));
    let watched = std::rc::Rc::clone(&failed);
    // Every callback in the host holds itself *weakly*, so something has to hold it strongly for
    // as long as the application runs; this is that something.
    let held: std::cell::RefCell<Vec<std::rc::Rc<std::cell::RefCell<Host>>>> =
        std::cell::RefCell::new(Vec::new());
    app.connect_activate(move |app| {
        trace.say(Topic::Launch, format_args!("GTK ready"));
        match Host::open(
            app,
            &arguments.document.path,
            arguments.document.fragment.clone(),
            arguments.widget_appearances,
            viewer_host::Settings {
                restrictions: arguments.restrictions,
                links: arguments.links,
                remote_documents: arguments.remote_documents,
                separations: arguments.separations,
            },
            trace,
        ) {
            Ok(host) => {
                host.borrow_mut().open_behind(arguments.also.clone());
                held.borrow_mut().push(host);
            }
            Err(error) => {
                eprintln!("{error}");
                watched.set(true);
                app.quit();
            }
        }
    });
    // GTK's own argument parsing is deliberately not given ours: `--trace` is this program's and
    // a document is a path rather than a GTK option.
    let code = app.run_with_args::<&str>(&[]);
    if failed.get() {
        return glib::ExitCode::FAILURE;
    }
    code
}

#[cfg(test)]
mod tests {
    use super::arguments;

    #[test]
    fn a_fragment_is_split_off_the_path_undecoded() {
        // Annex O, and ADR 0209's rule: the text after `#` crosses exactly as it was written,
        // because percent-decoding belongs to whoever knows which component it is decoding.
        let read = arguments(["doc/x.pdf#nameddest=A%26B".to_owned()].into_iter())
            .expect("a path with a fragment is a document");
        assert_eq!(read.document.path.to_string_lossy(), "doc/x.pdf");
        assert_eq!(read.document.fragment.as_deref(), Some("nameddest=A%26B"));
    }

    /// Every document word after the first is a tab of its own, in the order it was typed.
    #[test]
    fn every_path_after_the_first_is_a_document_beside_it() {
        let read = arguments(
            [
                "a.pdf".to_owned(),
                "--trace".to_owned(),
                "b.pdf#page=2".to_owned(),
                "c.pdf".to_owned(),
            ]
            .into_iter(),
        )
        .expect("three documents");
        assert_eq!(read.document.path.to_string_lossy(), "a.pdf");
        let also: Vec<_> = read
            .also
            .iter()
            .map(|named| {
                (
                    named.path.to_string_lossy().into_owned(),
                    named.fragment.clone(),
                )
            })
            .collect();
        assert_eq!(
            also,
            [
                ("b.pdf".to_owned(), Some("page=2".to_owned())),
                ("c.pdf".to_owned(), None)
            ]
        );
    }

    #[test]
    fn an_unknown_trace_topic_is_refused_rather_than_ignored() {
        let complaint =
            arguments(["--trace=frames,wrong".to_owned(), "x.pdf".to_owned()].into_iter())
                .expect_err("a topic that does not exist is a mistake worth reporting");
        assert!(complaint.contains("wrong"), "{complaint}");
    }

    /// §6.3.2.2's default is the standard's, and this host's default is the other one.
    ///
    /// Worth a test rather than a comment because it is the one place `quorra-gtk` and
    /// `quorra` disagree about what a page is, and the flag that undoes it is what ADR 0245's
    /// two photographs were taken with.
    #[test]
    fn the_widgets_are_delegated_unless_the_flag_asks_for_them() {
        use pdf_model::view::WidgetAppearances;
        let asked = arguments(["x.pdf".to_owned()].into_iter()).expect("a document");
        assert_eq!(asked.widget_appearances, WidgetAppearances::Delegated);
        let asked =
            arguments(["--draw-widget-appearances".to_owned(), "x.pdf".to_owned()].into_iter())
                .expect("a document");
        assert_eq!(asked.widget_appearances, WidgetAppearances::Drawn);
    }

    /// §12.6.4.8's level is the reader's, and the word that sets it is one every window takes.
    ///
    /// **The defect this is written against is `doc/todo/30`'s levelness rule**, which two hosts
    /// of three have already failed once over `--ignore-restrictions` (ADR 0604): a word one
    /// window's parser takes and another's rejects is a policy that exists in one build. This is
    /// this window's end of it; `viewer-host`'s `four_levels_decide_whether_a_link_reaches_this_
    /// machine` is the other (ADR 1155).
    #[test]
    fn the_level_a_link_is_opened_at_is_the_readers_to_set() {
        let asked = arguments(["x.pdf".to_owned()].into_iter()).expect("a document");
        assert_eq!(
            asked.links,
            viewer_host::Links::Ask,
            "nothing is handed to another program without a keypress, and nothing has to be \
             configured before a link works"
        );
        for level in viewer_host::Links::ALL {
            let word = format!("{}{}", viewer_host::LINKS, level.as_str());
            let asked = arguments([word.clone(), "x.pdf".to_owned()].into_iter())
                .unwrap_or_else(|complaint| panic!("{word}: {complaint}"));
            assert_eq!(asked.links, level);
        }
        let complaint = arguments(
            [
                format!("{}sometimes", viewer_host::LINKS),
                "x.pdf".to_owned(),
            ]
            .into_iter(),
        )
        .expect_err("a level that does not exist is a mistake worth reporting");
        assert!(complaint.contains("sometimes"), "{complaint}");
    }

    #[test]
    fn a_run_with_no_document_says_how_to_run_it() {
        let complaint = arguments(std::iter::empty()).expect_err("a document is required");
        assert!(complaint.starts_with("usage:"), "{complaint}");
    }

    /// The word this window's refusal names has to be a word this program takes.
    ///
    /// **Written against the defect rather than for the feature.** `Host::react` answered
    /// `viewer_core::Event::Refused` with a sentence naming `--ignore-restrictions` from this
    /// host's first session, and `arguments` answered that same word with *"is not an option this
    /// program has"* and exit 1 — so `CLAUDE.md`'s "it shall always be possible to turn them off"
    /// was true in one host of three while all three said it was true. The constant is what ties
    /// the sentence and the parser together; this asserts the parser's end of it and
    /// `viewer-host`'s `the_refusal_names_the_word_that_turns_the_restrictions_off` the other.
    /// ADR 0604.
    #[test]
    fn the_word_the_refusal_names_turns_the_restrictions_off() {
        use pdf_model::restriction::Operation;
        use viewer_core::{RestrictionLevel, RestrictionPolicy};
        let asked = arguments(
            [
                viewer_host::IGNORE_RESTRICTIONS.to_owned(),
                "x.pdf".to_owned(),
            ]
            .into_iter(),
        )
        .expect("a document");
        assert_eq!(
            asked.restrictions,
            RestrictionPolicy::uniform(RestrictionLevel::Off)
        );
        let asked = arguments(["x.pdf".to_owned()].into_iter()).expect("a document");
        assert_eq!(
            asked.restrictions,
            RestrictionPolicy::default(),
            "every operation off, because `CLAUDE.md` says a document's restrictions are low \
             priority and that turning them off shall always be possible"
        );

        let asked =
            arguments(["--restrictions=copy:warn".to_owned(), "x.pdf".to_owned()].into_iter())
                .expect("a document");
        assert_eq!(
            asked.restrictions.level(Operation::Extract),
            RestrictionLevel::Warn
        );
        assert_eq!(
            asked.restrictions.level(Operation::Annotate),
            RestrictionLevel::Off,
            "an operation the list did not name keeps the level it had"
        );
    }
}
