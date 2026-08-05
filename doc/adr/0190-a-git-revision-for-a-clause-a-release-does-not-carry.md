# ADR 0190 — A git revision for a clause no release carries

Status: accepted, 2026-08-05 (session 311).

## Context

`tests/jpeg2000.rs` has said since the two-hundredth session that thirteen of the corpus's thirty
JPEG 2000 codestreams decode to samples ISO/IEC 15444-5's reference software does not produce, and
that the discriminator is exact: every one states `qntsty` 2, the irreversible 9/7 path.
`doc/JPEG2000_FEEDBACK.md` measured *which way* the samples moved — two of every three toward the
image's own mean, standard deviation down 4% — and offered a hypothesis it was careful to label as
one: the reconstruction bias of ISO/IEC 15444-1 E.1.1.2, which places a nonzero coefficient at the
middle of its quantisation interval rather than at its edge.

**The hypothesis was right, and it was not a partial implementation — there was none.**
`hayro-jpeg2000` 0.4.0's `Coefficient::get` returns the truncated magnitude and nothing adds the
term. Upstream `9cce046b` (2026-07-18) adds it. **No published version carries it**: 0.4.0 was
released 2026-06-14 and is still the newest on crates.io.

What it is worth, measured by the same instrument with nothing else changed: the worst sample
error over the corpus falls from **87 levels to 3**. `S2.pdf` object 33 goes from 102 139 samples
wrong by up to 87 to **63 wrong by one**.

## Decision

**Pin `hayro-jpeg2000` to the git revision `cc9c402486c298f6986620d512372e72754697a1`, and write
the expiry date into both places that describe it.**

`deny.toml` denies unknown git sources and allowed exactly one — quorra, first-party and
commissioned. This is the second and it is neither, so the argument has to be made rather than
assumed:

- **What it buys is a clause, not a convenience.** §7.4.9 hands decoding entirely to ISO/IEC
  15444-1, which leaves a decoder no latitude, so a wrong reconstruction is a wrong page. Two
  corpus pages carry it into the rendering oracle by name.
- **It is pinned by revision, not by branch**, so `Cargo.lock` and the manifest name the same
  forty hex digits and a rebuild in a year gets the same bytes.
- **It is reviewable, and was reviewed.** The whole difference from the published 0.4.0 is three
  files: the reconstruction bias, the plumbing that carries a coefficient's state to it, and a
  one-character fix to JP2 `Lab` colour (`c2df2014` — `lab.ra` where `lab.rb` was meant). Nothing
  else changed under us.
- **It is temporary and says so in three places**: the workspace manifest, `deny.toml`'s
  allow-list, and this ADR. It goes back to crates.io the moment a release contains `9cce046b`.

The alternative — waiting for a release — was rejected because the wait is not a fix's wait. The
work is done upstream; what is missing is a publish.

## What was *not* claimed

**Not one codestream became byte-identical**, and the list of thirteen is the same thirteen. The
population did not move; the magnitude of its error did. `tests/jpeg2000.rs` now carries that
distinction in its own words, along with where to look next: `issue5475.pdf` and the two
`issue5481.pdf` plates did not move *at all*, and they were at 2 to 4 levels before the fix — so
if there is a second defect, it is visible there and nowhere else. Whether the residual is a
defect or the last place of two `f32` pipelines is **not established**, and saying so is cheaper
than finding out that a ratchet was moved on a guess.

The oracle's `AMBIGUOUS_IRREVERSIBLE_JPEG_2000` therefore keeps both its pages, and its note now
says why they stayed rather than implying nothing happened.

## Consequences

- **The worst JPEG 2000 error a user can see falls by an order of magnitude**, on twelve documents.
- **A third-party git dependency exists**, which is a standing cost: `cargo deny`'s source policy
  is one line longer and somebody has to notice the release that lets it be removed. The manifest
  comment is what they will read.
- **`version = "0.4.0"` stays on the git spec**, because `deny.toml` sets `wildcards = "deny"` and
  a git dependency without one is a wildcard. Found by the gate, not by reading.
- **The feedback document is answered rather than open**, which is its own rule: a feedback
  document that still reads as a complaint after the complaint was answered is worse than none.
