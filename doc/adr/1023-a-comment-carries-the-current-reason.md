# 1023 — A comment carries the current reason, and four documents say what is

Session 1003. Status: **accepted**. States the rule `doc/reviews/984-direction-and-boundaries.md`
Finding 2 asked for and ADR 1005 §2 priced, applies it to the four navigational documents, and
leaves the sweep over 1,353 session ordinals in Rust comments to the rounds that can run it
without colliding with the code beside them.

Context: `CLAUDE.md` (*Where knowledge lives*, one paragraph appended), `doc/PLAN.md`,
`doc/crate-map.md` (header prose; session 1000 owns the rows this round), `doc/state-of-play.md`,
`doc/HANDOVER.md`, `doc/history/1003-the-comment-rule-and-four-documents-as-what-is.md` (where the
removed prose went). ADRs 0232, 0281, 0428, 0974, 0983 are the four previous compactions and the
method this one reuses; ADR 1005 §2 the proposal; ADR 0974's four headings the classification.

## Context

The review measured a shape rather than a defect: **the tree's knowledge is indexed to a
chronology.** 1,353 session ordinals and 3,744 ADR references inside Rust comments, comment blocks
up to 768 lines, and — its sharpest example — `crates/pdf-model/src/lib.rs:24`, whose second
paragraph tells a reader what the file said in the two-hundred-and-twenty-first session. The
mechanism is a style rather than carelessness: **a correction is appended to the sentence it
corrects**, so the sentence a reader meets first is the retired one, followed by its retirement.

The cost is stated once and is decisive for principle 4: **the reading cost of every function is
the history of that function.** A student should be able to read `run_reader` and learn how a
content stream is interpreted, not learn which session moved a classification into a table.

The four previous compactions (ADRs 0232, 0281, 0428, 0974, 0983) each cut a document down and each
watched it grow back. None of them stated the rule that produces the growth. This one does, and the
rule is the only thing here that is new: everything else is ADR 0974's method applied to four more
files.

## Decision

### 1. The rule, in `CLAUDE.md`, once

> A comment states why the code is as it is *today* and cites by number the ADR that argued it; it
> does not say what the comment used to say, which session changed it, or that an earlier reading
> was wrong. A retired sentence is deleted rather than annotated — the retirement is `doc/adr/`'s
> and `doc/history/`'s, and both are still there.

It binds the four navigational documents in the same words. **It binds nothing else**, and the
exclusions are the point:

- **`doc/adr/`, `doc/history/` and `doc/reviews/` are records.** A record keeps its chronology.
  Rewriting one for tidiness is the single thing a record may not have done to it
  (`doc/history/README.md`), and this rule would be self-defeating without them: the "how" has to
  live somewhere, or a round deleting a retired sentence is deleting the argument.
- **A trap and a habit each *are* an incident with a rule attached.** There the history is the
  lesson, and deleting it deletes the reason anybody believes the rule. ADR 0974 kept them for the
  same reason and this one does not disturb them.

The paragraph names `grep -rE "hundred-and-|session" --include=*.rs crates tools` rather than a
count, because the count is exactly the kind of fact `CLAUDE.md`'s own preceding paragraph says is
not written down.

### 2. The four documents, rewritten as *what is*

Every sentence describing the tree as it is today stayed, verified against the tree rather than
against another document. The contradictions the review listed were each resolved by reading:

| the document said | the tree says | how it was checked |
|---|---|---|
| "Qt dropped" (`doc/PLAN.md` §1) | `crates/viewer-qt` is a Qt 6 Widgets host on `viewer-core`'s boundary and the only C++ here (ADR 0246); `crates/viewer-gtk` is the GTK4 one | `ls crates/`; `crates/viewer-qt/Cargo.toml` |
| a workspace of ten crates | the members are `crates/*`, `tools/*` and `raster/crates/*`, which is between three and four times that and moved by one during this round as session 1000 extracted `pdf-signature` — which is why the rewrite names the command and no count | `cargo metadata --no-deps` |
| "Not yet a git repository" | it is one | `git rev-parse --is-inside-work-tree` → `true` |
| the conformance gate is "Not built" (§3) and "built in the ninth session" (§5a) | built; `cargo test -p conformance` runs it | `ls tools/conformance/tests/` |
| dialogs are `ashpd` through the XDG portal | no crate depends on `ashpd`, and no window of this tree's own opens a chooser | `cargo metadata`; `ls crates/viewer-ui/src/` |
| `render-cpu` is the "startup path" | page one goes to the graphics device by the owner's decision; `render-cpu` is the oracle and the refusal fallback | `CLAUDE.md` principle 2 |
| `CCITTFaxDecode` is "the last absent image codec", nearly free | taken; it decodes in the confined worker beside JBIG2 and JPEG 2000 | `crates/pdf-sandbox/src/decode.rs:3` |
| "the *rest* of the renderer still runs in the main process" (§6) | `viewer-confined`'s `pdf-view-worker` confines the document, the interpreter and the rasteriser, and `quorra-confined` is a window on it; the flagship's move is what is left | `doc/state-of-play.md`; `ls crates/viewer-confined/` |
| the two speed fixes are "written up in *Where the time went* in `doc/HANDOVER.md`" | that section does not exist; `doc/performance.md` §4 owns the measurement | `grep -rn "Where the time went" doc/` |
| a `corpus` directory under `tools` (§2's tree drawing) | never existed in this tree | `ls tools/` |
| Type1 strategy undecided (§8) | implemented directly | `crates/pdf-font/src/type1.rs` |

`doc/crate-map.md`'s missing `pdf-archive` row and its row for a crate that does not exist are
**session 1000's**, which added `pdf-signature` and `pdf-archive` and corrected `render-quorra` to
`render-raster` in the same tree; this round wrote only the header, which now says that the
population is `cargo metadata`'s rather than this file's.

### 3. Every removal, classified under ADR 0974's four headings

Naming the headings ahead of the diff is what stops a compaction becoming an edit for taste, and
listing the counts under them is what makes the judgement reviewable.

**Duplication — 6.** The §1 stack table (`doc/stack.md` owns it, and its own header says this
section is the rationale for it); §1's `rustybuzz` exclusion (`doc/stack.md` again, at greater
length and with ADR 0348 — and `CLAUDE.md`'s own table sends a round there); the §4a speed table
(`doc/performance.md` §4 owns it); §7's machine and version block and its `sudo -u AI` caveat
(`doc/environment.md:243-275` owns both); §2's crate tree drawing (`Cargo.toml` owns the members,
`doc/crate-map.md` the map). **Both
duplicated pairs had already drifted**, which is the archetype: §1 said "CPU first, GPU behind a
trait" against `doc/stack.md`'s "GPU first, for page one and every page after it", and §4a said
5.9× aggregate and 1.62× median against `doc/performance.md`'s 6.9× and 2.15×.

