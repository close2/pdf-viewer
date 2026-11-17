//! A signature policy's trust anchors given by reference to a trusted list, determined from the
//! lists a host supplied: ETSI TS 119 172-2 clause 4.22.2.1 b), which TS 119 172-3 clause 4.22.2.1
//! adopts for its ASN.1 syntax.
//!
//! The clause lets a policy name its trust anchors as a reference to a TS 119 612 list — a specific
//! list, which lists trusted services, or a list of trusted lists, which lists none and points at
//! specific lists — optionally with service types and service statuses, and with an instant until
//! which those anchors are reliable. It determines the anchors in three steps, which
//! [`Supplied::anchors`] runs in order:
//!
//! 1. the initial set: every certificate of the services the referenced specific list holds, or of
//!    the services every specific list the list of trusted lists points at holds — the clause's
//!    Example 3 says the certificates *corresponding to the trusted services listed*, which is the
//!    services' digital identities (TS 119 612 clause 5.5.3), current and historical;
//! 2. where service types are given, only the certificates of a service of one of those types;
//! 3. where service statuses are given, only those of a service whose status *at current time* is
//!    one of them — the instant the host read its lists at, which is this program's current time
//!    (it reads no clock of its own).
//!
//! # Which supplied file a reference is
//!
//! This crate opens no socket (`CLAUDE.md` principle 3), so a reference is resolved among the
//! files a host supplied, by the names ADR 2842 gives them: the file a host stored from the
//! reference's location — [`super::refresh::refresh_with`] fetches a reference the clauses of TS
//! 119 615 do not obtain, and keeps it only once clause 4.2's checks pass on it (ADR 2932) — or the
//! file named its last path segment, or, where the list of trusted lists points at the reference,
//! the list of that pointer's territory — the way TS 119 615 clause 4.2 is run over the same files. A list
//! [`super::Lists::read`] refused is not reliable, so no anchor is taken from it; a pointed-to list
//! the host did not supply is counted, because its certificates might have been an anchor and a
//! condition one of them could meet is never called broken for want of it (ADR 2874).
//!
//! The ETSI texts are cited by clause and paraphrased, never quoted (ADR 1085).

use super::{Reading, TrustedList, authentication};
use crate::x509::Instant;

/// TS 119 172-2 clause 4.22.2.1 b)'s tuple as a policy states it, the instant aside: the reference,
/// and the service types and statuses that filter what it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListReference {
    /// `TLReference`, or TS 119 172-3's `uri`: the list's location.
    pub reference: String,
    /// `ServiceTypes`, or `serviceTypes`: step 2's identifiers; `None` where absent.
    pub service_types: Option<Vec<String>>,
    /// `ServiceStatuses`, or `serviceStatuses`: step 3's statuses; `None` where absent.
    pub statuses: Option<Vec<String>>,
}

/// The lists a host supplied, as read, at the instant they were read at: what a policy's
/// [`ListReference`] is resolved against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Supplied<'l> {
    /// The reading.
    pub reading: &'l Reading,
    /// The current time of clause 4.22.2.1 b)'s step 3: the host's instant the lists were read at.
    pub now: Instant,
}

/// The anchors one reference determines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Determined {
    /// The certificates the three steps keep, DER, each once.
    pub certificates: Vec<Vec<u8>>,
    /// The specific lists a referenced list of trusted lists points at that the host did not
    /// supply, by territory, or by location where the pointer states none: anchors this program
    /// could not determine.
    pub missing: Vec<String>,
}

/// Why a reference determined no anchor at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unresolved {
    /// No file was supplied at all, so no reference can be resolved.
    NoListSupplied,
    /// No supplied file is the referenced list.
    NotSupplied,
    /// The referenced file was supplied and [`super::Lists::read`] refused it, so it is not
    /// reliable.
    Refused,
}

