# ADR 0011 — The reference oracle runs over the corpus, bounded by the references' own spread

Status: accepted, 2026-07-27.

## Context

ADR 0005 built the triangulation harness and the previous four sessions left it wired to
exactly **one** hand-built fixture. Nothing compared our rendering of a real document
against anything, and the cost of that showed up in the fourth session: two defects —
every gradient mirrored about the page's centre line, every image sampled through a doubled
transform — shipped on pages that reported nothing wrong. No metric we own could see them,
because a page that reports `unsupported: []` is a statement about *what we know we
skipped*, not about what we drew.

Three reference renderers were installed, 988 documents holding 3143 pages were on disk, and
the harness that compares them was already written and tested. The only thing missing was
pointing it at them.

## The decision

`crates/pdf-model/tests/oracle.rs` renders **every page of every pdf.js corpus document**,
and page one of the specification PDFs in `doc/`, with our pipeline and with poppler, mupdf
and ghostscript, and applies the triangulation rule to all four. It is a ratcheted gate, not
a survey: every page the references contradict us on is **named** in the source, and both a
new disagreement and a stale entry fail the build.

Four sub-decisions carry the design.

### 1. Our own deviation is judged against the references' spread, not a fixed number

A fixed tolerance has to serve two populations at once. On a page of flat vector fills the
references agree to a worst tile of 0.4, so a worst tile of 5 from us is ten times their
entire spread and unmistakably a defect. On a page of small text they disagree at 26 among
*themselves*, so the same 5 says nothing at all. One threshold cannot separate signal from
noise on both, and the threshold that passes the second silently forgives the first.

So the consensus is still decided by the fixed [`Tolerance`] — deriving *that* from the
spread would be circular — and our own deviation is then judged against bounds widened to
**twice** the disagreement the consensus references show among themselves on that page
(`pdfref::Judgement::CORPUS`). The question asked is: *are we further from the consensus
than the consensus is from itself?*

Twice rather than once because a third correct implementation is not required to sit
between two others: it may differ from both in the same direction and by the same
magnitude, and a factor of one would fail it for being correct. Beyond two the bound starts
forgiving real defects on text pages, where the floor is already high.

The fixed bounds remain as a floor. Two references producing identical pixels — which
happens on simple pages — would otherwise demand of us an exactness no third
implementation can deliver.

Only pairs *within* the consensus widen anything. An outlier's distance measures the
outlier's error, and letting it widen the bound would buy us licence to be wrong by the
same amount.

### 2. Only pages we claim to draw completely are gated

A page whose interpretation reports an unsupported font or an undecodable image is expected
to differ from a renderer that implements it. Gating those would mean listing three hundred
documents whose disagreement we already predicted, and the signal would drown. They are
still compared and still printed — the count is a rough measure of what the missing
features cost visually — but they cannot fail this gate. `corpus.rs` owns them.

### 3. All pages of the corpus, page one of the specifications

The pdf.js corpus holds its files because each one broke a reader once, and a file reduced
from a bug report does not reliably put the interesting page first. Comparing only page one
asks 869 single-page documents everything they have and the other 100 almost nothing.

The specification PDFs are the opposite case: 1382 pages from 14 files, 1023 of them from
ISO 32000-2 alone, consistently typeset, where page 500 exercises what page 499 did. They
stay at page one, where they still contribute the heaviest fonts and the largest page trees
in the tree.

1794 pages rather than 988, for about 1.5× the wall clock. Rendering a late page was checked
before committing to this: all three references seek through the cross-reference table, so
page 300 of a 352-page document costs what page 1 does, and the run is linear rather than
quadratic in page count.

Pages are compared in parallel rather than documents, which matters because one corpus file
has 352 of them and would otherwise be the long pole of the entire run.

### 4. The ratchet is per page, and grouped by what the page carries

174 pages are listed, in four groups: pages carrying an annotation appearance we do not draw
(47), pages with optional content configured off (4), pages using a font nobody embeds (40),
and pages with nothing on them to explain the difference (83). 31 of them are pages beyond
the first, which a page-one comparison would never have seen.

