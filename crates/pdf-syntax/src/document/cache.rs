//! The decoded-stream cache: what a filter chain made of a stream, kept under a byte budget
//! (ADRs 0260, 0317, 0437).

use std::collections::HashMap;
use std::sync::Arc;

use super::StreamRefusal;
use crate::filter::{Damage, Decoded, FilterRefusal};
use crate::object::Dictionary;

/// What running a chain over one buffer produced, which is bytes or a refusal.
///
/// **A refusal is an outcome and not an absence**, which is the whole of ADR 0437: reaching
/// [`FilterRefusal::TooLarge`] costs up to [`crate::parser::Limits::max_stream_len`] of inflation, so a stream
/// one page refuses is a stream every page re-inflates unless the answer is kept. The two arms
/// are one memo rather than two maps because they are one question — what does this chain make
/// of these bytes — and because eviction, the byte budget and the liveness invariant are then
/// written once.
pub(super) enum Outcome {
    /// The bytes the chain produced, and whether it produced all of them.
    Decoded {
        /// What that chain produced.
        data: Arc<[u8]>,
        /// Whether it produced all of it, memoised with the bytes.
        ///
        /// Held rather than re-derived because it is a property of the *decode* and the decode
        /// is what this cache exists to avoid running twice. A hit that answered `None` here
        /// would make a damaged stream report on its first reading and stay silent on every
        /// later one, which is a report that depends on the cache's budget. ADR 0343.
        damage: Option<Damage>,
    },
    /// A filter refused, and the bound it refused under.
    Refused {
        /// What [`super::Document::decoded_under`] would have returned.
        why: StreamRefusal,
        /// The [`crate::parser::Limits::max_stream_len`] in force when it did.
        ///
        /// **A refusal is only an answer under a bound at least as tight as the one that
        /// produced it**, and this is the field that says so. [`super::Document::decoded_under`] is
        /// asked under two different bounds — the document's, and
        /// [`super::Document::nested_content_source`]'s smaller allowance — and
        /// [`FilterRefusal::TooLarge`] means "longer than *this* limit", which says nothing
        /// about a larger one. Serving such an entry to a caller under the document's own bound
        /// would refuse a stream this reader can decode, on the strength of a routing decision
        /// somewhere else. The other two refusals are properties of the bytes and the chain
        /// alone, and are kept under the same rule rather than under a second one.
        under: usize,
    },
}

/// One stream's decode, and what proves the key still names that stream.
pub(super) struct DecodedEntry {
    /// The encoded bytes the key was taken from, held for as long as the entry is.
    ///
    /// **This field is the whole soundness argument and it is not a copy of anything.** The key
    /// is an address, and an address is only an identity while the allocation lives: a freed
    /// buffer's address is handed to the next allocation, which would make a lookup for a
    /// *different* stream find these bytes. Holding the `Arc` makes that impossible — the
    /// allocation cannot be freed while the entry that names it exists, so no two live entries
    /// can share a key and nothing allocated after an entry can collide with it. `doc/todo/41`
    /// records the census where exactly this went wrong: below 4 KB its counts were worthless
    /// "\[because] an address freed with one document is handed to the next".
    encoded: Arc<[u8]>,
    /// §7.4's filter chain, with its parameters, as it was when these bytes were produced.
    ///
    /// Compared on every hit. One `Arc<[u8]>` of encoded bytes can be shared by two `Stream`s
    /// with different dictionaries — `pdf_model::thumbnail::significant` builds one such — and
    /// two chains over one buffer are two different decodes.
    chain: Vec<(Vec<u8>, Option<Dictionary>)>,
    /// What that chain made of those bytes.
    outcome: Outcome,
    /// The value of [`DecodedStreams::clock`] when this entry was last read or written.
    used: u64,
}

