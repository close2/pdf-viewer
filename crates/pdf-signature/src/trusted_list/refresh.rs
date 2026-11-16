//! The list of trusted lists, its pivots and the Member States' lists **fetched by a host and
//! authenticated before anything is kept**, and ETSI TS 119 615 PRO-4.1.4-16 and -17's updates of
//! the configuration (ADR 2882).
//!
//! This crate opens no socket (`CLAUDE.md` principle 3): [`refresh`] is handed a host's transport
//! as a function from a location to the octets obtained from it, asks it for exactly the files
//! clauses 4.1 and 4.2 obtain, and answers with what the host is to store, what it is to remove,
//! and what became of each location. The host persists the answer; nothing is written here.
//!
//! # What is fetched, in the clauses' order
//!
//! - PRO-4.1.4-01: the LOTL, from `OJEU-LOTL-Loc`.
//! - PRO-4.1.4-04 and -11: each pivot LOTL, the successive `https` URIs ending `.xml` in the
//!   LOTL's scheme information URIs before the one matching `OJEU-Loc`; PRO-4.1.4-05 and -06: the
//!   location of the LOTL's pointer to itself (scheme territory `EU`), where it is another.
//! - PRO-4.2.4-03 and -04: for each territory the authenticated LOTL points at, the XML file its
//!   pointer locates — fetched only after clause 4.1 passed over what was fetched, because
//!   PRO-4.2.4-02 stops clause 4.2 where the LOTL is not authenticated.
//!
//! # Nothing fetched is kept before it is authenticated
//!
//! Clause 4.1 runs over the host's files with the fetched ones in front of them; where it fails,
//! nothing is stored and the files the host held stay as they were. A Member State's list is
//! stored only where clause 4.2 passes on it under the authenticated LOTL; a list that does not,
//! or that could not be fetched, leaves the copy the host held in place, and is said. A reading of
//! the stored files authenticates them again ([`super::Lists::read`]), so a stored file is never
//! believed for having been stored.
//!
//! # The configuration's updates
//!
//! PRO-4.1.4-16 sets `OJEU-LOTL-Loc` to the location of the authenticated LOTL's pointer to itself
//! where the two differ. PRO-4.1.4-17 sets `OJEU-Loc` to the first `https` URI with a path not
//! ending `.xml` among the scheme information URIs where it is another, and with it the whole of
//! `OJEU-LOTL-Certs-Set`: b) 1) is the certificates of the `EU` pointer of the pivot whose location
//! immediately follows that URI, and b) 2) is the Official Journal's publication, which is a
//! document for people and which this program does not read. Where b) 1) has no pivot to read,
//! neither half is made — an `OJEU-Loc` moved without its certificates would authenticate the next
//! LOTL against a set the clause replaced — and the reason is said.

use super::authentication::{
    self, Configuration, OJEU_LOC, OJEU_LOTL_CERTS_SET, OJEU_LOTL_LOC, Outcome, stored_name,
};
use super::{Lists, TrustedList, XML_MIME_TYPE};
use crate::x509::Instant;

/// How many locations one refresh asks the host's transport for.
///
/// TS 119 615 states no ceiling. The LOTL of 2026 points at about thirty territories and names five
/// pivots; two hundred and fifty-six is a bound on the work a stranger's list can ask for, not a
/// reading of the standard.
pub const MAX_FETCHES: usize = 256;

/// The longest file name [`stored_name`] may produce for a location to be stored: the 255 octets
/// a file name may hold on the file systems the hosts run on.
pub const MAX_NAME_OCTETS: usize = 255;

/// Files by name (or by location, before they are named), each with its octets.
type Files = Vec<(String, Vec<u8>)>;

/// A host's transport: the octets obtained from a location, or the sentence saying why none were.
pub type Obtain<'t> = dyn FnMut(&str) -> Result<Vec<u8>, String> + 't;

/// What became of one location a refresh asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fate {
    /// Fetched and, as the clause asks of it, authenticated: the host stores it.
    Kept,
    /// The transport answered no octets; what the host held for it stays.
    NotFetched(String),
    /// Fetched, and not a list this program reads; what the host held stays.
    Unreadable(String),
    /// Fetched, and clause 4.2 did not authenticate it, with the clause's indications; what the
    /// host held stays.
    NotAuthenticated(Outcome),
    /// Not asked for: its [`stored_name`] is longer than [`MAX_NAME_OCTETS`].
    NameTooLong,
    /// Not asked for: [`MAX_FETCHES`] locations had been.
    PastTheBound,
}

