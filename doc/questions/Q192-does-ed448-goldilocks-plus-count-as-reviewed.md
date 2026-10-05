# Q192 — Does `ed448-goldilocks-plus` count as "reviewed" under `doc/stack.md`'s terms?

Source: round 1295, from `doc/todo/65` bucket 2, which named the package as a candidate on 2026-09-29.
Status: **open** — answered when `A192-does-ed448-goldilocks-plus-count-as-reviewed.md` exists beside this file.

## Why it needs the owner

The swap condition for `crates/pdf-signature/src/ed448.rs` is the owner's own, from A170 as ADR 1386
item 3 records it: the day a *stable, reviewed* crate covers the curve, the swap is decided on
`doc/stack.md`'s terms. "Stable" can be read off a version number. "Reviewed" is a word whose meaning
belongs to the owner, and one package now meets the first half and not, by its own statement, the
second.

## What was found (2026-09-30)

- **The package.** `ed448-goldilocks-plus` 0.18.1 (`cargo info`): Ed448, Curve448 and Decaf, with
  Ed448 signing and verification; `BSD-3-Clause`; no stated MSRV; default features bring `std`,
  `signing`, `pkcs8` and `kex`. It is on a stable version line, which RustCrypto's own
  `ed448-goldilocks` is not (`0.14.0-pre.15`, `Apache-2.0 OR MIT`, MSRV 1.85).
- **Where it comes from.** It is published from `mikelodder7/Ed448-Goldilocks`, a one-person
  GitHub fork of `crate-crypto/Ed448-Goldilocks`. RustCrypto's `ed448-goldilocks` lives in
  `RustCrypto/elliptic-curves` and is a different package. RustCrypto neither publishes nor
  maintains the `-plus` line, and nothing in either repository says it tracks RustCrypto's.
- **The review record.** Neither repository names an audit. **Both READMEs state, in capitals, that
  the code has not been audited or reviewed and is to be used at one's own risk**: the fork's and
  RustCrypto's `ed448-goldilocks`'s alike.
- **The licence.** `BSD-3-Clause` is not one of the `Apache-2.0 OR MIT` pair every other
  cryptographic package here carries. Whether `deny.toml` admits it has not been checked, because the
  question above comes first.

## What the tree does meanwhile

Nothing changes. `ed448.rs` stays the tree's own under A170, not advertised and built so the switch
is easy (ADR 1386). `doc/todo/65` names this question beside the candidate instead of judging it.

## Recommendation

**No, it does not count.** The package's own README says it has not been reviewed. Being stable is
not the same as being reviewed, and the tree's other curves were taken from RustCrypto because they
share a supplier with the ciphers and digests already here (`doc/stack.md`), which a one-person fork
does not. Keep the swap condition as written and keep checking RustCrypto's `ed448-goldilocks`. If
that crate reaches a stable line, this same question comes back about it, and its README carries the
same sentence today. If the owner wants "reviewed" to mean something weaker, such as "stable, and
from the supplier the tree already trusts", saying so here would settle both packages at once.

## Two more packages, judged on 2026-10-01

`cargo search ed448` finds two Ed448 packages this question did not name. Neither changes the
recommendation, under either reading of "reviewed".

- **`cx448` 0.1.1** (`dignifiedquire/cx448`, one owner, `BSD-3-Clause` in its metadata and no
  licence file in its repository). It covers what `ed448.rs` does: RFC 8032 verification with a
  context and Ed448ph, 57-byte point decoding, and the RFC's section 7.4 vectors among its tests.
  But its README says, as the two above do, that the code has not been audited or reviewed. It
  describes itself as a temporary port, to be retired once RustCrypto's stable releases land. It
  has not been touched since 2025-04-10, and it depends on the previous RustCrypto generation
  (`elliptic-curve` 0.13, `crypto-bigint` 0.5, `digest` 0.10). Its download count comes from the
  OpenPGP crates that bundle it, not from any review.
- **`tiny_ed448_goldilocks` 0.2.0** (`Dustin-Ray/tiny-ed448-goldilocks`, one owner, `MIT`). It has
  group and field arithmetic only: no RFC 8032 signature scheme and no point encoding. Its README
  says it is unaudited.

Neither package is published by RustCrypto, so the weaker reading offered above settles them as
well. The trigger to watch is still RustCrypto's `ed448-goldilocks`. On this date it is
`0.14.0-pre.15`, and its last stable release is 0.9.0.


## Five more packages, judged on 2026-10-05

Four of the five names `doc/todo/65` listed unjudged on 2026-10-02 leave the recommendation exactly
where it was. `ed448-rust`, `minimal-ed448` and `rs_ed448` have no review on record, and the last has
no code. `oxicrypto-sig` wraps RustCrypto's pre-release `ed448-goldilocks`. `doc/todo/65` has a
paragraph on each.

The fifth changes what the question is about, which is why this section exists. **`frost-ed448`
3.0.0 is the first Ed448 package with an audit on record.** It is published by the Zcash Foundation
under `MIT OR Apache-2.0`. NCC Group's 2023 assessment covered its 0.6.0, and also covered the curve
operations it takes from `ed448-goldilocks` 0.9.0, that package's last stable release. So the strict
reading of A170's "reviewed", meaning an audit names the code, is no longer met by nothing at all.

It is still not the swap, and the reason is the API, not the audit. The package's subject is RFC
9591's threshold scheme, and its single-signer verification is a by-product. In the published 3.0.0
source, every point is decoded through a check that refuses the identity and any point with a torsion
component. RFC 8032 section 5.2.7's cofactored equation, which `ed448.rs` takes on purpose, exists to
accept those points. The package would therefore refuse signatures the RFC calls valid. It also sits
on the previous RustCrypto generation (`sha3` 0.10, `rand_core` 0.6), beside the tree's. And the audit
was of 0.6.0, not of the version that would be taken.

**Recommendation, amended:** still no, for `ed448-goldilocks-plus` and for all eight packages judged
so far. The trigger is still RustCrypto's `ed448-goldilocks` reaching a stable line that carries the
signature scheme. One fact is new for the owner's reading of "reviewed". The curve arithmetic of that
package's own last stable line, 0.9.0, has been inside an audit's scope. Whether that scope reaches
the 0.14 line, which was reworked inside `RustCrypto/elliptic-curves`, is not established here.
