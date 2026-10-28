# 2498 — Both shipped profiles discard `image-smoothing`, and the verb takes "2b, or 4 where 2b refuses" as one request

Status: implemented, in `pdf-transform` (`archive::smoothing`, `archive::fallback`, `archive::prepare`,
`archive::report`, `bin/quorra-transform`) and in `doc/profiles/as-if-printed.toml` and
`doc/profiles/as-shown.toml`. Session 1832, the archive slot of batch one hundred and twenty-four.
Context: ADR 2490's table (R02 and R06b), `doc/questions/A681` item 1, ADRs 0816 (the converter edits no
producer's content stream), 1014 (the near side of the authoring line) and 1964 (a removal whose record
is a condition of it); the census is ADR 2499.

## R02: `image-smoothing` is `discard` under both profiles

The entry `[site."graphics/image-interpolation-is-off"] remedy = "discard"` is in both shipped profiles.
The rewrite already existed (`Rewrite::InterpolationOff`, written by `rewrite/streams.rs`); what changes
is that the profiles authorise its loss, so a document whose image dictionary states `/Interpolate true`
converts with the entry written false instead of being refused. ISO 32000-2 §8.9.5.3 says of the entry:

> However, this is only a hint, and a PDF processor may ignore it.

so a printed page, made by a device that sampled every image as it came, carried none of it, and the
loss is named for `as-if-printed` all the same. Under `as-shown` the loss is real, because this reader
honours the hint: a magnified low-resolution image is drawn smoothed in the source and from the nearest
sample in the output. ISO 19005-2 section 6.2.8.1 and ISO 19005-4 section 6.2.7.1 require the entry to
be false where an image states it.

**The premise that did not hold.** ADR 2490 says every loss named in the report is also in the file's
`xmpMM:History`. For this loss it was not: the report named `image-smoothing` and no history entry
existed. `archive/smoothing.rs` writes one (action `converted`, parameters naming the requirement's
citation, the count of images and the first twenty by object number, the rest counted), and the entry is a
condition of the rewrite in the way ADR 1964 makes a transfer function's: a packet that will not take it
withdraws the rewrite (`Prepared::smoothing_recorded`), because the entry is how the file itself says that
smoothing was turned off. The first build left it unconditional; a fixture whose packet already identified
the file wrote no packet at all and the entry was dropped without a word, which is why
`Rewriter::wants_metadata` now includes the rewrite. A conversion that records an action reads the
clock once (ADR 2306), so a smoothed image makes the output a function of the plan's instant as well;
tests state one.

**The inline form stays refused.** An inline image's `/I` sits inside a content stream, which this
converter carries byte for byte (ADR 0816), so `graphics/inline-image-interpolation-is-off` keeps its
`the-fence` refusal and its own sentence; the test plants it under each profile at each part and
asserts that it stays. Two documents of the two corpora reach it (`TAMReview.pdf` and `bug1065245.pdf`,
under every profile and part, ADR 2499).

## R06b: `--or-else-to`

`quorra-transform archive in.pdf --to 2b --or-else-to 4 -o out.pdf` is one request. The verb converts to
the first target and, only where that conversion wrote no file, converts to the second, so a document that
can be the part the person prefers is never made the other. Decisions:

- **The fallback is the face's, not the plan's.** `ArchivePlan` names one target and `apply` is a pure
  function of one plan; widening the plan to a list would have changed every one of its call sites in
  `crates/` and `raster/`, and the library's meaning (a refusal is a report, not a retry). The binary's
  `convert` is the old pass loop extracted, and `fall_back` runs it a second time over the same inputs,
  policy and budget, reading the configuration again for the second target because a profile's answers
  can differ by part.
- **The file says which part it is the way every output does**, in `pdfaid:part`; no new field is added to
  the output. The report says it in words (its first line, "PDF/A-2b was asked for first and refused ...")
  and in JSON as `refused_first`: `{requested, refused}`, `null` when the first target wrote the file
  (`archive::Fallback`, set by the face because the library never sees a second attempt).
- **Exit status.** A second target that writes the file exits as that conversion does; the first
  conversion's refusals live in the record. Both refusing exits 4, as one refusal does, and lists each
  target's refusals led by its target (`at PDF/A-2b: ...`, `at PDF/A-4: ...`). A source the verb declines
  (no report) does not fall back.
- **Usage errors.** `--or-else-to` belongs to `archive`; the second target must be one of the six and
  differ from the first; and a configuration that declares a tool is refused, because the confined broker
  fills a tool's `{target}` from the request's first target alone (`route.rs`'s `tools()` reads `--to`),
  so a program declared under either part would be handed the wrong part in the other.

## What remains

The owner's stance of 2026-10-27, applied to this slot: for any refusal left on a document the viewer
draws, name the elegant remedy and say that, failing it, the loss can be stated on an appended page
under the profile (ADR 1014).

- **The inline `/I`.** The elegant remedy is to write the key false inside the content stream, which
  ADR 0816 keeps from this converter; the entry being a hint, the rewrite would change nothing but the
  smoothing, and relaxing ADR 0816's rule for it is the owner's.
  Failing it, an appended page stating the loss would not make the file conform, since an inline `/I true`
  still fails ISO 19005-2 section 6.2.8.1 and ISO 19005-4 section 6.2.7.1; under ADR 0816 the conforming
  fallbacks are ADR 2394's photographed page (R03, allowed by `doc/questions/A694` item 1) or the owner
  relaxing ADR 0816. None is built here.
- **A document both parts refuse.** `--or-else-to` removes the refusals that are about the part
  (among them `file-structure/file-header`, `implementation-limits/*`,
  `fonts/symbolic-truetype-program-has-a-usable-cmap`, `metadata/extension-schemas-embedded` and
  `graphics/rendering-intent-entries-name-one-of-four`); what both refuse is another remedy's
  (R03, R04, R01 and the rest of ADR 2490's order), each with the same two answers when it is built.
- **The windows' archive command** (`viewer_host::archiving`) does not offer a second target.
