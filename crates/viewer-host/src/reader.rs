//! The reader's three policy words, read the same way by every window that takes them.
//!
//! [`crate::policy`] states the words — [`TRUST_ANCHORS`] and [`ACCEPT_UNKNOWN_REVOCATION`] for
//! §12.8.1's third question, [`REFERENCE_FILES`] for §8.10.4's target documents, and
//! [`READER_NAME`], [`READER_TITLE`], [`READER_ORGANISATION`] and [`INTERFACE_LANGUAGE`] for
//! §8.11.4.4's two categories about the reader — and what each value becomes. This is the other
//! half: which words a command line holds, refused where one is mistyped, and the three commands
//! they become, in the order a window sends them before its first document.
//!
//! **One reading rather than one per window**, because the levelness rule `doc/todo/30` keeps is
//! that a word one window takes and another refuses is a policy that exists in one build. A
//! toolkit's own parser (`GApplication`'s options, `QCommandLineParser`) would give each window a
//! second spelling of the same words and a second set of refusals, and ADR 1581 is the decision
//! not to.

use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use viewer_core::Command;

use crate::policy::{
    ACCEPT_UNKNOWN_REVOCATION, INTERFACE_LANGUAGE, READER_NAME, READER_ORGANISATION, READER_TITLE,
    REFERENCE_FILES, TRUST_ANCHORS, audience, reference_files, trust_anchors,
};

/// What a person typed of the reader's three policy words.
///
/// Every field defaults to the word not having been typed, under which the three commands
/// [`ReaderWords::commands`] makes are the ones a host that never sent them would have meant: no
/// anchor, no target document, nobody reading.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReaderWords {
    /// The directory [`TRUST_ANCHORS`] named: RFC 5280 section 6.1.1's input (d).
    pub trust_anchors: Option<PathBuf>,
    /// Whether [`ACCEPT_UNKNOWN_REVOCATION`] was typed.
    pub accept_unknown_revocation: bool,
    /// The directory [`REFERENCE_FILES`] named: §8.10.4's target documents.
    pub reference_files: Option<PathBuf>,
    /// Every value of [`READER_NAME`], in the order typed — Table 100's `Ind`.
    pub names: Vec<String>,
    /// Every value of [`READER_TITLE`] — Table 100's `Ttl`.
    pub titles: Vec<String>,
    /// Every value of [`READER_ORGANISATION`] — Table 100's `Org`.
    pub organisations: Vec<String>,
    /// The value of [`INTERFACE_LANGUAGE`], the last one typed where it was typed twice.
    pub language: Option<String>,
}

/// The three commands the words became, and every file a person named that did not take.
#[derive(Debug)]
pub struct ReaderSupply {
    /// `Command::Trust`, `Command::References` and `Command::Audience`, in that order, each sent
    /// before the window's first `Command::Open`.
    pub commands: [Command; 3],
    /// One sentence per file in a named directory that is not an anchor or not a target document.
    ///
    /// Said rather than dropped: a person who named six files and got four would otherwise read
    /// verdicts and pages computed under a supply they did not make (trap 5).
    pub refused: Vec<String>,
}

impl ReaderWords {
    /// The words as a usage line spells them, for a window's own usage message.
    pub const USAGE: &'static str = "[--trust-anchors <dir>] [--accept-unknown-revocation] \
         [--reference-files <dir>] [--reader-name <name>] [--reader-title <title>] \
         [--reader-organisation <organisation>] [--interface-language <tag>]";

    /// Takes `word` if it is one of the seven, reading the value after it from `rest`.
    ///
    /// `Ok(false)` where `word` is none of them, which leaves it to the window's own parser.
    ///
    /// # Errors
    ///
    /// The sentence to print where a word wants a value and has none, or where a directory word
    /// names something that is not a directory. Refused rather than ignored: a launch that
    /// swallowed the word would answer every signature's third question with *nobody*, draw every
    /// proxy, or choose layers for an audience nobody named, while the person who typed it
    /// believed otherwise.
    pub fn take<W: Into<OsString>>(
        &mut self,
        word: &OsStr,
        rest: &mut impl Iterator<Item = W>,
    ) -> Result<bool, String> {
        if word == ACCEPT_UNKNOWN_REVOCATION {
            self.accept_unknown_revocation = true;
        } else if word == TRUST_ANCHORS {
            self.trust_anchors = Some(directory(
                TRUST_ANCHORS,
                "a directory of PEM or DER certificates",
                rest.next(),
            )?);
        } else if word == REFERENCE_FILES {
            self.reference_files = Some(directory(
                REFERENCE_FILES,
                "a directory of PDF files",
                rest.next(),
            )?);
        } else if word == READER_NAME
            || word == READER_TITLE
            || word == READER_ORGANISATION
            || word == INTERFACE_LANGUAGE
        {
            let value = rest
                .next()
                .map(|value| value.into().to_string_lossy().into_owned())
                .ok_or_else(|| format!("{} wants a value", word.to_string_lossy()))?;
            if word == READER_NAME {
                self.names.push(value);
            } else if word == READER_TITLE {
                self.titles.push(value);
            } else if word == READER_ORGANISATION {
                self.organisations.push(value);
            } else {
                self.language = Some(value);
            }
        } else {
            return Ok(false);
        }
        Ok(true)
    }

