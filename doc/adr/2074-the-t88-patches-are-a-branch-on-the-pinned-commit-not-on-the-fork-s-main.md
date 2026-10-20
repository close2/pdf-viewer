# 2074 — The three T.88 patches are a branch on the pinned commit, not on the fork's `main`

Session 1619, slot 5 of batch ninety-seven, a clause round. Status: **accepted**; the branch is
**built**, pushed by the owner (`doc/questions/A514`) and pinned, and section 4's steps are done
(ADR 2102). Context: ADRs 1447, 1459, 1485
(the three patches), 1714 (the codecs taken at their release commit), 0014 (a codec this project
does not own). Clauses: ISO 32000-2 §7.4.7; ITU-T T.88 7.4.6.2, cited and paraphrased. ADR 2075 is
unused.

## 1. What the fork holds

Read on 2026-10-20 from a bare clone in a fresh directory, nothing run from it. `close2/hayro` has
four branches and no tags. `main` and `feat/reduced-resolution-allocates-less` are the same commit,
`1dc833f7`: upstream's `cc9c4024` plus two JPEG 2000 commits of the owner's.
`feat/reduced-resolution-rebased` is two commits on upstream's `a7e3aace`, later than `main`'s base and still
before the pin; `fix/reconstruction-midpoint-when-fully-decoded` is `main`'s first commit, one more
of its own and two merges of `main`. None of
the four contains `ea9c81dc`, the commit the manifest pins and upstream tags `hayro-jbig2-v0.3.1`,
and no commit of the fork's own touches `hayro-jbig2/` or `hayro-ccitt/`. So no branch carries any
of the three patches.

## 2. Why the branch is not built on `main`

The brief asked for the patches on a clone of the fork's `main`. That premise did not hold.
**`main` is behind the pin**: its base is 31 upstream commits before `ea9c81dc`, and its manifests
state `hayro-jbig2` 0.3.0 and `hayro-jpeg2000` 0.4.0. Going from its `hayro-jpeg2000/` to the
pinned one adds 160 lines and removes 137 over 9 files. Moving the three stanzas to a commit on `main` would take the
JPEG 2000 codec back past 0.4.1, the release ADR 1714 pins for its three ISO/IEC 15444-1 fixes. It
would be a codec change of its own that nobody asked for. Each patch also names its base as
`ea9c81dc` in its own header, and applies there.

So the branch is `quorra/t88-patches` in `/home/AI/hayro-fork/`, three commits on `ea9c81dc`, one
per patch, in the order bounds (ADR 1447), extended template (ADR 1459), Annex H.2 test (ADR 1485).
Its tip is `7e39af411b807cc267fb7a72545083f10ca2077f`. The commits were made through a temporary
index, so no file of the fork was ever written to disk. The three codec trees at the base are
byte-identical to cargo's checkout of `ea9c81dc`, which this tree already builds; the licence
symlinks are the only entries that comparison does not hash. The branch's `hayro-jbig2/` is
byte-identical to that checkout with the three patches applied. In a copy of that checkout, never
the fork, `cargo test -p hayro-jbig2 --lib` passes 3 of 3, Annex H.2's
`table_h1_decision_by_decision` among them. The crate's `asset_suite` has no inputs until upstream's
`sync.py` downloads them, so it was not run, at the base or on the branch. A branch name rather
than `main` leaves the owner's own `main` and its JPEG 2000 work as they are.

## 3. What upstream did meanwhile

Upstream's `main` (`af0dc88a`, 2026-10-19) is 119 commits past the pin. Among them is #1485,
`e7c49ef5`, "Support the extended template with 12 AT pixels", of 2026-10-13. It is upstream's own
implementation of the requirement ADR 1459's patch implements. It is unreleased: no tag after
`ea9c81dc` exists. That commit is evidence that two readings of T.88 7.4.6.2 meet, and nothing more
(principle 5). Taking it would mean taking 119 commits, the restructuring of #1530 into files and pages among them, and so a
codec change with its own round. Bounding three symbol-dictionary loops (ADR 1447) is not on
upstream's `main`. The fork branch is still the smallest step that moves §7.4.7. A release of
upstream's carrying #1485 is the other way out, and it would make the extended-template patch
unnecessary.

## 4. What the row does

§7.4.7 stays `partial`, and its note now names the branch and the question. When the owner has
pushed, the round that moves the stanzas does Q514's six steps. It points the three stanzas at
`close2/hayro` at the tip, adds the fork's URL to `deny.toml`'s `allow-git`, and updates the lock.
It deletes the three patch files and `decode::extended_template_region`'s refusal, and replaces its
test with a decode. It moves the row to `implemented` on the Annex H.2 test and on Annex K item 7's
generic region, and moves the heading rows with it (ADR 1599). Last, it compares the six arms by
page digest. Only the extended-template patch changes what a valid stream decodes to, and no
corpus stream uses that template (ADR 1459 section 6), so the arms are expected not to move. That
is a prediction for that round to check.
