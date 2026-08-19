# ADR 0439 — The shape a lossy name makes, and where else it lived

Status: accepted, 2026-08-19. Session 604. Generalises ADR 0438 from one defect to the class it
belongs to, sweeps the tree for the class, fixes the five further instances it found, and records
the one site left owed with its reason. Amends the ledger rows for §7.3.5, §7.9.6, §8.6.8, §8.9.7,
§9.6.4, §12.5.5, §14.7.3 and §14.8.6.2.

## The generalisation

ADR 0438 is one defect: a content stream's resource name carried as a `String` built with
`from_utf8_lossy`, so a name outside UTF-8 found nothing and two such names were one. **That is a
shape rather than an incident**, and the shape is stated as a rule:

> A byte string that becomes a `String`, a `str` or a `char` by any *non-injective* route — a
> lossy decode, a case fold, a trim, a normalisation — and then reaches a **decision** is a
> defect. Reaching a *report* is not.

§7.3.5 draws the line in exactly that place itself. The lookup side:

> Uniquely defined means that any two name objects that, after all escaping is expanded (see
> below), and the resulting sequences of bytes are not an exact binary match denote different
> objects.

and the text side, which is the permission the reports rely on:

> Ordinarily, the bytes making up the name are never treated as text to be presented to a human
> user or to an application external to a PDF processor.

## How the sweep was done, and the sweep that would not have worked

**Two sweeps, and only the second one discriminates.** This is the finding worth keeping.

The obvious sweep is over the *conversion*: every `from_utf8_lossy` in `crates/` and `tools/`,
which is 180 sites, filtered to those whose enclosing expression is a lookup or a comparison.

```sh
grep -rn 'from_utf8_lossy' crates/*/src tools/*/src --include=*.rs            # 180 hits
grep -rnE '(get|get_key|contains_key|insert|starts_with|==)[^;]*String::from_utf8_lossy' \
     crates/*/src tools/*/src --include=*.rs                                  # the one-expression form
```

**Run against a scratch copy of the three files ADR 0438 changed, at their pre-fix revision, that
grep prints nothing.** 603's defect was two steps: `run::name_at` produced the `String` in one
function and `resource_entry` consumed it in another, and no grep over a single line joins them.

The sweep that does discriminate looks at the **lookup** instead, and its premise is a fact about
this tree rather than about text: a dictionary key that ISO 32000-2 states is an ASCII literal in
this source — `"Type"`, `"Kids"`, `"CharProcs"` — so **a key that is not a literal is a key the
document supplied**, and there are only two right ways to hold one.

```sh
grep -rnE 'get_key(_of)?\([^,]+, *[^"]' crates/*/src tools/*/src --include=*.rs \
  | grep -v get_key_by_name          # a resolved lookup whose key is not a literal
grep -rnE '\.(get|remove)\(&?[a-z_]' crates/*/src --include=*.rs   # and the unresolved one
```

Run over the same scratch copy, that one prints `table.get(name)` five times in
`content/resources.rs` — 603's defect, named. Over the tree as it stood at the start of this
session it printed six more sites that are the same shape, and about two hundred that are not: a
slice indexed by a number, and a `key: &str` parameter every caller passes a literal to. Those
were read rather than filtered, which is what the second grep costs.

**The case folds were swept too** — `to_lowercase`, `to_uppercase`, `to_ascii_lowercase`,
`eq_ignore_ascii_case` — twelve sites, none of them a PDF name: a font family matched
heuristically for substitution, an HTML tag in `/RC` rich text, a file extension, a page label's
own uppercase alphabet, and the user's search box, which is *meant* to fold.

## What it found

Six sites, each a name the **document** invents used as a key into a dictionary the **same
document** wrote — which is precisely the condition under which a fold can both miss and collide.
A name ISO 32000-2 defines is not in the class, because a fold of an ASCII literal is injective
over ASCII: that is why `by_name`'s device families are compared against byte literals here and
nothing more was needed for them.

| site | vocabulary | clause |
|---|---|---|
| `type3.rs` | a Type 3 glyph name, looked up in `/CharProcs` | §9.6.4 step b) |
| `annotation.rs` | `/AS`, looked up in `/AP`'s `/N` | §12.5.5 |
| `appearance.rs` ×2 | a button's on-state, in the same subdictionary | §12.7.5.2.3 |
| `colour.rs` | a `cs`/`CS` operand, in `/ColorSpace` | §8.6.8 |
| `inline_image.rs` | an inline image's `/CS`, in the same place | §8.9.7 |
| `structure.rs` ×2 | `/S` in `/RoleMap`, a class in `/ClassMap` | §14.7.3, §14.8.6.2 |

