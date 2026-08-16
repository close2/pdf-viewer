# hayro's issue tracker, read once, sorted

Status: **record** — what was read, and what each issue turned out to be for *this* tree.
Read by: a round that is about to look at hayro's tracker, so that it does not read it again.
Written in the five-hundred-and-fifty-seventh session, over all 167 issues open and closed on
`LaurenzV/hayro` as of that date. ADR 0392 has the round's decisions.

**Why this is a file of its own rather than a section of `doc/oracle-and-corpus.md`.** That
document is about the *instrument* — which references vote, what a tolerance admits, why an
agreement is or is not evidence. This is a reading of somebody else's defect list, which is a
different kind of thing: it decays as they fix things, it is a one-time sweep rather than a
standing method, and its main job is to stop a later round spending an afternoon re-reading 167
issues. Putting it in `oracle-and-corpus.md` would mix a durable method with a dated survey.
`doc/JPEG2000_FEEDBACK.md` is the precedent — a record of one conversation with one upstream,
kept beside the method rather than inside it.

**The rule this whole file is written under is `CLAUDE.md` principle 5.** An issue on hayro's
tracker is a fact about hayro. It is never a statement about what is correct. Every entry below
that says "this tree is right" says so because of a clause, and names it.

Bucket 3 — the issues that are quorra's business rather than ours — is
[`doc/HAYRO_ISSUES_FOR_QUORRA.md`](HAYRO_ISSUES_FOR_QUORRA.md), which is written to be handed
over. This file holds the other three.

Counts, and they are the round's own: **17** in bucket 1, **36** in bucket 2, **37** in bucket 3,
**77** in bucket 4; 167 in all, each in exactly one bucket.

---

## Bucket 1 — a question about a clause, put to this tree

