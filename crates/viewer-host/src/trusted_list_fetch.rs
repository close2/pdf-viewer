//! The European Commission's list of trusted lists, its pivots and the Member States' lists
//! **fetched by this host into the directory [`crate::policy::TRUSTED_LISTS`] named**, at the
//! reader's network level, and kept only where ETSI TS 119 615 clauses 4.1 and 4.2 authenticate
//! them (`doc/todo/77` item 4, ADR 2882).
//!
//! **A transmission this host makes, never the confined worker's** (`CLAUDE.md` principle 3): the
//! renderer names no path and opens no socket, and it is handed the lists as it was before, read
//! from the directory by [`crate::policy::trusted_lists`]. What to fetch, and what is kept, are
//! `pdf_signature::trusted_list::refresh`'s, which is handed this module's transport; this module
//! asks the level, makes the GETs through [`crate::submit::fetch`] — its bounds, its scheme and
//! address checks, its rule that a redirect is not followed — and persists the answer.
//!
//! **The level is [`Submissions`]', as ADR 1728's fetch of a signature policy's copy is**: a GET to
//! the Commission's or a Member State's server tells it this reader is validating a signature now,
//! which is the act class of the fetches that level already governs, so no fifth level is
//! invented and the default is `ask` (`doc/questions/A98`). It is asked about the directory rather
//! than a document, because nothing a document says chooses what is fetched: the locations are the
//! configuration GPR-4.0-02 names and the lists the authenticated LOTL points at.
//!
//! **TS 119 615 PRO-4.1.4-16 and -17's updates are persisted** as the files the configuration is
//! read from (`OJEU-LOTL-Loc`, `OJEU-Loc`, `OJEU-LOTL-Certs-Set-<n>.der`), so the next reading and
//! the next fetch start from them.

use std::path::Path;

use pdf_signature::trusted_list::refresh::{self, Fate, Refresh, RefreshRefusal};
use pdf_signature::x509::Instant;

use crate::Sending;
use crate::policy::Submissions;

/// Whether the lists may be **fetched** into `directory`, at `level`.
///
/// A host given no [`crate::policy::TRUSTED_LISTS`] directory has nowhere to keep what it would
/// fetch, and the core reads lists only from there, so it refuses at every level before the level
/// is read — as a URL that names no server is refused before it in
/// [`crate::policy::may_fetch_signature_policy`].
#[must_use]
pub fn may_fetch_trusted_lists(directory: Option<&Path>, level: Submissions) -> Sending {
    let Some(directory) = directory else {
        return Sending::Refuse(format!(
            "no directory of trusted lists was named ({}), so there is nowhere to keep them",
            crate::policy::TRUSTED_LISTS
        ));
    };
    match level {
        Submissions::Refuse => Sending::Refuse(format!(
            "this reader is set to send nothing to a server ({}: {}); {} puts the request to you \
             first",
            crate::restriction::SUBMITTING,
            Submissions::Refuse.as_str(),
            Submissions::Ask.as_str()
        )),
        Submissions::Ask => Sending::Ask(asked_to_fetch_trusted_lists(directory)),
        Submissions::Warn => Sending::Warn(format!(
            "the trusted lists were fetched without asking you first, because this reader is set \
             to {} ({})",
            Submissions::Warn.as_str(),
            crate::restriction::SUBMITTING
        )),
        Submissions::Send => Sending::Send,
    }
}

/// What a window puts in front of a person when [`may_fetch_trusted_lists`] answers
/// [`Submissions::Ask`]: where the lists come from, where they are kept, and what fetching does.
#[must_use]
pub fn asked_to_fetch_trusted_lists(directory: &Path) -> crate::restriction::Question {
    crate::restriction::Question {
        reasons: format!(
            "Signatures are checked against the European Union's trusted lists in {}. Updating \
             them fetches the European Commission's list of trusted lists and each Member State's \
             list from the servers that publish them, which tells those servers this reader is \
             checking signatures now; what comes back is kept only where its own signatures \
             authenticate it (ETSI TS 119 615 clauses 4.1 and 4.2).",
            directory.display()
        ),
        choice: format!(
            "You have set this reader to ask before anything is sent to a server ({submitting}: \
             {}). \"{}\" fetches them this once and leaves the level where it is; \"{}\" fetches \
             nothing and keeps the lists as they are. Setting {submitting} to {} in the \
             restrictions menu stops the question being asked, and {} stops anything being \
             fetched at all.",
            Submissions::Ask.as_str(),
            crate::restriction::GO_AHEAD,
            crate::restriction::DO_NOT,
            Submissions::Send.as_str(),
            Submissions::Refuse.as_str(),
            submitting = crate::restriction::SUBMITTING,
        ),
    }
}