**A counted fact — 14 sites.** §5a's first-green-run table and the three-session progression after
it (26 figures between them); its "823 rows / 663 leaves", "52 annex rows", "875", "860 `##`
headings", "33 of them", "36 rows"; §4's oracle figures (974, 14, 1794, 34 s, 97, 42, 1020); §4a's
"108 of 1478", "692 of 974" and "our 974 documents"; §4's reference version column; §3's per-spike
test counts and the Arlington and syscall figures; §5's "3468 TSVs"; §5a's "thirty-one lines",
twice; `doc/HANDOVER.md`'s "halved five times". Each is replaced by the command that prints it —
`tools/state.sh` for nine of them, `pacman -Q`, `pdftoppm -v`, `ls | wc -l` for the rest.

**Session narrative — 12 passages.** §3's phase checkboxes, its `~~…~~ **Done.**` spike markers and
"Since the hundred-and-thirty-second session"; the launch-path bullet's "which this line named for
nine hundred rounds" and its "This line used to end …, corrected in the nine-hundred-and-twenty-
second session"; §5a's "Status: built in the ninth session" and its seven per-session review
paragraphs, plus five datings inside sentences ("since the three-hundred-and-sixtieth", "since the
nine-hundred-and-seventy-third", "session 137" twice, "session 311"); §4's "an assumption for six
sessions"; §1's "This paragraph used to say"; §4's "Correction to an earlier assumption here"; §4a's
"(Two of its figures have since moved…)"; §7's "Installed in the last round"; `doc/state-of-play.md`'s
four appended corrections and two smaller ones; `doc/HANDOVER.md`'s two.

**A decayed claim — 13**, each corrected with a sentence saying what replaced it: the twelve rows
of the table in §2 above, and §1's heading "Why CPU first", which the owner's page-one decision had
turned into a contradiction of the paragraph under it.

### 4. What looked removable and was kept

The more important half, because the next compaction meets the same paragraphs.

- **§4's reference-disagreement table** (vector 0.002–0.047, text worst tile 26–28 at 2.7% of
  pixels). It looks like a gate figure and is a *measurement that sets a bound* — ADR 0005's, and
  the thing that makes "tolerance is a design requirement, not a concession" checkable.
- **§5a's "fourteen of this tree's twenty-five references"**. It is the evidence for rejecting a
  gate that was built — "a gate that is more exceptions than rule is not a gate" — and without the
  ratio the sentence is an opinion.
- **The `§9.3.6 Table 106` incident** and the jbig2dec one in §4. Both are incidents with rules
  attached, which is a trap's shape; only the jbig2dec sentence's "for six sessions" came off.
- **`doc/state-of-play.md`'s "115 beads"** and **"two rounds read 'eleven queries' off it"**. Each
  is the evidence for a rule about *claims* (ADRs 0405, 0603), and ADR 0974's second heading
  exempts a figure that is the evidence for a lesson.
- **`doc/state-of-play.md`'s structure.** ADR 0974 decided this already: it is a capability list
  written as narrative, and turning it into a plain list is a rewrite rather than this round's job.
  What came off is six sentences; the prose stayed.
- **FORCEDENTRY**, which says *which* codecs run confined rather than dating anything.
- **§5a's status vocabulary in full.** Eight statuses, each with the situation it keeps from wearing
  another's word. Not narrative, not derivable, and the argument for having statuses at all.
