# 570 — A census of the absences this tree asserts, and the five that were not

**The question.** ADR 0403 found one ledger row parked for thirty-one rounds on a `grep` that had
classed a corpus of PDFs as binary and printed nothing. It left the obvious question unanswered:
how many other "no corpus document does X" sentences are wrong? This round counted them and re-ran
them.

**The population.** About 165 live sentences of that shape — 75 in `doc/conformance/ledger.toml`,
around 25 across `doc/todo/` and the standing `doc/*.md`, around 65 in `crates/` source and tests.
(A regular expression over the whole tree matches 320; the rest are in `doc/adr/` and
`doc/history/`, which are records of what a round found rather than live claims, and were left
alone.) Sixty distinct PDF names came out of them, and every one was re-measured.

**The instrument, and there are two of it.** Re-using a suspect tool would have learned nothing, so
neither half is a grep. `examples/witness_census` asks each name three ways of each of the 1251
PDFs under `doc/pdf.js`, `doc/corpora/` and `doc/corpora-own/`: the file's raw bytes — exactly what
`grep -a` reads — every object the cross-reference table names *including the ones inside §7.5.7
object streams*, walked as `Name` tokens rather than as text, and every stream's decoded data. The
three counts print side by side, so a term a byte search undercounts is a number rather than an
argument. `examples/absence_audit` re-asks seven of the written claims through the readers that
would act on them, because a name being stated is not the structure being stated.

**`grep -a` was only half the fix, and this is the number that says so.** Two corpus documents
state a `/SetOCGState` and `grep -a` finds **neither**: both are inside object streams, where no
byte search over the file can reach. Four of the twelve `/IDTree` witnesses are in the same place.

**Five claims were false, in three different ways.**

*A correction that stopped at the file it was written in.* Three places said "no corpus document
has an `/IDTree` at all — the 89 tagged ones state none". Twelve of the 974 state one, holding
between 1 and 285 identifiers — and `Tree::element_by_id`'s own doc comment, in the same crate as
one of the three, already carried that correction and that number, put there by the round that
built `cell_header_census`. Two independent censuses, the same twelve.

*A ledger contradicting itself.* §14.10's row said "[n]o corpus document writes a `/SpiderInfo`"
while §7.7.2's row, in the same file, listed "`/SpiderInfo` (§14.10's web capture, **5
documents**)". Both written from a measurement; one of the measurements taken.

*A count wrong by nine.* §8.6.5.6's "one corpus document names a default at all" is nine — eight a
`/DefaultRGB`, one a `/DefaultCMYK` — one of them stating it inside an object stream. The fixture
that rested on it is still right to exist and its reason is now an argument rather than a count.

*And twice, a population the sentence did not name.* §12.4.3's articles and §12.3.5's collections
were each "no corpus document" and each **true of pdf.js**. The corpus this project measures over
is wider — `signatures.rs` measures `/Lock` over "the pdf.js corpus and the four under
`doc/corpora/`", `cell_header_census` runs over 1251 files — and over that population four
documents state an article with **115 beads** between them and one is a portable collection with a
folder tree. A sentence that does not say which corpus cannot be checked by reading it, which is
exactly why these two survived: every round that read one read a true statement about a different
population.

**Two clauses acquire producers' files, and both readers are right about them.** `Articles::read`
walks `PDFBOX-3110-poems-beads.pdf` to two threads titled `Erlkönig` and `Moulière` — Table 162
`/I` titles decoded as §7.9.2.2 text strings, the first settling that the decoding is not
byte-for-byte — with all eleven beads naming their page and each ring closing on its own first bead
after exactly as many `/N` steps as it has beads. That last assertion is the one a hand-built
fixture cannot make honestly: a fixture written beside the reader closes because its author closed
it. `Collection::read` reads `digitally_signed_3D_Portfolio.pdf`'s eight schema columns and its
`/Folders` tree, and `folder_of` splits all five `/EmbeddedFiles` keys into §12.3.5.2's folder
number and file name — a producer's own keys rather than this reader's. Both are pinned in
`pdf-model/tests/{articles,collections}.rs`, skipping where the optional submodule is absent and
panicking where it is present and wrong.

**And most of it was sound**, which is the honest headline: `/Lock`, `/Trans`, `/Dur`,
`/PresSteps`, `/Hide`, `/Legal`, `/TrapNet`, `/DPartRoot`, `/FL`, `/PV`, `/PI`, `/Requirements`,
`/ReversedChars`, `/Alternates`, `/RF`, `/NoRotate`, `/CalCMYK`, `Adobe.PubSec`, §12.6.4.7's
`/Thread` action, §12.7.6.4's `ImportData`, §14.8.5.6's `/Checked`, §14.11.3's `/MarkStyle`,
§12.2's four boundary entries, every PAdES sub-filter, `DocTimeStamp`, `MacExpertEncoding`,
§8.11.4.4's `/Zoom`, `/User` and `/Language` — all re-measured, all absent.

**One survived a census that would have called it false**, and it is why the second instrument
exists. §12.9.2's number-format algorithm is recorded as having no corpus witness, and one document
does state a `/VP`. The row is right: `bug1146106.pdf`'s viewport is `GEO`, §12.10's geospatial
dictionary, and §12.9.2's arithmetic is `RL`'s — which `measurement.rs` says three paragraphs above
the claim. A name census at the wrong granularity manufactures a finding as readily as it finds
one.

**Not a gate.** `doc/todo/01` gains a sixteenth sweep instead, for ADR 0403's reason: a checker
would have to decide from prose which population and which entry each sentence is about, and one
firing on every "no corpus document" would fire 165 times for five findings. What makes the failure
not recur is that re-checking is cheap and named — two commands in `doc/verify.md`, three failure
shapes in the sweep, and `--names`, which turns "is there a witness for this entry" into a lookup.

ADR 0405. Gates: `cargo nextest run` 2119 passing (2117 plus the two new witness tests), clippy
clean over all targets, the conformance gate green, ledger 875 subclauses unchanged in status.
