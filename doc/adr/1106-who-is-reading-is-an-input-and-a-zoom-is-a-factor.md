# 1106 — Who is reading is an input a host supplies, and a change of zoom reapplies the dictionaries

Session 1092. Status: **accepted**.

ISO 32000-2 §8.11.4.4 gives seven usage categories rules, and two of them ask this *processor* a
question no document can answer:

> User : The Name entry shall specify a name or names to match with the user's identification. The
> Type entry determines how the Name entry shall be interpreted (name, title, or organisation). If
> there is an exact match, the ON state shall be used; otherwise OFF shall be used.

> Language : This category shall allow the selection of content based on the language and locale of
> the application.

§8.11.4.5 then writes one more sentence that no part of this tree reached:

> Whenever there is a change to a factor that the usage application dictionaries with event type
> View depend on (such as zoom level), the corresponding dictionaries shall be reapplied.

Six ledger rows stood `partial` on those two sentences. This ADR decides both, and it decides them
the same way, because they are the same shape: a clause that reads a fact about the machine, and a
`CLAUDE.md` principle 3 rule about where such a fact may come from.

## 1. "otherwise OFF" is the answer to a comparison, not to there being nobody to compare with

This tree answered `User` and `Language` with `Recommendation::Unanswerable` and called that a
departure from the clause, in eight separate notes, for five hundred sessions. **It was not a
departure, and reading the sentence carefully is what says so.**

"If there is an exact match, the ON state shall be used; otherwise OFF shall be used" has an
antecedent: *an exact match* — and the clause says what the match is against, "the user's
identification". The sentence has two branches and both are inside a comparison. A processor that
has never been told who is reading is not in the second branch; it is outside the sentence, because
the thing the comparison is made against does not exist. The same holds one bullet down: `Language`'s
three cases — exact match, partial match, "[a]ll other groups" — all begin with "the language and
locale of the application", and a machine nobody has told has no such thing.

The consequence matters and it is why the reading is worth the words. §8.11.4.4's rule above these
bullets is an AND — "[i]f all the entries yield a recommended state of ON , the group's state shall
be set to ON ; otherwise, its state shall be set to OFF" — so a category answering OFF turns the
*group* off. Reading "otherwise OFF" as covering the no-answer case would hide a producer's content
on the strength of a question nobody asked, on every document that states a `/User` category, for
ever.

So the division is the clause's own:

- **With a host answer**, §8.11.4.4's sentences decide exactly as written. An exact match is ON, any
  other identification is the clause's own OFF, and the group goes off through the AND.
- **With none**, the category is `Recommendation::Unanswerable`, the configuration's initial state
  stands, and the page reports which category it could not answer.

The second is what this program did all along. What changes is that the first is now reachable, so
the clause is met rather than declined, and the rows move off `partial` to `implemented`.

**Three lists of names, not one.** Table 100 makes `/Type` decide what the names beside it mean —
"either Ind (individual), Ttl (title or position), or Org (organisation)" — so a host that said who
the individual is has not said which organisation they belong to. `Reader` keeps them apart, and a
`/User` asking about a type nobody answered stays *unanswered* rather than becoming a match that
failed. A `/User` whose `/Type` is none of the three states nothing to match against, and is treated
as a usage dictionary with no `/User` at all.

**`Language` is evaluated over the list and not over the group**, because the clause says so: the
match is sought "among the Lang entries of the optional content groups in the usage application
dict ionary's OCGs list", and only where no exact match is found anywhere does `Preferred` decide
among the partial ones. "All other groups shall receive an OFF recommendation" is taken literally,
including a group in the list with no `/Language` entry at all — this is the one place the clause
states the OFF outright rather than leaving an absent entry unaffected, and the contrast with its
own `Zoom` example ("Object 4 has none; therefore, it is not affected by zoom level changes") is
what makes it a reading rather than an oversight. Comparison is case-insensitive on §14.9.2.2's
authority: "all language tags shall be treated as case-insensitive".

## 2. The surfaces are ADR 1076's and ADR 1101's, unchanged

A document that could assert who is reading would be choosing its own audience, which is the same
hazard ADR 1101 names for a file specification and ADR 1039 for a trust anchor. So the answer is a
host's, asked once, in the places a host can supply:

- **`pdf_model::optional_content::Audience`** holds it, with `Audience::NONE` the default that every
  caller saying nothing gets.
