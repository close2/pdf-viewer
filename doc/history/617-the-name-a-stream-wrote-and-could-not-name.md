# 617 — The name a stream wrote and could not name

Session 604 swept the tree for one defect shape and fixed seven instances of it, leaving one with
an argument: the `/DA` string's font name, because the module that reads it also **writes** it and
the writer did no `#xx` escaping at all. This round settled both halves together, which is what
that argument asked for.

## What the clause states, which is the half nobody had read

§7.3.5 states escaping in two directions and this tree had implemented one. The reading half — a
name is its bytes, "an exact binary match" — was quoted in four ADRs. The writing half is rules a),
b) and c) plus two sentences that narrow them, and it lived inside `write::write_name`, private to
the object serialiser, where the one module in this tree that *builds* a content stream could not
reach it. `spec-errata emit` over clauses 7 and 12 moves neither half; §7.3.5's one annotation is
about hexadecimal strings and §7.3.7's confirms the reading.

## The two defects

**The read**: the `Tf` operand left the lexer as bytes and became a `String` through
`from_utf8_lossy` before probing `/DR`'s `/Font`, so a font name outside UTF-8 missed the
definition the document supplied and two such names were one.

**The write**, which is the serious one because §7.5.6 puts the result in a file somebody else
opens: `/{name} {size} Tf` written raw. `/Times#20New#20Roman` came out as `/Times New Roman`,
which §7.2.3 reads as a name and two keywords — while the stand-in font was registered in the same
stream's `/Resources` under the whole name.

## What moved

`pdf_syntax::Name::escaped` is new and is the only place in this tree that spells §7.3.5's writing
direction; the lexer is still the only place that spells the reading one. `write::write_name`
delegates to it. `variable_text` carries a `Name` from the `Tf` operand to the `/DR` probe to the
`/Resources` key, and writes all three through `escaped`.

**Two more sites of the same shape**, which is ADR 0439's question asked of this fix: the `/DA`'s
own operand names (`gs`, `cs`, `scn`), replayed into the constructed stream with the same hole; and
`appearance.rs`'s `/{name} Do`, where the name is this program's own so nothing was wrong today and
the `unwrap_or("Icon")` was the shape. The strings and the numbers this writer emits were checked
too and are right — the second for a reason that belongs to the lexer and is now written down.

## The population, which is not zero

`examples/variable_text_census` gained the count, learned to lex a `/DA` rather than split it on
white space — it would otherwise have missed exactly the construct it was counting — and learned to
walk directories, because the crawl is 66 000 files.

The corpus states no such name. The crawl states five, over two documents, both naming a font with
spaces in it — so the *write* half has the witnesses and the *read* half has none. All five are
free text annotations that carry an `/AP`, so no page draws differently: `display_list_digest` over
both documents is identical with the defect planted and with it fixed. What those five reach is a
save or an edit.

## How it is pinned

`crates/pdf-syntax/tests/name_escaping.rs` is new — the four shapes, Table 4, the round trip over
every byte a name may hold, one-token-ness, and three one-byte pairs.
`crates/pdf-model/tests/names_are_bytes.rs` gains the sixth vocabulary: the read pair, the
collision, and a write round trip through §7.5.6's update and back out through the lexer.

Each was planted back before it was believed, and each named its defect.

## Gates

The full §2 sequence, because the change is in `pdf-syntax` and `pdf-model`, run whole after the
last edit; plus `doc/todo/05`'s save round-trip instrument, which is what judges the path this
round's write defect was on. Every gate's numbers are in the run, not here.

§5's binaries were **not** rebuilt: `tools/round.sh` says this is not a fifth round and this round
measured nothing on the launch path.
