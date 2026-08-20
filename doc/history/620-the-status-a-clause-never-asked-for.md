# 620 — The status a clause never asked for

A spec-driven round on the band session 616 named — the bottom of `doc/todo/01`'s blame ordering
rather than the top. Five of the eight rows were wrong, two of the five were the *status* rather
than the note, and the second of those two was hiding behind the first.

Date: 2026-08-20.
ADR: [0455](../adr/0455-the-status-a-clause-never-asked-for.md).

Touched: `crates/pdf-syntax/src/crypt.rs` (a new unit test),
`doc/conformance/ledger.toml` (§7.9, §7.9.2, §7.6.4.4.2, §7.6.5.1, §7.6.5.3, §8.10, §8.10.4,
§8.10.4.1, §8.10.4.3), `doc/todo/01-ledger-partial-rows.md`, the ADR and this file.

## How the band was ordered, and where blame disagreed with 616

The ordering was re-derived rather than taken, which is 616's own lesson: `git blame
--line-porcelain doc/conformance/ledger.toml`, each `partial` or `reported` row's own `note = `
line, ordered by where its commit falls in `git log --reverse`.

**It agreed with 616 on seven rows and disagreed on the eighth — the one 616 recommended reading
first.** §12.5.6.12 was read in the six-hundred-and-fourteenth session and its note rewritten; 614
and 616 ran in parallel, so 616's base did not hold 614's commit. The band is eight rows, not nine,
and the one that could have changed a pixel had already been taken.

**The threshold 616 quoted does not survive a merge either.** "38 below commit 534, 18 below 200"
becomes 29 and 17 on this base, and almost none of that is rows being read: `git log --reverse`
linearises merges, so a parallel round landing renumbers every index above it. `doc/todo/01` now
says to quote ranks and never commit indices.

## §7.9.2, and the sweep that had been right fifteen times

`partial` since the ninety-sixth commit, for a reason that is a statement about this codebase:

> What is not here is the *typing*: the object model carries one string type and each reader
> decides which of the four it holds.

§7.9.2 opens "PDF supports one fundamental string object", and **§7.9.2.1's own row is
`implemented` and says "[t]his crate holds exactly that"** — so the family held two opposite
readings of one sentence and the parent's was the wrong one. All six children are `implemented`;
§7.9.2 states no prose of its own.

The sixth sweep — which parents are behind their children — has printed this row every run since
the three-hundred-and-seventy-fifth session, and every run answered it by citing a sentence in
`doc/todo/01` written in the three-hundred-and-forty-second. **A dismissal that cites is not a
dismissal that checks**, and that is ADR 0455's rule: a sweep hit is retired with the clause
quoted, the code named and the population measured, never with a note of what session N did.

Before moving it I audited what the retired observation was about — every `String::from_utf8_lossy`
over an `Object::String` in `pdf-model`, against the type its table gives the key. Both survivors
are correct: Table 270's `/WKT` and Table 49's `/URL` are *not* text strings (`ASCII string` and
plain `string`), and `ExtensionRevision` beside the second one is a text string and goes through
`text_string`. The observation stays in the note as an observation, with that audit as the check to
repeat.

## §7.9, which the child had been standing in front of

Moving §7.9.2 made the sweep print its parent, and the parent was wrong too. §7.9's note said
`partial` was "§7.9.3's text streams alone, which is `reported`" — **§7.9.3 has been `implemented`
since the three-hundred-and-eighty-seventh session** (ADR 0224, Table 177's `/RC`), and the
five-hundred-and-twenty-fifth's "read and kept" repeated the sentence rather than checking the row
it named. All seven children are `implemented`; §7.9 states no prose of its own.

**The sweep is a chain rather than a list**: it prints a parent only when *every* child is complete,
so one wrong `partial` conceals its parent's. A round that clears a hit runs the sweep again in the
same session. Two hits are now one — §O, which has `doc/todo/39` and ADR 0209 behind it.

## §7.6.4.4.2, and a failure shape this file had not named

The row said "[s]teps (a) to (d) are implemented" and cited
`encryption.rs::a_document_with_a_password_opens_with_it_and_not_without`. The steps *are*
implemented — checked one at a time against `unwrap_owner_entry`, including two things worth
recording so nobody "fixes" them later:

- step (c)'s fifty hashes re-hash the **whole** previous digest, where §7.6.4.3.2 step (h) re-hashes
  "the first n bytes"; the difference is the standard's own wording;
- step (d)'s n keys off the **revision** ("shall always be 5 for security handlers of revision 2")
  where §7.6.3.2 step (b) keys the same number off `/V`, and `key_length` implements the second —
  Table 21 makes R 2 exactly when V is below 2, so the two can only disagree on a file that has
  already broken Table 21.

**The citation is what was wrong.** The cited test does not reach the path. The corpus's eight
password-protected documents were checked one at a time: `issue15893_reduced.pdf` (R3),
`issue3371.pdf` and `bug1782186.pdf` (R4) all open on their *user* password, which returns before
Algorithm 7 is reached, and the remaining five — including `print_protection.pdf`, the one whose
known password is the owner's — are revision 6, where §7.6.4.4.11's Algorithm 12 replaces this path
entirely. Nothing in the tree reached `unwrap_owner_entry`.

So the round wrote the test. `an_owner_entry_unwraps_to_the_padded_user_password` runs Algorithm 3's
steps (e) to (h) forward — the half a writer performs and this crate never does — and asserts the
reader unwraps the `/O` it produces back to the padded user password, at revisions 2, 3 and 4. It
was mutation-checked: 50 hashes to 49 fails it.

**And one thing it deliberately does not pin, because claiming otherwise would be the round's own
failure shape.** RC4 is a stream cipher, so twenty applications XOR twenty keystreams and XOR is
commutative — reversing either the writer's "from 1 to 19" or the reader's "from 19 to 0" leaves
the test green. Reading the clause is the only way to get that order right; the doc comment says so
rather than claiming a coverage the cipher makes impossible.

The new shape, for `doc/todo/01`: **the row is right and its evidence is not.** Every other shape in
that file is a note saying something false; this one says something true and points at a test that
cannot show it, which is worse, because a reader who checks the citation gets a green test. The
eighth sweep asks whether the file a note names exists; nothing asks whether the test it names
executes the requirement, and no program can.

## The three §8.10 rows, and a population nobody had counted

- **§8.10.4.3** said "[t]hree considerations" and the clause states **two** — annotations and
  logical structure. The third the row listed, the proxy's `/Group` applying to the imported page,
  is the closing sentence of **§8.10.4.1** and Table 93's own `/Group` cell. That is the ninth
  sweep's shape one row over from a table.
- **§8.10.4** and **§8.10.4.1** are confirmations, and both gained the population they had never
  stated: nothing in `crates/`, `tools/` or `fuzz/` names `Ref` or a reference XObject at all — so
  "PDF processors that do not recognise the Ref entry" is literally true of this tree — and
  `witness_census` over all 1251 PDFs on this disk finds **two** documents stating `/Ref` as a
  name, `bug1997343.pdf` and `bug2009627.pdf`, where in both it is §14.7.2 Table 355's `/Ref` on a
  `/TOCI` structure element. **No document states a reference XObject.**
- **§8.10.4.1 stays `partial`**, and the argument for moving it was made and rejected in ADR 0455.
  Its note compares itself to §10.7.2, which is `implemented` because the whole of its normative
  content is a permission; §8.10.4.1 grants a permission in one sentence and spends the rest of
  itself on the processors that do import.
- **§8.10** is read and kept: it states no prose of its own and owes only §8.10.2's unread entries
  and §8.10.4's unbuilt import.

`spec-errata emit` over the family before writing found Issue #463 adding a `shall` to Table 93's
`StructParents` cell — "[a]t most one of the entries StructParent or StructParents shall be
present" — which is a `shall` on the file over two entries §8.10.2's row already records as unread
and as not changing a mark on the page. Nothing moved for it.

## §7.6.5's two rows

- **§7.6.5.3** said "[t]he SHA-1 and AES seed algorithms", and the clause names no such thing. It
  names a *digest* that turns the recovered seed into the file encryption key — SHA-1 for 128 bits
  and **SHA-256** for 256, over the 20-byte seed, then each `/Recipients` item in array order, then
  0xFF four times where document-level encryption leaves metadata plaintext — and separately seven
  CMS content ciphers, six of them "deprecated in PDF 2.0". Getting that right matters because it is
  the half this tree already has; what is absent is `EnvelopedData` and an RSA decryption against a
  private key nobody has decided how to reach. The row also never recorded that Errata Collection 3
  Issue #196 renumbers it to §7.6.5.4, which §7.6.5.2's row carries and this is where a reader
  looking up the number arrives.
- **§7.6.5.1** said "As §7.6.5." — true and unfalsifiable. It now quotes the one `shall` the clause
  puts on a reader ("the PDF reader shall scan the recipient list … and shall attempt to find a
  match with a certificate that belongs to the user") and says where it is refused instead.

## Gates

The change is in `pdf-syntax`, which is the change→gate map's first row, so the whole sequence ran
even though the diff is one `#[cfg(test)]` block.

`fmt` clean. `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` exit 0 and silent
apart from `viewer-qt`'s gcc `cargo:warning=` lines, which `doc/todo/02` §2 names. `cargo nextest
run --workspace` 2284 passed / 16 skipped. Doctests clean. Corpus 974 documents, 68 incomplete.
Oracle 1794 pages: 907 agree, 66 contradicted, 786 ambiguous, 2 our geometry, 2 reference geometry,
13 not comparable, 18 no render. Text extraction 10969/11163 matched words in bounds over 508
documents. `selection_census`, `accessibility_census`, `dates` (1545 strings, 1514 conforming),
`xmp` (319 documents) and `jpeg2000` all green; `render-quorra` 957 pages, 932 agree / 23 differ /
2 refused. `cargo test -p conformance` 157 passed. Both of §2's `--profile gates` builds
(`pdf-sandbox` bins, `pdfref-hayro`) succeeded before the gates that need them ran, which is trap
10.

Sweeps run because the ledger moved: `quotations` — 1604 ledger quotations, 1 diverging, and that
one is §8.9.5's and was there before; `applied` 1246 comparisons with 10 on the read-first list,
unchanged; `counts`, `tables` and `pointers` no new hits; the sixth sweep re-derived twice, two hits
down to one.
