//! The remedy configurations this program ships, compiled in, and what one adds to a conversion.
//!
//! `doc/rfc/0007` section 5a's profiles: each is a list of refusal sites with an answer beside
//! each, a lower bound on what a conversion may lose and never an instruction to lose it (ADR
//! 1942). They are compiled in so that a profile named by its name works wherever the program is
//! installed, and **they are listed here, in the library, once**: the command-line program's
//! `--profile <name>` and a window's archive command read the same list, so a profile added to
//! `doc/profiles/` reaches both or neither (ADR 1954). `tests/profiles.rs` holds every one of them
//! to every target and this list to the directory.

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

/// Every profile this program ships, in the order a person is offered them.
pub const SHIPPED: [Shipped; 6] = [
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

    #[test]
    fn a_name_not_shipped_is_none_and_the_names_are_offered() {
        assert!(named("as-if-printed").is_some());
        assert!(named("as-if-printd").is_none());
        assert!(names().contains("as-shown"), "{}", names());
    }
}