The grouping is a hypothesis about the cause, not a diagnosis, and the source says so.
`calgray.pdf` proves the point: it sits in the substituted-font group because it labels its
swatches with a non-embedded font, while what actually differs is the swatches. The groups
earn their place by making the list readable and by being the unit in which it shrinks —
drawing annotation appearances should empty the first — but the ratchet is checked over
their union, so a page moving between groups is not a build failure.

## What it cost, and what it found immediately

The whole run is **125 seconds** over 1794 pages on 24 cores. That is an ordinary gate, not
a nightly job.

Where that time goes was measured rather than assumed, and the gate now prints it:
**1596 seconds of processor time in the three reference renderers against 149 in ours** — a
ratio of about eleven to one, which decides what would be worth optimising if this ever
needs to be faster. Caching the reference renders, keyed on renderer version, full command
line and a hash of the file bytes, would remove most of it; it is not done, because 125
seconds is affordable for a gate run a few times a session and a cache key that silently
omits a variable would compare against stale renders — the exact defect class the crop-box
fix above was. The trigger to revisit is this gate moving into a per-commit CI job.

It found four things on its first runs, three of which were silent:

- **The harness was comparing against the wrong page box.** `pdftoppm` and `gs` default to
  the media box; §7.7.3.3 makes `/CropBox` "the region to which the contents of the page
  shall be clipped (cropped) when displayed", which is what a viewer shows and what we and
  `mutool` use. 54 documents' first pages were beyond comparison entirely, and on a page
  whose crop box has the same *size* as its media box but a different origin the harness
  would have compared a correct render against a displaced one and called us wrong. Fixed
  by telling both renderers which box to use.
- **Text render modes 4 to 7 add the glyphs to the clipping path** (§9.3.6, §9.4.1) and we
  build no such clip, so a rectangle painted afterwards to be seen only through the letters
  covers its whole area. `text_clip_cff_cid.pdf` showed a solid blue bar where all three
  references show the word "ABC123" — with `unsupported: []`.
- **An image's `/Mask`** — stencil (§8.9.6.4) or colour-key (§8.9.6.5) — was ignored
  entirely; only `/SMask` was honoured. `colorkeymask.pdf` drew a red band all three
  references correctly hide.
- **`/UserUnit`** (§7.7.3.3) is neither applied nor reported: `mutool` and `gs` scale a page
  by it, we and `poppler` do not.

The first is fixed. The next two are now *reported*, which moved ten documents from
"complete" to "incomplete" in `corpus.rs` — a rise that is the ratchet working, exactly as
when content-stream decoding started reporting. Implementing them, and `/UserUnit`, is
written down rather than done: principle 1 says a thing not doable properly now is not
started now, and each already has a test that will fail when it lands.

## Consequences

- A defect of the class that shipped in the fourth session now fails the build the first
  time the gate runs. That is the whole point.
- The gate depends on three external programs and their versions. A reference renderer
  upgrade can move an entry, so the run prints every renderer's version, and a moved entry
  is a question about which changed rather than an automatic defect.
- One group in the ratchet — pages using non-embedded fonts — could legitimately differ on
  another machine, because font substitution is the only machine-dependent code in the
  tree. They are listed rather than excluded, because such a page can *also* be wrong for a
  real reason and dropping it would hide that.
- Artefacts are written for every page that is not agreement and deleted for every page that
  is — 570 MB as it stands, against about a gigabyte if agreeing pages were kept too.
- Reference renderers are now given a 30-second budget and killed if they exceed it, matching
  the budget `corpus.rs` holds us to. A corpus contains files written to make a reader loop,
  and an unbounded `wait` on one hangs the suite.

## The one number to read with care

Of the 1424 pages we call complete, 548 agree, 174 are contradicted and **691 are
ambiguous** — the references cannot agree with each other. That last share is far larger
over all pages than over first pages (49% against 21%), and it is not a statement about the
corpus: 370 of the 691 are two long books, `freeculture.pdf` and `pdkids.pdf`, whose text
uses fonts nobody embedded, so each renderer substitutes a different one and the structural
bound separates them. Ambiguity concentrated in a handful of documents is worth knowing
before anyone reads it as "half the corpus is unsettled".

## The rule this leaves behind

The gate finds the page. It never says what the answer is. Every entry on the list is a
question to take back to the specification — principle 5 — and agreement with poppler,
mupdf and ghostscript remains evidence that we read a clause the same way they did, never
the definition of right.
