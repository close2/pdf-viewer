# 0787 — A gap is not a prefix: §7.3 read for resynchronisation, and door 2 closed

Session 863. Status: **accepted**.

## Context

ADR 0784 read §7.3.7 and found that the entries of a damaged dictionary read whole before the
damage are **a subset of the dictionary's, every member of it the producer's own**, and built the
first door into one: a recovery takes such a prefix where the prefix itself declares Table 31's
`/Type /Page`. ADR 0786 built the second, where §7.7.3.2's `/Kids` names the object and the prefix
holds an entry only a page object may carry.

`doc/todo/03` §34 handed on a third, which it called door 2 and declined to argue:

> **Resynchronising past an unreadable value.** Two documents, and it is a bigger claim than the
> prefix rule: §7.3.7 states no extent for an entry's value, so a reader that skips to the next
> `/Name` has guessed where the bad value ended. The one thing in its favour is that no valid
> object begins with the keyword the guess steps over — but that is an argument about *these* files
> rather than about the clause, which is where it should stop until somebody reads §7.3 properly
> for it.

This is that reading. Its two witnesses are `GHOSTSCRIPT-698887-0.pdf`, whose object 2 is
`<< /Pare R /Type /Page /Contents 4 0 R >>` — a `/Type /Page` four bytes past the damage — and
`GHOSTSCRIPT-699695-1.pdf`, whose object 2 is
`<< /\xff \xff /\xff\xff\xff\xff\xff \xff /Resources 6 0 R /Contents 4 0 R /\xff… >>`.

## What §7.3 and §7.2.3 actually give, which is more than §34 credited

Three findings, and the first two say door 2's framing was too pessimistic.

### 1. Tokens are a layer below objects, and the standard says so

§7.2.1:

> At the most fundamental level, a PDF file is a sequence of bytes. These bytes can be grouped into
> tokens according to the syntax rules described in subclauses 7.2.2 , ' Representation ' through
> 7.2.4, ' Comments ' . One or more tokens are assembled to form higher-level syntactic entities,
> principally objects

So a token's extent is decided by rules that do not mention objects at all, and an object's extent
is decided by assembling tokens. Where §7.3.7 states no extent for a *value*, §7.2.3 still states
one for the token the value would have begun with.

### 2. A token's extent is stated, not guessed

§7.2.3:

> All characters except the white-space characters and delimiters are referred to as regular
> characters. These characters include bytes that are outside the ASCII character set. A sequence
> of consecutive regular characters comprises a single token.

and

> Any of these delimiters terminates the entity preceding it and is not included in the entity.

So a run of regular bytes has exactly one end, and the clause names it.

### 3. The set of tokens that may begin a value is closed

§7.3.1 enumerates the types and does not leave the list open:

> PDF syntax includes nine basic types of objects: boolean values, integers, real numbers, strings,
> names, arrays, dictionaries, streams, and the null object.

and each subclause states its own introducer — §7.3.2's `true` and `false` keywords, §7.3.3's "one
or more decimal digits" with an optional sign and period, §7.3.4's `(` and `<`, §7.3.5's "a SOLIDUS
(2Fh) (/) shall be used to introduce a name", §7.3.6's `[`, §7.3.7's `<<`, §7.3.8's dictionary
followed by `stream`, §7.3.9's `null`, and §7.3.10's two integers then `R`.

Together those three make **§34's own objection wrong on its own terms**. Where a dictionary's value
position holds a run of regular characters that is none of `true`, `false`, `null` and no §7.3.3
number, no object of any of the nine types begins there — so stepping over that run is not a guess
about a value's extent. There is no value there whose extent could be guessed at, and the run's own
extent is §7.2.3's to state. Both witnesses fail exactly this way: `R` and `\xff` are each a whole
token that begins nothing.

Door 2 could have been built on that, and this is where the argument would have stopped if the
reading had stopped at §7.3.

## Why it is refused anyway

### The sentence that bounds §7.2.3 is the one that decides

§7.2.3's second paragraph:

> The rules defined in this subclause apply to all characters in the file except within strings,
> streams, and comments.

Tokenisation is decidable **from a position known to be outside those three**, and a reader knows
it is outside them only by having tokenised continuously from one that was — the `<<` that opened
the dictionary. Every token it has consumed since either entered a string, a stream or a comment
(and it knows it did, from the `(`, the `<`, the `stream` keyword or the `%`) or did not.

**Resynchronisation is the deliberate surrender of that continuity**, and it surrenders the one
thing that made §7.3.7's subset argument work. ADR 0784's sentence is not "the entries before the
damage are readable"; it is that they are **the producer's own**. Continuity is what delivers that.
After a skipped token there is a gap, and whether the `/` on the far side of the gap introduces a
name or is a byte inside a literal string is no longer decidable — because the `(` that would have
said so is exactly the kind of byte the damage destroys.

