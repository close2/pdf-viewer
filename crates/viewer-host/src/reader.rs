//! The reader's policy words, read the same way by every window that takes them.
//!
//! [`crate::policy`] states the words — [`TRUST_ANCHORS`] and [`ACCEPT_UNKNOWN_REVOCATION`] for
//! §12.8.1's third question, [`REFERENCE_FILES`] for §8.10.4's target documents,
//! [`READER_NAME`], [`READER_TITLE`], [`READER_ORGANISATION`] and [`INTERFACE_LANGUAGE`] for
//! §8.11.4.4's two categories about the reader, [`SCRIPTS`] for RFC 0008 section 6.3's level, and
//! [`RECIPIENT_CERTIFICATE`] and [`RECIPIENT_KEY`] for §7.6.5.1's match — and what each value
//! becomes. This is the other half: which words a command line holds, refused
//! where one is mistyped, and the five commands they become, in the order a window sends them
//! before its first document.
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
    RECIPIENT_CERTIFICATE, RECIPIENT_KEY, REFERENCE_FILES, SCRIPTS, Scripts, TRUST_ANCHORS,
    TRUSTED_LISTS, audience, now, recipient_key, reference_files, scripting, scripts,
    trust_anchors, trusted_lists,
};

/// What a person typed of the reader's policy words.
///
/// Every field defaults to the word not having been typed, under which the five commands
/// [`ReaderWords::commands`] makes are the ones a host that never sent them would have meant: no
/// anchor, no target document, nobody reading, no script run, no recipient key.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReaderWords {
    /// The directory [`TRUST_ANCHORS`] named: RFC 5280 section 6.1.1's input (d).
    pub trust_anchors: Option<PathBuf>,
    /// The directory [`TRUSTED_LISTS`] named: ETSI TS 119 612 lists beside the anchors.
    pub trusted_lists: Option<PathBuf>,
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
    /// The level [`SCRIPTS`] named, the last one typed where it was typed twice: `off` where none
    /// was (`doc/questions/A193`).
    pub scripts: Scripts,
    /// The conversion [`crate::archiving::ARCHIVE`] chose, the last one typed where it was typed
    /// twice: what the archive key asks for (ADR 1954). Not a command sent at launch — a
    /// conversion is something a person asks for later, by the key.
    pub archive: Option<viewer_core::Archiving>,
    /// The file [`RECIPIENT_CERTIFICATE`] named: the certificate §7.6.5.1's match looks for.
    pub recipient_certificate: Option<PathBuf>,
    /// The file [`RECIPIENT_KEY`] named: the private key that belongs to it.
    pub recipient_key: Option<PathBuf>,
}

/// The five commands the words became, and every file a person named that did not take.
#[derive(Debug)]
pub struct ReaderSupply {
    /// `Command::Trust`, `Command::References`, `Command::Audience`, `Command::Scripts` and
    /// `Command::Recipient`, in that order, each sent before the window's first `Command::Open`.
    pub commands: [Command; 5],
    /// One sentence per file in a named directory that is not an anchor or not a target document.
    ///
    /// Said rather than dropped: a person who named six files and got four would otherwise read
    /// verdicts and pages computed under a supply they did not make (trap 5).
    pub refused: Vec<String>,
}

impl ReaderWords {
    /// The words as a usage line spells them, for a window's own usage message.
    pub const USAGE: &'static str = "[--trust-anchors <dir>] [--trusted-lists <dir>] [--accept-unknown-revocation] \
         [--reference-files <dir>] [--reader-name <name>] [--reader-title <title>] \
         [--reader-organisation <organisation>] [--interface-language <tag>] \
         [--scripts off|ask|warn|on] [--archive <target>:<profile>] \
         [--archive-profiles <dir>] [--recipient-certificate <file>] \
         [--recipient-key <file>]";

