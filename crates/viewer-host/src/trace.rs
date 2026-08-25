//! What a launch cost, printed in the shape `viewer-ui`'s `--trace` prints it.
//!
//! `CLAUDE.md` makes the launch path a measured thing rather than an assumed one, and this host
//! is a program a person runs — so it owes the same discipline. The line format is deliberately
//! identical to `viewer-ui`'s (`trace: <seconds since main> <what>`) so that the two hosts' launch
//! timelines can be read side by side; ADR 0227's finding is kept with it, that a line carrying
//! only a *duration* cannot say how long a person waited.

use std::time::Instant;

/// What a `--trace` line is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Topic {
    /// The launch timeline: arguments, the toolkit's own start-up, the document, the first frame.
    Launch,
    /// One line per frame, with what rasterising it cost.
    Frames,
    /// Commands handed to `viewer-core` and the events that came back.
    Events,
    /// What the three trees and the form controls were built from.
    Panel,
    /// What the pointer is over — §12.5.6.5's activation region, and so the cursor.
    ///
    /// Its own topic rather than a line under [`Topic::Events`], because a pointer move is already
    /// a command and this is the *answer* to a question asked beside it. It is also the one thing
    /// a window does that cannot be photographed on a headless display: `xwd` does not capture the
    /// pointer, so a trace line is the only instrument there is for it here.
    Pointer,
    /// §14.7's structure as it is published to an assistive technology, and what a client asked for.
    ///
    /// Its own topic for [`Topic::Pointer`]'s reason and a sharper version of it: what a screen
    /// reader is told cannot be photographed at all, and on a machine with no screen reader
    /// installed the accessibility bus and this line are the whole of the instrument. `viewer-ui`
    /// has had a topic of this name in its own private table since ADR 0214; the two native hosts
    /// gained one when they gained the bridge (ADR 0623).
    Access,
}

impl Topic {
    /// Every topic, in the order `--trace=?` lists them.
    const ALL: [Self; 6] = [
        Self::Launch,
        Self::Frames,
        Self::Events,
        Self::Panel,
        Self::Pointer,
        Self::Access,
    ];

    /// What a person types after `--trace=`.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Launch => "launch",
            Self::Frames => "frames",
            Self::Events => "events",
            Self::Panel => "panel",
            Self::Pointer => "pointer",
            Self::Access => "access",
        }
    }

    /// This topic's place in [`Trace`]'s set.
    fn bit(self) -> u8 {
        match self {
            Self::Launch => 1,
            Self::Frames => 1 << 1,
            Self::Events => 1 << 2,
            Self::Panel => 1 << 3,
            Self::Pointer => 1 << 4,
            Self::Access => 1 << 5,
        }
    }

    /// The topic a word names, or `None` for a word that is not one.
    fn parse(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|topic| topic.name() == word)
    }
}

/// Every topic at once, which is what a bare `--trace` asks for.
///
/// **Derived from [`Topic::ALL`] rather than written down, and it was written down until the
/// seven-hundred-and-thirty-first session.** The literal said `0b1111` while the enumeration had
/// five topics, so `--trace` and `--trace=all` silently asked for four of them and
/// [`Topic::Pointer`] — added one round earlier, and the *only* instrument for a thing that cannot
/// be photographed — printed nothing unless a person named it. A constant that has to be edited
/// when an enumeration grows is a constant that will not be, which is `doc/todo/02` §7's own
/// "a count that improves is not a picture" one file over.
fn every_topic() -> u8 {
    let mut topics = 0;
    let mut at = 0;
    while at < Topic::ALL.len() {
        topics |= Topic::ALL[at].bit();
        at = at.saturating_add(1);
    }
    topics
}

