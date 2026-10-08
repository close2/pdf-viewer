# 1714 — The `hayro` codecs are taken at their release commit, from a git source, and the fork is where the patches land

Session 1441. Status: **accepted**. Amends ADRs 0190, 0233 and 0321 on where `hayro-jpeg2000` comes
from, and ADRs 1447 and 1459 on what the three `hayro-jbig2` patches are written against; their
arguments stand.
Context: `Cargo.toml` (the `hayro-jbig2`, `hayro-jpeg2000` and `hayro-ccitt` stanzas), `Cargo.lock`,
`fuzz/Cargo.lock`, `deny.toml` (`allow-git`), `doc/patches/hayro-jbig2-*.patch` (preambles),
`tools/main-checkout.py` (`patches`), `doc/stack.md` (the image-codec row).

## 1. What the pin was for, and which release carries it

The three crates were taken from `close2/hayro` at `64efcaca`, which is upstream `4aaabad7` plus one
commit of the owner's. Each stanza named its own expiry: a release carrying what it was pinned for.
On 2026-10-04 upstream published `hayro-jbig2` 0.3.1, `hayro-jpeg2000` 0.4.1 and `hayro-ccitt`
0.4.0, all from `ea9c81dc` (tags `hayro-jbig2-v0.3.1` and `hayro-jpeg2000-v0.4.1`). Each crate's
`.cargo_vcs_info.json` names that commit, and each published `src/` is byte for byte the tag's.

| crate | the pin was for | in the release | what else differs from `64efcaca` |
|---|---|---|---|
| `hayro-jbig2` 0.3.1 | `1be7ab10` (#1278): a text region's instance count bounded by its segment's length, not a flat 10 000 | yes | `#[inline(never)]` on one function, and an import in `simd.rs`, which `default-features = false` does not compile |
| `hayro-ccitt` 0.4.0 | being the source `hayro-jbig2` names | yes | the version number alone |
| `hayro-jpeg2000` 0.4.1 | the reduced-resolution allocation (ADR 0233) | yes: #1352, `5a5f0e24`, whose `src/` equals the fork's `feat/reduced-resolution-rebased` (`88e7e9cb`) | six upstream commits (section 3) |

**The pin was also fragile.** `git ls-remote` on `close2/hayro` names four refs, and `64efcaca`
is on none of them. Every build could fetch it only because GitHub serves an unreferenced commit
of a fork network by its hash.

## 2. What the pin still bought, and the decision

The pin bought no compiled code: the table's last column is everything that differs. The fork has
taken none of the three `hayro-jbig2` patches (its `main` is `1dc833f7`), so no build ever carried
one. **It bought two other things, and the second was found only by trying crates.io.**

1. The patches' base was pinned, which `tools/main-checkout.py` counts as a patch still owed.
2. **Its copy was a different package from the reference renderer's.** `tools/hayro-compare`'s
   `hayro` 0.7.1 reaches `hayro-jbig2` `^0.3` through `hayro-syntax`. `hayro-interpret` turns on
   `hayro-syntax`'s `unsafe` feature unconditionally, and that feature turns on `hayro-jbig2/simd`.
   Cargo unifies one package's features over the packages a build selects. With
   `hayro-jbig2 = "0.3.1"` from crates.io, `cargo +nightly build --release --bin
   pdf-sandbox-worker --unit-graph` (the form `tools/batch.sh install` builds with) gave the
   worker's `hayro-jbig2` the features `fearless_simd, simd, std`. That is the one `unsafe` the
   stanza's `default-features = false` exists to leave out of untrusted input's path (principle 3).
   With a git source it gives `std`, and with `-p pdf-sandbox` alone `std` in both cases. The
   reference renderer's `hayro-jpeg2000` is `^0.3.5`, so today only JBIG2 would be reached, and a
   `hayro` that asks for a 0.4 would reach JPEG 2000 too.

**So the three are taken from upstream's own repository at `ea9c81dc`** — the commit the releases
were published from, on a tag, so the source is the release and the reference renderer's features
stay with its copy. This keeps `default-features = false` and keeps one revision for all three.
The cost is a git fetch of upstream's repository and an `allow-git` entry,
`https://github.com/LaurenzV/hayro`, in place of `close2/hayro`. The lock changes three packages and
nothing else. All three crates are `#![forbid(unsafe_code)]`, and a grep of their `src/` finds
`unsafe` only in that attribute and in prose. The licence is unchanged (`Apache-2.0 OR MIT`), and
the MSRV is 1.92 against this tree's 1.97. Going to crates.io is possible only once no build can
select the reference renderer and the worker together with `simd` on. That needs
`tools/hayro-compare` outside the workspace, or a `hayro` without that feature.

**The patches keep the fork as their destination.** Their preambles now read `Repository:`
upstream, `Base: ea9c81dc`, `Fork: https://github.com/close2/hayro`. All three apply there, alone
and together. With all three applied, `cargo test -p hayro-jbig2 --lib --no-default-features
--features std` passes its three tests at that base, Annex H.2's among them. The owner applies them
to the fork on `ea9c81dc`, pushes, and moves the three stanzas to the fork at the pushed commit;
`hayro-ccitt` has to move with `hayro-jbig2`, because one crate from two sources is two crates with
one name. `tools/main-checkout.py` names the fork in that step: the manifest pins the patch's
repository at its base, and the step names the `Fork:` it carries.

## 3. What 0.4.1 changes in a JPEG 2000 decode

These are the six upstream commits since the fork's base, each read against ISO/IEC 15444-1. What
the gates found is in the session's record.

- **#1355** drops the 60 000-pixel ceiling on a side and checks the products it allocates from for
  overflow. Nothing new reaches the decoder: `pdf_sandbox::decode::jpx_within_budget` counts samples
  from the header and refuses or steps down before any sample is decoded.
- **#1352** is the fork's own commit as upstream merged it.
- **#1373** holds one component's coefficients at a time. Peak memory falls and no value changes.
- **#1374** refuses a code-block size outside Table A.18's range: each exponent at most 10, and the
  two together at most 12. That is a refusal of a codestream the standard does not admit.
- **#1377** reads a YCCK `colr` enumeration (T.801 Table M.25) as CMYK after the inverse YCC
  transform. Before, the decoder refused it as unsupported.
- **#1381** clamps a palette index that a lossy decode reconstructs outside the palette. ISO/IEC
  15444-1 I.3.4 and I.5.3.4 give an entry only for the indices the palette has, so the standard
  states nothing for such an index. The repair is the codec's choice, and this tree did not make
  it. Before, the decode was refused, so a page this reaches draws where it used to report.
