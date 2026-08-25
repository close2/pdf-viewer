# ADR 0694 — The copy a cached font was still paid for, and the table a lazy cell built in full

Status: accepted, 2026-08-25. Session 766. Interpreting page 101 of ISO 32000-2 costs **−7.52%**,
with the display list identical command for command and every gate unmoved.
Clauses: ISO 32000-2 §9.3.1 (`Tf`), §9.10.2 (a code's text through the Adobe Glyph List).

## How this was chosen

The rule these general rounds have converged on, sharpened by 757 and confirmed by 762: **find a
number this project wrote down and has not re-run, and prefer a composition to a total, because a
total is what a later round re-takes and a breakdown is not.** ADR 0687 applied it to
`callgrind_rasterise` four rounds ago. This is its mirror: `callgrind_interpret`.

The total is re-taken constantly — sessions 153, 162, 175, 185, 195, and in ADR 0677's own table
four rounds ago. The **composition** is one paragraph in `doc/performance.md`, and it is the
fifty-eighth session's:

> **Where interpretation goes on the median page** (session 58, the specification's own page):
> `zlib_rs::inflate` **28.0%**, `Interpreter::show_text` 6.5%, `Lexer::next_token` 5.1%,
> `inflate_table` 4.0%, AGL name lookup 3.2%

Seven hundred rounds. The baseline re-taken this session is **1 278 427 485** against ADR 0677's
**1 278 428 629** — 1144 instructions apart, which is the same tree measured twice and is worth
stating because it removes any doubt about which half of the pair had gone stale.

## What the composition says now

Same instrument, same page, same fifty repetitions:

| | session 58 | this session, before |
|---|---:|---:|
| `zlib_rs::inflate` | **28.0%** | 3.11% |
| `Interpreter::show_text` (self) | 6.5% | **13.83%** |
| `Lexer::next_token` (self) | 5.1% | 7.69% |
| `zlib_rs::inflate_table` | 4.0% | 0.54% |
| `read_fonts::ps::agl::name_to_char` | 3.2% | **6.52%** |

**The ranking is inverted.** The largest item in the tree by a factor of four is now the fifth;
the smallest has doubled and is the largest thing under our own control that is not the lexer or
the text loop itself. §7.4's decompression fell to a twelfth of its share because ADRs 0317, 0365
and 0429 happened to it; nothing happened to the Adobe Glyph List.

Two shares moved because the denominator did — interpretation of this page has fallen from session
195's 2 184.4 M to 1 278.4 M — so the absolute figures are what the table above should be read
with, and ADR 0370's rule is why: `show_text` is 176.8 M and `inflate` 39.7 M, and neither of those
numbers is a percentage of anything.

## The two things the tree was paying for, and neither was in a document

### 1. `Tf` copied the font dictionary for a load it did not do

`Interpreter::font` answered §9.3.1's operator like this:

```rust
let label = String::from_utf8_lossy(name.as_bytes());
let entry  = self.resource_entry(resources, "Font", name)…;
let key    = entry.as_ref().and_then(Object::as_reference).map(FontKey::Referenced);
let dict   = entry.map(|object| self.document.resolve(&object))
                  .and_then(|object| object.as_dict().cloned());
self.load_font(key, dict.as_ref(), &label)
```

The cache is inside `load_font`, which is correct and is where ADR 0115's argument put it — but
every line above it runs first. `Document::resolve` copies the font dictionary out of the object
cache, `.as_dict().cloned()` copies it **again**, and a font dictionary holds `/Widths`, an array
of up to 256 numbers. Page 101 states `Tf` **280 times for seven fonts**, so 273 of those double
copies were made, dropped, and never looked at.

Callgrind, per fifty renders:

| callee of `Interpreter::font` | calls | Ir |
|---|---:|---:|
| `Document::get` | 14 000 | 10.60 M |
| `drop_glue::<Object>` | 42 000 | 7.64 M |
| `drop_glue::<BTreeMap::IntoIter<Name, Object>>` | 14 000 | 6.71 M |
| `BTreeMap::clone::clone_subtree::<Name, Object>` | 14 000 | 6.23 M |
| `String::from_utf8_lossy` | 14 000 | 1.08 M |

**The fix is to ask the cache before paying for what a load would need**, and to stop the second
copy while the line is open — `load_font` wants a `&Dictionary`, so the resolved object can lend
one instead of being cloned into a new one. `load_font` still asks the cache itself, because it is
the authority and Table 57's `/ExtGState` route reaches it without coming through `Tf`.

After: `Document::get`, `from_utf8_lossy` and `load_font` are called **350 times** rather than
14 000, `clone_subtree` is not called from here at all, and `drop_glue::<Object>` is 14 700.
**1 278 427 485 → 1 247 561 146, −2.41%.**

### 2. The Adobe Glyph List table was lazy, and its entries were not

`LoadedFont::agl_by_code` is §9.10.2's second method memoised, and its own doc comment states the
argument for the laziness:

> Lazy rather than built at load, because a font whose `/ToUnicode` covers its codes never reaches
> the list at all, and 256 AGL searches is not a cost to pay on the page-one path for nothing
> (`CLAUDE.md` principle 2).

That argument is right and it was applied one level too high. The cell held a **whole 256-entry
table**, so the first code that reached this route resolved all 256 — and a page shows a few dozen
of them. The cost the comment declines to pay at load was paid in full by the first character
extracted from the font. `encoding::text_for` was called **67 200 times** over fifty renders;
the codes page 101 actually shows are **8 850**.

A cell per code, inside the cell that holds the array. The function and its argument are
unchanged, so the table this converges to is the one it used to build in one go — exact by
construction, not by tolerance. **1 247 561 146 → 1 182 345 844, −5.10%** of the original.

Together: **1 278 427 485 → 1 182 345 844, −7.52%**, and `name_to_char` falls from 6.52% to 4.47%
with only its two load-time callers left — `fill_substitute_table` and `truetype::named_glyph`,
both once per font, which is where the field's comment claimed the remainder was and is now right
again.

## The cost that was measured rather than assumed

The two cells cost one extra `OnceCell` check per glyph. Measured, because a nested cell is
exactly the kind of thing that gets waved through: an arm with the array allocated eagerly at load
— one cell, not two — is **1 179 692 330**, so the outer cell costs **2.65 M, +0.22%** on this
page. It is kept anyway, and the reason is that this page is the arm where it loses: every font
here reaches the route. A font whose `/ToUnicode` answers every code returns before it, and the
eager arm would allocate and zero 8 KiB for it at load — on the page-one path, for nothing, which
is the sentence the field's own comment is made of. Principle 2 decides it, with the price
written down.

## What says it is exact

- The example's own output — the display list's command count — is **150 350** before and after.
- The corpus gate, the oracle, `text_extraction`'s three gates, both censuses, quorra's corpus
  gate, `fixed_documents`, `dates`, `xmp`, `jpeg2000` and the conformance gate all pass unmoved.
  The text gates are the ones that matter here: change 2 is on the path a readback takes, and a
  code resolved at a different moment that resolved to something different would move them.
- Neither change touches a clause's meaning. §9.3.1 still selects the same font for the same
  operand, and §9.10.2's methods are still asked in the order and with the arguments they were.

## What this leaves

`Interpreter::font` is still **21.4%** of interpreting this page, and almost all of it is
`LoadedFont::load` — **seven font loads, 20.6%**, of which `embedded_program` is 2.51% and
`fill_substitute_table` 4.4%. That is a cost paid once per page per font by an `Interpreter` that
lives for one page, so a document read end to end pays it per page. Whether a font cache can
outlive an interpretation is a question about `Document`'s immutability and about memory, and it
is not this round's.