Each is two defects. **The miss** draws nothing where the file states something: a Type 3 font
whose glyph names carry such a byte paints no glyph at all, and §9.6.4 makes that silent — "If the
name is not present as a key in CharProcs , no glyph shall be painted" — so it is not even a
report. **The collision** is the worse one and has no witness anywhere: every invalid byte folds to
one replacement character, so `/g#F4` and `/g#F5` were one glyph name, one appearance state, one
colour space.

`Document::get_key_by_name` is new and is what the resolved lookups probe with; the unresolved ones
use `Dictionary::get_by_name`, which has existed since the parser was written.

## The other name-like vocabularies, and the clause each one answers to

The round asked whether §7.3.5's rule is the only one, because it is not the only vocabulary.

- **A name tree's keys are `string`s, not names** — §7.7.4 sends every entry of Table 32 to §7.9.6,
  which says "Unlike the keys in a dictionary, which are name objects, those in a name tree are
  strings". A different *type*, and the same rule: "Any encoding of the keys may be used as long as
  it is self-consistent; **keys shall be compared for equality on a simple byte-by-byte basis**."
  This tree was already right — `TreeKey::Name` carries `&[u8]` and `Destination::named` probes a
  catalog `/Dests` with `get_by_name` — and the row now quotes the sentence that makes it right
  rather than leaving it to be inferred.
- **A field's name is a text string and is joined with a full stop** (§12.7.4.2), so its equality is
  a *different* question again and is not this one; `form.rs` groups widgets by the joined name and
  is out of scope here.
- **A glyph name inside a font program** is the CFF or Type 1 format's vocabulary rather than ISO
  32000-2's, and `pdf-font` folds *both* sides of that comparison the same way, so the miss
  direction cannot occur; the collision direction can, and is recorded below.
- **No clause anywhere states a case fold or a Unicode normalisation for any of them.** That was
  looked for and not found, which is the answer rather than the absence of one: §7.3.5 says binary
  match, §7.9.6 says byte-by-byte, and the only normalisation ISO 32000-2 states in this
  neighbourhood is `#xx` expansion, which the lexer does before a `Name` exists.

## What each fix can reach, measured

- **The corpus cannot see any of them.** The full §2 sequence was run before and after: no verdict,
  no page, no report and no count moves in the 974-document corpus, the oracle, the three
  text-extraction gates or either census. That is the same measurement ADR 0438 recorded for the
  resource-name site, taken with the same instrument, and it is the honest form of the answer: a
  name outside UTF-8 is not a construct the pdf.js corpus contains.
- **The crawl's rate is 1 in 2000 for the *resource* vocabulary**, measured in session 603, and it
  is the only vocabulary with a measured rate. The five new sites have **no witness on this disk**,
  and the naive way to look for one does not work: a raw-byte scan for `/` followed by a high byte
  finds 732 of the 974 corpus documents, every one of them a false positive inside compressed
  stream data. Counting them honestly needs a walk that parses each document and classifies a name
  by the entry that names it — `damaged_stream_census`'s `who_names_what` shape — which is a round
  of its own and is what `doc/todo/03` §16 already asks for.
- **The collision direction is unmeasurable by any corpus**, in the way trap 8 states: it is a
  property of the *reader*, and a document that would expose it is one nobody has written. It is
  pinned by hand-built pairs instead — `crates/pdf-model/tests/names_are_bytes.rs`, one pair per
  vocabulary, each pair differing only in the byte the rule is about.

## What is left owed, and why it is not attrition

**One site of the shape is not fixed: the `/DA` string's font name** (`variable_text.rs`). A
default appearance string is a content-stream fragment, its `Tf` operand is a name, and this crate
tokenises it into a `String` before looking the font up in `/DR`'s `/Font`. So a `/DA` naming a
font outside UTF-8 gets the stand-in font and a report, and two such names are one.

It is left because **fixing the lookup alone would break the appearance**, and that is an argument
rather than an excuse. This module does not only *read* that name — it **writes** it, into the
appearance stream it constructs (`/{name} {size} Tf`) and, for an invented font, into that stream's
own `/Resources`. Writing a name is §7.3.5's *other* half, the `#xx` escaping this code does not do
at all today: a `/DA` font name containing a space or a `#` already produces a stream that names
something else. So the fix is one decision covering the read and the write, and it belongs with the
escaping question rather than beside a one-line probe. `doc/todo/22` carries it.

## The rule this leaves behind

`Document::get_key_by_name` exists so that the sweep above stays cheap for the round after this
one: **a lookup whose key is a literal is ISO's, a lookup whose key is a `Name` is the document's,
and a lookup whose key is neither is the thing to look at.** The greps are in this file so that
they can be re-run, and the discrimination test is in it so that a later round can check they still
find something — plant 603's defect back into a scratch copy of `content/resources.rs` and confirm
the second grep names it.
