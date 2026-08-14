# 502 — The codespace inverted, and the axis a mode decides

**Finding.** `doc/todo/22`'s last edge was a composite `/DA` font, refused for four hundred
sessions on a reason that reads like one and is not: *a character cannot become a code without
inverting §9.7.6.2's codespace ranges*. Every clause in that sentence is true; none of them says
why it was not done. Inverting them is a **test** rather than a construction — offer a code only
if the same `CMap` would extract exactly it from its own bytes — and the test is one line beside
`next_code`. What the refusal was hiding was a smaller, sharper pair of refusals with a clause
each: a `CMap` stating more codes than a bounded walk will visit, and §9.7.5.1's writing mode 1,
which decides *which metrics* place a glyph and so cannot be laid out along §12.7.4.3's horizontal
axis. Three things were found on the way: a line break travelling through the layout as the code
for a line feed, which a one-byte `CMap` code 10 would have turned into a swallowed character; a
Type 0 font's `/Ascent` and `/Descent` never being found, because Table 119 puts the descriptor on
the descendant; and two claims in the tree — one of them a ledger row — that had been false since
the thirty-sixth and the hundred-and-twenty-third sessions.

**Date.** 2026-08-14.
**ADR.** [0337](../adr/0337-the-codespace-inverted-and-the-axis-a-mode-decides.md).
**Touched.** `crates/pdf-font/src/cmap.rs` (`each_addressable_code`, `stated_code_count`,
`extracted`, three tests), `crates/pdf-font/src/loading.rs` (`addressable_codes`,
`build_addressable_codes`, `code_for`, `addresses_characters`, `MAX_ADDRESSABLE_CODES`, one new
test and one whose contract changed), `crates/pdf-model/src/variable_text.rs` (`Placed`,
`descriptor_of`, `show`, `code_bytes`, `set_in`, `Owed::VerticalWritingMode`, the module comment),
`crates/pdf-model/tests/variable_text.rs` (a composite fixture builder and three tests),
`crates/pdf-model/examples/variable_text_census.rs` (the composite tally),
`doc/conformance/ledger.toml` (§12.7.4.3, §9.7.6.2, §9.7.5.1, §9.7.4.1, §7.3.4.2, §9.8.1),
`doc/todo/22-variable-text-edges.md` (amended — one item left and it is `21`'s),
`doc/adr/0337-*` (new), this file.

## The measurement that came before the code

`examples/variable_text_census` grew a count of the `/DA` fonts `/DR` defines that are Table 119's
Type 0, and of those in writing mode 1: **0 and 0**, over the 964 corpus documents it opens. So
the corpus can rank none of this, the gates are expected to be identical, and the rules are held
by pairs of hand-built files differing in one entry. That is trap 8's instrument used deliberately
rather than an apology for not having a witness.

## What a later round should know

- **A note that names the clause making something hard has not said why it is not done.** The
  refusal here cited §9.7.6.2 accurately and stopped. `doc/todo/22` copied the citation and read
  it as a reason for four hundred sessions. The tell is a note whose whole content is a
  restatement of the problem — no cost, no population, no alternative weighed.
- **A sentinel value in a domain a document controls is a defect waiting for a feature.**
  `BREAK` was `Code::single_byte(b'\n')` and was correct while nothing could produce that code.
  The feature that produced one was this round's. Nothing reported, nothing measured; the enum
  costs nothing and the compiler keeps them apart.
- **Reading one table led to a false claim in the row of another.** §9.7.4.1's note was opened for
  `/FontDescriptor` and was found saying `/DW2` and `/W2` are unread — false since session 36,
  while two neighbouring rows said the opposite. Opening the row of the table you are reading is
  cheap and it is where `doc/todo/01`'s fourth sweep would have gone anyway.
- **`doc/todo/22` is one item from being deleted**, and that item is `doc/todo/21`'s: the Arabic
  in `freetext_no_appearance.pdf`. Whoever takes `21` should take the file's last section with it.
