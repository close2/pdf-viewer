# 57 — The transform suite: what RFC 0002 still owes after its first landing

Status: **open**, on the long-lived branch the transform rounds share (`round-867` onward).
Priority: 50-band — the largest items are **blocked on the owner's answers** to RFC 0002 §13,
and the unblocked ones are listed first.
Corpus witnesses: none yet — the suite's own gate (RFC §9) is item 2 below.
Clauses: §7.5.4, §7.5.5, §7.5.7, §7.5.8 and §14.4 on the way out (the serializer, item 3);
§8.9.7 inline images and §12.5.6.15 file-attachment annotations (item 4); §7.6.4.2 Table 22
(item 6).
Code: `crates/pdf-transform/`, ADR 0800.

## What is done

RFC 0002 §14's first landing: the seam, the range grammar, the name patterns, the report, the
exit statuses, and `render`, `images`, `attachments` (read) — ADR 0800, session 867. Everything
below is what the RFC proposed and that round did not take, in the order the next round should.

## 1. The CPU-time gap in parallel `render` — unblocked, and first

Session 867's baseline (`doc/history/867-*.md` §3): 24 threads render 200 pages of the standard
in 1.0 s wall at 18.9 s CPU, one thread in 8.1 s at 7.9 s CPU — an 8× wall gain at 2.4× the CPU.
Two suspects, to be measured before either is chosen (principle 2): each rayon worker holds its
own `FontCache` and re-parses fonts one shared cache would parse once (`FontCache` is behind a
`Mutex` nothing in the workspace contends for from two threads; a transform is the first thing
that would), and `interpret` already bands §8.9.5's colour conversion across the global pool, so
the outer parallelism contends with the inner. `crates/pdf-model/examples/parallel_sweep.rs` and
ADR 0260 are the instrument and the prior reading.

## 2. A transform gate, with the perf floor RFC §12 asks for — unblocked

There is no gate over the verbs yet, so §12's "transform gates carry perf floors from their
first landing" is owed by the round that creates one. Shape: the pdf.js corpus through `render`
at a modest dpi and `images --list`, holding exit statuses and — for `render` — that every page
is the oracle backend's raster (RFC §9 layer 3, which `tests/verbs.rs` holds for one page). The
floor starts from §3's baseline. It needs the sandbox worker beside the gate binary (trap 10) and
a line in `doc/todo/02` §2 with `tools/conformance/tests/sandbox_gates.rs` satisfied.

## 3. The serializer and the writing verbs — blocked on RFC §13 question 1

`split`, `merge`, `pages`, `optimize` and `attachments --attach` all need RFC §10's
structure-preserving serializer in `pdf-syntax`, and that needs the owner to ratify RFC §11.1's
redrawn authoring exclusion in `CLAUDE.md`. Nothing here starts before that sentence.
`attachments --attach` is the one verb §7.5.6's incremental writer could serve today, and would
be the smallest first consumer of any writer.

## 4. `images` and `attachments`, the halves not taken

- **Inline images** (§8.9.7): a content-stream construct the interpreter already parses; `images`
  enumerates only `XObject`s. One interpreter touch.
- **`--native`** pass-through of DCT and JPX bytes as `.jpg` / `.jp2` (RFC §6.3), with JBIG2 and
  CCITT saying so per image rather than inventing sidecar formats.
- **`--no-mask`**, keeping a soft mask as `img-%d.mask.png` beside the image.
- **File-attachment annotations** (§12.5.6.15) as the third home of an embedded file;
  `attachments` reads the name tree and the catalog's `/AF` only.

## 5. `render`'s two absent flags

`--page-box` (§7.7.3's boxes, crop by default) and `--no-annotations` (§6.3.2.2's obligation,
opted out of) are RFC §6.4's and were not taken because `interpret` offers no knob for either;
adding one is a first-row change with the whole gate sequence behind it. `--format pgm` waits on
a stated grey conversion; JPEG output on §13 question 2.

## 6. Two things in the wrong crate, deferred for the same reason

- `pdf_transform::Operation` — `Print` and `Extract` over Table 22's bits 3 and 5 — belongs in
  `pdf_model::restriction::Operation` beside `FillInForm` and `Annotate`, so that one module
  reads all six restriction sources for every operation this tree performs. First-row change.
- `--password-prompt`: an interactive prompt that suppresses echo needs a terminal-mode
  dependency (`doc/stack.md` decides), or a host that owns a terminal. `--password-fd` is the
  scripted route and is what exists.

## 7. The confinement tranche — RFC §13 question 3, defaulted to in-process

ADR 0800 §6 states the cost. The worker split is a transport change on the `pdf-view-worker`
pattern — plan in, report out, sources and sinks as descriptors the broker opened — and the seam
was written so that it is one; `viewer-confined` is the precedent. Taken when the verbs settle,
or earlier if the owner requires it before the first release.