- **`doc/HANDOVER.md`'s trap index**, for ADR 0974's reason: an index that does not repeat is not
  an index.

### 5. The Rust-comment sweep, and why this round did not run it

**Not touched this round, deliberately.** Five sibling rounds were editing `crates/` in the same
tree; a comment rewrite touches nearly every file of a crate, and a diff that is both a comment
rewrite and somebody else's code change is a diff nobody can review. The rule costs nothing to
state and the sweep costs a round per crate, so they separate cleanly.

**How it should be run:**

- **A `--bin` in `tools/conformance` that lists the sites**, one line each with file, line and the
  phrase matched, and a per-crate total. The existing `retired` sweep is the same machinery pointed
  the other way — it looks for a claim that went stale, this looks for the annotation that says so.
- **Three classes, and only two of them ratchet.** A *session ordinal* ("hundred-and-", "the Nth
  session", "session 535") and a *retired sentence* ("used to say", "this comment said", "this
  sentence", "was wrong", "no longer") are unambiguous and their counts may only fall. An **ADR
  reference does not ratchet and is listed only**, because a regex cannot tell "ADR 0123 argues
  this" — which the rule *requires* — from "the round that added ADR 0123 changed this", and a
  sweep that failed on the first would delete the citations principle 5 asks for.
- **One crate per round, largest first**, so each round is one reviewable diff and each fences
  cleanly against a sibling. `crates/pdf-model` is the largest and is also being split (ADR 1005
  §1), so it goes after that split rather than before it.
- **Calibrated before it is believed** (trap 13): put a known site back and watch the count rise.
- **The rewrite is not deletion.** Where a comment's history carries an argument, the argument moves
  to the ADR it belongs to — or the comment cites the ADR that already has it — before the sentence
  goes. `crates/pdf-model/src/lib.rs:24` is the archetype and should be the sweep's first fixture.

## Consequences

**`doc/PLAN.md` 697 → 609 lines, 7250 → 6566 words.** `doc/state-of-play.md` 555 → 550 lines,
8304 → 8171. `doc/HANDOVER.md` 155 → 157 lines, 2228 → 2243. `doc/crate-map.md` 44 → 54 lines,
6138 → 6280 words of header prose, on top of session 1000's rows (42 → 44 lines, 5741 → 6138).
`CLAUDE.md` 393 → 412 lines, 4116 → 4401, which is the one file here that grew and the only one
that should have.

**Measured in words as well as lines, because ADR 0974 asked the next compaction to.** Its own
finding was that 2527 of 3866 removed words were inside four table cells and no line count could
have seen them. Here the ratio runs the other way —
because what came out was paragraphs rather than cells: `doc/PLAN.md` lost 88 lines and 684 words
and gained prose where a claim had to be stated rather than annotated, which is why the word count
moved less than the line count. Both numbers are reported so that the next round can see which
shape it is dealing with.

**The instruments say nothing got worse**, and the moved prose is why: it keeps its words, so a
pointer inside it stays live and a quotation inside it stays verbatim — ADR 0232's rule about a
move, doing exactly the work it was written for.

`--bin pointers`: 193 absent and 14 undefined symbols before, 221 and 17 after — **and none of the
rise is this round's**. Twenty-eight of the new absent hits name `crates/pdf-model/src/{cms,der,
x509,pkcs1,pss,dsa,ecdsa,eddsa,bigint,signature}.rs`, which session 1000 moved into
`crates/pdf-signature` in the same tree while this ran. Filtering the sweep's output to the six
files this round wrote leaves nothing: the one hit it did produce was a `tools/corpus` path quoted
inside this ADR's own table of retired claims, which is this sweep's documented oldest false
positive, and the sentence was rewritten so as not to spend it.

`--bin quotations`: 3802 verbatim and 49 diverging in documents before, **3808 and 49 after** —
the six added are quotations moved into `doc/history/1003-…` with their words; 1939/5 in ledger
notes before, 1940/5 after. The *unrelated* class rose 5343 → 5391, which is the sweep's name for a
quoted fragment that is not a quotation of the standard: the record quotes this project's own
retired sentences by the dozen, which is what a record of retired sentences is.

**What this costs.** The sweeps that find a retired claim by its phrasing will find fewer, because
the phrasing goes. That is the intended effect rather than a loss of coverage: the claims move to
`doc/history/`, which no round reads to do its work. And the four documents can now drift silently
in a way an appended correction could not — a sentence that says "this used to say X" is at least
evidence somebody checked. The answer is the same one ADR 0281 gave: a claim a command can check
should be a command, and the rewrite pushed nine of them onto `tools/state.sh`.

**The risk this round takes on** is the one every compaction here takes: a classification is a
judgement and four headings make it look mechanical. The mitigation is unchanged — every removal is
named under its heading above, the prose itself is in `doc/history/1003-…`, and the seven things
that looked removable and were kept are written down rather than left as absence.

**One pointer moved and is recorded rather than repaired.** `doc/questions/Q25` cites this file's
"Phase 4 — Test layers" heading by name; that heading is now "The test layers", in the same section
with the same material. Q25 and A25 are records and are not edited to follow a file that moved
underneath them (ADR 0232 §2).