### The counterexample, one byte wide

Two files differing in a single byte, both damaged, both with a tree that names object 2:

```
A   2 0 obj << /Note (junk /Contents 9 0 R more) /Rotate [0 >] >> endobj
B   2 0 obj << /Note Zjunk /Contents 9 0 R more) /Rotate [0 >] >> endobj
```

`A`'s prefix is one entry, `/Note`, whose value is a string; no entry of it is one only a page
object carries, so ADR 0786's door declines and the object is refused — correctly, because the
producer wrote no `/Contents`.

`B` differs only in that the string's opening `(` has become a regular byte. Under door 2 — §34's
own sketch of it, *skip to the next name after an unreadable value* — the token `Zjunk` begins no
object, `/Note` loses its value and therefore itself, and the reading resumes where a key belongs:
`/Contents 9 0 R` becomes an **entry**, `more` and `)` are skipped as the non-names they now are,
and `/Rotate`'s unclosed array stops the parse. The prefix is now `{/Contents 9 0 R}` — a Table 31
page-only entry — so ADR 0786's door fires and object 2 is taken as a page whose content stream is
object 9. The producer wrote those bytes inside a string.

That is the substitutive direction `doc/traps/parsers-and-streams.md` trap 5 forbids, and it is
worse than the usual shape of it: the manufactured entry is not noise the recovery tolerates, it is
the **discriminator the recovery acts on**. One byte of damage decides both that the object is a
page and what that page draws.

### Three ways of trying to save it, and why each fails

- **Require the resumed reading to reach the closing `>>`.** It makes the failure worse rather than
  better: an object assembled across a gap that then closes cleanly is no longer a
  `DamagedDictionary` at all, so it would go through `Document::get` and reach every reader in the
  tree instead of the one recovery that asks for a prefix by name.
- **Refuse where an unmatched `)` appears after the resynchronisation point.** The tell is
  destructible by the same damage. `GHOSTSCRIPT-699695-1.pdf`'s damage is *runs of `0xFF`
  overwriting arbitrary bytes*, and a mechanism that eats a `(` eats the matching `)` as readily —
  after which `/Note \xff\xff\xff /Contents 9 0 R \xff\xff\xff /Rotate …` manufactures the same
  entry with no unbalanced delimiter anywhere.
- **Take the witnesses' own evidence — that object 4 and object 6 in `699695-1` really are a
  content stream and a resource dictionary.** That is corroboration from the rest of the file, and
  it is what §34 already refused to accept: an argument about *these* files rather than about the
  clause. The second witness cannot even distinguish its own case from the counterexample, since
  its damage is precisely the mechanism that would destroy a `(`.

## Decision

**Door 2 is closed.** A dictionary's reading stops at the first value that will not parse, and the
entries whole before it are the prefix ADR 0784 defined. No resynchronisation, and the reason is
not that a token's extent is unknowable — §7.2.3 states it — but that **continuity from a known
position is what makes a prefix the producer's, and a gap ends it**. A prefix is a subset; a
reading across a gap is a guess wearing a subset's clothes.

`GHOSTSCRIPT-698887-0.pdf` and `GHOSTSCRIPT-699695-1.pdf` stay refused, with the standing `/Count`
and a refusal out loud, which ADR 0782 already makes the right answer for a page whose count is
known and whose bytes are not.

## Consequences

- The two witnesses do not draw, and no other document changes. Nothing was built.
- The refusal is pinned rather than remembered: `crates/pdf-model/tests/damaged_page_dictionaries.rs`
  carries the `A`/`B` pair above, so a future round that builds door 2 fails a test that names this
  ADR instead of rediscovering the counterexample on a corpus.
- `Parser::read_dictionary_body` and `PageIdentification` name the closed door where a round would
  be standing when it thought of it.
- **The general form is worth more than the clause**, and it is the third sentence in this family
  after ADRs 0343 and 0784: *ask what a prefix of the thing is* (0343), *ask whether the thing's
  parts are ordered* (0784), and now **ask what made the prefix the producer's**. Where the answer
  is byte continuity from a known position, no recovery may skip bytes and keep the guarantee — the
  bytes after the skip are a second reading of a second file.

## What this does not touch

`read_dictionary_body` already steps over a *non-name where a key belongs* and keeps going, and
that stays. It is not resynchronisation: no byte of it is inside an unparsed region, every token
before and after is accounted for, and the continuity above is unbroken. What it costs is at most
one entry the producer wrote, never one it did not.
