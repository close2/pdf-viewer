//! The remedy configurations this program ships, compiled in, and what one adds to a conversion.
//!
//! `doc/rfc/0007` section 5a's profiles: each is a list of refusal sites with an answer beside
//! each, a lower bound on what a conversion may lose and never an instruction to lose it (ADR
//! 1942). They are compiled in so that a profile named by its name works wherever the program is
//! installed, and **they are listed here, in the library, once**: the command-line program's
//! `--profile <name>` and a window's archive command read the same list, so a profile added to
//! `doc/profiles/` reaches both or neither (ADR 1954). `tests/profiles.rs` holds every one of them
//! to every target and this list to the directory.

use std::path::{Path, PathBuf};

use pdf_archive::Target;

use crate::archive::{
    Authorisations, ConfigError, Configuration, Departure, Derivation, FormAnswers, Preservation,
    Resolution, Supply, Unbuilt,
};
use crate::tool::Tool;

/// One profile this program ships: the name it states for itself, and its text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shipped {
    /// The `[profile] name` the file states, which is the word a person types or chooses.
    pub name: &'static str,
    /// The configuration, as `doc/profiles/<name>.toml` holds it.
    pub text: &'static str,
}

impl Shipped {
    /// The one sentence the profile states about itself, `[profile] sentence`, which is what a
    /// window offering the profile says beside its name (ADR 1966).
    ///
    /// Read from [`Self::text`] rather than written beside [`Self::name`], so the sentence a person
    /// is offered is the file's and cannot drift from it; `None` where the file states none.
    ///
    /// # Errors
    ///
    /// [`ConfigError`] where the text does not read as a configuration, which `tests/profiles.rs`
    /// holds no shipped profile to.
    pub fn sentence(&self) -> Result<Option<String>, ConfigError> {
        // The sentence is the same whichever target the profile is read for, and every shipped
        // profile reads for all six, so the first target is as good as any.
        Configuration::read(self.text, Target::ALL[0]).map(|config| config.sentence)
    }
}

/// Every profile this program ships, in the order a person is offered them.
pub const SHIPPED: [Shipped; 8] = [
    Shipped {
        name: "refuse-any-loss",
        text: include_str!("../../../doc/profiles/refuse-any-loss.toml"),
    },
    Shipped {
        name: "as-if-printed",
        text: include_str!("../../../doc/profiles/as-if-printed.toml"),
    },
    Shipped {
        name: "as-shown",
        text: include_str!("../../../doc/profiles/as-shown.toml"),
    },
    Shipped {
        name: "only-metadata-loss",
        text: include_str!("../../../doc/profiles/only-metadata-loss.toml"),
    },
    Shipped {
        name: "keep-everything",
        text: include_str!("../../../doc/profiles/keep-everything.toml"),
    },
    Shipped {
        name: "derive-attachments",
        text: include_str!("../../../doc/profiles/derive-attachments.toml"),
    },
    // `doc/rfc/0014` section 7's two, shipped on `doc/questions/A706` items 3 and 4.
    Shipped {
        name: "no-data-loss",
        text: include_str!("../../../doc/profiles/no-data-loss.toml"),
    },
    Shipped {
        name: "state-every-loss",
        text: include_str!("../../../doc/profiles/state-every-loss.toml"),
    },
];

/// The shipped profile called `name`, where there is one.
#[must_use]
pub fn named(name: &str) -> Option<&'static Shipped> {
    SHIPPED.iter().find(|shipped| shipped.name == name)
}

