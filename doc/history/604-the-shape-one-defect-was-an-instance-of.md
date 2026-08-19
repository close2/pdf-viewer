# 604 — The shape one defect was an instance of

Session 603 found a resource name that could not be looked up because it had been made into text
(ADR 0438). This round asked where else that lives, and answered it with a sweep rather than with a
recollection.

## The sweep, and the sweep that would not have worked

Two, and the round's finding is that only the second discriminates.

**Over the conversion**: every `from_utf8_lossy` in `crates/` and `tools/` — 180 sites — narrowed
to those whose own expression is a lookup or a comparison, plus the twelve case folds
(`to_lowercase`, `to_ascii_lowercase`, `eq_ignore_ascii_case` and the rest).

**Over the lookup**: every `Document::get_key` and `Dictionary::get` whose key is *not a string
literal*. The premise is a fact about this tree — ISO 32000-2's own keys are literals in this
source — so a key that is not one is a key the document supplied.

Discrimination was established rather than assumed: 603's defect was planted back by checking out
`content/resources.rs`, `run.rs` and `xobject.rs` at `d164416c^` into a scratch directory. **The
conversion sweep prints nothing over them** — the defect was two functions apart, and no grep over
one line joins them — and the lookup sweep names it five times over. That is why the first sweep
would have reported this tree clean.

## What it found

Six sites, each a name the *document* invents used to probe a dictionary the *same* document wrote:
a Type 3 glyph name in `/CharProcs` (§9.6.4), an `/AS` and a button's on-state in `/AP`'s `/N`
(§12.5.5, §12.7.5.2.3), a `cs`/`CS` operand and an inline image's `/CS` in `/ColorSpace` (§8.6.8,
§8.9.7), and a structure type in `/RoleMap` with a class in `/ClassMap` (§14.7.3, §14.8.6.2).

Each is two defects, and the second has no witness anywhere: the miss draws nothing, and the
collision draws something the file did not name. The Type 3 one is the sharpest — §9.6.4 makes a
name absent from `/CharProcs` paint nothing *and say nothing*, so a whole font's glyphs could go
missing with no report at all.

## What the clauses say, which was the round's other half

Read verbatim from `doc/md/`: §7.3.5 makes two names one when the expanded bytes are "an exact
binary match"; §7.7.4 sends every name-tree category to §7.9.6, which makes those keys *strings*
and then states the same rule for them — "keys shall be compared for equality on a simple
byte-by-byte basis". **Different type, same rule**, and this tree was already right about the second
one. No clause states a case fold or a normalisation for either. `spec-errata emit` over clause 7
has one annotation on §7.3.5 and it is about hexadecimal escaping in keys, which the lexer already
expands.

## What moved

`Document::get_key_by_name` is new; the six sites use it or `Dictionary::get_by_name`;
`Type3Font`'s encoding table is a `Name` rather than a `String`; `ColourSpace::by_name` matches
ISO's own families as byte literals. Every lossy conversion left in those files is at a report or a
trace label, which is the use §7.3.5 permits.

`crates/pdf-model/tests/names_are_bytes.rs` is new: a hand-built pair per vocabulary, each pair
differing only in the byte the rule is about.

**No gate moved**, which is the measured form of the reach: no corpus document contains a name
outside UTF-8. The naive check for one does not work and the round says so in ADR 0439 — a raw-byte
scan for `/` followed by a high byte calls 732 of the 974 documents positive, every one of them
compressed stream data.

## What it leaves

One site of the same shape, unfixed with an argument: the `/DA` string's font name. That module
*writes* the name as well as reading it, and its writer does no `#xx` escaping at all, so the read
and the write are one decision. `doc/todo/22` carries it with the full shape.

## Gates

The full §2 sequence, because the change is in `pdf-syntax` and `pdf-model`, and run whole a second
time after the last edit — the first run predated a rename that `cargo fmt` then reflowed, which is
exactly the case `doc/todo/02` §2's rule about running last is for.

fmt, clippy silent, 2216 tests, the doctests, the sandbox build, corpus, `pdfref-hayro`, oracle,
the three text-extraction gates, both censuses, dates, xmp, jpeg2000, quorra and conformance: all
green, and the oracle's contradicted and ambiguous counts are where 603 left them.

§5's binaries were **not** rebuilt: `tools/round.sh` flags them as older than `HEAD`, this is not a
fifth round, and this round measured nothing on the launch path. The round after a measurement owes
that rebuild first.
