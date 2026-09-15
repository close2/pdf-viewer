# 1076 — A verdict cannot be constructed without an anchored proof

Session 1062. Status: **accepted**.

ADR 1039 decided that a trust store is an input, priced the two defaults and refused both, and
ended by naming what it had not built: "what the next round owes is the host end — who supplies the
anchors". Forty rounds later nobody had. ADR 1067 read revocation out of the file and ADR 1071
established a timestamp chain, and each of them closed with the same sentence — nothing gains the
word *valid*, because nobody names an anchor. This ADR builds the host end and decides the shape of
the word it makes reachable.

## 1. The host end, in one sentence per surface

`pdf_signature::trust::Supply` is RFC 5280 section 6.1.1's inputs (d) and (b) as a host can hold
them: the DER, each certificate under the name the host knows it by, the instant to validate at,
and **the sentence saying where they came from**. The last is not decoration. Section 6.1 says an
anchor is believable because "[t]he trust anchor information is trusted because it was delivered to
the path processing procedure by some trustworthy out-of-band procedure" — a procedure this program
did not perform and cannot describe, so a reader told a signature is valid is owed *valid according
to whom*, and `viewer_core::notes` prints that sentence beside every verdict that rests on it.

`TrustAnchors` could not be that type: every anchor in it borrows from certificate bytes somebody
has to own, and a host holds the owner. `Supply::read` is the lending, and the certificates it will
not take are named one by one rather than counted (trap 5): a person who pointed at a directory of
six files and got four anchors would otherwise read verdicts computed under a store they did not
supply.

- **`viewer_core::Command::Trust(TrustPolicy)`** carries it, on `Command::Restrict`'s rules and for
  a sharper version of its reason: a state machine over a file cannot know whom its reader
  believes, and a file that could name its own anchors would be vouching for itself. It is the one
  thing that invalidates `Open::about`'s `OnceCell`, because the report is a function of the
  document *and* of this policy.
- **`viewer_host::policy::trust_anchors`** is the one place a host answers *which anchors, if any*,
  beside `may_submit` and the six decisions before it. `--trust-anchors <dir>` names a directory;
  PEM and DER are both taken because both are what a person has on a disk; the clock is this
  machine's, because `viewer-core` has none and `pdf-signature` asks none.
- **`viewer-confined`** carries the DER across the confinement (command kind 28), and the direction
  matters: the party that reads certificates off a disk is outside, and the party that builds a
  certification path is inside.
- **`quorra_trust_anchors`** is the C ABI's. It takes no struct by value, so `QUORRA_ABI_VERSION`
  does not move — an entry point *added* is one an old caller never calls.

**Nothing changes for a host that supplies none**, which is every host by default: `Supply::none`,
`Trust::NoAnchorSupplied`, and the paragraph `viewer_core::notes` has printed since the
three-hundred-and-seventy-seventh session, word for word.

## 2. The rule that may not be re-litigated

**`Valid` has no public constructor and no public field, and the only function that makes one takes
an `Anchored` — which can only be made from a `Trust::Anchored`.**

Every module in this crate has spent its life refusing the word, and the discipline that kept them
apart was a *convention*: a rule written in doc comments and kept by whoever read them. A convention
is the wrong instrument for the sentence a reader acts on. So `pdf_signature::verdict` states it in
the type system instead: three of `Trust`'s four variants answer `None` to `Anchored::of`, and a
caller with no anchor cannot reach `Valid` by any path, including a wrong one.

What the word claims is exactly five things and nothing beyond them — question 1 answered
`Unchanged`, question 2 answered `Verified`, a path to a host's anchor, revocation `Good` or an
`Unknown` the host's `Acceptance` admits in the open, and §12.8.5's chain established where the
document carries one. It is not a claim that the anchor is a good anchor, that the signer is who
the certificate says, or that the document means what it appears to.

**`revision::Judgement` did not gain the variant, and that is deliberate.** That enumeration ranks
*objects* against Table 257's `/P`, its own documentation says "[n]o answer here is a verdict on a
signature", and a `Valid` there would have made a statement about a `/DocMDP` level answer a
question about a person. The word belongs to the type that has all three of §12.8.1's answers in
hand, which is this one.

**And `Acceptance` is a host's, not a threshold.** ADR 1067 section 2 stands untouched: a
`Revocation::Good` is still "only ever the output of arithmetic this program performed", and an
absence is still `Unknown` with its reason kept. What `Acceptance` decides is whether a *reader*
acts on a verdict resting on one — principle 3's shape, because RFC 5280 section 6.3.3's remedy is
to fetch a newer list and that is the one branch a document cannot supply.

## 3. What the calibration is, and why it could not be a corpus document

`crates/pdf-signature/src/verdict/tests.rs` says the word once, over a document this project issued
the hierarchy for, and then takes away one input at a time: a different root, no root, a moved byte,
a revoked certificate, an unestablished chain. Each must move the answer off `Valid` and name which
question it fell at. A type that only ever refuses is trivially correct and says nothing about
whether the refusal is load-bearing (trap 13).

**No corpus document can be the positive witness**, and this is trap 8 rather than a shortage: a
path whose outcome is known in advance needs a hierarchy whose private keys somebody here held, and
every certificate in the crawl's signatures was issued by a real authority under a key nobody here
has. The only anchors that exist for those files are their own issuers' self-signed roots, carried
inside the files themselves — which is a document vouching for itself and precisely what a trust
store exists to prevent. `examples/signature_algorithm_census` does exactly that on purpose and
labels the column an *upper bound* rather than a verdict: what it measures is which of §12.8.1's
three questions actually stops these documents once the third is no longer first in the way.

The fixture's own provenance is re-walkable rather than magic: the `/Contents` hole is a fixed size
filled with zeros, so the bytes `/ByteRange` names do not depend on the signature that goes in it,
and `the_signature_was_made_over_the_bytes_the_range_names` recomputes RFC 5652 section 5.4's
`message-digest` over them.

## 4. What this closes, and what it does not

Every §12.8 row whose stated residue was "the anchor and nothing else" is now executed end to end,
and those move to `implemented`. What stays `partial` stays for something else: ETSI EN 319 122's
profiles (§12.8.3.4.3, §12.8.3.4.4), ISO/TS 32002's two uncomputed curves (§12.8.3, §12.8.3.1,
§12.8.3.3), X.690 clause 10's other DER restrictions (§12.8.3.4.2), and §12.8.2's transform
parameters selecting no objects. None of those is a trust question and none of them moved.

Nothing here answers *whose* anchors a reader should use. That is the question ADR 1039 refused to
answer on a reader's behalf, and building the input is not the same as choosing a value for it.
