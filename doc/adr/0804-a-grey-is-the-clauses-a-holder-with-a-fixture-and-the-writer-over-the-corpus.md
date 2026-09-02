# 0804 — A grey is the clause's, a holder with a fixture, and the writer over the corpus

Session 875. Status: **accepted**. The fifth decision record of RFC 0002's implementation, on
the long-lived branch `round-867`.

## Context

`doc/todo/57`, as session 872 left it, put three things before the serializer that RFC 0002
§13's first question still blocks: `--format pgm`, which "waits on a stated grey conversion";
a fixture for the one holder shape `attachments --attach` rewrites without a witness; and, in
§5, "a corpus-wide attach-and-read-back walk", which RFC §9 calls the writer's equivalent of
the render corpus gate and which was "owed when the serializer lands and worth taking before
it". This round takes the three, in that order, and none of them touches the serializer, the
four verbs on it, or `CLAUDE.md`'s authoring exclusion — all of which stay blocked until the
owner ratifies RFC §11.1 in words.

Three questions, each answered from the standard before any code:

1. What does the standard say an RGB colour's grey is, and what does an image in some other
   colour space become when a grey file is asked for?
2. What does `attach` rewrite when the catalog's `/Names` is an indirect object and the tree
   inside it is direct — and does the corpus have that shape at all?
3. What does a corpus pass over the writer assert, what does it count, and what may it hold?

## Decision

### 1. `--format pgm`: §10.4.2.2's grey, once, of the picture as this tree draws it

ISO 32000-2 §10.4.2.2 defines the conversion outright:

> The gray value for a given RGB value shall be computed according to the NTSC video
> standard, which determines how a colour television signal is rendered on a black-and-white
> television set:

with `gray = 0.3 × red + 0.59 × green + 0.11 × blue` set out under it. **That formula is
already the tree's, in `pdf_render::Color::grey_level`** — the one place the three weights
are stated, which a `/Luminosity` soft mask and `pdf_model`'s ink both take from — and trap 6
is that a second copy of a three-constant formula is how three `DeviceCMYK` conversions came
to disagree once. So `render::grey_of` is a byte in, that function, and a byte out, rounded to
the nearest level; `ImageFormat::Pgm` writes a `P5` header over one such byte a pixel, and the
unit tests pin the clause's own arithmetic — a grey pixel is its own grey for all 256 values,
because the clause's other direction ("[a] gray level shall be equivalent to an RGB value with
all three components the same") composed with this one is the identity when the weights sum
to 1.0, and the green and blue primaries land where `0.59 × 255` and `0.11 × 255` say. Pure
red is `76.5` exactly, a tie floating-point arithmetic settles one way or the other, and is
deliberately not pinned. The integration test writes the formula out in `f64` from the clause
and holds every byte of a rendered page to within half a level of it plus a thousandth: a
correctly rounded byte is within half a level of the exact value, and a conversion that
truncated or weighed the channels otherwise is a whole level out on most of a page.

**What a colour in another space becomes is decided before this clause is reached, and it is
a choice.** The raster handed to the encoder is RGB already — a page through `render-cpu`,
an image through `pdf_model::image::decode` — with every `DeviceCMYK`, `ICCBased`, `Indexed`
or `Separation` colour taken to RGB by the interpreter's own conversion, which §10.4.2.1 ranks
above this family for an ICC-enabled processor ("[a]lthough ICC enabled PDF processors should
always follow the provisions and recommendations provided in 10.3 … a less-capable PDF
processor may choose to use the algorithms specified in the following subclauses"). So a grey
file is the grey of the picture as this tree draws it; a `DeviceGray` image comes back as
itself, by the identity above; a `/Decode` array, a soft mask or a colour key applied on the
way to RGB stays applied. There is no second route reading a grey image's samples directly,
which would be a fourth conversion for the case where the first three already agree.

**`--format` is a statement about decoded samples and never about a native stream.** For
`images`, the format names the file form of every image that is *decoded* — PNG, PPM or PGM —
and `--native` writes a JPEG or a JP2 as it is whatever its own colour model: a grey JPEG is
grey already, and a CMYK JPEG stays CMYK, because a stream converted is no longer the native
stream and the flag would be lying. A person who wants the grey of a CMYK JPEG asks for it
decoded, and gets the route above. The two flags therefore compose rather than conflict: under
`--native --format pgm`, the codecs with a file form are written as files and the rest —
JBIG2, CCITT, Flate — are decoded to PGM, the warning naming the format.

**A netpbm file has no alpha, so the mask goes beside it.** ADR 0802's rule — the mask is
written beside the image wherever the file cannot hold it — now has three such files rather
than two, and the mask beside a PPM or a PGM is a `P5` PGM, because a mask is one channel and
the one-channel netpbm form is the grey one whichever form the image took. `ImageFile` gains
`Ppm`, `Pgm` and `MaskPgm`; `mask_name` takes the format.

### 2. The `/Names`-indirect holder: a fixture built, and the shape counted

`attach` rewrites "the nearest indirect object": the old root's number where the tree was
indirect, the name dictionary's where that was, the catalog's otherwise. The two committed
documents and the corpus's `attachment.pdf` cover the first and the third; the second had no
witness (ADR 0802 said so). The fixture is built in `tests/writer.rs` the way
`crates/pdf-syntax/tests/incremental_update.rs` builds its documents — seven objects and a
classic table, with one file already filed so that the rewrite has an entry to keep — and the
test holds what the shape implies: the name dictionary's object is in the update, the
catalog's is not (its cross-reference entry still points into the source's bytes), the
rewritten dictionary reaches the new tree by reference, the tree lists the old entry as the
leaf stated it and the new one in §7.9.6's order, both removals leave what they should, and
the page route leaves the tree alone. `qpdf --check` accepts the results where it is
installed — evidence about the reading, never its definition. The fixture's page states an
empty `/Resources` because qpdf repairs a page without one and reports the repair as a
warning, and a foreign reader's warning about the fixture is not evidence about the update.