/// The shipped profiles' names, joined for a sentence that offers them.
#[must_use]
pub fn names() -> String {
    SHIPPED
        .iter()
        .map(|shipped| shipped.name)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Reads `text` at each of the six targets, and answers the first at which it does not read.
///
/// **An operator's file is checked before any document is**, the check `tests/profiles.rs` holds
/// every shipped profile to (`doc/rfc/0014` section 6, `doc/questions/A706` item 5): a
/// configuration whose answer one target does not admit is the operator's mistake whichever
/// target a run asks for, and finding it in the middle of a batch of documents — or never, because
/// no document of the batch reached that site — is what the check is for. A site a target does not
/// bind is no error at that target ([`Configuration::read`]), so a file answering only part 4's
/// sites reads at part 2 as well.
///
/// # Errors
///
/// The first target in [`Target::ALL`]'s order at which the text does not read, with the
/// [`ConfigError`] that says why.
pub fn read_at_every_target(text: &str) -> Result<(), (Target, ConfigError)> {
    for target in Target::ALL {
        Configuration::read(text, target).map_err(|error| (target, error))?;
    }
    Ok(())
}

/// One profile an operator wrote, read from a directory a host names.
///
/// `doc/rfc/0014` section 6: **a window offers the operator's profiles from a directory the host
/// names, never one a document names**, beside the shipped list — ADR 1954's one list, widened by
/// the host's directory. Each file is held to what [`read_at_every_target`] holds a shipped one
/// to, so a profile a window offers reads at every target before any document is converted under
/// it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorProfile {
    /// The `[profile] name` the file states, which is the word a person chooses.
    pub name: String,
    /// The `[profile] sentence` it states, where it states one.
    pub sentence: Option<String>,
    /// The configuration, as the file holds it.
    pub text: String,
    /// The file it was read from.
    pub path: PathBuf,
}

/// A file of an operator's directory that is not offered, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotOffered {
    /// The file.
    pub path: PathBuf,
    /// The sentence saying why it is not offered.
    pub why: String,
}

/// How many files of an operator's directory are read.
///
/// `CLAUDE.md` principle 3's explicit bound: a directory can hold anything, and a chooser offering
/// ten thousand profiles is no choice. Far past the shipped list's length.
pub const MOST_OPERATOR_PROFILES: usize = 64;

/// The longest operator's profile read, in bytes; checked before the file is read.
///
/// The longest shipped profile is a few tens of kilobytes, so this is a ceiling on a mistake
/// rather than on a profile.
pub const MOST_OPERATOR_PROFILE_BYTES: u64 = 256 * 1024;

/// Every `.toml` file of `directory`, in file-name order: the profiles that read at all six
/// targets and state a name no shipped profile and no earlier file states, and every other file
/// with the reason it is not offered.
///
/// Read when asked rather than at launch (`CLAUDE.md` principle 2's *no configuration scanning*):
/// a window asks when its chooser opens and again when it converts, so a file an operator edits
/// between the two is read as it then is, and held to the same check again.
#[must_use]
pub fn operators_profiles(directory: &Path) -> (Vec<OperatorProfile>, Vec<NotOffered>) {
    let mut offered: Vec<OperatorProfile> = Vec::new();
    let mut refused = Vec::new();
    let mut paths: Vec<PathBuf> = match std::fs::read_dir(directory) {
        Ok(entries) => entries
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "toml")
            })
            .collect(),
        Err(error) => {
            refused.push(NotOffered {
                path: directory.to_path_buf(),
                why: format!("the directory does not read: {error}"),
            });
            return (offered, refused);
        }
    };
    paths.sort();
    for (index, path) in paths.into_iter().enumerate() {
        if index >= MOST_OPERATOR_PROFILES {
            refused.push(NotOffered {
                path,
                why: format!(
                    "the directory holds more than the {MOST_OPERATOR_PROFILES} profiles a window \
                     offers (MOST_OPERATOR_PROFILES)"
                ),
            });
            continue;
        }
        match operators_profile(&path, &offered) {
            Ok(profile) => offered.push(profile),
            Err(why) => refused.push(NotOffered { path, why }),
        }
    }
    (offered, refused)
}

/// One operator's file, read and checked, or the sentence saying why it is not offered.
fn operators_profile(path: &Path, earlier: &[OperatorProfile]) -> Result<OperatorProfile, String> {
    let length = std::fs::metadata(path)
        .map_err(|error| format!("the file does not read: {error}"))?
        .len();
    if length > MOST_OPERATOR_PROFILE_BYTES {
        return Err(format!(
            "the file is {length} bytes, past the {MOST_OPERATOR_PROFILE_BYTES} an operator's \
             profile may be (MOST_OPERATOR_PROFILE_BYTES)"
        ));
    }
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("the file does not read: {error}"))?;
    read_at_every_target(&text)
        .map_err(|(target, error)| format!("the profile does not read at {target}: {error}"))?;
    let config = Configuration::read(&text, Target::ALL[0])
        .map_err(|error| format!("the profile does not read: {error}"))?;
    let name = config.name.clone().ok_or_else(|| {
        "the profile states no [profile] name, which is the word a person chooses".to_owned()
    })?;
    if named(&name).is_some() {
        return Err(format!(
            "the profile is named {name:?}, which is a shipped profile's name, and the shipped \
             one keeps it"
        ));
    }
    if earlier.iter().any(|profile| profile.name == name) {
        return Err(format!(
            "the profile is named {name:?}, which an earlier file of the directory already states"
        ));
    }
    Ok(OperatorProfile {
        name,
        sentence: config.sentence,
        text,
        path: path.to_path_buf(),
    })
}

