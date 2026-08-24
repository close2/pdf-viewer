//! The GTK4 viewer: `pdf-viewer-gtk [--trace[=topics]] <file.pdf[#fragment]>`.
//!
//! A second program beside `pdf-viewer`, and deliberately not a flag on it: the two differ in
//! their *toolkit* and in nothing else, which is the claim `viewer-core` exists to make and which
//! a single binary linking both would stop making.
//!
//! `CLAUDE.md` makes the launch path a measured thing, so `--trace=launch` prints the same shape
//! of timeline `pdf-viewer` does: arguments read, window built, first allocation, document
//! opened, first frame on the screen.

#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::time::Instant;

use pdf_model::view::WidgetAppearances;

use gtk4::prelude::*;
use gtk4::{gio, glib};
use viewer_core::RestrictionLevel;
use viewer_gtk::Host;
use viewer_host::{IGNORE_RESTRICTIONS, Topic, Trace, parse_topics};

/// What the command line asked for.
#[derive(Debug)]
struct Arguments {
    /// The document.
    path: PathBuf,
    /// Annex O's fragment — the text after `#`, undecoded, because splitting a URI is the host's
    /// and percent-decoding belongs to whoever knows which component it is decoding (ADR 0209).
    fragment: Option<String>,
    /// The topics `--trace` asked for, zero for a run without it.
    topics: u8,
    /// Who draws §12.7's widgets, per `--draw-widget-appearances`.
    widget_appearances: WidgetAppearances,
    /// What this reader does with the restrictions a document asserts, per
    /// [`viewer_host::IGNORE_RESTRICTIONS`].
    ///
    /// **Not a user interface for them**, which `doc/todo/38` says is not to be built yet: it is
    /// the one policy value `viewer-core` asks for, supplied the way this host supplies the other
    /// one it has (§6.3.2.2's widget appearances, one field up). `CLAUDE.md` is why it is here at
    /// all — "it shall always be possible to turn them off" — and until ADR 0604 this program
    /// printed the word and then refused it.
    restrictions: RestrictionLevel,
}

/// Reads the command line, or says what is wrong with it.
fn arguments(words: impl Iterator<Item = String>) -> Result<Arguments, String> {
    let mut path: Option<PathBuf> = None;
    let mut fragment = None;
    let mut topics = 0;
    let mut widget_appearances = WidgetAppearances::Delegated;
    let mut restrictions = RestrictionLevel::On;
    for word in words {
        if word == "--draw-widget-appearances" {
            widget_appearances = WidgetAppearances::Drawn;
        } else if word == IGNORE_RESTRICTIONS {
            restrictions = RestrictionLevel::Off;
        } else if word == "--trace" {
            topics = parse_topics("")?;
        } else if let Some(list) = word.strip_prefix("--trace=") {
            topics = parse_topics(list)
                .map_err(|unknown| format!("--trace: {unknown} names no topic"))?;
        } else if word.starts_with("--") {
            return Err(format!("{word} is not an option this program has"));
        } else if path.is_some() {
            return Err("one document at a time".to_owned());
        } else {
            // Annex O: the fragment is the text after `#` in the URI the bytes came from. A path
            // is not a URI, but a path with a `#` in it is how a person types one on a command
            // line, and `pdf-viewer` reads it the same way.
            match word.split_once('#') {
                Some((before, after)) => {
                    path = Some(PathBuf::from(before));
                    fragment = Some(after.to_owned());
                }
                None => path = Some(PathBuf::from(word)),
            }
        }
    }
    let path = path.ok_or_else(|| {
        format!(
            "usage: pdf-viewer-gtk [--trace[=topics]] [--draw-widget-appearances] \
             [{IGNORE_RESTRICTIONS}] <file.pdf>"
        )
    })?;
    Ok(Arguments {
        path,
        fragment,
        topics,
        widget_appearances,
        restrictions,
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
            &arguments.path,
            arguments.fragment.clone(),
            arguments.widget_appearances,
            arguments.restrictions,
            trace,
        ) {
            Ok(host) => held.borrow_mut().push(host),
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
        assert_eq!(read.path.to_string_lossy(), "doc/x.pdf");
        assert_eq!(read.fragment.as_deref(), Some("nameddest=A%26B"));
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
    /// Worth a test rather than a comment because it is the one place `pdf-viewer-gtk` and
    /// `pdf-viewer` disagree about what a page is, and the flag that undoes it is what ADR 0245's
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
        use viewer_core::RestrictionLevel;
        let asked = arguments(
            [
                viewer_host::IGNORE_RESTRICTIONS.to_owned(),
                "x.pdf".to_owned(),
            ]
            .into_iter(),
        )
        .expect("a document");
        assert_eq!(asked.restrictions, RestrictionLevel::Off);
        let asked = arguments(["x.pdf".to_owned()].into_iter()).expect("a document");
        assert_eq!(
            asked.restrictions,
            RestrictionLevel::On,
            "obeying is the default, because a reader that ignored a document's restrictions \
             without being asked would be choosing on the person's behalf in the other direction"
        );
    }
}
