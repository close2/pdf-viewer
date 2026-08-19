# 603 — A name nobody could look up

`doc/todo/03`'s chunk, taken over the one population on this disk that had never been put beside a
reference — the SafeDocs crawl — and the clause the chunk's finding turned out to be about, §7.3.5.

## Which chunk, and why the file's own evidence points here

`doc/todo/03` §14 closes with "[e]very population on this disk is ranked", and the sentence is
true of the *curated* corpora only. The crawl has only ever been **surveyed**: five questions this
reader asks of itself, one process per archive. 64 507 of its documents are called *complete* and
not one of them had ever been compared with another renderer, which is the gap between
`CLAUDE.md`'s two questions written out in documents.

Two archives whole, **2000 documents**, `0100` and `7680`. Which archives does not matter and that
is ADR 0261's finding: the crawl is sorted by SHA-256, so an archive is a hash bucket.

The instrument is `doc/todo/00` step 7's number, the one sessions 505, 544, 554 and 558 pointed at
the curated corpora: page one at 72 dpi from here and from `pdftoppm`, `mutool` and `gs`, every
invocation explicit about the page box (trap 3), ranked by our ink minus the lightest live
reference's, with the raster size of each panel beside it (§13's lesson). About a third of a second
a document at sixteen workers.

## What it found

The head separates from the body by three orders of magnitude: **−225.633 of 255** against a
next-largest of −5.040. `0100223.pdf` is a full-page scan that `poppler`, `mupdf` and `ghostscript`
each draw — their inks agree to 0.93 — and that this tree drew as a blank sheet, saying the file
named an `XObject` it does not define.

The file does define it. It is a Hewlett-Packard scanner naming its image after a Windows path, so
the name's sixth-from-last-but-one byte is 0xF4 — *ô* — written `#F4` in both the resource
dictionary and the `Do`, byte for byte the same. What could not match them was this program:
`run::name_at` handed the operand on as a `String` built with `from_utf8_lossy`, and §7.3.5 says
two names are the same one when "the resulting sequences of bytes are … an exact binary match".
U+FFFD is not 0xF4.

**The direction with no witness is the worse one**, and it is why this is a defect rather than a
missed page: every invalid byte becomes the *same* replacement character, so `/A#F4` and `/A#F5`
were one name and a `Do` could have drawn the object the stream did not name, in silence — trap 1's
archetype. There is a hand-built pair for both directions now.

The lookups take a `Name` and probe with `Dictionary::get_by_name`; the lossy conversion stayed at
each report and trace label, which is the one use of a name as text the same clause allows. ADR
0438 has the argument, the seven operator families it reaches, and the two conversions left as text
on purpose — a marked-content tag and a rendering intent are names *the standard* defines.

## What moved, and what did not

- The witness: 0 commands and a report → 1 command and no report; ink 0.000 → 225.476 against the
  three references' 226.567 / 225.640 / 225.633, so its gap is −0.158.
- Archive `0100` re-ranked whole afterwards and diffed panel by panel: **exactly one of the 1000
  rows differs**, and every reference panel is byte-identical.
- **No gate moved at all.** No document of the 974 states such a name, which is the measured form
  of why a whole blank page on the web was invisible to every ratchet this project has.

## The clause

§7.3.5's ledger row was already `implemented` and had the right sentence quoted in it — for the
*parser*. What it did not say is that the same sentence binds the interpreter that looks a resource
up, and the row says it now, with the two tests named. `spec-errata emit` over the specification
PDFs has one annotation on this clause and it is about hexadecimal strings, not about names.

## Gates

The full §2 sequence, because the change is in `pdf-model`: fmt, clippy, the workspace tests, the
doctest line, the sandbox build, corpus, `pdfref-hayro`, oracle, the three text-extraction gates,
both censuses, dates, xmp, jpeg2000, quorra's corpus and the conformance gate. Run once, then run
again whole after the last edit, because two of the edits were made while the first run was going.

§5's binaries were **not** rebuilt: `tools/round.sh` flags them as older than `HEAD`, this is not a
fifth round, and the round's own measurements are of an example binary built from the working tree
rather than of anything in `target/`. A round that measures the launch path owes that rebuild first.

## What the chunk leaves

2000 of 65 944 crawled documents are ranked. The rest is about eleven minutes per 2000 at these
settings — six hours for the population — and the *oracle* proper over it is a different question
that wants a decision first, in ADR 0393's shape: what a verdict means where nothing supplies an
expected value. `doc/todo/03` §16 says so.