impl std::fmt::Display for Fate {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Kept => formatter.write_str("fetched, authenticated and kept"),
            Self::NotFetched(why) => write!(formatter, "not fetched: {why}"),
            Self::Unreadable(why) => write!(formatter, "fetched and not read as a list: {why}"),
            Self::NotAuthenticated(outcome) => {
                write!(formatter, "fetched and not authenticated: {outcome}")
            }
            Self::NameTooLong => write!(
                formatter,
                "not fetched: its location is too long to name a file after \
                 ({MAX_NAME_OCTETS} octets)"
            ),
            Self::PastTheBound => write!(
                formatter,
                "not fetched: {MAX_FETCHES} locations had been asked for already"
            ),
        }
    }
}

/// One location a refresh asked for, and what became of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    /// The URI.
    pub uri: String,
    /// The scheme territory the pointer names, for a Member State's list; `EU` for the LOTL and
    /// the files clause 4.1 obtains.
    pub territory: String,
    /// What became of it.
    pub fate: Fate,
}

/// What a refresh asks the host to persist, and what it says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refresh {
    /// Clause 4.1's outcome over the files fetched in front of the host's.
    pub lotl: Outcome,
    /// Every location asked for, or that would have been, in the order the clauses obtain them.
    pub locations: Vec<Location>,
    /// The files to store, each under its name: the fetched ones under [`stored_name`], and the
    /// configuration's under [`OJEU_LOC`], [`OJEU_LOTL_LOC`] and [`OJEU_LOTL_CERTS_SET`]`-<n>.der`.
    pub store: Vec<(String, Vec<u8>)>,
    /// The host's files to remove: a file a host stored from a location the authenticated LOTL no
    /// longer leads to, and the certificate files a PRO-4.1.4-17 update replaced.
    pub remove: Vec<String>,
    /// One sentence for each of PRO-4.1.4-16 and -17 that made an update, or that could not.
    pub configuration: Vec<String>,
}

/// Why a refresh asked for nothing, or stopped at the first location.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RefreshRefusal {
    /// No `OJEU-LOTL-Certs-Set` file among the host's: PRO-4.1.4-12 could authenticate no LOTL,
    /// so nothing is fetched.
    #[error(
        "the Official Journal's certificates are not among the files (no {OJEU_LOTL_CERTS_SET} \
         file), so ETSI TS 119 615 clause 4.1 could authenticate nothing fetched"
    )]
    NoOfficialJournalCertificates,
    /// The LOTL itself, PRO-4.1.4-01's input, could not be obtained.
    #[error("the list of trusted lists at {uri} was {fate}")]
    LotlNotObtained {
        /// `OJEU-LOTL-Loc`.
        uri: String,
        /// Why.
        fate: Fate,
    },
}

/// The host's files with the fetched ones in front of them: a fetched file replaces one of the
/// same name, and [`authentication`]'s lookups find a [`stored_name`] before a last path segment.
fn overlaid(lists: &Lists, inputs: &[(String, Vec<u8>)]) -> Lists {
    let mut files: Vec<(String, Vec<u8>)> = inputs
        .iter()
        .map(|(uri, octets)| (stored_name(uri), octets.clone()))
        .collect();
    for (name, octets) in lists.lists() {
        if !files.iter().any(|(file, _)| file == name) {
            files.push((name.clone(), octets.clone()));
        }
    }
    Lists::of(files, lists.source().to_owned())
}

/// Whether a host's file is one a host stored from a location: a [`stored_name`] of an `http` or
/// `https` URI, which no name a person gives a file by hand begins with.
fn is_stored(name: &str) -> bool {
    name.starts_with("https%3A%2F%2F") || name.starts_with("http%3A%2F%2F")
}

/// The locations clause 4.1 obtains from the LOTL beyond the LOTL itself: the pivots of
/// PRO-4.1.4-04, newest first, and the LOTL's pointer to itself of PRO-4.1.4-05 and -06.
fn clause_4_1_locations(lotl: &TrustedList, configuration: &Configuration) -> Vec<String> {
    let mut locations: Vec<String> = Vec::new();
    if let Some(position) = lotl
        .scheme_information_uris
        .iter()
        .position(|uri| *uri == configuration.ojeu_loc)
    {
        locations.extend(
            lotl.scheme_information_uris
                .iter()
                .take(position)
                .filter(|uri| authentication::is_https_with_path(uri, true))
                .cloned(),
        );
    }
    if let Some(pointer) = lotl
        .pointers
        .iter()
        .find(|pointer| pointer.territory.as_deref() == Some("EU"))
        && !locations.contains(&pointer.location)
    {
        locations.push(pointer.location.clone());
    }
    locations.retain(|uri| *uri != configuration.ojeu_lotl_loc);
    locations
}

/// The host's transport, asked within [`MAX_FETCHES`] and [`MAX_NAME_OCTETS`].
struct Fetcher<'o, 't> {
    obtain: &'o mut Obtain<'t>,
    asked: usize,
}

