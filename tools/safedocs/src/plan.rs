//! What a chunk *is*, what it would cost, and the budget that refuses it.
//!
//! A plan is made before anything is transferred and is printed whether or not the caller
//! asked for a download. That ordering is the project owner's constraint made structural:
//! a person on a mobile connection finds out the number first.

use crate::source::Corpus;
use crate::{Error, transport, zip};

/// How many bytes one invocation may transfer.
#[derive(Debug, Clone, Copy)]
pub struct Budget(u64);

impl Budget {
    /// What a plan may cost when nobody says otherwise: 32 MiB.
    ///
    /// Chosen against the corpus rather than as a round number: the median member of
    /// `CC-MAIN-2021-31` is a little over a megabyte, so this is a chunk of a few dozen
    /// documents — enough to be a survey and small enough to be a mistake somebody can
    /// afford. Raising it is one argument and it appears in the refusal's own text.
    pub const DEFAULT: Self = Self(32 << 20);

    /// A budget of this many mebibytes.
    #[must_use]
    pub const fn mebibytes(count: u64) -> Self {
        Self(count.saturating_mul(1 << 20))
    }

    /// The budget in bytes.
    #[must_use]
    pub const fn bytes(self) -> u64 {
        self.0
    }
}

impl Default for Budget {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Which members of which archive a caller wants.
#[derive(Debug, Clone)]
pub struct Chunk {
    /// The corpus's own name.
    pub corpus: &'static Corpus,
    /// The archive, in the corpus's four-digit naming.
    pub archive: String,
    /// Index of the first member to take, in the central directory's own order.
    pub from: usize,
    /// How many members to take.
    pub count: usize,
    /// Take only these members, by name, instead of a window.
    ///
    /// This is what makes a promoted file re-fetchable exactly: `doc/adr/0258`'s names-only
    /// rule records an archive and a member name, and that pair comes back here.
    pub named: Vec<String>,
    /// Take every member the archive holds.
    ///
    /// A gigabyte, and on an unmetered connection that is a reasonable thing to ask for — which
    /// is why it exists. It is still one contiguous byte range and still refused unless
    /// `--budget-mb` admits it, so it is a *deliberate* gigabyte rather than a default one.
    pub all: bool,
}

/// A resolved chunk: the members, the single byte range that covers them, and the total.
#[derive(Debug)]
pub struct Plan {
    /// The archive's URL.
    pub url: String,
    /// The members this chunk names, in archive order.
    pub members: Vec<zip::Entry>,
    /// First byte of the one range that covers them all, inclusive.
    pub first: u64,
    /// Last byte of that range, inclusive.
    pub last: u64,
    /// What reading the archive's central directory already cost.
    pub directory_bytes: u64,
}

impl Plan {
    /// How many bytes fetching this chunk would transfer.
    #[must_use]
    pub fn transfer(&self) -> u64 {
        self.last.saturating_sub(self.first).saturating_add(1)
    }

    /// How many bytes the members hold once inflated.
    #[must_use]
    pub fn uncompressed(&self) -> u64 {
        self.members.iter().fold(0u64, |total, entry| {
            total.saturating_add(entry.uncompressed)
        })
    }

