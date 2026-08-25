# ADR 0620 — The entry a list audited three times never named

Status: accepted, 2026-08-25. Session the seven-hundred-and-thirtieth, a clause round under
`doc/todo/01`, reading one family's `partial` rows against each other as well as against the code —
ADR 0538's method in its ninth round (0551, 0560, 0567, 0579, 0593, 0600, 0610, this), with 710's
two rules for reading the ranking and 0593's third. Amends §8.6.5.8 and §8.9.5.1 in the ledger;
corrects two doc comments on `content::colour::Intent`; adds `content/xobject.rs` to §8.9.5.1's
`code` array. **No status moves, no pixel moves, and no
report is added or removed.** Extends ADRs 0009, 0034, 0295, 0319, 0380, 0417, 0456 and 0468.

## 1. The pair, and why the ranking gave it

The search was run on this base rather than read out of any document. Its head is unchanged — §12.5
first, §12.8 second — and once the clause-level parents are stripped the three strongest pairs
anywhere in the ledger are the same three ADR 0610 §1 names: §12.4.4 ~ §12.4.4.1, §12.8 ~ §12.8.3
and §10.7.4 ~ §10.7.5. All three are spent. 0600 read the first, 0567's round wrote §12.8.3 and its
whole subtree, and 0610 read the third — so 0593's third rule, *take the strongest pair the previous
round named and did not read*, has nothing left above the fourth rank and the fourth rank is a
**tie**: §11.4.7 ~ §11.7.2 and §8.6.5.8 ~ §8.9.5.1 both score 31 shared rare five-word sequences.

0579's rule breaks it, and it is the rule that ranked this method's last three findings: prefer the
pair where the two rows do not merely quote the same sentence but **disagree about what it leaves
standing**. §11.4.7 and §11.7.2 share a long narration about the press budget and §11.7.2's
non-isolated inheritance quotation, and they agree everywhere. §8.6.5.8 and §8.9.5.1 share the
sentence about a literal `true` at six `Compositing::paint` calls, and they do not:

- §8.9.5.1 says the flag reaches every conversion now and that the six literal `true`s are gone;
- §8.6.5.8 says, in its **opening list**, that "the third, an image dictionary's `/Intent`, is not"
  read — and then closes it four hundred words later;
- and §8.9.5.1 cites *that opening sentence* as current: "§8.6.5.8's row already records that this
  third route to the rendering intent is unread".

So one row's correction was cited by the other as a live claim, which is 710's shape read from the
far end. That disagreement is the whole reason the pair scores, and it was the first thing the
reading confirmed.

## 2. What was wrong

### `/OC` — the fifth hole in a list two sessions had audited for holes

§8.9.5.1's row is a disposition of Table 87: every entry named, and every one placed as read,
unread or a boundary. Its own prose says why that is the shape it has — "[a] list that has been
wrong three times about itself is a list to check rather than to read" — after `/Interpolate`,
`/Alternates` and `/Mask` had each been recorded unread while the tree read them. The
five-hundred-and-twenty-fifth session checked the unread five; the five-hundred-and-eighty-second
found "a fourth hole — four entries it disposed of neither way" and named `/Intent`, `/AF`,
`/Measure` and `/PtData`.

**There were five.** Table 87 states `/OC` and no sentence of the row disposes of it. The word
occurs once, in the `/Alternates` sentence — "`Interpreter::alternate_image` walks the array,
honours each entry's `/OC` and draws the first one shown (§8.9.5.4)" — and that is Table 89's key
on an *alternate image* dictionary, not this table's key on the image XObject itself.

The entry it left out is the one that decides whether the image is drawn at all. Table 87 puts two
`shall`s on it:

> Before the image is processed by a PDF processor, its visibility shall be determined based on
> this entry. If it is determined to be invisible, the entire image shall be skipped, as if there
> were no Do operator to invoke it.

Both are executed, and in neither of the two files the row lists as its code: `content/xobject.rs`
reads `/OC` off the stream dictionary **unresolved** — §8.11.2.2 identifies a group by which object
it is — asks `shows_optional_content`, and returns before the `/Alternates` are examined, which is
§8.9.5.4 step a) as Errata Collection 3 amends it. So this is a documentation defect and not a
drawing one; what it costs is that a reader of the row would conclude the base image's own optional
content is unhandled, when it is handled and §6.3.2.2 makes it one of a rendering processor's three
obligations.