impl DecodedEntry {
    /// What this entry is charged against [`DecodedStreams::budget`].
    ///
    /// **Three terms, each of them something the entry holds**, and no constant chosen by
    /// anybody: the entry itself, the encoded bytes it pins — see [`Self::encoded`] for why it
    /// must — and the decoded bytes where there are any. A refusal holds none of the third,
    /// which is why the first term had to exist before a refusal could be kept at all: charged
    /// nothing, refusals would accumulate without a ceiling, and this cache's whole shape is
    /// that it has one.
    ///
    /// What the charge does **not** count is heap the entry reaches rather than holds: the
    /// chain's names and parameter dictionaries, and the filter name inside a
    /// [`StreamRefusal::Filter`] — tens of bytes apiece, uncounted before this and uncounted
    /// now. The budget's own rule is that it be legible rather than exact (ADR 0317).
    pub(super) fn charge(&self) -> usize {
        let held = match &self.outcome {
            Outcome::Decoded { data, .. } => data.len(),
            Outcome::Refused { .. } => 0,
        };
        size_of::<Self>()
            .saturating_add(self.encoded.len())
            .saturating_add(held)
    }
}

/// The decoded streams one open document is holding, under a byte budget.
///
/// # Why a document memoises this at all
///
/// A resource is decoded once per *use*, not once per document: a font program referenced by
/// eight hundred pages is inflated eight hundred times, and one sweep of ISO 32000-2 spends
/// 24.6% of its wall clock in [`super::Document::decoded_stream_data`] of which 23.4% is decoding
/// something it had already decoded — 830 MB of re-inflation against 46 MB of first decodes.
/// `doc/todo/41` priced this at 0.7% over the pdf.js corpus and was right about that
/// population: a corpus interpreted one page per document has nothing to repeat. What repeats
/// is what a *reader* does — turning pages, and searching a document from end to end.
///
/// # The lock, and why a hit takes the exclusive one
///
/// [`super::Document::get`] is asked 829 times a page and this is asked 12; a hit updates the
/// recency stamp, so it takes the write lock, which adds 12 586 exclusive acquisitions to a
/// sweep that already takes 61 836 for the object cache (ADR 0260's first table). Keeping
/// recency in an atomic so that a hit could share the read lock would buy a parallel sweep
/// something and cost every reader a less obvious structure than this one; the viewer sweeps on
/// one thread, and ADR 0260 declined the parallel sweep on memory rather than on locking.
pub(super) struct DecodedStreams {
    /// What is held, by the address and length of the encoded bytes.
    held: HashMap<(usize, usize), DecodedEntry>,
    /// How many bytes that is, counted as it changes rather than summed on every insertion.
    bytes: usize,
    /// The ceiling those bytes are held under.
    budget: usize,
    /// A counter that only goes up, which is what "least recently used" is ordered by.
    clock: u64,
    /// How many lookups since the document opened were answered without running a filter.
    hits: u64,
    /// How many were not.
    misses: u64,
    /// How many entries the budget has dropped.
    evicted: u64,
}

/// The address and length of some encoded bytes, which is what a decode is keyed by.
///
/// Two values, because a length is a cheap disagreement to find and the address alone is what
/// [`DecodedEntry::encoded`] has to work to make trustworthy.
fn allocation(data: &Arc<[u8]>) -> (usize, usize) {
    (Arc::as_ptr(data).cast::<u8>().addr(), data.len())
}

impl DecodedStreams {
    /// A cache holding at most `budget` bytes.
    ///
    /// The budget is a parameter so that eviction can be exercised on a budget of a few bytes,
    /// and so that a measurement can build the same tree with the cache off; every document
    /// opens with [`super::DECODED_BUDGET`].
    pub(super) fn with_budget(budget: usize) -> Self {
        Self {
            held: HashMap::new(),
            bytes: 0,
            budget,
            clock: 0,
            hits: 0,
            misses: 0,
            evicted: 0,
        }
    }