    /// What the words become: reads the two directories and builds the three commands.
    ///
    /// Called on a window's document thread rather than before its window, because nothing in it
    /// is needed to show page one (`CLAUDE.md` principle 2), and with no directory named it reads
    /// nothing at all. A window sends the commands before its first `Command::Open`, for
    /// `Command::Restrict`'s reason — a policy applied halfway through is not a policy — and once:
    /// each applies to every document opened afterwards.
    #[must_use]
    pub fn commands(&self) -> ReaderSupply {
        let (trust, anchors_refused) = trust_anchors(
            self.trust_anchors.as_deref(),
            self.accept_unknown_revocation,
        );
        let (files, files_refused) = reference_files(self.reference_files.as_deref());
        let refused = anchors_refused
            .iter()
            .map(ToString::to_string)
            .chain(files_refused.iter().map(ToString::to_string))
            .collect();
        ReaderSupply {
            commands: [
                Command::Trust(trust),
                Command::References(files),
                Command::Audience(audience(
                    &self.names,
                    &self.titles,
                    &self.organisations,
                    self.language.as_deref(),
                )),
            ],
            refused,
        }
    }
}

/// The directory after a directory word, or the sentence that refuses it.
fn directory<W: Into<OsString>>(
    word: &str,
    wants: &str,
    value: Option<W>,
) -> Result<PathBuf, String> {
    let directory = PathBuf::from(value.ok_or_else(|| format!("{word} wants {wants}"))?.into());
    if directory.is_dir() {
        Ok(directory)
    } else {
        Err(format!("{word} {}: not a directory", directory.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::ReaderWords;

    /// Reads a whole command line the way a window's loop does, keeping what no word took.
    fn read(line: &[&str]) -> Result<(ReaderWords, Vec<String>), String> {
        let mut words = ReaderWords::default();
        let mut left = Vec::new();
        let mut line = line.iter().map(|word| (*word).to_owned());
        while let Some(word) = line.next() {
            if !words.take(std::ffi::OsStr::new(&word), &mut line)? {
                left.push(word);
            }
        }
        Ok((words, left))
    }

    /// §8.11.4.4's two categories: three lists kept apart, as Table 100's `/Type` keeps them.
    #[test]
    fn the_audience_words_are_kept_apart_by_what_they_name() {
        let (words, left) = read(&[
            "--reader-name",
            "Ada",
            "doc.pdf",
            "--reader-organisation",
            "Acme",
            "--reader-name",
            "Grace",
            "--reader-title",
            "Editor",
            "--interface-language",
            "es-MX",
        ])
        .expect("every word has its value");
        assert_eq!(left, ["doc.pdf"], "a document word is the window's");
        assert_eq!(words.names, ["Ada", "Grace"]);
        assert_eq!(words.titles, ["Editor"]);
        assert_eq!(words.organisations, ["Acme"]);
        assert_eq!(words.language.as_deref(), Some("es-MX"));
        let supply = words.commands();
        let viewer_core::Command::Audience(audience) = &supply.commands[2] else {
            panic!(
                "the third command is the audience: {:?}",
                supply.commands[2]
            );
        };
        assert_eq!(audience.reader.individual, ["Ada", "Grace"]);
        assert_eq!(audience.reader.organisation, ["Acme"]);
    }

    /// A word with nothing after it, or a directory that is not one, is a sentence and not a run.
    #[test]
    fn a_mistyped_word_is_refused_rather_than_ignored() {
        let complaint = read(&["doc.pdf", "--reader-name"]).expect_err("a name is wanted");
        assert!(
            complaint.contains("--reader-name wants a value"),
            "{complaint}"
        );
        let complaint =
            read(&["--trust-anchors", "/nowhere/at/all", "doc.pdf"]).expect_err("not a directory");
        assert!(complaint.contains("not a directory"), "{complaint}");
        let complaint = read(&["--reference-files"]).expect_err("a directory is wanted");
        assert!(
            complaint.contains("a directory of PDF files"),
            "{complaint}"
        );
    }

    /// Nothing typed is nothing supplied, and the three commands say so in their own defaults.
    #[test]
    fn no_word_is_no_anchor_no_target_and_nobody_reading() {
        let (words, left) = read(&["doc.pdf"]).expect("a document alone");
        assert_eq!(words, ReaderWords::default());
        assert_eq!(left, ["doc.pdf"]);
        let supply = words.commands();
        assert!(supply.refused.is_empty());
        match &supply.commands {
            [
                viewer_core::Command::Trust(trust),
                viewer_core::Command::References(files),
                viewer_core::Command::Audience(audience),
            ] => {
                assert!(trust.anchors.is_empty(), "no anchor");
                assert!(files.files.is_empty(), "no target document");
                assert!(audience.is_empty(), "nobody reading");
            }
            other => panic!("Trust, References, Audience in that order: {other:?}"),
        }
    }

    /// The revocation word takes no value, so the word after it is still the window's.
    #[test]
    fn the_revocation_word_takes_no_value() {
        let (words, left) =
            read(&["--accept-unknown-revocation", "doc.pdf"]).expect("a flag and a document");
        assert!(words.accept_unknown_revocation);
        assert_eq!(left, ["doc.pdf"]);
    }
}
