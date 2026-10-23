# 2260 — The Status line and the evidence rule are held by tests, and a numbering gap is declared, never filed

Session 1713, slot 2 of batch one hundred and ten, a docs round. Status: **accepted**. Context: ADR
2242 decisions 3 and 4, ADR 0265 (an ADR's state moves in its Status line), ADR 1100 (a cap nothing
counts is exceeded), review 1704 findings 3, 5, 7 and 8, trap 13. It cites no clause and moves no
ledger row. ADR 2261 is unused.

## 1. Decisions

1. **The Status line is read by `cargo test -p conformance --test adr_status`, from ADR 2242 on.**
   The opening, before the first `## `, carries `Status:` and the decision's state, whose first word
   is `accepted`, `rejected`, `deferred`, `superseded`, `amended`, `implemented` or `proposed`; the
   state is read to the end of its sentence, and "pending", "under validation" and a `proposed` that
   names no `Q` are refused, because each describes the merge and the merge never edits the line.
   The seven words are the ones ADRs 2100–2257 already use for a decision (`implemented` is a docs
   round's "accepted and built", `deferred` ADR 2189's own word); an ADR waits on the owner and on
   nothing else, so `proposed` names the question. It is a population of its own rather than a
   widening of `conformance::documents`: that module's `§` rule reaches every file it reads, and a
   record may not be pointed at, so `no_record_is_in_the_population` stands. The ADRs from 2178 that
   do not meet the rule are printed as before it.
2. **Twenty-one ADRs of 2178–2241 got the line the format owes**, each stating its state at the
   merge that took it, by commit, and nothing else in them changed. Twenty-one, not the review's
   nineteen: 2188 and 2189 wrote "Accepted:" and "Deferred:" and no `Status:`. Sixteen had none and
   five were frozen; a frozen line keeps the round's words in a second sentence. Four are
   `assessment` (2215, 2220, 2221, 2237), the four of finding 5's nine that had no line; 2221's
   also says its acceptance of the red launch gate is superseded by ADR 2242 decision 2. The other
   five of the nine had a line, which is before the rule and is not rewritten.
3. **What a record relies on is held by `--test evidence`**, over the records and reviews from
   session 1712 and the ADRs from 2258. A sentence naming a path under `/home/AI/` or `scratchpad/`
   is a workplace only when it says the session "worked in" or "worked under" it and every such
   path is a directory written with its `/`; any other names evidence, and is refused, while a
   marker on its own names the kind of place and points at nothing. The use is
   in the sentence's words so that a reader and the test read the same thing, and a file is never a
   workplace, because a sentence names a file for what it holds. A batch commit's message is under
   the same rule and outside the tree, so `doc/todo/02` section 8 carries it.
4. **A number a slot does not take is declared in its record's opening and never filed**; the
   four gaps review 1704 names (2205, 2238, 2239, 2243) are listed in `doc/todo/02` section 6,
   beside the rule, each with whose it was. No test reads the gaps: mid-batch the highest ADR is often a later slot's, so every
   number below it not yet written would read as a gap and the test would be red for each round
   until the batch's last record landed.
5. **`doc/todo/75` item 3's figures and gate verdicts became the two commands that print them**
   (`zoom_frame`'s `lanes:` line, `replay_line_census`), and its `/home/AI/` path went to the ADR
   that already names it (finding 7).

## 2. What it costs

- A workplace sentence that names a file is refused; naming the directory is the remedy.
- The check reads words, not intent: a directory of evidence called a workplace passes, and the
  review reads the rest. A full stop inside a sentence ("e.g. ") can only split a workplace from
  its words, which refuses; nothing joins two sentences, so nothing passes by it.
- A round that writes an ADR without the line, or names an arms directory as its comparison, is
  red at `cargo test -p conformance` until it says what the tree holds instead.