    /// Takes `word` if it is one of the thirteen, reading the value after it from `rest`.
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
        } else if word == TRUSTED_LISTS {
            self.trusted_lists = Some(directory(
                TRUSTED_LISTS,
                "a directory of ETSI TS 119 612 trusted lists",
                rest.next(),
            )?);
        } else if word == RECIPIENT_CERTIFICATE {
            self.recipient_certificate = Some(file(
                RECIPIENT_CERTIFICATE,
                "a DER or PEM certificate",
                rest.next(),
            )?);
        } else if word == RECIPIENT_KEY {
            self.recipient_key = Some(file(
                RECIPIENT_KEY,
                "a DER or PEM private key",
                rest.next(),
            )?);
        } else if word == REFERENCE_FILES {
            self.reference_files = Some(directory(
                REFERENCE_FILES,
                "a directory of PDF files",
                rest.next(),
            )?);
        } else if word == crate::archiving::ARCHIVE_PROFILES {
            crate::archiving::offer_profiles_from(directory(
                crate::archiving::ARCHIVE_PROFILES,
                "a directory of archive profiles (.toml)",
                rest.next(),
            )?);
        } else if word == crate::archiving::ARCHIVE {
            let value = rest
                .next()
                .map(|value| value.into().to_string_lossy().into_owned())
                .ok_or_else(|| {
                    format!(
                        "{} wants <target>:<profile>, as in 2b:as-if-printed",
                        crate::archiving::ARCHIVE
                    )
                })?;
            self.archive = Some(crate::archiving::chosen(&value)?);
        } else if word == SCRIPTS {
            let level = rest
                .next()
                .map(|value| value.into().to_string_lossy().into_owned())
                .ok_or_else(|| format!("{SCRIPTS} wants one of off, ask, warn, on"))?;
            self.scripts = scripts(&level)?;
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

    /// What the words become: reads the two directories and the two recipient files, and builds
    /// the five commands.
    ///
    /// Called on a window's document thread rather than before its window, because nothing in it
    /// is needed to show page one (`CLAUDE.md` principle 2), and with no directory named it reads
    /// nothing at all. A window sends the commands before its first `Command::Open`, for
    /// `Command::Restrict`'s reason — a policy applied halfway through is not a policy — and once:
    /// each applies to every document opened afterwards.
    #[must_use]
    pub fn commands(&self) -> ReaderSupply {
        let (mut trust, anchors_refused) = trust_anchors(
            self.trust_anchors.as_deref(),
            self.accept_unknown_revocation,
        );
        let (lists, lists_refused) = trusted_lists(self.trusted_lists.as_deref());
        // A verdict through a list, and ETSI TS 119 615 clause 4.1's "has passed", are asked at
        // the instant the anchors carry; with no anchor directory named the anchors carry none,
        // so the lists bring this machine's clock with them.
        if trust.anchors.is_empty() && !lists.is_empty() {
            trust.anchors = pdf_signature::trust::Supply::of(Vec::new(), String::new(), now());
        }
        trust.lists = lists;
        let (files, files_refused) = reference_files(self.reference_files.as_deref());
        // §7.6.5.1's two files, read on this side of the confinement and carried as bytes (ADR
        // 2822); nothing is read where neither word was typed.
        let (recipient, recipient_refused) = recipient_key(
            self.recipient_certificate.as_deref(),
            self.recipient_key.as_deref(),
        );
        let refused = anchors_refused
            .iter()
            .map(ToString::to_string)
            .chain(lists_refused.iter().map(ToString::to_string))
            .chain(files_refused.iter().map(ToString::to_string))
            .chain(recipient_refused.iter().map(ToString::to_string))
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
                Command::Scripts(scripting(self.scripts)),
                Command::Recipient(recipient),
            ],
            refused,
        }
    }
}

/// The file after a file word, or the sentence that refuses it.
fn file<W: Into<OsString>>(word: &str, wants: &str, value: Option<W>) -> Result<PathBuf, String> {
    let file = PathBuf::from(value.ok_or_else(|| format!("{word} wants {wants}"))?.into());
    if file.is_file() {
        Ok(file)
    } else {
        Err(format!("{word} {}: not a file", file.display()))
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
                viewer_core::Command::Scripts(viewer_core::Scripting::Off),
                viewer_core::Command::Recipient(None),
            ] => {
                assert!(trust.anchors.is_empty(), "no anchor");
                assert!(files.files.is_empty(), "no target document");
                assert!(audience.is_empty(), "nobody reading");
            }
            other => panic!(
                "Trust, References, Audience, Scripts off, no Recipient in that order: {other:?}"
            ),
        }
    }

    /// The archive word chooses a target and a shipped profile, and a profile not shipped is refused
    /// by naming the ones that are (ADR 1954).
    #[test]
    fn the_archive_word_chooses_what_the_archive_key_asks_for() {
        let (words, left) = read(&["--archive", "2b:as-if-printed", "doc.pdf"]).expect("chosen");
        assert_eq!(left, ["doc.pdf"]);
        let chosen = words.archive.expect("a conversion chosen");
        assert_eq!(chosen.profile, "as-if-printed");
        let complaint = read(&["--archive", "2b:nothing-lost"]).expect_err("no such profile");
        assert!(complaint.contains("as-shown"), "{complaint}");
        let complaint = read(&["--archive"]).expect_err("a value is wanted");
        assert!(complaint.contains("2b:as-if-printed"), "{complaint}");
    }

    /// RFC 0008 section 6.3's four words, the last one typed standing, and a fifth refused by name.
    #[test]
    fn the_scripts_word_takes_one_of_four_levels() {
        let (words, left) =
            read(&["--scripts", "ask", "doc.pdf", "--scripts", "warn"]).expect("two levels");
        assert_eq!(words.scripts, crate::Scripts::Warn);
        assert_eq!(left, ["doc.pdf"]);
        assert!(matches!(
            words.commands().commands[3],
            viewer_core::Command::Scripts(viewer_core::Scripting::Run(_))
        ));
        let (words, _) = read(&["--scripts", "ask"]).expect("one level");
        assert!(matches!(
            words.commands().commands[3],
            viewer_core::Command::Scripts(viewer_core::Scripting::Ask(_))
        ));
        let complaint = read(&["--scripts", "maybe"]).expect_err("no such level");
        assert!(complaint.contains("off, ask, warn, on"), "{complaint}");
        let complaint = read(&["--scripts"]).expect_err("a level is wanted");
        assert!(complaint.contains("--scripts wants"), "{complaint}");
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