Whether the corpus has the shape is the walk's question, and the walk counts every holder
shape it meets — so the census `doc/todo/57` §1 asked for is a line the gate prints rather
than a sentence here.

### 3. The writer over the corpus: exact assertions, refusals by reason, nothing cached

`crates/pdf-transform/tests/writer_corpus.rs` walks `doc/pdf.js`'s corpus — the population
every other gate rasterises, opened with the same eight known passwords `save_round_trip.rs`
uses, so that the denominator is every document the suite can open — and for each one:
attaches a file into the tree and holds §7.5.6's prefix property ("leaving its original
contents intact"); reads it back by `--save` with the bytes equal and by `--list` with every
file the document already carried still beside it; removes it, the prefix property holding
again and the listing back to the source's; and files it on page 1 by §12.5.6.15's annotation
and reads it back from there. Everything is in memory, nothing is written to the corpus, and
nothing spawns a process: the reader that judges the writer is this tree's own, which is
trap 8's measurement with the instrument under test and is said so in the file. The
foreign evidence stays where it was — `qpdf --check` over the committed fixtures — and a
corpus-wide foreign readback is the instrument ADR 0334 priced and this round does not take.

Four things are asserted and admit no known-failure list: no panic, every update a prefix of
its input, every payload read back equal, every listing what it should be. A refusal is not a
failure: a document the writer declines by `UpdateError` — a table rebuilt by scanning has no
offset to chain to, an attachment-only-encrypted file's key cannot encrypt what is written —
or one with no page to file on is the document's, counted by reason and printed, never
folded into a failure count (trap 11). What the walk cannot yet explain goes in `HELD` with a
diagnosis, in the oracle's style, and an undiagnosed refusal fails the run. The census counts
are printed and not ratcheted, on ADR 0323's rule that a number enters a ratchet after it has
held.

**It is a gate line in `doc/todo/02` §2, its own binary, and it runs under `bounded.sh`.** Its
own binary for the reason `save_round_trip.rs` gives — `-- --ignored` runs every ignored test
in a binary, and the transform gate's wall-clock floor is not a place for a corpus pass. In
§2's block from its first landing, as the transform gate was (ADR 0801), because RFC §9 asks
for exactly this and its assertions are exact rather than ratcheted; `tools/state.sh writer`
prints its summary. It decodes no image, so it carries the exemption line
`tools/conformance/tests/sandbox_gates.rs` demands instead of `require_the_sandbox`. And it is
a corpus walk, so `doc/environment.md`'s rule binds it: one walk on the machine at a time,
under `tools/bounded.sh`, which this round ran it under with `--data 4 --tree 8` after waiting
for a neighbouring round's survey to finish.

## Consequences

- `doc/todo/57` §1 is empty of unblocked items but `--password-prompt`; what waits on RFC §13
  is exactly the serializer and the four verbs on it.
- `ImagesPlan` gains a `format` field, so every consumer constructing one states it; the CLI
  defaults it to PNG. `mask_name` takes the format. `ImageFile` has seven variants and the
  JSON report's `file` field can now say `pgm`, `ppm` and `mask.pgm`.
- `doc/todo/02` §2's sequence is one line longer, and so is a round's cost by the walk's
  wall clock, which `tools/state.sh writer` prints.
- The thread curve `doc/todo/57` §5 asks a round that touches `render`'s parallel shape to
  re-take was not owed — this round did not touch that shape — and was taken anyway, on the
  `gates` binary the walk had just built, pages 1–200 of ISO 32000-2 at 150 dpi to PNG, two
  runs a row, with a neighbouring round's work on the machine (load average about 2 to 3):

  | threads | wall | CPU (user) | ADR 0801, after its fix |
  |---|---|---|---|
  | 2 | 4.14 s, 4.28 s | 8.0 s, 8.3 s | 4.12 s |
  | 4 | 2.32 s, 2.34 s | 9.0 s, 9.1 s | 2.43 s |
  | 24 | 0.96 s, 1.00 s | 18.2 s, 18.6 s | 1.08 s |

  Every row is at or under ADR 0801's, so nothing here regressed `render` and no code
  changed for it. The 24-thread CPU figure is the machine's, for the reason that ADR gives.