Seventeen. hayro and this tree are independent implementations of the same standard, so an issue
about a clause is a question to ask here. **One found a defect** (#1337) and it is fixed in this
round. The rest were already right, and each is now pinned by a test naming the issue it guards
against — because "we checked and we were fine" is worth nothing to the round after next unless
something fails when it stops being true.

| issue | clause | what this tree does |
|---|---|---|
| [#1337](https://github.com/LaurenzV/hayro/issues/1337) CCITT `/Rows` below `/Height` | §7.4.6 Table 11 | **was wrong; fixed.** See below. |
| [#994](https://github.com/LaurenzV/hayro/issues/994) `5f` lexed as `5` + `f` | §7.2.3 | right about the ink, silent about the token. See below. |
| [#1341](https://github.com/LaurenzV/hayro/issues/1341) mantissa wraps past 19 digits | §7.3.3 | right: the fast path refuses past 15 digits and `str::parse` answers. |
| [#1336](https://github.com/LaurenzV/hayro/issues/1336) bowtie taken for a rectangle | §8.5.3.3 | right: the rectangle test is per-*edge*, so a diagonal rejects it. |
| [#1273](https://github.com/LaurenzV/hayro/issues/1273) `/Length` slices past an MD5 digest | §7.6.3.2 | right: `key_length` refuses outside Table 20's range before the slice. |
| [#1347](https://github.com/LaurenzV/hayro/issues/1347) stack overflow on nested arrays | §7.3.6 | right: `max_depth` 256, a typed `LimitExceeded`, plus a fuzz target. |
| [#1189](https://github.com/LaurenzV/hayro/issues/1189) `/Contents 8`, a bare integer | §7.3.10 | right: the parser rewinds; the page reports `NotAStream`. |
| [#11](https://github.com/LaurenzV/hayro/issues/11) `/Identity#2DH` not decoded | §7.3.5, §9.7.5.2 | right: one lexer, escapes resolved before any lookup. |
| [#1334](https://github.com/LaurenzV/hayro/issues/1334) Type 1 built-in encoding lost | §9.6.5.2 | right: an embedded program's own encoding *is* the base. |
| [#1331](https://github.com/LaurenzV/hayro/issues/1331) Type 3 without `/ToUnicode` | §9.10.2, §9.6.4 | **had the same defect, fixed in session 326.** See below. |
| [#494](https://github.com/LaurenzV/hayro/issues/494) JPEG frame ≠ dictionary size | §7.4.8, §8.9.5.1 | right: the codestream's grid on the unit square, and reported. |
| [#4](https://github.com/LaurenzV/hayro/issues/4) `/All` and `/None` colourants | §8.6.6.4 | right, and decided before the alternate space is read. |
| [#404](https://github.com/LaurenzV/hayro/issues/404) huge `/XStep` allocates a pixmap | §8.7.3.3 | not our failure mode: tiles are geometry, and a huge step means *fewer*. |
| [#141](https://github.com/LaurenzV/hayro/issues/141) optional content groups ignored | §8.11 | implemented, `/VE` included — the ledger's §8.11 rows, and trap 9. |
| [#1259](https://github.com/LaurenzV/hayro/issues/1259), [#1260](https://github.com/LaurenzV/hayro/issues/1260) `w × h` allocated before decoding | §7.4.7 | bounded: `pdf_sandbox::MAX_PIXELS`, checked before the buffer exists. |
| [#1258](https://github.com/LaurenzV/hayro/issues/1258) row buffer indexed out of range | §7.4.6 | cannot arise: `PackedRows` appends and checks its length at the end. |

### The one defect: #1337, `/Rows` is not the row count

`decode_ccitt` passed the `DecodeParms` `/Rows` to the codec whenever it was non-zero. Table 11
does not let it: `/EndOfBlock` is "[a] flag indicating whether the filter shall expect the encoded
data to be terminated by an end-of-block pattern, **overriding the Rows parameter**", and only
"[i]f false" does the filter "stop when it has decoded the number of lines indicated by Rows".
`/EndOfBlock` defaults to true, so in the ordinary file `/Rows` does not bind at all and the
decode is bounded by §8.9.5.1's `/Height`.

Producers get `/Rows` wrong — hayro's witness is a CAD driver stamping `/Columns` onto a 523-line
logo — and honouring it truncates the raster. hayro paints the undelivered rows as sample 0,
which under a dark `/Indexed` index 0 is a black block; this tree refused the image on its height
check and reported. Both are ways of not drawing the page. `pdf_model::ccitt_rows` now derives the
count from the two entries together, with the seven cases of Table 11 as its unit test.

**What is still refused**, honestly: `/EndOfBlock` false with `/Rows` below `/Height`. There the
clause *does* bind the filter to `/Rows`, the raster is genuinely short, and padding it to the
image needs a decode bound and an image height to travel separately over `pdf-sandbox`'s pipe,
which carries one number. Unwitnessed by the corpus. [`doc/todo/53`](todo/53-what-hayros-tracker-asked.md).

A second thing came off it: `pad_to_height`'s doc comment and §7.4.6's ledger note both quoted
"whichever occurs first" as the rule, which is the *conditional* half of the `/EndOfBlock` row
with its "If false" dropped. Both now quote the `/Rows` row, which says the same thing
unconditionally.

### #994, where we are right about the page and wrong about the report

`5f` is one token under §7.2.3 — `f` is a regular character and a token ends only at a delimiter
or white space — and it spells no number and no operator, so nothing is painted. hayro split the
run and filled the rectangle. This tree paints nothing, which is the clause's answer; but it gets
there by salvaging the run to the number 5 and dropping the letters, where §7.8.2 would have an
unrecognised operator reported. The leniency is ADR 0303's, scoped deliberately to digit-*less*
runs because the same code reads `12pt` as 12 for the streams that need it. Not changed here.
[`doc/todo/53`](todo/53-what-hayros-tracker-asked.md).

### #1331, the same defect, found here 231 sessions earlier

Their report: for a Type 3 font `Glyph::as_unicode()` consults only `/ToUnicode`, where a simple
outline font also falls back to the encoding's glyph name. This tree said exactly that — `type3.rs`
held that a Type 3 glyph name "names a procedure, so … the name is no evidence at all about the
character" — for three hundred sessions, and it was wrong for a reason the standard states:
§9.6.4's step b) is "[g]et the glyph name from the Encoding entry" and §9.6.5.3 makes
`/Differences` "the complete character encoding for this font", so the glyph selection algorithm
does use a name and §9.10.2's second method applies. Corrected in session 326; the readback went
98.2% to 99.1% of `pdftotext`'s words. §9.10.2's ledger row has it.

Worth recording as the shape rather than the fact: **two independent implementations reached the
same wrong conclusion from the same plausible argument**, and the standard settles it in two
sentences neither had read. That is the strongest single argument for principle 5 this round found.

### A smaller defect found on the way, not fixed

Following #1334 into `pdf-font` turned up something adjacent. `read-fonts` pre-fills a Type 1
program's custom encoding table with `GlyphId::NOTDEF`, so `type1.rs` records `Some(0)` for every
code the array's length covers but does not assign — where the CFF sibling records `None`. The two
producers of `NameKeyed` therefore disagree about what "unencoded" means, which reaches the
whitespace departure in `loading.rs` and the "no code maps to a glyph" refusal.
[`doc/todo/53`](todo/53-what-hayros-tracker-asked.md) has it with the file and line.

---

## Bucket 2 — the three codec crates this tree links

Thirty-six issues touch `hayro-jbig2`, `hayro-jpeg2000` or `hayro-ccitt`. This tree ships all
three; `hayro` itself is linked only by `tools/hayro-compare` and reaches no shipped binary.

**The containment answer applies to every panic in this bucket and is worth stating once.** All
three codecs are reached from exactly one place — `pdf_sandbox::decode` — and `Isolation` defaults
to `Sandboxed`, so a decode runs in a confined worker process. A panic there is a dead worker and
a reported refusal, not a dead viewer. That is the argument ADR 0014 made when these dependencies
were taken, and it is what makes a fuzzer's panic in a codec a quality problem rather than a
security one here.

### What the pinned versions actually carry

| crate | this tree pins | published | fixes landed after it |
|---|---|---|---|
| `hayro-ccitt` | 0.3.0 (2026-03-15) | newest | **none** — only two refactors (`#1304`, `#1306`) |
| `hayro-jbig2` | 0.3.0 (2026-04-12) | newest | **four**, see below |
| `hayro-jpeg2000` | `close2/hayro` `1dc833f7` | 0.4.0 lags | pin is *ahead* of the release |

**`hayro-jbig2` 0.3.0 is the newest published version and it predates four commits.**
`69c9a37d` "Avoid overflow when decoding a zero-width bitmap" (#1262, the fix for
[#1261](https://github.com/LaurenzV/hayro/issues/1261)) adds a two-line guard to
`decode_bitmap_arithmetic_coding` that 0.3.0 does not have; `c88d984e` and `c3b06228` are the
generic-decoding fast paths; `1be7ab10` bounds the symbol-instance count (#1278), which is the
allocation shape of [#1259](https://github.com/LaurenzV/hayro/issues/1259)/#1260.

**Does #1261 bite our pin? Measured, and the answer is no.** The regression file the fix commit
added upstream (`hayro-tests/pdfs/load/issue1321.pdf`) was fetched and its JBIG2 segment run
through `pdf_sandbox::decode` on a debug build, where overflow checks are on: it comes back
`JBIG2: unexpected end of input` — a clean typed refusal. The reason is worth keeping, because it
is not the reason one would guess: the overflow was reachable in `5b3cbae` because of the June
fast-path rewrite, and 0.3.0 predates *that* as well as its fix. **Our version is older than the
defect.** The practical consequence is that there is nothing to take: taking the fix means taking
the rewrite, and no release carries either.

**`hayro-jpeg2000`: the pin is now one commit from being retirable.** `Cargo.toml` says to go back
to crates.io "the moment a release carries both" fixes. As of this round both are on hayro's
`main` — `9cce046b` (their [#1283](https://github.com/LaurenzV/hayro/issues/1283)/#1284, the
truncated-code-block mid-point, reported independently by `ruffsl` for DICOM lossless data) and
`49037586` (our own PR #1340, merged 2026-08-16, the fully-decoded case from
`doc/JPEG2000_FEEDBACK.md` §8). **What is *not* upstream is the fork's third fix** — the
reduced-resolution allocation of `1dc833f7`, `doc/JPEG2000_FEEDBACK.md` §10 — and no pull request
for it exists. So the pin stays, and the un-pin condition has three parts rather than two.

**And a fourth part, found here.** [#1188](https://github.com/LaurenzV/hayro/issues/1188)'s finding
C-27 is real: `hayro-jpeg2000/src/lib.rs` reads `let rb = lab.ra.unwrap_or(200)` where `lab.rb` is
meant, so a JPEG 2000 image with explicit CIE Lab range parameters converts wrongly. It is present
in **both published versions** — 0.3.5 and 0.4.0 — and fixed on `main`, which our fork rev carries.
So going back to crates.io 0.4.0 today would *regain* it. Two consequences: the un-pin condition
must include this fix, and **the oracle's fourth reading has it right now**, because `hayro` 0.7.1
reaches `hayro-jpeg2000` 0.3.5 through `hayro-syntax`. A `pdfref-hayro` disagreement on a Lab
JPEG 2000 plate is explained.

### The rest of the bucket, in shape

- **Sixteen JPEG 2000 fuzz panics** (#472, #506, #507, #513, #514, #515, #520, #563, #564, #577,
  #578, #579, #585, #645, #684, #705), all filed and closed between 2025-11-07 and 2025-12-15,
  all long before the fork revision this tree pins. None bite us.
- **Nine CCITT panics** (#52, #156, #676, #677, #678, #679, #681, #683, #1258). Only #676 is in
  `hayro-ccitt` itself; the rest are in `hayro-syntax`'s own filter wrapper, which this tree does
  not use — `pdf-sandbox` has its own. #676 closed 2025-12-13, well inside 0.3.0.
- **#202** JBIG2 index out of bounds, closed 2026-01-25, inside 0.3.0.
- **#993** "possible OOB / underflow / arithmetic overflows", an LLM-generated list. The
  maintainer worked through the one concrete example and concluded it is unreachable
  (`total_symbols` is zero only when there are no new symbols, so the path is not entered).
  Recorded because a *plausible* finding that turns out unreachable is worth not re-deriving.
- **Not defects, kept in this bucket because they are about crates we ship**: #7 (rewriting the
  decoders idiomatically — the CCITT one is done, JBIG2 is not), #13 (how JPEG 2000 came to be a
  pure-Rust port at all), #871 (exposing `decode_into` for buffer reuse — of interest if a round
  ever wants to decode into the worker's own buffer), #924 (no HTJ2K; not used in PDF, so nothing
  is owed under §7.4.9), #1036 (packaging: `hayro-jpeg2000` now excludes test assets, and its
  embedded ICC profiles are CC0-1.0 — relevant to `/NOTICE` if that crate is ever vendored),
  #1041 (an OpenJPEG conformance codestream, `p0_15.j2k`, still unsupported: a marker we would
  refuse too), #1054 (a `moxcms` bump; we build with `default-features = false`, which drops
  `moxcms` entirely), #1055 (an `image` crate API question, not ours).

---

## Bucket 4 — not relevant, seventy-seven of them, in six shapes

Listed by shape rather than one line per issue, because the shapes are the information.

**Forty-two fuzzer panics in hayro's own crates** — `hayro-syntax` (xref, `lzw_flate`,
`ascii_hex`, `object/string`, `function/type0`, `function/type4`, `crypto`, `reader`),
`hayro-interpret` (`context`, `x_object`, `shading`, `color`, `font/*`, `interpret/text`),
`hayro-font` (`type1`), `hayro/src/ctx.rs` and `renderer.rs`, and their dependencies `kurbo`
(`uninitialized subpath (missing MoveTo)`, four times), `zune-jpeg` (`bits_left`, twice, reported
upstream) and `qcms`. This tree shares none of that code. #49, #50, #51, #53, #54, #55, #56, #61,
#62, #67, #68, #83, #152, #153, #154, #157, #178, #180, #182, #203, #204, #206, #207, #208, #222,
#223, #224, #234, #236, #256, #273, #323, #324, #325, #356, #372, #388, #389, #391, #409, #538,
#675, #680, #682, #716, #1051.

Two of them are worth a second's thought even so, and both were checked: the four `kurbo` "missing
`MoveTo`" panics are a path builder receiving a segment before a start point, and this tree's
`PathCommand` sequence is validated where it is built rather than where it is consumed; the
`crypto` one is #1273 and is in bucket 1 because it *is* clause-shaped.

**Nine API, packaging and release questions** — #160 (SPDX vs licence file), #333 (a private
error type in a public signature), #380 (documenting how to run the tests), #407 (reading document
properties), #459 (re-exporting sub-crates), #714 and #600 (SVG background colour and scale),
#788 and #849 (releases and a yanked dependency), #836 (a `fast_image_resize` `no_std` feature
break). hayro's API is not ours.

**Six features this tree already has, asked of hayro** — #10 and #535 (encryption; §7.6 has been
here since the twenty-sixth session), #44 (soft masks, §11.6.5), #141's PDF-export half, #519
(form field appearances, §12.7), #452 and #419 (a text-extraction device and text as paths in SVG;
§9.10.2 and `pdf-retrieve` are the equivalents). Reading them is a reminder of what a renderer
that is younger than this one is missing, and nothing more.

**Four features outside this project's scope** — #103 (an "animation" PDF, which is §13's
multimedia, excluded by `CLAUDE.md`'s closed list), #1270 (DjVu, declined by hayro too), #188 and
#1059 (`hayro-write` and a `Device` callback for optional content groups — their architecture).

**Ten defects with a witness file and no diagnosis** — #5, #9, #42, #175, #411, #508, #629, #1178,
#1184, #1265. A PDF and a screenshot with no analysis carries nothing across an implementation
boundary; the corpus and the oracle are this tree's instrument for that question, not somebody
else's screenshot. #9 was fixed upstream in `zune-image`, #508 was already fixed when reported,
#1184 (two identical functions, one obviously meant to differ) and #1265 (`Debug` impls that never
call `.finish()`) are hayro's own code, and #1178 is a thank-you note.

**Six left over** — #6's font-fallback callback is in the quorra document for its host-boundary
shape rather than here; #815 is a sponsorship question; the remainder are duplicates or
cross-references.

---

## What a later round should do with this

- **Do not re-read the tracker.** Read this file, then read only what is newer than the date at
  the top. `gh issue list --repo LaurenzV/hayro --state all --search "created:>2026-08-16"`.
- **The un-pin condition for `hayro-jpeg2000` now has four parts**, and `Cargo.toml`'s comment
  says two. Three of the four are on `main`; the fourth (`1dc833f7`'s reduced-resolution
  allocation) has no pull request. A round with an hour should offer it, on
  `doc/environment.md`'s standing route.
- **`hayro-jbig2` has four commits and no release.** If a 0.3.1 or 0.4.0 ever appears, take it and
  re-run the JBIG2 gate; until then there is nothing to take and this tree is older than the
  defect rather than exposed to it.
- **Bucket 1's tests are the durable part of this round.** Each names the issue it guards against,
  so the next person to read one knows what it is for.
