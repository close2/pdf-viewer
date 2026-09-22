# 1214 — The population of §11.4.4's second element run, counted on the interpreter's condition

Status: accepted and **measured**. Session 1188.
Context: `crates/pdf-model/examples/non_isolated_group_census.rs`, `doc/conformance/ledger.toml`
§11.4.4, `doc/verify.md`, `doc/todo/65`.
Amends: `doc/adr/1107` section "What it cost, and what it did not move", whose corpus count is
restated here on a condition the tree has since changed under it. ADR 1107's decision and its
arithmetic stand untouched.
Builds on: `doc/adr/1170` (the two groups §11.7.4 builds), trap 13 (calibrate a census against the
defect), trap 11 (name the population a census is about).
Clauses: ISO 32000-2 §11.4.4, §11.4.5, §11.4.6, §11.3.5, §11.7.4.3, §11.7.4.4.

## 1. The premise that expired

ADR 1107 recorded that nothing in the tracked corpus states §11.4.4's result step — "0 of 1477,
counted by a probe on the interpreter's own condition" — and 440 groups over 89 286 crawled first
pages. That probe asked which **files** state a non-isolated group under a blend mode.

ADR 1170 then had the *interpreter* state one. §11.7.4.3's last paragraph wraps an object painted
while overprinting is enabled under a non-Normal mode in a non-isolated, non-knockout group, and
§11.7.4.4's first bullet builds another; `content::overprint::non_isolated_group` emits exactly the
command `CpuRasterizer::remove_the_backdrop` fires on. A count taken from a file's dictionaries
cannot see those, so the recorded number had stopped being a count of what the rasteriser pays for.

## 2. The condition is the rasteriser's, and the census reads commands

The predicate is the conjunction `CpuRasterizer::group_buffer` tests before it calls for the second
run: no blending colour space of its own, `isolated` false, `knockout` false, and a blend mode at
the `Do` that is not Normal. It is read off the display list `pdf_model::interpret` produced —
every command list a page holds, which is §11.4.7's page pair, every nested group, every
`Command::Shaped`'s object and every §11.6.5.1 soft-mask group.

That is trap 13's second shape stated positively: a census derived from the *clause* counts a
different population from one derived from the *program*, and the number the ledger carries is
about what this tree draws. The two exclusions the conjunction makes — a non-isolated knockout
group, which takes ADR 0327's construction, and a non-isolated group under Normal, where NOTE 3's
cancellation collapses the two steps — are printed beside the count, because a predicate's
exclusions are part of it.

**The pre-filter cannot lose a page.** §11.3.5 makes the current blend mode the `/BM` entry of a
graphics state parameter dictionary, so a document no object of which carries that key reaches no
non-Normal mode on any page — whether the group is the file's or one the interpreter built, since
both take the mode from the same parameter. The filter asks for the key and not its value, and it
searches nested dictionaries as well as each object's own top level: reading only the top level
excluded the planted fixture, whose `ExtGState` is direct inside a `/Resources`, which is the false
zero the calibration caught before the run.

## 3. The numbers

Curated — `doc/pdf.js/test/pdfs` and `doc/corpora`, 1477 documents, 1450 opened, first page each:

| | |
|---|---|
| groups stating §11.4.4's result step | **2**, on **1** page of **1** document |
| built by §11.7.4's implicit construction | 2 |
| stated by a file's own `/Group` | 0 |
| holding an element that blends | 2, both under `/Multiply` |

The page is `doc/pdf.js/test/pdfs/issue12798_page1_reduced.pdf`, page 1, and it is the first corpus
page that reaches the construction. The file-stated population alone is **0 of 1477**, which is
ADR 1107's number and is why that ADR was right when it was written.

Crawled — 89 286 documents, 88 890 opened, 33 995 carrying a `/BM` and therefore interpreted:
**1233** groups on **205** first pages of 205 documents, 781 of them §11.7.4's construction and 452
a file's own, 1072 under `/Multiply`. Excluded by the conjunction: 225 non-isolated knockout
groups and 7869 non-isolated groups under Normal. The blending-space self-check is **0**, which is
`pdf-model`'s own guarantee that `blending` is emitted for an isolated group alone.

## 4. Calibrated, and on both halves

Trap 13, and the plant is two fixtures rather than one because the population is now two
constructions. A page whose form XObject states `/Group << /S /Transparency >>` and is drawn under
`/BM /Multiply` with a blending element — ADR 1107's own fixture, written out as a file — is
named. A page whose
group is `/DeviceCMYK`, whose content sets `/OP`, `/op` and `/OPM 1` and paints a zero-tint `k`
colour under `/Multiply` is named twice, once per half of §11.4.7's page pair. Two controls are
silent: the same form with `/I true`, and the same form under Normal — the second producing no
group command at all, because `pdf-model` emits NOTE 3's collapsed elements inline.

## 5. What this does not do

**It does not re-price the second run.** ADR 1107 priced it on the fixture; what a corpus page
costs is a measurement on the page named above, and it is a round's rather than a census's. The
ledger row keeps its status.