    /// What this chain made of these bytes before, marking it most recently used.
    ///
    /// `bound` is the [`crate::parser::Limits::max_stream_len`] the asking caller will decode under, and it is
    /// read only by [`Outcome::Refused`] — see that variant's `under` field for why a refusal
    /// under a tighter bound is not an answer under a looser one.
    pub(super) fn get(
        &mut self,
        encoded: &Arc<[u8]>,
        chain: &[(Vec<u8>, Option<Dictionary>)],
        bound: usize,
    ) -> Option<Result<Decoded, StreamRefusal>> {
        self.clock = self.clock.saturating_add(1);
        let clock = self.clock;
        let answer = self
            .held
            .get_mut(&allocation(encoded))
            .filter(|entry| entry.chain == chain)
            .and_then(|entry| {
                let answer = match &entry.outcome {
                    Outcome::Decoded { data, damage } => Ok(Decoded {
                        data: Arc::clone(data),
                        damage: *damage,
                    }),
                    Outcome::Refused { why, under } if bound <= *under => Err(why.clone()),
                    Outcome::Refused { .. } => return None,
                };
                entry.used = clock;
                Some(answer)
            });
        if answer.is_some() {
            self.hits = self.hits.saturating_add(1);
        } else {
            self.misses = self.misses.saturating_add(1);
        }
        answer
    }

    /// The bound a window has already read this decode to without finding anything in it.
    ///
    /// **A question about routing rather than a request for bytes**, which is why it counts
    /// neither a hit nor a miss: [`Self::hits`] and [`Self::misses`] tally lookups that asked
    /// what a chain produces, and adding a second population to them would make the instrument
    /// mean two things. What it does do is stamp the entry, because an entry that answers is an
    /// entry in use and eviction is ordered by that stamp.
    ///
    /// Only [`FilterRefusal::TooLarge`] answers. The other refusals are properties of the bytes
    /// rather than of a bound, and a caller asking this one has a chain a window can pump, so
    /// there is nothing for them to say here.
    pub(super) fn nothing_under(
        &mut self,
        encoded: &Arc<[u8]>,
        chain: &[(Vec<u8>, Option<Dictionary>)],
        bound: usize,
    ) -> Option<usize> {
        self.clock = self.clock.saturating_add(1);
        let clock = self.clock;
        let entry = self
            .held
            .get_mut(&allocation(encoded))
            .filter(|entry| entry.chain == chain)?;
        let Outcome::Refused {
            why:
                StreamRefusal::Filter {
                    why: FilterRefusal::TooLarge { limit },
                    ..
                },
            under,
        } = &entry.outcome
        else {
            return None;
        };
        // The same rule [`Outcome::Refused`]'s `under` field states: an answer reached under a
        // tighter bound says nothing about a looser one.
        if bound > *under {
            return None;
        }
        let limit = *limit;
        entry.used = clock;
        Some(limit)
    }

    /// How many decoded bytes this cache would keep beside `encoded`.
    ///
    /// [`Self::keep`]'s own condition, asked before the decode rather than after it: the entry
    /// is charged for every part of itself ([`DecodedEntry::charge`]), so a decode longer than
    /// this is one the cache declines however empty it is.
    /// [`super::Document::nested_content_source`] is what asks, and it is the whole of that routing
    /// rule — which is why the two are one function rather than two comparisons that can drift.
    pub(super) fn allowance(&self, encoded: &Arc<[u8]>) -> usize {
        self.budget
            .saturating_sub(encoded.len())
            .saturating_sub(size_of::<DecodedEntry>())
    }

    /// Keeps a decode, dropping least-recently-used entries until it fits.
    ///
    /// A decode too large for the whole budget is not kept, rather than emptying the cache to
    /// hold one entry that the next insertion would drop. [`DecodedEntry::charge`] is what
    /// "too large" is measured with.
    pub(super) fn put(
        &mut self,
        encoded: &Arc<[u8]>,
        chain: Vec<(Vec<u8>, Option<Dictionary>)>,
        decoded: &Decoded,
    ) {
        self.keep(
            encoded,
            chain,
            Outcome::Decoded {
                data: Arc::clone(&decoded.data),
                damage: decoded.damage,
            },
        );
    }

