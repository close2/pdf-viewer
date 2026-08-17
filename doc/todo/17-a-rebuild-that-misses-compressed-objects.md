# A rebuild that misses every compressed object

Status: **diagnosed with a witness, not taken.** Found in the five-hundred-and-fifty-eighth session
by taking `doc/corpora/pdf-differences` (`doc/todo/03` §14).
Priority: 17
Corpus: `doc/corpora/pdf-differences/UnknownFilter/UnknownFilter-Linearized.pdf`, one page whose
text this tree does not draw and which the PDF Association's own README says "should be fully
processable". Population unmeasured — that is the first thing this item owes.
Clauses: §7.5.7 (object streams), §7.5.5, §7.5.8, Annex C.4
Code: `crates/pdf-syntax/src/xref.rs` (`rebuild`, `scan_for_objects`),
`crates/pdf-syntax/src/document.rs` (`expand_object_stream`)

## What happens

`xref::rebuild` recovers a file whose cross-reference chain is unusable by scanning for `N G obj`
headers. That finds every object written at the top level of the file and **no object inside an
object stream**, because a compressed object has no header to scan for — it exists only as bytes
inside a `/Type /ObjStm` whose own header says which object numbers they are.

So a document whose cross-reference stream cannot be read loses every object §7.5.7 packed, in
silence. The witness is precise about the cost: `UnknownFilter-Linearized.pdf` puts `/XXXDecode` on
the *first-page* cross-reference stream of a linearised file, this tree falls back to the scan,
finds the page (object 9) and the image (13) because both are top-level, and does not find the font
(17) because it is one of three objects inside object stream 11. The page draws its cat and reports
`no /Font resource named /TT0`; `pdftoppm` and `gs` draw the word *Hello!* as well.

## Why it is worth doing, and what makes it cheap

§7.5.7 states the recovery itself, which is unusual — no guessing is involved:

> The first N pairs of integers shall contain, in order, the object number and byte offset of each
> object

An object stream **says what is in it**. So the missing step is not a heuristic: for every object
the scan found that is an `ObjStm`, read its own header and register `Location::InObjectStream` for
each object number it names.

**And the rule can be made strictly additive**, which is what would keep the risk near zero: an
entry the top-level scan already found wins, and object-stream members fill in only numbers that
resolve to nothing at all today. Under that rule no document that currently opens can change, and
the only documents that move are ones now missing objects entirely.

## What it costs, which is why it was not taken in the round that found it

The step belongs in `Document` rather than in `xref.rs`, because expanding an object stream needs
the filter chain and the decryption `xref.rs` deliberately does not have — `Document::expand_object_stream`
is already the code that does it. So the rebuild grows a second phase that runs after the table
exists, which is a change in the shape of opening a damaged file rather than a line in a scan loop.

Two things a round taking it owes:

- **The population.** How many documents on this disk are recovered by scan *and* contain an
  object stream? `XrefTable::recovered_by_scan` is already carried, so the census is a short
  example over the 974, the four corpora and the crawl. The corpus gate's `unopenable` and
  `pageless` counters are ratcheted, so a change here has to be able to say which documents move
  and why before it lands.
- **A bound.** Expanding every scanned `ObjStm` on a hostile file is work an attacker chooses; the
  existing `Limits` cover the decode, but the *number* of streams expanded during recovery is a
  new axis and needs its own answer.