- **`ViewState::set_audience`** is the only channel into interpretation, beside `set_magnification`
  and `set_widget_appearances`, for `viewer_core`'s rule 1.
- **`viewer_core::Command::Audience`** carries it, on `Command::Trust`'s rules: it applies to every
  open document and every one opened afterwards, because it is a fact about the *reader*.
- **`viewer_host::audience`** is the one place a host answers, beside `trust_anchors` and
  `reference_files` and the seven decisions before them. `--reader-name`, `--reader-title`,
  `--reader-organisation` and `--interface-language` are the words.
- **`viewer-confined`** carries it as command kind 30.
- **`quorra_audience`** is the C ABI's. It takes no struct by value, so `QUORRA_ABI_VERSION` does not
  move.

**Nothing is read off this machine, and that is the decision rather than an omission.** A host could
take the language from a locale and the name from a login. Both would be this program deciding on a
reader's behalf what a *document* is told about them — ADR 1039's refusal repeated — and both would
put an environment read in front of a mark, which principle 3 forbids outright. A person says it or
nobody does.

## 3. The reapplication is a property of the magnification changing

§8.11.4.5's `shall` was "measured rather than met": the magnification has reached `ViewState` since
ADR 0168, and the reason the row gave for not acting on it was that no document names the `Zoom`
category. That is still true (section 4), and it is not a reason — a clause is owed on the
standard's say-so and not the corpus's.

**It happens inside `ViewState::set_magnification`, not in a second call a caller has to remember.**
A reapplication a host must ask for is one a host can forget, and then conformance is a property of
each caller's diligence rather than of the code. The cost of putting it there is what made it
possible: `OptionalContent::read` resolves the `/View` usage application dictionaries once — each
one's `/Category` names and each group's Table 100 entries — so reapplying reads no document, and a
document that states no `/AS` at all costs one empty-vector test per zoom step. `launch_path` does
not move, because nothing new happens when a document opens.

**Two consequences the clause and the tree each force:**

- Every group an application names starts again from §8.11.4.5 b)'s *initial* state — after
  `/BaseState` and the array opposite it — rather than from the answer to the last zoom, so the
  result is a function of the factors and not of the order the steps arrived in.
- A group a person switched is pinned and never readjusted: "[m]anual changes shall override the
  states that were set automatically. The states of these groups remain overridden and shall not be
  readjusted based on usage application dictionaries with event type View as long as the document is
  open." Both routes the clause names in the sentence before it pin — a panel's `Command::SetGroup`
  and §12.6.4.13's action — and so does a radio-button exclusion made as a consequence of one,
  because turning an excluded member back on would show two members of one collection.

**And the answer separates two obligations that were one `bool`.** §12.5.3's `NoZoom` moves an
annotation without changing what the page draws, so a host holding a picture of the old
magnification may go on standing in with it (ADR 0775). A reapplication that switches a layer off
changes the ink, and that picture is then of something else. `Magnified` is `Unchanged`, `Placement`
or `Visibility`, and `viewer_core` calls `reinterpret` for the second and `stale` for the third.

## 4. What holds it, and why no corpus document could

`examples/oc_usage_census` now reads the `/Usage` entries of **every group `/OCProperties /OCGs`
declares** and prints the values of the three categories, where before it read only the groups a
usage application dictionary named — two different populations, and the wider one is the one a
fixture has to be built against. Over 963 of the pdf.js corpus's documents that open and 65 720 of
the crawl's: not one group states `/Zoom`, `/User` or `/Language`, under either population. The
instrument was calibrated by planting a document that states all three, which it names and whose
values it prints (trap 13); the zero is therefore about the corpus and not about the census.

So the fixtures are the evidence, and each was calibrated by planting the defect it guards against
(trap 8, trap 13): the reapplication made to return early — caught by three tests; the manual
override never recorded — caught by one; `Preferred` ignored in a partial match — caught by one; the
`/User` `/Type` ignored so that any name answers any question — caught by one.

`raster_golden` does not move and could not: no gate supplies an audience, and none states a
magnification, so every group is answered exactly as it was.

## 5. What this does not decide

Nothing here chooses a value for anybody, which is ADR 1039's refusal repeated for the third time:
building the input is not choosing what goes in it. Nor does it reach §8.11.4.4's `Print` and
`Export` events, whose changes "persist only for the duration" of operations this program does not
perform; §8.11.4.5's row keeps that residue, named.