    /// Keeps a refusal, under the bound it was reached under.
    ///
    /// **The bound is what makes this sound and the pin is what makes it addressable**, and
    /// neither is new: [`Outcome::Refused`] argues the first and [`DecodedEntry::encoded`] the
    /// second. What is new is that a refusal costing a gibibyte of inflation to reach is
    /// reached once per document rather than once per page that names the stream (ADR 0437).
    ///
    /// A refusal whose *encoded* bytes do not fit the budget is declined, exactly as an
    /// oversized decode is — so a bomb in a very large stream still costs its inflation per
    /// read, and `doc/todo/41` says what that leaves.
    pub(super) fn refuse(
        &mut self,
        encoded: &Arc<[u8]>,
        chain: Vec<(Vec<u8>, Option<Dictionary>)>,
        why: &StreamRefusal,
        under: usize,
    ) {
        self.keep(
            encoded,
            chain,
            Outcome::Refused {
                why: why.clone(),
                under,
            },
        );
    }

    /// Puts one outcome in, evicting until its charge fits under the budget.
    pub(super) fn keep(
        &mut self,
        encoded: &Arc<[u8]>,
        chain: Vec<(Vec<u8>, Option<Dictionary>)>,
        outcome: Outcome,
    ) {
        self.clock = self.clock.saturating_add(1);
        let entry = DecodedEntry {
            encoded: Arc::clone(encoded),
            chain,
            outcome,
            used: self.clock,
        };
        let size = entry.charge();
        if size > self.budget {
            return;
        }
        let key = allocation(encoded);
        if let Some(previous) = self.held.remove(&key) {
            self.bytes = self.bytes.saturating_sub(previous.charge());
        }
        while self.bytes.saturating_add(size) > self.budget {
            if !self.evict() {
                return;
            }
        }
        self.bytes = self.bytes.saturating_add(size);
        self.held.insert(key, entry);
    }

    /// Drops the least recently used entry, answering whether there was one.
    ///
    /// A scan rather than a second index ordered by use, for the reason `viewer-core`'s readback
    /// cache gives: two maps kept in step are more lines and one more thing to get wrong, and
    /// this walks a few thousand `u64`s against the inflation that filled the entry it drops.
    fn evict(&mut self) -> bool {
        let Some(oldest) = self
            .held
            .iter()
            .min_by_key(|(_, entry)| entry.used)
            .map(|(key, _)| *key)
        else {
            return false;
        };
        if let Some(entry) = self.held.remove(&oldest) {
            self.bytes = self.bytes.saturating_sub(entry.charge());
            self.evicted = self.evicted.saturating_add(1);
        }
        true
    }

    /// Forgets everything, keeping the tally of what has happened.
    pub(super) fn clear(&mut self) {
        self.held.clear();
        self.bytes = 0;
    }

    /// What this cache is holding, for a caller that wants to say so.
    pub(super) fn report(&self) -> DecodedStreamCache {
        DecodedStreamCache {
            streams: self.held.len(),
            bytes: self.bytes,
            budget: self.budget,
            hits: self.hits,
            misses: self.misses,
            evicted: self.evicted,
        }
    }
}

/// What one open document's decoded-stream cache is holding, and how it has been used.
///
/// The bound this project asks of a memory budget is that it be *legible* rather than small,
/// so the number can be read off. Reported by [`super::Document::decoded_streams`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodedStreamCache {
    /// How many decoded streams are held.
    pub streams: usize,
    /// How many bytes that is, counting each entry's encoded bytes as well as its decoded ones.
    pub bytes: usize,
    /// The ceiling those bytes are held under, which is [`super::DECODED_BUDGET`] for every document.
    pub budget: usize,
    /// How many lookups since the document opened were answered without running a filter.
    pub hits: u64,
    /// How many were not.
    pub misses: u64,
    /// How many entries the budget has dropped.
    pub evicted: u64,
}