/// The set of topics a `--trace=` argument asks for, or the word that named none of them.
///
/// Empty asks for everything, which is what a bare `--trace` means in `viewer-ui` and what the
/// project owner's own invocation types. A list beginning with a subtraction is read as
/// "everything except", for the same reason it is there: `--trace=-panel` can mean nothing else.
///
/// # Errors
///
/// The first word that names no topic, so that a typo is refused rather than silently tracing
/// less than was asked for.
pub fn parse_topics(list: &str) -> Result<u8, String> {
    if list.is_empty() {
        return Ok(every_topic());
    }
    let mut topics = if list.starts_with('-') {
        every_topic()
    } else {
        0
    };
    for word in list.split(',') {
        let (remove, word) = match word.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, word),
        };
        if word == "all" {
            topics = if remove { 0 } else { every_topic() };
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

/// Which topics are being printed, and the clock every line is stamped against.
#[derive(Debug, Clone, Copy)]
pub struct Trace {
    /// The set of [`Topic`] bits, zero when `--trace` was not given.
    topics: u8,
    /// `main`'s first [`Instant`], so that every line carries a time and not only a duration.
    began: Instant,
}

impl Trace {
    /// Nothing is traced, which is what a run without the flag gets.
    #[must_use]
    pub fn off(began: Instant) -> Self {
        Self { topics: 0, began }
    }

    /// The topics a parsed `--trace` argument asked for.
    #[must_use]
    pub fn of(topics: u8, began: Instant) -> Self {
        Self { topics, began }
    }

    /// Whether lines about `topic` are wanted.
    #[must_use]
    pub fn on(self, topic: Topic) -> bool {
        self.topics & topic.bit() != 0
    }

    /// How long this process has been running, for a line that wants to say so itself.
    #[must_use]
    pub fn since_start(self) -> std::time::Duration {
        self.began.elapsed()
    }

    /// One line, stamped with the seconds since this process started.
    ///
    /// The check is repeated here even where the caller has already made it: a caller makes it to
    /// avoid *formatting* its arguments and this makes it to avoid *printing* them, so a call
    /// site that forgot the first is quiet rather than wrong.
    pub fn say(self, topic: Topic, what: std::fmt::Arguments<'_>) {
        if !self.on(topic) {
            return;
        }
        println!("trace: {:9.3} {what}", self.began.elapsed().as_secs_f64());
    }

    /// A continuation of the line above, carrying no clock because it happened at the same moment.
    pub fn more(self, topic: Topic, what: std::fmt::Arguments<'_>) {
        if !self.on(topic) {
            return;
        }
        println!("trace: {:9}   {what}", "");
    }
}

#[cfg(test)]
mod tests {
    use super::{Topic, parse_topics};

    /// A bare `--trace` asks for every topic there is, including the one added last.
    ///
    /// **Run against the defect before it was believed** (trap 13): with the literal `0b1111` this
    /// file carried until the seven-hundred-and-thirty-first session, this test fails on
    /// [`Topic::Pointer`] and on [`Topic::Access`] and passes on the other four — which is exactly
    /// what a person typing `--trace` saw, and exactly what nothing said.
    #[test]
    fn a_bare_trace_asks_for_every_topic() {
        let asked = parse_topics("").expect("the empty list names no unknown topic");
        for topic in Topic::ALL {
            assert!(
                asked & topic.bit() != 0,
                "--trace did not ask for the {} topic",
                topic.name()
            );
        }
        assert_eq!(asked, parse_topics("all").expect("`all` is a word here"));
    }

    /// "Everything except" is the same set with one taken out, whatever the set has grown to.
    #[test]
    fn a_subtraction_takes_one_topic_out_of_all_of_them() {
        let asked = parse_topics("-panel").expect("`panel` is a topic");
        assert_eq!(asked & Topic::Panel.bit(), 0);
        for topic in Topic::ALL {
            if topic != Topic::Panel {
                assert!(asked & topic.bit() != 0, "{} was lost", topic.name());
            }
        }
    }

    /// A word that names no topic is refused rather than quietly tracing less than was asked for.
    #[test]
    fn a_word_that_names_no_topic_is_the_error() {
        assert_eq!(parse_topics("frames,acess"), Err("acess".to_owned()));
    }
}
