# 1164 — A departure's argument gets a reader

Instruments round of batch twenty-four, on the owner's `doc/adr_revisit/` note about ADR 1119. ADRs 1165 (the sweep), 1166 (the instrument).

## The sweep, by hand

Twelve of the fourteen `departed` rows re-derived against the clause in `doc/md/`, the deciding ADR and the tree today —
§10.7.5 and §12.5.2 are siblings'. **Eleven held and one had expired.**

§12.11.6 moves `departed` → `partial`. ADR 0460 declined the clause's refusal on `CLAUDE.md` principle 3's shape and said the
four restriction levels were "a change in the host and nothing below it". That change was made for every other restriction a
document asserts — `restriction::Level`, `RestrictionPolicy` per `Operation`, `Refused`/`Asking`/`Warned` — and §12.11.6's
threshold was attached to none of it, so it is the one restriction a reader cannot turn on, be asked about or be warned about.
Owed is the policy question, not the refusal; the host work is named in `doc/todo/65`.

Each of the other eleven notes gained a sentence saying what was re-derived against what. §7.4.8's premise is a count over the
974 and the crawl has never been asked that entry (a census, named in `doc/todo/65`); §8.6.5.7's rests on the output device,
the one premise of the twelve that would expire if a path to a device with other colourants arrived.

## The instrument

`cargo run -q -p conformance --bin departures`, and `tools/state.sh departures`. Per `departed` row: the ADR its note's first
sentence names, the ones it names elsewhere, and every later ADR citing that argument or the row's clause number. Nothing is
scored — a program that judged a premise would be believed. `Problem::ArgumentMissing` fails `cargo test -p conformance` where
a `departed` row names an ADR with no file; `ArgumentsUnreadable` where the directory will not open. `doc/todo/01`'s twenty-fifth sweep.

## The TOML reader, and what was actually wrong

The suspected gap — a bare `"` in a note accepted silently — **is not there**, and four placements prove it: a basic string must
be the whole value, so anything after the early closing quote is `nothing after the value`, and in an array `a comma between
array items`. What was wrong is that the refusal named a line out of 883 rows and not the row; `Ledger::parse` names the clause
above it now, which a neighbour's note proved at §14.7.2 within the hour.

## `doc/todo/65`, and what is left open

Buckets 4 and 6 re-derived from the live ledger: bucket 6 had lost five rows to `implemented` and keeps two, bucket 4 gains the
three the map's own "what left" section had moved there and never listed (§8.9.6.4, §12.7.8.3.1, §12.7.8.3.2), and §12.8.3.4.4
is still owed a line in bucket 2. `doc/questions/Q72` asks what the sweep could not settle — where a clause names a mark and
states no quantity, is reporting the documented choice `CLAUDE.md` describes, or is a number of this project's own owed?
