# 669 — A price for two cases

Eleventh merge round, four branches, one conflict — both sides additions to §10.7.4's note, spliced
so that both survive. And the third batch running in which **a price recorded by an earlier round
turned out to be wrong**, this time in a shape the habit did not yet name.

## The sequence, whole, on a quiet machine

`fmt`, `clippy --workspace --all-targets` under `-D warnings`, the fuzz check — all silent ·
`nextest` **2426 passed, 17 skipped** · doctests, conformance (**171** + 5 + 1) · corpus **974
documents, 68 incomplete** · oracle **908 agrees, 65 contradicted, 786 ambiguous** ·
`render-quorra` **933 agree, 22 differ** · `fixed_documents` **40 checked, 0 absent** · text, both
censuses, dates, XMP, JPEG 2000 · `cargo deny` all four ok. Ledger unchanged at 875 rows, 224
`partial`, no `silent` row.

## 666: a price for two cases

`doc/todo/11` item 4 priced its last bullet at "a shape channel beside a group's raster, which
nothing in this tree carries", and left it for nineteen sessions on that. **It was one price quoted
for two cases.** §11.3.7.1 makes alpha shape × opacity and §11.6.4.2 gives every elementary object
opacity 1.0, so **the two results are one number exactly where the group's opacity is 1.0
throughout** — decidable while the content stream runs. That case costs one boolean and one linear
pass over the band. The shape channel is the price of the *other* case only.

The layers held more than the item assumed, as they did in 646 and 660: `pdf-model` has built a
group's shape since ADR 0234, `Command::Shaped` carries one, `render-cpu` draws one, and
`scan::intersected` already composes `min(M·S, P)`.

The clause is Table 139's two named results — a computed shape "used as the object shape when the
group is treated as an object" and a computed alpha, which Table 140 accumulates apart — with §8.5.4
constraining the *first* by the clip at the blit and §11.4.8's summary putting the clip inside `𝑓𝑗`.
This tree multiplied the clip into the finished **alpha**: the wrong bracket, so a group whose
`/BBox` is its content's rectangle painted its boundary at the **square** of that pixel's coverage.

All eight rungs of 662's probe now agree. The corpus witness's two boundary rows go 0.2549/0.2079 →
**0.5059/0.4549**, and `issue21346.pdf`'s edge `0.827^4.0` → `0.827^1.9`. **Five oracle lines moved,
every one toward the references, no verdict changed; the cross-backend gate cost zero pages.**

So `doc/habits.md` gains the third shape: ADR 0474's price was too high because a library held the
pieces, ADR 0469's named the wrong *place*, and this one named no condition — **a price that names no
condition is a price for the hardest case, charged to every case.**

## 665: the paragraph a correction said it had not touched

The instrument this batch was sent to build. It measured all four candidates first and **two of the
briefing's premises were false**: `--bin pointers` already reads `oracle.rs` and reports nothing
there, and the claim that notes quote figures the oracle recomputes does not hold (`ssim` appears 12
times in 7523 lines). A fifth was prototyped and abandoned for the right reason — *the same
measurement stated twice with two values* finds the live defect but is **silent on the plant**, since
restoring the stale sentence makes the tree agree with itself.

What it chose is better than any of them: **an ADR number is a date**, the only machine-readable one
this tree keeps, so a note's newest cited ADR against the newest ADR naming one of its own pages says
whether the note has read the decisions taken about its pages. `--bin overtaken`, a fraction of a
second, 123 notes against 490 ADRs.

Its finding is almost too apt. `CONTRADICTED_ANTIALIASED_EDGES` still carried the pre-ADR-0476
quantised figures — and sits **directly below the ADR 0476 correction, which ends "the paragraph
below is unaffected — which it predicted."** A correction that scopes itself is a claim, and that one
was false about the only sentence in scope. 48 of 123 notes are overtaken; three acted on, 45 ranked.

## 668: shared data that is on no disk

The fifth criterion, and it audits what nobody had: a contradicted verdict claims not only that the
standard decides the page but — ADR 0005's premise — that **the agreement outvoting us is evidence**.
So: does the note name a mechanism for that agreement, and is it *verified*? Ten of fourteen groups
verify; three infer from the picture; one names none and wrote that the pair "happen to agree".

**Trap 9's eighth mechanism**: `ghostscript` carries `gsicc_create_from_cal`, `mupdf` exports
`fz_new_icc_data_from_cal` and 437 Little CMS symbols — the voting pair each **synthesise** an ICC
profile from Table 63 and run it through the same CMM. `objdump`, digest comparison and the `desc`
tag all return empty, because the file exists nowhere. Proved by construction: `gs -sDEVICE=pdfwrite`
writes the synthesised profile out as a 585-byte stream whose colorants are *the diagonal of an
adaptation that is not diagonal*, 4.4% off its own white point — and **this tree rendering that file
reproduces ghostscript's rendering of the dictionary to 0.07 of 255.**

Against the closed form, from published constants and none of this crate's code: **`poppler` 0.013,
ours 0.025**, against `hayro` 2.15, `ghostscript` 4.30, `mupdf` 4.84.

## 667: a sentence that was true about its instrument

**"No document in *any population this project measures* states `/Cap true`"** was true — of
`witness_census`, whose scope was hard-coded to the curated corpora. The crawl holds two. That is one
step worse than 655's decay, because the sentence *names* its own limitation and still misleads. Same
shape again at "0 of 73 free text annotations state a `/CL`", which is **33 of 1724**; and §12.5.3's
claim rested on "a scan of every **uncompressed** `/F`", blind to object streams — counted properly,
`ToggleNoView` is set by **none of 806 668 annotations in 66 829 documents**.

Two refusals got sizes for the first time: `/SA` stated by **19 211 of 65 703**, and §11.5.3's report
firing on **0 of 1126 curated and 0 of 65 703 crawled** against 21 834 `/DeviceCMYK` mask groups.

## One tension for a later round

665 concluded there is little in a note to anchor a figure to (12 `ssim` mentions in 7523 lines);
668 found **two more stale numbers by hand** in the note it took. Both are probably right — few
figures, and those few decay — but whether that is a gate is unsettled, and it is the one place this
batch left an instrument question open.

## Owed

- Item 4's remainder: a stroke's coverage, an image's edge, a **non-isolated** group's raster, and a
  group whose opacity is below 1.0 somewhere — where the shape channel genuinely is the price, and
  where no corpus document has been shown to need it. Plus `doc/QUORRA_FEEDBACK.md` §36, an ask for a
  flag and a `min` rather than a channel.
- **`doc/md/` breaks "backdrop" across a space in §8.5.4**, so the group sentence cannot be quoted
  verbatim past a point; both quoting sites stop there and say why.
- The owner's session for `tmp/pi.pdf`, and a push — the CI fix (`7d8695af`) has not faced a run.
