//! The Qt 6 viewer: `pdf-viewer-qt [--trace[=topics]] <file.pdf[#fragment]>`.
//!
//! A third program beside `pdf-viewer` and `pdf-viewer-gtk`, and deliberately not a flag on
//! either: the three differ in their *toolkit* and in nothing else, which is the claim
//! `viewer-core` exists to make and which a single binary linking all three would stop making.
//!
//! `CLAUDE.md` makes the launch path a measured thing, so `--trace=launch` prints the same shape
//! of timeline the other two do: arguments read, window built, first resize, document opened,
//! first frame on the screen — in `viewer-host`'s one format, so that the two native hosts'
//! timelines can be read side by side.

#![deny(unsafe_code)]

use std::path::PathBuf;
use std::time::Instant;

use pdf_model::view::WidgetAppearances;
use viewer_core::RestrictionLevel;
use viewer_host::{IGNORE_RESTRICTIONS, Topic, Trace, parse_topics};
use viewer_qt::Host;

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
    /// one it has. `CLAUDE.md` is why it is here at all — "it shall always be possible to turn them
    /// off" — and until ADR 0604 this program printed the word and then refused it.
    restrictions: RestrictionLevel,
    /// How many milliseconds to run for before quitting, or zero to run until closed.
    ///
    /// A window under `Xvfb` has nobody to close it, and a test that killed the process could not
    /// tell a clean exit from a crash. `viewer-gtk` has no equivalent and is stopped with a
    /// signal; this is the better of the two and is the one thing this host has that the other
    /// does not.
    quit_after: i32,
}

/// Reads the command line, or says what is wrong with it.
fn arguments(words: impl Iterator<Item = String>) -> Result<Arguments, String> {
    let mut path: Option<PathBuf> = None;
    let mut fragment = None;
    let mut topics = 0;
    let mut widget_appearances = WidgetAppearances::Delegated;
    let mut quit_after = 0;
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
        } else if let Some(millis) = word.strip_prefix("--quit-after=") {
            quit_after = millis
                .parse::<i32>()
                .map_err(|_| format!("--quit-after: {millis} is not a millisecond count"))?;
        } else if word.starts_with("--") {
            return Err(format!("{word} is not an option this program has"));
        } else if path.is_some() {
            return Err("one document at a time".to_owned());
        } else {
            // Annex O: the fragment is the text after `#` in the URI the bytes came from. A path
            // is not a URI, but a path with a `#` in it is how a person types one on a command
            // line, and the other two hosts read it the same way.
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
            "usage: pdf-viewer-qt [--trace[=topics]] [--draw-widget-appearances] \
             [{IGNORE_RESTRICTIONS}] [--quit-after=<ms>] <file.pdf>"
        )
    })?;
    Ok(Arguments {
        path,
        fragment,
        topics,
        widget_appearances,
        restrictions,
        quit_after,
    })
}

fn main() -> std::process::ExitCode {
    let began = Instant::now();
    let arguments = match arguments(std::env::args().skip(1)) {
        Ok(arguments) => arguments,
        Err(complaint) => {
            eprintln!("{complaint}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let trace = if arguments.topics == 0 {
        Trace::off(began)
    } else {
        Trace::of(arguments.topics, began)
    };
    trace.say(Topic::Launch, format_args!("arguments read"));

    let host = match Host::open(
        &arguments.path,
        arguments.fragment,
        arguments.widget_appearances,
        arguments.restrictions,
        trace,
    ) {
        Ok(host) => host,
        Err(error) => {
            eprintln!("{error}");
            return std::process::ExitCode::FAILURE;
        }
    };
    trace.say(
        Topic::Launch,
        format_args!("host ready, handing Qt the loop"),
    );
    let code = viewer_qt::run(host, arguments.quit_after);
    if code == 0 {
        std::process::ExitCode::SUCCESS
    } else {
        std::process::ExitCode::FAILURE
    }
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
    /// The same test `pdf-viewer-gtk` carries, because the two hosts must agree about it: a native
    /// form host places a control over every widget, so leaving the appearance underneath would
    /// be the duplication ADR 0244 photographed and ADR 0245 removed.
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
    fn a_quit_after_that_is_not_a_number_is_refused() {
        let complaint = arguments(["--quit-after=soon".to_owned(), "x.pdf".to_owned()].into_iter())
            .expect_err("a millisecond count is a number");
        assert!(complaint.contains("soon"), "{complaint}");
    }

    #[test]
    fn a_run_with_no_document_says_how_to_run_it() {
        let complaint = arguments(std::iter::empty()).expect_err("a document is required");
        assert!(complaint.starts_with("usage:"), "{complaint}");
    }

    /// The word this window's refusal names has to be a word this program takes.
    ///
    /// The same test `pdf-viewer-gtk` carries, and the same defect behind it: `Host::react`
    /// answered `viewer_core::Event::Refused` with a sentence naming `--ignore-restrictions` while
    /// `arguments` answered that word with *"is not an option this program has"* and exit 1. Two
    /// hosts wrote the sentence independently and both got it wrong the same way, which is what a
    /// copied sentence does. ADR 0604.
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