impl Fetcher<'_, '_> {
    fn fetch(&mut self, uri: &str) -> Result<Vec<u8>, Fate> {
        if stored_name(uri).len() > MAX_NAME_OCTETS {
            return Err(Fate::NameTooLong);
        }
        if self.asked >= MAX_FETCHES {
            return Err(Fate::PastTheBound);
        }
        self.asked = self.asked.saturating_add(1);
        (self.obtain)(uri).map_err(Fate::NotFetched)
    }
}

/// TS 119 615 clauses 4.1 and 4.2 over files obtained through `obtain`, at `at` (PRO-4.1.4-13's
/// "has passed" and PRO-4.2.4-10's), in front of the host's `lists`; and PRO-4.1.4-16 and -17's
/// updates of the configuration the host's files hold.
///
/// # Errors
///
/// [`RefreshRefusal`]: no Official Journal certificate among the host's files, which asks for
/// nothing; or the LOTL not obtained. Clause 4.1 failing over what was fetched is not an error:
/// it is a [`Refresh`] that stores nothing, with the clause's indications.
pub fn refresh(
    lists: &Lists,
    at: Instant,
    obtain: &mut Obtain<'_>,
) -> Result<Refresh, RefreshRefusal> {
    let configuration = Configuration::of(lists);
    if configuration.certificates.is_empty() {
        return Err(RefreshRefusal::NoOfficialJournalCertificates);
    }
    let mut fetcher = Fetcher { obtain, asked: 0 };
    let (inputs, mut locations) = clause_4_1_inputs(&configuration, &mut fetcher)?;
    let overlay = overlaid(lists, &inputs);
    let authenticated = match authentication::lotl(&overlay, &configuration, at) {
        Ok(authenticated) => authenticated,
        Err(outcome) => {
            // Nothing of clause 4.1's inputs is kept where it failed over them.
            for location in &mut locations {
                if location.fate == Fate::Kept {
                    location.fate = Fate::NotAuthenticated(outcome.clone());
                }
            }
            return Ok(Refresh {
                lotl: outcome,
                locations,
                store: Vec::new(),
                remove: Vec::new(),
                configuration: Vec::new(),
            });
        }
    };
    let mut store: Vec<(String, Vec<u8>)> = inputs
        .iter()
        .map(|(uri, octets)| (stored_name(uri), octets.clone()))
        .collect();
    member_states(&authenticated, at, &mut fetcher, &mut locations, &mut store);
    let planned: Vec<String> = locations
        .iter()
        .map(|location| stored_name(&location.uri))
        .collect();
    let mut remove: Vec<String> = lists
        .lists()
        .iter()
        .map(|(name, _)| name)
        .filter(|name| is_stored(name) && !planned.contains(name))
        .cloned()
        .collect();
    let sentences = updated_configuration(
        &authenticated.list,
        &configuration,
        &overlay,
        lists,
        &mut store,
        &mut remove,
    );
    Ok(Refresh {
        lotl: authenticated.outcome,
        locations,
        store,
        remove,
        configuration: sentences,
    })
}

/// PRO-4.1.4-01's LOTL, then the further files clause 4.1 obtains from it, each with what became
/// of its fetch.
fn clause_4_1_inputs(
    configuration: &Configuration,
    fetcher: &mut Fetcher<'_, '_>,
) -> Result<(Files, Vec<Location>), RefreshRefusal> {
    let lotl_uri = configuration.ojeu_lotl_loc.clone();
    let octets = fetcher
        .fetch(&lotl_uri)
        .map_err(|fate| RefreshRefusal::LotlNotObtained {
            uri: lotl_uri.clone(),
            fate,
        })?;
    let further = TrustedList::read(&octets)
        .map(|lotl| clause_4_1_locations(&lotl, configuration))
        .unwrap_or_default();
    let mut locations = vec![Location {
        uri: lotl_uri.clone(),
        territory: "EU".to_owned(),
        fate: Fate::Kept,
    }];
    let mut inputs = vec![(lotl_uri, octets)];
    for uri in further {
        let fate = match fetcher.fetch(&uri) {
            Ok(file) => {
                inputs.push((uri.clone(), file));
                Fate::Kept
            }
            Err(fate) => fate,
        };
        locations.push(Location {
            uri,
            territory: "EU".to_owned(),
            fate,
        });
    }
    Ok((inputs, locations))
}

