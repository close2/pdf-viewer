# 774 — The strike the outline filed one clause late

The errata selection rule's ninth use, and the first time its two rankings agreed at the head —
on two live rows, with the cell-over-prose tie-break deciding between them. Both heads confirmed
their rows; the walk downward paid twice. One of §14.5's unnamed issues turned out to be §14.4's,
filed a clause late by the outline's page-straddle — two strikes under `check`'s four-word floor,
with a gated blockquote, its prose and §14.4's own ledger note all standing on the struck words,
and the record's earlier verdict for the strike beside them written against the wrong clause's
row. And §14.7.6.2, `implemented`, carries an inserted class-route precedence rule the code
satisfies by construction and whose only fixture — a single class object — no ordering of that
route can fail: the settled-row mechanism's fifth shape in five consecutive uses.

Date: 2026-08-28.
ADR: [0712](../adr/0712-the-strike-the-outline-filed-one-clause-late.md), the number the
briefing reserved.

Touched: `crates/pdf-model/src/structure.rs` (one new test, calibrated per trap 13),
`crates/pdf-syntax/src/write.rs` (doc comment only — `identify`'s warrant re-anchored),
`doc/conformance/ledger.toml` (§7.6.4.1, §7.6.6, §14.4, §14.5, §14.7.6.2, reformatted by its own
binary), `doc/errata-read.md` (ninth-use section, and the Issue #691 row corrected in place),
`doc/todo/01`, the ADR and this file.

## What the rule gave

Under the recipe's own single-issue line parse, 302 issue numbers in
`doc/ISO_32000-2_sponsored_EC3.pdf` carry a strike or a caret and **111 were named nowhere** at
this round's base — one more than the eighth use's closing figure, because one of its five
issues had carried a verdict in `doc/errata-read.md`'s tables since the four-hundred-and-
eighteenth session, so only four newly left the population. A parse that also reads multi-issue
annotation lines counts 307 and 112 and moves no head. Seven issues gain verdicts this round:
the two heads', §13.6.7.3.3's, §14.5's two — of which one is §14.4's — and §14.7.6.2's two. The
four-annotation plateau's other settled rows were read far enough to rank only, and their issues
stay in the population on purpose.

## What paid

- **Issue #328 (§14.4, filed under §14.5)**: both file-identifier sentences lose their
  "contents" wording, and three places stood on it — `write.rs::identify`'s gated blockquote and
  prose, and §14.4's ledger note. Behaviour unchanged everywhere: the appended-to bytes are the
  file at the time it was last updated, so deriving the changing identifier from them sits
  inside the amended sentence. The strikes are three words and two — under `spec-errata check`'s
  four-word floor, this rule's third find there.
- **Issue #691's recorded verdict, corrected**: written in the four-hundred-and-eighteenth
  against §14.5's row ("a NOTE about detecting a changed page … names no digest") for a strike
  that is §14.4's uniqueness paragraph, a `should` on writers, one clause from the writer that
  names MD5. First time the outline's coarseness reached a recorded verdict; the sharpened rule
  is in ADR 0712 and `doc/todo/01`.
- **Issue #289 (§14.7.6.2, `implemented`)**: the inserted rule — class-route attribute objects
  may repeat `/O`, later in array order wins — is what `Tree::attributes` has always done, and
  nothing could fail: the row's one fixture attached one class object.
  `an_attribute_two_class_objects_state_goes_to_the_later_one` is the evidence now. Calibrated
  per trap 13: a plant walking the `/C` classes in reverse passes the older single-class test
  and fails only the new one — confirmed by running both against the plant, then reverting it.
- **The heads**: Issue #16 makes Table 27's `/Recipients` a *byte string or array* — read by
  nobody, behind §7.6.5's named refusal, and §7.6.6's note now says why its enumeration stops at
  Tables 25 and 26. Issue #89 makes each of §7.6.4.1's three filter names denote one filter;
  `crypt::crypt_filters` resolves any `/CF` name, now stated in the row as a reader's tolerance.

## Gates

Full §2 sequence, `PDFREF_CACHE` at the shared warm cache. `fmt`, `clippy -D warnings` (silent),
doctests, the fuzz `check`, both trap-10 workers, corpus (974 documents: 0 unopenable, 8 locked,
2 encrypted beyond us, 6 pageless, 67 incomplete, 0 slow), oracle (1945 pages in 57.7s:
61 contradicted, 836 ambiguous, 3 our geometry, 2 reference geometry, 42 not comparable, 18 no
render — exit 0, no ratchet moved), the three text gates, both censuses, dates, XMP, JPEG 2000,
quorra (957 pages: 932 agree, 22 differ, 3 refused, 17 not comparable — the third refusal is the
frame-budget page the seven-hundred-and-seventy-third priced, present at this round's base),
`fixed_documents` (40 checked, 0 absent) and `cargo test -p conformance`, all green.

**The known flake failed twice under load and passed alone five times running.**
`viewer-host::a_launch_waits_for_page_one_instead_of_polling_for_it` failed in the sequence's
nextest (which then left 183 tests unrun) and again in a full `--no-fail-fast` run
(2706 of 2707 passed, and that one failure), at a one-minute load between 8 and 21 with three
sibling rounds beside this one; alone it passed five consecutive runs. Same shape 765 and 771
recorded; not changed, per the briefing and their standing note.

The warm reference cache still carries mupdf failure messages quoting a sibling worktree's path
(r707) — trap 10a's shape in the message text only, the cached verdict being path-independent,
as 771 noted.

## Sweeps

Sixteen sweeps before the edits (re-run against the pristine tree via a reverse-applied patch,
after the first baseline was found contaminated by the round's own edits landing mid-run) and
after them; `quoted` and `unpriced` not run, no page-list note touched. Every delta is the
round's own work: `pointers` +12 paths and +6 symbols, all live; `overtaken` +1 decision record,
overtaken unchanged at 45; `quotations` +14 document and +1 ledger quotations with **diverging
unchanged at 38 and 2** (one former near-quote in §14.4's note is now verbatim); `tables` +9
sentences, +4 key citations, all agreeing; `counts` +56 sentences, every bucket unchanged;
`owed` +5 unnamed terms on two rows — the public-key filter names §7.6.4.1's and §7.6.6's new
sentences deliberately name as unread; `inapplicable` +4 terms from §14.5's note, and MD5's
naming files 8 → 9, which is `identify`'s comment now saying in words what its code says in a
path; `check` in-clause 106 from 107 (the corrected #691 row left the bucket) and elsewhere
+2, both the known correction-quoting-retired-wording shape; `applied` +17 places naming an
erratum, +1 "reads like a correction", read-first list unchanged at 10. `entries`, `unread`,
`blockers`, `capabilities`, `callers`, `overstated` and `parts` moved only in their compile
lines.

## What contradicts the briefing

- **The briefing called round 771 the rule's seventh use and this its eighth; the tree records
  765 as the seventh and 771 as the eighth** (`doc/todo/01`, `doc/errata-read.md`, ADRs 0691 and
  0708), so this round is the ninth use. The tree wins.
- **The briefing's "110 unnamed" was the eighth use's closing figure and is off by one** — 111
  at this round's base, the arithmetic above.
- Main's CI failure `round.sh` flags (run 33121581297, two `float_cmp` errors in `viewer-ui`'s
  test target) is on merge commit 48bb1167, five commits before this round's base; this round's
  `clippy --workspace --all-targets` under `-D warnings` is silent at the base, so the defect
  does not reproduce there and the newest main commits have no CI run yet.