impl<'l> Supplied<'l> {
    /// Clause 4.22.2.1 b)'s three steps for `reference`.
    ///
    /// # Errors
    ///
    /// [`Unresolved`] where no supplied list is the one referenced, or the one that is was refused.
    pub fn anchors(self, reference: &ListReference) -> Result<Determined, Unresolved> {
        let list = self.resolve(&reference.reference)?;
        let mut missing = Vec::new();
        // Clause 4.22.2.1's own definition: a list of trusted lists lists no trusted services and
        // points at specific lists; any list that lists services is a specific one.
        let specific: Vec<&TrustedList> = if lists_services(list) {
            vec![list]
        } else {
            let mut found_lists: Vec<&TrustedList> = Vec::new();
            for pointed_at in pointed(list) {
                match pointed_at {
                    Pointed::Territory(territory) => match self.of_territory(&territory) {
                        Some(found) => found_lists.push(found),
                        None => missing.push(territory),
                    },
                    Pointed::Unplaced(location) => missing.push(location),
                }
            }
            found_lists
        };
        let mut certificates: Vec<Vec<u8>> = Vec::new();
        for service in specific
            .iter()
            .flat_map(|list| &list.providers)
            .flat_map(|provider| &provider.services)
        {
            // Step 2: the service's type, which TS 119 612 clause 5.5.1 states in its current
            // information.
            if let Some(types) = &reference.service_types
                && !types.contains(&service.current.type_identifier)
            {
                continue;
            }
            // Step 3: its status at current time; a service with no status yet has none of them.
            if let Some(statuses) = &reference.statuses
                && !service
                    .at(self.now)
                    .is_some_and(|state| statuses.contains(&state.status))
            {
                continue;
            }
            for certificate in std::iter::once(&service.current)
                .chain(&service.history)
                .flat_map(|state| &state.certificates)
            {
                if !certificates.contains(certificate) {
                    certificates.push(certificate.clone());
                }
            }
        }
        Ok(Determined {
            certificates,
            missing,
        })
    }

    /// The supplied list `reference` names: the file a host stored from it, or the file named its
    /// last path segment, else the list of the territory of the list of trusted lists' pointer to
    /// it.
    fn resolve(self, reference: &str) -> Result<&'l TrustedList, Unresolved> {
        let reading = self.reading;
        let stored = authentication::stored_name(reference);
        let name = authentication::file_name(reference);
        for wanted in [stored.as_str(), name] {
            if let Some((_, list)) = reading
                .named()
                .chain(reading.referenced())
                .chain(reading.authenticated_lotl())
                .find(|(file, _)| *file == wanted)
            {
                return Ok(list);
            }
        }
        if reading
            .refused
            .iter()
            .any(|(file, _)| authentication::is_obtained_from(file, reference))
        {
            return Err(Unresolved::Refused);
        }
        let pointed = self
            .lotls()
            .flat_map(|lotl| &lotl.pointers)
            .find(|pointer| pointer.location == reference)
            .and_then(|pointer| pointer.territory.as_deref());
        let nothing_read = reading.lists.is_empty()
            && reading.referenced().next().is_none()
            && reading.authenticated_lotl().is_none()
            && reading.refused.is_empty();
        match pointed {
            Some(territory) => self.of_territory(territory).ok_or(Unresolved::NotSupplied),
            None if nothing_read => Err(Unresolved::NoListSupplied),
            None => Err(Unresolved::NotSupplied),
        }
    }

    /// The lists of trusted lists supplied: the one clause 4.1 authenticated, and any that read
    /// where it did not run.
    fn lotls(self) -> impl Iterator<Item = &'l TrustedList> {
        self.reading
            .authenticated_lotl()
            .map(|(_, list)| list)
            .into_iter()
            .chain(
                self.reading
                    .lists
                    .iter()
                    .filter(|list| !lists_services(list)),
            )
    }

    /// The specific list of `territory` among those that read and were authenticated.
    fn of_territory(self, territory: &str) -> Option<&'l TrustedList> {
        self.reading
            .lists
            .iter()
            .find(|list| list.territory == territory && lists_services(list))
    }
}

/// Whether a list lists trusted services, which is what makes it a specific list rather than a
/// list of trusted lists (clause 4.22.2.1 b)).
fn lists_services(list: &TrustedList) -> bool {
    list.providers
        .iter()
        .any(|provider| !provider.services.is_empty())
}

/// One specific list a list of trusted lists points at.
enum Pointed {
    /// The list of this territory, which is how ADR 2842 places a pointed-to list among the files.
    Territory(String),
    /// A pointer that states no territory, by its location: a list this program cannot place, and
    /// so one it counts as missing.
    Unplaced(String),
}

/// The specific lists a list of trusted lists points at, each once and in its order. A pointer to
/// the list's own territory is the list of trusted lists pointing at itself (TS 119 612 clause
/// 5.3.13), and is not a specific list.
fn pointed(lotl: &TrustedList) -> Vec<Pointed> {
    let mut territories: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for pointer in &lotl.pointers {
        match &pointer.territory {
            Some(territory) if *territory == lotl.territory => {}
            // The XML and the PDF form of one list are two pointers of one territory.
            Some(territory) if territories.contains(territory) => {}
            Some(territory) => {
                territories.push(territory.clone());
                out.push(Pointed::Territory(territory.clone()));
            }
            None => out.push(Pointed::Unplaced(pointer.location.clone())),
        }
    }
    out
}

#[cfg(test)]
mod tests;