/// Fetches the lists into `directory` over the network, blocking, at `at`, and says what became
/// of them, one sentence a line.
///
/// Asked first: a window calls this only after [`may_fetch_trusted_lists`] answered
/// [`Sending::Send`] or [`Sending::Warn`], or a person answered its question yes, and off its
/// event thread, for [`crate::submit`]'s reason. `warned` is the [`Sending::Warn`] sentence,
/// where the level gave one.
#[must_use]
pub fn fetch_trusted_lists(directory: &Path, at: Instant, warned: Option<&str>) -> Vec<String> {
    let mut obtain = |uri: &str| obtained(crate::submit::fetch(uri));
    let mut said = refresh_trusted_lists(directory, at, &mut obtain);
    if let Some(warned) = warned {
        said.insert(0, format!("trusted lists: {warned}"));
    }
    said
}

/// The octets of a fetch's answer, where it was a 2xx, for `crate::submit::reply`'s reason.
///
/// # Errors
///
/// Any other answer, and no answer, as the sentence saying so.
pub fn obtained(
    answer: Result<crate::submit::Response, crate::submit::TransmitError>,
) -> Result<Vec<u8>, String> {
    let response = answer.map_err(|error| error.to_string())?;
    if (200..300).contains(&response.status) {
        return Ok(response.body);
    }
    Err(match response.location {
        Some(location) => format!(
            "the server answered {} and points to {location}, which this reader does not follow",
            response.status
        ),
        None => format!("the server answered {}", response.status),
    })
}

/// The lists in `directory` refreshed through `obtain`, at `at`: the procedure run over the files
/// the directory holds, its answer persisted, and every outcome said.
#[must_use]
pub fn refresh_trusted_lists(
    directory: &Path,
    at: Instant,
    obtain: &mut refresh::Obtain<'_>,
) -> Vec<String> {
    let (lists, refused) = crate::policy::trusted_lists(Some(directory));
    let mut said: Vec<String> = refused
        .iter()
        .map(|refusal| format!("trusted lists: {refusal}"))
        .collect();
    match refresh::refresh(&lists, at, obtain) {
        Err(refusal @ RefreshRefusal::NoOfficialJournalCertificates) => {
            said.push(format!("trusted lists: nothing was fetched: {refusal}"));
        }
        Err(refusal @ RefreshRefusal::LotlNotObtained { .. }) => {
            said.push(format!(
                "trusted lists: nothing was kept: {refusal}; the lists in {} are as they were",
                directory.display()
            ));
        }
        Ok(refreshed) => {
            said.extend(described(&refreshed));
            said.extend(persist(directory, &refreshed));
        }
    }
    said
}

/// One sentence for clause 4.1's outcome, one for each location, and the configuration's.
fn described(refreshed: &Refresh) -> Vec<String> {
    let mut said = vec![format!(
        "trusted lists: the list of trusted lists is {} (ETSI TS 119 615 clause 4.1){}",
        refreshed.lotl,
        if refreshed.lotl.passed() {
            ""
        } else {
            ", so nothing fetched was kept"
        }
    )];
    for location in &refreshed.locations {
        let held = match location.fate {
            Fate::Kept => "",
            _ => "; the copy held, if any, is kept",
        };
        said.push(format!(
            "trusted lists: {} ({}) {}{held}",
            location.uri, location.territory, location.fate
        ));
    }
    said.extend(
        refreshed
            .configuration
            .iter()
            .map(|sentence| format!("trusted lists: {sentence}")),
    );
    said
}

/// Whether `name` is a file name this host writes or removes: one component, never `.` or `..`.
/// Every name a refresh gives is one already — a location is stored percent-encoded, and a removal
/// names a file the directory listed — so this is the last place that guarantee is made.
fn is_one_component(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\\', '\0'])
}

/// Writes each file a refresh keeps beside its final name in `directory` and renames it into
/// place, so a reading never meets half a list, then removes what the refresh removed; every
/// removal and every failure said.
#[must_use]
pub fn persist(directory: &Path, refreshed: &Refresh) -> Vec<String> {
    let mut said = Vec::new();
    for (name, octets) in &refreshed.store {
        if !is_one_component(name) {
            said.push(format!(
                "trusted lists: {name} is not a file name; not kept"
            ));
            continue;
        }
        let part = directory.join(format!(".{name}.part"));
        let written = std::fs::write(&part, octets)
            .and_then(|()| std::fs::rename(&part, directory.join(name)));
        if let Err(error) = written {
            // A rename that failed leaves the part behind; removing it is the tidy-up, and a
            // failure of that is the same failure already being said.
            let _ = std::fs::remove_file(&part);
            said.push(format!("trusted lists: {name} could not be kept: {error}"));
        }
    }
    for name in &refreshed.remove {
        if !is_one_component(name) {
            continue;
        }
        match std::fs::remove_file(directory.join(name)) {
            Ok(()) => said.push(format!(
                "trusted lists: {name} was removed: the authenticated list of trusted lists no \
                 longer leads to it, or a new Official Journal set replaced it"
            )),
            Err(error) => said.push(format!(
                "trusted lists: {name} could not be removed: {error}"
            )),
        }
    }
    said
}
