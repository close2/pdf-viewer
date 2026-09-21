//! The Qt 6 viewer: `quorra-qt [--trace[=topics]] <file.pdf[#fragment]>`.
//!
//! A third program beside `quorra` and `quorra-gtk`, and deliberately not a flag on
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
use viewer_core::{RestrictionLevel, RestrictionPolicy};
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
    restrictions: RestrictionPolicy,
    /// §12.6.4.8: what this window does when a link asks for a URI, per [`viewer_host::LINKS`].
    ///
    /// The other direction from the entry above: what a *document* asserts over its reader there,
    /// and what a document asks this machine to start here. `viewer_host::Links::Ask` unless a
    /// person said otherwise, which is the level that hands nothing over without a press and still
    /// performs the act the clause describes (ADR 1155).
    links: viewer_host::Links,
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
    let mut restrictions = RestrictionPolicy::default();
    let mut links = viewer_host::Links::default();
    for word in words {
        if word == "--draw-widget-appearances" {
            widget_appearances = WidgetAppearances::Drawn;
        } else if word == IGNORE_RESTRICTIONS {
            restrictions = RestrictionPolicy::uniform(RestrictionLevel::Off);
        } else if let Some(list) = word.strip_prefix(viewer_host::RESTRICTIONS) {
            // `CLAUDE.md`'s four levels, one operation at a time. Not a user interface and said so
            // where it is written down (`doc/todo/38`): it is the channel a window has today, and
            // the levels behind it are the ones a menu will set (ADR 1144).
            restrictions = viewer_host::restrictions(list, restrictions)?;
        } else if let Some(level) = word.strip_prefix(viewer_host::LINKS) {
            // §12.6.4.8's act, at one of the same four levels (ADR 1155).
            links = viewer_host::links(level)?;
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
            "usage: quorra-qt [--trace[=topics]] [--draw-widget-appearances] \
             [{IGNORE_RESTRICTIONS}] [--restrictions=copy:ask,annotate:on] \
             [--links=refuse|ask|warn|open] [--quit-after=<ms>] <file.pdf>"
        )
    })?;
    Ok(Arguments {
        path,
        fragment,
        topics,
        widget_appearances,
        restrictions,
        links,
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
        arguments.links,
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
    /// The same test `quorra-gtk` carries, because the two hosts must agree about it: a native
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
    /// The same test `quorra-gtk` carries, and the same defect behind it: `Host::react`
    /// answered `viewer_core::Event::Refused` with a sentence naming `--ignore-restrictions` while
    /// `arguments` answered that word with *"is not an option this program has"* and exit 1. Two
    /// hosts wrote the sentence independently and both got it wrong the same way, which is what a
    /// copied sentence does. ADR 0604.
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

        // And the level a person can now name per operation, which is the half of `doc/todo/38`
        // that was owed (ADR 1144).
        let asked = arguments(
            [
                "--restrictions=copy:ask,annotate:on".to_owned(),
                "x.pdf".to_owned(),
            ]
            .into_iter(),
        )
        .expect("a document");
        assert_eq!(
            asked.restrictions.level(Operation::Extract),
            RestrictionLevel::Ask
        );
        assert_eq!(
            asked.restrictions.level(Operation::Annotate),
            RestrictionLevel::On
        );
        assert_eq!(
            asked.restrictions.level(Operation::FillInForm),
            RestrictionLevel::Off,
            "an operation the list did not name keeps the level it had"
        );
        assert!(
            arguments(["--restrictions=copy:maybe".to_owned(), "x.pdf".to_owned()].into_iter())
                .is_err(),
            "a level this program does not have is a sentence rather than a guess"
        );
    }
}