/// What one configuration adds to an archive plan for one target.
///
/// **Read once, by whichever face is converting**, so that the command line and a window fold a
/// configuration into a plan the same way: the losses its `discard` remedies stand for, the
/// departures it names, its answers at the two form sites, and the remedies that need something
/// a pass cannot do for itself. What a face does with the rest — saying the unbuilt rows and the
/// declared tools, refusing a departure nobody stated at the call site — is the face's, because
/// how a person is told differs between a terminal and a window (ADR 1954).
#[derive(Debug, Clone)]
pub struct Contribution {
    /// The `[profile] name` the configuration states for itself, where it states one.
    pub name: Option<String>,
    /// The losses its built `discard` remedies stand for.
    pub authorised: Authorisations,
    /// The departures from ISO 19005 it names that this version carries out.
    pub departures: Vec<Departure>,
    /// What it answers at the two interactive form sites (`construct`, `dynamic`).
    pub forms: FormAnswers,
    /// The `derive` remedies it names, each with the tool it declares.
    pub derivations: Vec<Derivation>,
    /// The `supply` remedies it names.
    pub supplies: Vec<Supply>,
    /// The `preserve` remedies it names that append pages.
    pub preservations: Vec<Preservation>,
    /// Per-object annotation flag answers, where the profile names shape rows.
    pub annotation_flags: Option<crate::archive::AnnotationFlagAnswers>,
    /// The `preserve` remedies it names that fetch what a stream keeps outside the file.
    pub resolutions: Vec<Resolution>,
    /// The sites it answers with a remedy this version does not carry out yet.
    pub unbuilt: Vec<Unbuilt>,
    /// Every external program it declares, in name order.
    pub tools: Vec<Tool>,
}

impl Contribution {
    /// Reads `text` as a configuration for `target`.
    ///
    /// # Errors
    ///
    /// [`ConfigError`] where the text is not a configuration this version reads: a site that
    /// does not exist, a remedy word it does not know, a `derive` with no tool.
    pub fn read(text: &str, target: Target) -> Result<Self, ConfigError> {
        let config = Configuration::read(text, target)?;
        Ok(Self {
            name: config.name.clone(),
            authorised: config.authorisations(target),
            departures: config.built_departures(target),
            forms: config.form_answers(target),
            derivations: config.derivations(target),
            supplies: config.supplies(target),
            preservations: config.preservations(target),
            annotation_flags: config.annotation_flags,
            resolutions: config.resolutions(target),
            unbuilt: config.unbuilt(target),
            tools: config.tools().cloned().collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every shipped profile states the name it is listed under, so the word a person chooses is
    /// the word the report names.
    #[test]
    fn each_shipped_profile_states_the_name_it_is_listed_under() {
        for shipped in SHIPPED {
            let read = Contribution::read(shipped.text, Target::Two(pdf_archive::Level::B))
                .expect("a shipped profile reads");
            assert_eq!(read.name.as_deref(), Some(shipped.name), "{}", shipped.name);
        }
    }

    /// Every shipped profile states the one sentence a window offers beside its name.
    #[test]
    fn each_shipped_profile_states_its_sentence() {
        for shipped in SHIPPED {
            let sentence = shipped.sentence().expect("a shipped profile reads");
            assert!(
                sentence.is_some_and(|sentence| sentence.ends_with('.')),
                "{}",
                shipped.name
            );
        }
    }

    #[test]
    fn a_name_not_shipped_is_none_and_the_names_are_offered() {
        assert!(named("as-if-printed").is_some());
        assert!(named("as-if-printd").is_none());
        assert!(names().contains("as-shown"), "{}", names());
    }
}
