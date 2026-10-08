# 1745 — The reference renderer's duplicate codecs are skipped by version

Session 1454. Status: **accepted** and **built**. A `deny.toml` decision a later round should not
re-open: which of `cargo deny`'s duplicate warnings the tree has answered, and how. Supersedes
nothing.
Context: ADRs 1714, 1730.
Code: `deny.toml`'s `[bans] skip`, and its `allow-git` list.

## 1. A duplicate the tree chose is a line, not a warning

`cargo deny check` warned about 38 duplicate packages at HEAD f0ab0216. Five of them are there by
decision, each the reference renderer's crates.io copy beside the copy this program ships.
`tools/hayro-compare` drives `hayro` 0.7.1 for the oracle and is not shipped. `hayro-syntax` 0.7.2
takes `zune-jpeg` 0.5.15 and `zune-core` 0.5.1 from crates.io, and the program takes both from the
owner's fork (ADR 1730). `hayro` 0.7.1 takes `hayro-jbig2` 0.3.0, `hayro-jpeg2000` 0.3.5 and
`hayro-ccitt` 0.3.0, and the worker takes 0.3.1, 0.4.1 and 0.4.0 from upstream's release commit
(ADR 1714). **The brief named the JPEG pair. The three codecs are the same decision, so they take
the same line.** Leaving them as warnings would leave a later round to settle the same question
again. Each is skipped by the exact version the reference renderer takes, with its reason. The
duplicate warnings go from 38 to 33, and every one left is a version split nobody chose.

## 2. What a version cannot see, and what reports a stale line

Cargo-deny's package spec names a crate and a version, and not a source. The JPEG pair is one
version from two sources, so its skip covers both copies. A third version of either is still a
warning, and a fork that moves to another version leaves the crates.io copy alone under the skip.
That is the same answer. For the three codecs the skip names only the crates.io version. A release
of `hayro` that moves any of them makes cargo-deny print `unmatched-skip`, and the line goes.
That was calibrated by planting a skip for a version not in the lock (`zvariant@5.0.0`): cargo-deny
named it (trap 13). One stale line is still silent. If the fork is ever given back to crates.io, the
JPEG skip still matches the one copy left, and it skips nothing that is a duplicate. That costs a
sentence and no warning.

## 3. A source nobody uses

The same run printed `unmatched-source` for `allow-git`'s `https://github.com/close2/quorra`. The
rasteriser joined this workspace under `raster/` on 2026-09-06 (commit `eacf4951`), and `Cargo.lock`
has had no git source for it since. The line is gone and the list's comment says what it holds. A
git source that comes back has to be allowed again, which is what the list is for.
