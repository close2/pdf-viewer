# 0478 — The refusal three rows had already been taught to prove

Status: accepted.
Session: 648. Follows ADR 0472, which read this band's lower half and left three rows named in
`doc/todo/01`. Follows ADR 0460, whose end-to-end click test is the thing two of this round's rows
should have been pointed at twenty-two rounds ago. Follows ADR 0455 for the rule that chose the
reading — *rank by `git blame` over each `note =` line, then read the row whose stated reason is a
claim about this codebase rather than about the standard*.

## The decision

**Four things, and none of them changes a status.** Every row read here keeps the status it had;
what changes is whether its stated evidence reaches its stated claim, and whether its numbers can
be produced by anything.

1. **Two `reported` rows are pointed at a test that can fail if the report stops.** §12.6.4.3's
   remote go-to and §12.7.6.2's submit-form both cited
   `action.rs::a_name_the_table_does_not_hold_is_not_an_action`, which asserts that `/Teleport`
   yields *no* action and therefore never calls `action::refused`. Both now cite
   `headless.rs::a_click_on_an_action_this_program_will_not_perform_says_which_and_why`, which the
   round extends from three action types to five, and both `code` arrays name the whole path the
   notes claim — `pdf-model`, `viewer_core::interact`, `viewer-ui`'s `dispatch.rs`.

2. **A census of Table 201's action types**, `pdf-model/examples/refused_action_census`. It asks,
   per action type, how many dictionaries the corpus states and what `action::read` answered for
   each — the standard's table on one side and the reader's own verdict on the other, so that
   neither side is a copy of the other.

3. **`der::Value::had_indefinite_length`**, a public accessor that makes this reader's deliberate
   tolerance for X.690's indefinite lengths *measurable*. `signature_algorithm_census` grows two
   counters on it and on §12.8.2.2's `/Perms /DocMDP`.

4. **Four counted claims are re-derived**; three hold and one was wrong.

## Why the refusal needed a test at all, and why two rows went without one

`reported` is defined in the ledger's own header as "not implemented yet, but detected and
**reported at runtime**". Half of that status is a negative and needs no test; the other half is a
positive assertion about what a person sees, and it is the half that decays silently — a refusal
can stop being reached by a click, or its sentence can change, and nothing fails.

The six-hundred-and-twenty-sixth session found exactly this for §12.6.4.6, §12.6.4.9 and
§12.6.4.10, and wrote the test that fixes it. It did not ask which *other* rows rested on the same
function. `action::refused` has ten arms; five of the types it names carry a `reported` row and
four carry an `out-of-scope` one, which owes nothing. Three of the five were covered and two were
not — and the two kept citing the dead test, so nothing said they were uncovered.

**The general form is the part to keep.** When a round fixes a defect a family shares, the
population it has fixed is *the arms of the function*, not the rows it happened to be reading. The
ledger's ordering could not have found §12.6.4.3: its note has been rewritten since, so it ranks
nowhere near the top of any staleness order. Enumeration found it in one query.

Mutation-checked rather than asserted: with both refusal arms made to return `None`, the extended
test fails naming the action, and it passes with them restored.

## Why the census walks inside objects, and why that is a finding

`structure_destination_census` walks every object the cross-reference table lists, for a good
reason — an action hangs off a link annotation, an outline item, a catalog `/OpenAction`, a
widget's `/AA` and another action's `/Next`, so a census that visited any one of those would
measure the walk rather than the corpus. This census was written the same way and its first run
reported **zero** `/S /GoToR` and **zero** `/S /SubmitForm`, which would have said that neither of
the two rows being fixed describes anything a reader ever meets.

A `grep -l` over the corpus's raw bytes contradicted it: two files hold `/GoToR` and one holds
`/SubmitForm`. Both are written *directly* inside the annotation or outline item that owns them,
and a direct dictionary has no object number to be found by. So the census walks each numbered
object's body as well — every nested dictionary and array, never through a `Reference`, because
what a reference names is another numbered object with its own turn and following one would count
a shared action once per outline item pointing at it.

The same bound was behind the wrong count in ADR 0460's own comment: "of the 974 corpus documents
exactly one states a `/S /Launch` action" missed `externalLink.pdf` for precisely this reason.

**The rule this yields** is in `doc/todo/01`: a count over the corpus is a claim about a walk as
much as about the world, and the cheapest probe of a zero is the crudest one.

## The four counts

Over the 1249 documents of `doc/pdf.js/test/pdfs` and `doc/corpora`, 1237 of which open:

| the claim | where it stood | re-derived |
|---|---|---|
| §12.8.2.2: "the corpus's one certification signature states `/P 2`" | ledger, no command | **holds** — one, `xfa_filled_imm1344e.pdf`, at level 2, which the row had never named |
| §12.8.3.4.2: "four corpus documents write [indefinite lengths]" | ledger, no command | **holds** — and so does `der.rs`'s differently-denominated "four of the ten signature values"; each of the four documents holds exactly one such value |
| §7.6.4.1: "eight corpus documents reach it" | ledger, no command named | **holds** — and the command already existed, as the corpus gate's `MAX_LOCKED` ratchet |
| ADR 0460: "exactly one states a `/S /Launch` action" | a test comment, no command | **wrong** — two, for the walk reason above |

§7.6.4.1's is the one worth generalising in the other direction: **a note whose count already has a
command owes the command's name, not a new census.** Three rounds have now written an instrument;
this one nearly wrote a fourth before finding the gate that had been printing the number all along.

## Two findings beside the rows

**`action::refused`'s doc comment said "Table 201's other seventeen types"**, and no reading of the
table produces seventeen: ISO 32000-2's Table 201 lists twenty, `one` performs eleven, nine are
left with no arm, and the function names ten because `Thread` sits on both sides. Corrected to what
the function holds, with the arithmetic written down.

**An erratum was filed two clauses from where it belongs.** `spec-errata emit` printed Issue #469 —
`shall be` struck, `is` inserted — under `## 7.6.4.1 General`, and §7.6.4.1's row is one this round
was reading. Reading the strikeout's `/Rect` against page 91's text boxes puts it on "[t]he number
of bytes to be encrypted or decrypted **shall be** given by the Length entry in the stream
dictionary", which is §7.6.3.3's. `emit` files a note under the clause §12.3.3's outline puts its
*page* in, and page 91 opens inside §7.6.3.3 before beginning §7.6.4.1 — so a note two clauses from
its printed heading is the normal case rather than the odd one. Recorded in §7.6.3.3's row, whose
note already quotes the sentence beside it. The change itself is a `shall` becoming a statement of
fact and costs this reader nothing: `/Length` is how much stream there is either way.

## The cost

`Value` grows a `bool`. `Value` is `Copy` and built once per ASN.1 value while reading a signature,
so this is one byte on a struct that already carries a slice, a `u8` and a `u8`, inside a path that
runs a handful of times per signed document — measured against nothing because there is nothing to
measure. The accessor is `const` and reads a field.

The census is an example rather than a gate, deliberately: it answers a question about the world,
and `CLAUDE.md`'s two-denominators table puts that on the robustness side, where a ratchet would
freeze a corpus rather than a requirement.

## What this does not do

It does not change any status, close any refusal, or move a pixel. §12.6.4.3 and §12.7.6.2 stay
`reported` for the reasons their notes already gave — a second `Document` in `viewer-core`'s
vocabulary and a host's decision about which files a document may name for the first, a network for
the second — and both reasons are capabilities rather than restrictions, so none of `CLAUDE.md`'s
four levels would turn either on.