### The two keys the list opens with that are not Table 87's

The same sentence opens "Table 87's entries, and which of them are read" and ends the list with
`/Filter` and `/DecodeParms`. Those are **Table 5's**, and this clause's own first sentence says so:
an image dictionary may contain Table 87's entries "in addition to the usual entries common to all
streams". A wrong table number, in the population the ninth sweep exists to walk.

### `A2B1` named as a table the tree does not select, in the row that says it reads it

§8.6.5.8 says twice that "selecting a profile's `A2B1` or `A2B2` table by intent is not done", and
says once, three sentences from the end, that "`A2B1` is that table, and it is what this tree
reads". Both cannot be true. `icc.rs` resolves it: it looks for `A2B1` and falls back to `A2B0`,
never `A2B2`, and its inline comment calls `A2B1` the relative-colorimetric table and `A2B0` the
perceptual one — which the row's own `6696954.pdf` paragraph then repeats, calling `A2B0` "the
*perceptual* table". So the transforms a `Perceptual` or a `Saturation` would need are the profile's
*other* two, and the pair the row named includes the one it does read for every intent.

**The module that owns the choice had it right the whole time.** `icc.rs`'s crate documentation
says rendering intents beyond picking `A2B1` over `A2B0` are not modelled. ADR 0101's shape and
710's: the correct statement in the file that decides, the wrong one in every row that depends on
it — and here also in the code, because `content::colour::Intent`'s doc comment carries the same
pair.

### Table 52 for a device-independent parameter

Beside it, in the same doc comment: `Intent::Relative` is "Table 69's `RelativeColorimetric`, which
Table **52** also makes the initial value". Table 51 is §8.4.1's device-*independent* graphics state
parameters and states `[i]nitial value: RelativeColorimetric`; Table 52 is the device-*dependent*
list — flatness, smoothness, overprint, stroke adjustment. §8.6.5.8's ledger row cites Table 51 and
is right; the comment depending on it is wrong, which is the third instance of one shape in one
reading.

## 3. Why no sweep prints any of the four, and each answer is in the sweep's own doc

This is the part worth keeping, because three of the four sit *inside* a sweep's population:

| the claim | the sweep for its kind | why it cannot print it |
|---|---|---|
| `/OC` disposed of by no sentence | fifteenth, `--bin entries` | it prints an entry the row's code does not name, and six files here name `/OC` |
| the same | second, `--bin unread` | it *does* print `/OC` under this row — on the rung it marks as *the row's own code naming it*. The alternates' `/OC` is what put the word in the note, so the hit is true of the word and false of the entry, and it sorts with the documented noise |
| `/Filter` and `/DecodeParms` given to Table 87 | ninth, `--bin tables` | `keys_within` stops a list at the first word that neither is a key nor continues one, and the list's third item carries a parenthesis — "(any family but `Pattern`, which the table excludes)". Everything after it is attributed to nothing |
| Table 52 for the initial rendering intent | the same | the key it attributes is a **value**, `RelativeColorimetric`, and it stands *before* the citation. No `/Key` follows "Table 52", so the citation lands in the keyless count — 0611's finding, one family over and with a wrong number under it this time |

The last row is the sharper one: 0611 recorded that a wrong number lands in the keyless count when
the table it names states no entries. Here the table states plenty; what is keyless is the
*sentence*, because the thing being attributed is a value rather than an entry. Two different routes
into one blind spot, found five rounds apart.

## 4. What was not changed, and why

- **No status moves.** §8.6.5.8 stays `partial` for the reason it always gave, now with the right
  transforms named: a document asking for `Perceptual` or `Saturation` gets `A2B1`. §8.9.5.1 stays
  `partial` for `/AF`, `/Measure` and `/PtData`, which are unchanged.
- **No code changes but two doc comments.** `xobject.rs::draw_xobject` already does what Table 87's
  `/OC` requires, with the clause cited above the block; there was nothing to build. What did change
  is §8.9.5.1's `code` array, which now names that file — and `--bin entries` reports two entries
  fewer for the row as a result, because `/OC` and `/Subtype` are read by the file the row had not
  been naming.
- **`icc.rs` is untouched.** It is the one place in this that was never wrong.
