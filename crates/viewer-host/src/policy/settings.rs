//! A window's [`Settings`] read from what a person typed or a host stated, once for every window
//! (RFC 0010 section 5, ADR 2120).
//!
//! **One reading rather than one per window**, for [`crate::ReaderWords`]' reason (ADR 1581): a
//! word one window takes and another refuses is a policy that exists in one build, and four
//! binaries each matching the same six prefixes is where the first of them stops agreeing.
//! [`Settings::take`] is the command line's reading and [`Settings::set`] the reading for a host
//! with no command line — the C ABI's caller — so both arrive at the same value through the same
//! levels.

use super::{
    EMBEDDED_DOCUMENTS, EmbeddedDocuments, IGNORE_RESTRICTIONS, LAUNCHED_DOCUMENTS, LINKS,
    LaunchedDocuments, Links, REMOTE_DOCUMENTS, RESTRICTIONS, RemoteDocuments, SEPARATIONS,
    SUBMISSIONS, Scripts, Settings, Submissions, embedded_documents, launched_documents, links,
    remote_documents, restrictions, separations, submissions,
};

/// One of the reader's decisions about what a document asks this machine to do, as a host names
/// it without a command line.
///
/// **The restrictions are not here**, and the division is the one [`LINKS`] draws: a restriction
/// is what a *document* asserts over its reader, and it already has its own operations and
/// levels — `viewer_core::RestrictionPolicy`'s, which the C ABI's `QUORRA_RESTRICTED_*` and
/// `QUORRA_RESTRICT_*` name — while each policy below is an act a document asks of this machine,
/// on the one scale [`Level`] states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Policy {
    /// §12.6.4.8's URI handed to another program ([`LINKS`]).
    Links,
    /// §12.6.4.3's other PDF file opened beside this one ([`REMOTE_DOCUMENTS`]).
    RemoteDocuments,
    /// §O.2.1's `ef` opened as a document ([`EMBEDDED_DOCUMENTS`]).
    EmbeddedDocuments,
    /// §12.7.6.2's form, or Table Annex O.4's `fdf`, sent to a server ([`SUBMISSIONS`]).
    Submissions,
    /// A document's scripts run (`super::SCRIPTS`).
    Scripts,
    /// §10.8.3's separation simulation asked for ([`SEPARATIONS`]): a preference with two settings,
    /// [`Level::Refuse`] for `off` and [`Level::Proceed`] for `on`.
    Separations,
    /// §12.6.4.6's launched document that is not a PDF handed to this machine's own handler
    /// ([`LAUNCHED_DOCUMENTS`]).
    LaunchedDocuments,
}

/// A level on the scale every four-level policy shares, least permissive first: the act refused,
/// put to the person first, done and said afterwards, or done.
///
/// **Each policy's own `ALL`, in its own order**, which is why one scale serves five spellings:
/// [`Links`]' `refuse` to `open`, [`Submissions`]' `refuse` to `send` and [`Scripts`]' `off` to `on`
/// are the same four places, and the word a person reads stays the policy's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Level {
    /// The act is not done, and said.
    Refuse,
    /// The act is put to the person, and done on a yes.
    Ask,
    /// The act is done, and said afterwards.
    Warn,
    /// The act is done.
    Proceed,
}

impl Level {
    /// Where this level stands in a policy's `ALL`.
    const fn place(self) -> usize {
        match self {
            Self::Refuse => 0,
            Self::Ask => 1,
            Self::Warn => 2,
            Self::Proceed => 3,
        }
    }
}