/// PRO-4.2.4-03 and -04 for each territory the authenticated LOTL points at with an XML list, and
/// clause 4.2 on each before it is kept.
fn member_states(
    authenticated: &authentication::AuthenticatedLotl,
    at: Instant,
    fetcher: &mut Fetcher<'_, '_>,
    locations: &mut Vec<Location>,
    store: &mut Vec<(String, Vec<u8>)>,
) {
    for pointer in authenticated.list.pointers.iter().filter(|pointer| {
        pointer.mime_type.as_deref() == Some(XML_MIME_TYPE)
            && pointer
                .territory
                .as_deref()
                .is_some_and(|territory| territory != "EU")
    }) {
        let uri = pointer.location.clone();
        if locations.iter().any(|location| location.uri == uri) {
            continue;
        }
        let fate = match fetcher.fetch(&uri) {
            Err(fate) => fate,
            Ok(octets) => match TrustedList::read(&octets) {
                Err(error) => Fate::Unreadable(error.to_string()),
                Ok(mut list) => {
                    match authentication::member_state(&mut list, &octets, authenticated, at) {
                        Ok(_) => {
                            store.push((stored_name(&uri), octets));
                            Fate::Kept
                        }
                        Err(outcome) => Fate::NotAuthenticated(outcome),
                    }
                }
            },
        };
        locations.push(Location {
            uri,
            territory: pointer.territory.clone().unwrap_or_default(),
            fate,
        });
    }
}

/// PRO-4.1.4-16 and -17 over the authenticated LOTL `lotl`: the files they set added to `store`,
/// the certificate files a new set replaces added to `remove`, and one sentence for each update
/// made or not made.
fn updated_configuration(
    lotl: &TrustedList,
    configuration: &Configuration,
    overlay: &Lists,
    lists: &Lists,
    store: &mut Vec<(String, Vec<u8>)>,
    remove: &mut Vec<String>,
) -> Vec<String> {
    let mut sentences = Vec::new();
    // PRO-4.1.4-16.
    if let Some(location) = lotl
        .pointers
        .iter()
        .find(|pointer| pointer.territory.as_deref() == Some("EU"))
        .map(|pointer| pointer.location.clone())
        && location != configuration.ojeu_lotl_loc
    {
        sentences.push(format!(
            "{OJEU_LOTL_LOC} is now {location}, the location the authenticated list of trusted \
             lists gives itself (ETSI TS 119 615 PRO-4.1.4-16); it was {}",
            configuration.ojeu_lotl_loc
        ));
        store.push((OJEU_LOTL_LOC.to_owned(), location.into_bytes()));
    }
    // PRO-4.1.4-17.
    let uris = &lotl.scheme_information_uris;
    let Some(first) = uris
        .iter()
        .position(|uri| authentication::is_https_with_path(uri, false))
    else {
        return sentences;
    };
    let Some(publication) = uris
        .get(first)
        .filter(|uri| **uri != configuration.ojeu_loc)
    else {
        return sentences;
    };
    let certificates = uris
        .get(first.saturating_add(1))
        .filter(|uri| authentication::is_https_with_path(uri, true))
        .and_then(|pivot| authentication::obtained(overlay, pivot))
        .and_then(|octets| TrustedList::read(octets).ok())
        .map(|pivot| authentication::eu_pointer_certificates(&pivot))
        .unwrap_or_default();
    if certificates.is_empty() {
        sentences.push(format!(
            "{OJEU_LOC} stays {}: the authenticated list of trusted lists names {publication} as \
             the latest publication of the Official Journal (ETSI TS 119 615 PRO-4.1.4-17), and \
             no pivot list follows it to give its certificates by b) 1); b) 2) is that \
             publication, which this program does not read, so {OJEU_LOTL_CERTS_SET} files from \
             it are the person's to supply",
            configuration.ojeu_loc
        ));
        return sentences;
    }
    sentences.push(format!(
        "{OJEU_LOC} is now {publication}, the latest publication of the Official Journal the \
         authenticated list of trusted lists names, and {OJEU_LOTL_CERTS_SET} the {} \
         certificate(s) of the pivot list that follows it (ETSI TS 119 615 PRO-4.1.4-17 a) and \
         b) 1)); it was {}",
        certificates.len(),
        configuration.ojeu_loc
    ));
    store.push((OJEU_LOC.to_owned(), publication.clone().into_bytes()));
    let mut written = Vec::new();
    for (index, certificate) in certificates.into_iter().enumerate() {
        let name = format!("{OJEU_LOTL_CERTS_SET}-{}.der", index.saturating_add(1));
        written.push(name.clone());
        store.push((name, certificate));
    }
    remove.extend(
        lists
            .lists()
            .iter()
            .map(|(name, _)| name)
            .filter(|name| name.starts_with(OJEU_LOTL_CERTS_SET) && !written.contains(name))
            .cloned(),
    );
    sentences
}

#[cfg(test)]
mod tests;