    /// Refuses a plan the budget does not cover.
    ///
    /// # Errors
    ///
    /// [`Error::OverBudget`], naming both numbers.
    pub fn within(&self, budget: Budget) -> Result<(), Error> {
        if self.transfer() > budget.bytes() {
            return Err(Error::OverBudget {
                wanted: self.transfer(),
                budget: budget.bytes(),
                // Rounded up, so that repeating the command with the number in the message
                // works rather than failing by one mebibyte.
                needed: self
                    .transfer()
                    .saturating_add((1 << 20) - 1)
                    .saturating_div(1 << 20),
            });
        }
        Ok(())
    }
}

/// How much of the object's tail is read to find the end-of-central-directory record.
///
/// 64 KiB covers the record and any comment APPNOTE permits, which is 65 535 bytes plus the
/// record's own 22 — so a comment at the maximum needs one byte more than this and is the one
/// case that reports [`zip::ArchiveError::NoDirectory`] rather than reading. No archive in
/// this corpus carries a comment at all.
const TAIL: u64 = 64 << 10;

/// Resolves a chunk into a plan, reading the archive's central directory and nothing else.
///
/// Two range requests, about 100 KB against a gigabyte object. Nothing here transfers a
/// member.
///
/// # Errors
///
/// Whatever the transport or the archive's own structure says.
pub fn plan(chunk: &Chunk) -> Result<Plan, Error> {
    let url = chunk.corpus.archive_url(&chunk.archive)?;

    // The archive's own length, asked for with `HEAD` and no body: every offset below is
    // relative to it, and it is also the number a caller wants to see before anything else.
    let length = transport::length(&url)?;
    let tail_start = length.saturating_sub(TAIL);
    let tail = transport::fetch_range(&url, tail_start, length.saturating_sub(1))?;
    let (directory_offset, directory_size, count) = zip::locate_directory(&tail, tail_start)?;

    // The directory is usually inside the tail already; when it is not, it is one more range.
    let directory_bytes;
    let directory = if directory_offset >= tail_start {
        directory_bytes = 0;
        let within = usize::try_from(directory_offset.saturating_sub(tail_start)).unwrap_or(0);
        let to = within.saturating_add(usize::try_from(directory_size).unwrap_or(0));
        let slice = tail.get(within..to.min(tail.len())).unwrap_or(&[]);
        zip::read_directory(slice, directory_offset, count)?
    } else {
        directory_bytes = directory_size;
        let last = directory_offset
            .saturating_add(directory_size)
            .saturating_sub(1);
        let bytes = transport::fetch_range(&url, directory_offset, last)?;
        zip::read_directory(&bytes, directory_offset, count)?
    };

    let selected = select(&directory, chunk)?;
    let (first, last) = span(&directory, &selected);
    Ok(Plan {
        url,
        members: selected
            .iter()
            .filter_map(|&index| directory.entries.get(index).cloned())
            .collect(),
        first,
        last,
        directory_bytes: directory_bytes.saturating_add(u64::try_from(tail.len()).unwrap_or(0)),
    })
}

/// Which entries the chunk names, as indices into the directory.
fn select(directory: &zip::Directory, chunk: &Chunk) -> Result<Vec<usize>, Error> {
    if chunk.all {
        return Ok((0..directory.entries.len()).collect());
    }
    if chunk.named.is_empty() {
        let end = chunk.from.saturating_add(chunk.count);
        return Ok((chunk.from..end.min(directory.entries.len())).collect());
    }
    let mut indices = Vec::with_capacity(chunk.named.len());
    for name in &chunk.named {
        let (index, _) = directory.find(name)?;
        indices.push(index);
    }
    indices.sort_unstable();
    indices.dedup();
    Ok(indices)
}

/// The one byte range that covers every selected entry.
///
/// A ZIP lays its members out consecutively, so a window of the listing is a window of the
/// object, and a chunk is one request rather than one per member. A *named* selection that
/// happens to be scattered widens this range — which is exactly what the plan then prints,
/// so the caller sees what a scattered selection costs before paying for it.
fn span(directory: &zip::Directory, selected: &[usize]) -> (u64, u64) {
    let mut first = u64::MAX;
    let mut last = 0u64;
    for &index in selected {
        if let Some(entry) = directory.entries.get(index) {
            first = first.min(entry.local_header);
            last = last.max(directory.extent_after(index).saturating_sub(1));
        }
    }
    if selected.is_empty() {
        return (0, 0);
    }
    (first, last)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The refusal is the owner's constraint, so it has to fire on the plan rather than on
    /// the transfer — and it has to name both numbers, because "too large" is not an answer
    /// somebody can act on.
    #[test]
    fn a_plan_over_the_budget_is_refused_with_both_numbers_in_the_sentence() {
        let plan = Plan {
            url: "https://example.invalid/0000.zip".to_owned(),
            members: Vec::new(),
            first: 0,
            last: (64 << 20) - 1,
            directory_bytes: 0,
        };
        let refused = plan.within(Budget::DEFAULT).unwrap_err();
        let sentence = format!("{refused}");
        assert!(sentence.contains("67108864"), "{sentence}");
        assert!(sentence.contains("33554432"), "{sentence}");
        // And the number that would admit it, because a refusal that does not say what works
        // is a wall rather than a bound on accident.
        assert!(sentence.contains("--budget-mb 64"), "{sentence}");
        assert!(plan.within(Budget::mebibytes(64)).is_ok());
    }

    #[test]
    fn the_default_budget_is_thirty_two_mebibytes() {
        assert_eq!(Budget::DEFAULT.bytes(), 33_554_432);
        assert_eq!(Budget::mebibytes(8).bytes(), 8_388_608);
    }
}