impl Settings {
    /// Takes `word` if it is one of the words a window's settings are read from — [`RESTRICTIONS`],
    /// [`IGNORE_RESTRICTIONS`], [`LINKS`], [`REMOTE_DOCUMENTS`], [`EMBEDDED_DOCUMENTS`],
    /// [`SUBMISSIONS`], [`LAUNCHED_DOCUMENTS`] and [`SEPARATIONS`] — answering which of those
    /// constants it was.
    ///
    /// `Ok(None)` where `word` is none of them, which leaves it to the window's own parser. `--scripts`
    /// is not among them: it takes the word after it, and [`crate::ReaderWords::take`] reads it
    /// with the reader's other words.
    ///
    /// # Errors
    ///
    /// The sentence to print, naming the level that is wrong and the ones that would have been
    /// right — each word's own parser's sentence, unchanged.
    pub fn take(&mut self, word: &str) -> Result<Option<&'static str>, String> {
        if word == IGNORE_RESTRICTIONS {
            self.restrictions =
                viewer_core::RestrictionPolicy::uniform(viewer_core::RestrictionLevel::Off);
            return Ok(Some(IGNORE_RESTRICTIONS));
        }
        if let Some(list) = word.strip_prefix(RESTRICTIONS) {
            // `CLAUDE.md`'s four levels, one operation at a time, composed onto what an earlier
            // word said (ADR 1144).
            self.restrictions = restrictions(list, self.restrictions)?;
            return Ok(Some(RESTRICTIONS));
        }
        if let Some(level) = word.strip_prefix(LINKS) {
            // §12.6.4.8's act (ADR 1155).
            self.links = links(level)?;
            return Ok(Some(LINKS));
        }
        if let Some(level) = word.strip_prefix(REMOTE_DOCUMENTS) {
            // §12.6.4.3's act, at four levels of its own (ADR 1227).
            self.remote_documents = remote_documents(level)?;
            return Ok(Some(REMOTE_DOCUMENTS));
        }
        if let Some(level) = word.strip_prefix(EMBEDDED_DOCUMENTS) {
            // §O.2.1's act, at four levels of its own (ADR 1331).
            self.embedded_documents = embedded_documents(level)?;
            return Ok(Some(EMBEDDED_DOCUMENTS));
        }
        if let Some(level) = word.strip_prefix(SUBMISSIONS) {
            // §12.7.6.2's and a fetched import's act, at four levels of its own (ADR 1527).
            self.submissions = submissions(level)?;
            return Ok(Some(SUBMISSIONS));
        }
        if let Some(level) = word.strip_prefix(LAUNCHED_DOCUMENTS) {
            // §12.6.4.6's document that is not a PDF, at four levels of its own (ADR 2171).
            self.launched_documents = launched_documents(level)?;
            return Ok(Some(LAUNCHED_DOCUMENTS));
        }
        if let Some(setting) = word.strip_prefix(SEPARATIONS) {
            // §10.8.3's simulation, which is a preference and has two words (ADR 1228).
            self.separations = separations(setting)?;
            return Ok(Some(SEPARATIONS));
        }
        Ok(None)
    }

    /// Sets one policy at one level, as [`Self::take`] sets it from the policy's own word.
    ///
    /// # Errors
    ///
    /// The sentence to print where [`Policy::Separations`] is given [`Level::Ask`] or
    /// [`Level::Warn`]: a preference with two settings has no question to put and nothing to say
    /// afterwards, and nothing changes.
    pub fn set(&mut self, policy: Policy, level: Level) -> Result<(), String> {
        let place = level.place();
        match policy {
            Policy::Links => self.links = Links::ALL[place],
            Policy::RemoteDocuments => self.remote_documents = RemoteDocuments::ALL[place],
            Policy::EmbeddedDocuments => self.embedded_documents = EmbeddedDocuments::ALL[place],
            Policy::Submissions => self.submissions = Submissions::ALL[place],
            Policy::Scripts => self.scripts = Scripts::ALL[place],
            Policy::LaunchedDocuments => self.launched_documents = LaunchedDocuments::ALL[place],
            Policy::Separations => {
                self.separations = match level {
                    Level::Refuse => false,
                    Level::Proceed => true,
                    Level::Ask | Level::Warn => {
                        return Err(format!(
                            "{SEPARATIONS}: §10.8.3's simulation is a preference with two \
                             settings, on and off, and has no level between them"
                        ));
                    }
                };
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Level, Policy};
    use crate::policy::{
        EMBEDDED_DOCUMENTS, EmbeddedDocuments, IGNORE_RESTRICTIONS, LAUNCHED_DOCUMENTS, LINKS,
        LaunchedDocuments, Links, REMOTE_DOCUMENTS, RESTRICTIONS, RemoteDocuments, SEPARATIONS,
        SUBMISSIONS, Scripts, Settings, Submissions,
    };
    use viewer_core::{RestrictionLevel, RestrictionPolicy};

    /// Every word a window takes sets the field its own parser set, and says which word it was.
    #[test]
    fn each_word_sets_its_own_field_and_names_itself() {
        let mut settings = Settings::default();
        let words = [
            ("--links=open", LINKS),
            ("--remote-documents=refuse", REMOTE_DOCUMENTS),
            ("--embedded-documents=warn", EMBEDDED_DOCUMENTS),
            ("--submissions=send", SUBMISSIONS),
            ("--separations=on", SEPARATIONS),
            ("--restrictions=copy:ask", RESTRICTIONS),
            ("--launched-documents=ask", LAUNCHED_DOCUMENTS),
        ];
        for (word, named) in words {
            assert_eq!(settings.take(word), Ok(Some(named)), "{word}");
        }
        assert_eq!(settings.links, Links::Open);
        assert_eq!(settings.remote_documents, RemoteDocuments::Refuse);
        assert_eq!(settings.embedded_documents, EmbeddedDocuments::Warn);
        assert_eq!(settings.submissions, Submissions::Send);
        assert!(settings.separations);
        assert_eq!(settings.launched_documents, LaunchedDocuments::Ask);
        assert_eq!(
            settings.restrictions,
            RestrictionPolicy::default().with(
                RestrictionPolicy::operation_named("copy").expect("copy is an operation"),
                RestrictionLevel::Ask
            )
        );
        assert_eq!(
            settings.take(IGNORE_RESTRICTIONS),
            Ok(Some(IGNORE_RESTRICTIONS))
        );
        assert_eq!(
            settings.restrictions,
            RestrictionPolicy::uniform(RestrictionLevel::Off)
        );
        assert_eq!(settings.scripts, Scripts::Off, "--scripts is ReaderWords'");
    }

    /// A word that is not one of them is left to the window, and a level that is none is refused
    /// in its own parser's sentence, with nothing changed.
    #[test]
    fn another_word_is_left_and_a_wrong_level_is_refused_in_its_parser_s_sentence() {
        let mut settings = Settings::default();
        assert_eq!(settings.take("--cpu"), Ok(None));
        assert_eq!(settings.take("document.pdf"), Ok(None));
        assert_eq!(settings.take("--scripts"), Ok(None));
        assert_eq!(
            settings.take("--links=sometimes"),
            Err(crate::policy::links("sometimes").expect_err("not a level"))
        );
        assert_eq!(settings, Settings::default());
    }

    /// `set` puts each policy at the place its own `ALL` gives the level, so a host without a
    /// command line arrives where the word would have.
    #[test]
    fn a_level_set_is_the_level_the_word_takes() {
        let levels = [Level::Refuse, Level::Ask, Level::Warn, Level::Proceed];
        for (place, level) in levels.into_iter().enumerate() {
            let mut set = Settings::default();
            for policy in [
                Policy::Links,
                Policy::RemoteDocuments,
                Policy::EmbeddedDocuments,
                Policy::Submissions,
                Policy::Scripts,
                Policy::LaunchedDocuments,
            ] {
                set.set(policy, level)
                    .expect("a four-level policy takes every level");
            }
            let mut taken = Settings::default();
            for word in [
                format!("{LINKS}{}", Links::ALL[place].as_str()),
                format!("{REMOTE_DOCUMENTS}{}", RemoteDocuments::ALL[place].as_str()),
                format!(
                    "{EMBEDDED_DOCUMENTS}{}",
                    EmbeddedDocuments::ALL[place].as_str()
                ),
                format!("{SUBMISSIONS}{}", Submissions::ALL[place].as_str()),
                format!(
                    "{LAUNCHED_DOCUMENTS}{}",
                    LaunchedDocuments::ALL[place].as_str()
                ),
            ] {
                taken.take(&word).expect("the word names a level");
            }
            taken.scripts = Scripts::ALL[place];
            assert_eq!(set, taken, "{level:?}");
        }
    }

    /// The preference takes its two settings and refuses the two levels it has no meaning for.
    #[test]
    fn separations_take_two_settings_and_refuse_the_levels_between() {
        let mut settings = Settings::default();
        settings
            .set(Policy::Separations, Level::Proceed)
            .expect("on");
        assert!(settings.separations);
        assert!(settings.set(Policy::Separations, Level::Ask).is_err());
        assert!(settings.set(Policy::Separations, Level::Warn).is_err());
        assert!(settings.separations, "a refused level changes nothing");
        settings
            .set(Policy::Separations, Level::Refuse)
            .expect("off");
        assert!(!settings.separations);
    }
}
