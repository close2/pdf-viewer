# 1234 — Three losses the clause states the fallback for

Status: accepted and **built**.
Context: `crates/pdf-archive/src/table/interaction.rs` (three populations),
`crates/pdf-transform/src/archive/{decision,prepare,sites,rewrite,mod}.rs`,
`crates/pdf-transform/tests/archive.rs`.
Answers: `doc/todo/66`'s **annotations** and **optional content** families —
`annotations/printable-and-visible`, `annotations/appearance-dictionary-holds-only-normal` and
`optional-content/no-automatic-states`, each of which `doc/pdf-a-mitigations.md` catalogued and no
version carried out.
Builds on: `doc/adr/1099` (an annotation the part forbids is removed, and that is an *Ask*),
`doc/adr/0947` (nothing is prepared or changed that no failed requirement asked for),
`doc/adr/0816` (the fence), `doc/adr/1175` (one reading of a tree serving several rewrites).
Clauses: ISO 32000-2 §12.5.2 (Table 166), §12.5.3 (Table 167), §12.5.5 (Table 170), §8.11.4.1
(Table 98), §8.11.4.3, §7.7.3.3 (Table 31); ISO 19005-2 sections 6.3.2, 6.3.3, 6.9; ISO 19005-4
sections 6.3.2, 6.3.3, 6.10.

## 1. What the three have in common, and it is the reason they went together

Each is a requirement this converter refused because a *word* and a *rewrite* were missing, not
because the standard leaves anything open. And in each of the three the base standard states what a
reader does after the removal, which is what turns "delete the producer's entry" from a guess into
a priced loss:

- **§12.5.5's Table 170** makes `/R` and `/D` optional and gives each the same default — the value
  of the `/N` entry. So an appearance dictionary reduced to `/N` draws, in the rollover and down
  states, exactly what the standard already has a reader draw for a dictionary that states neither.
- **§8.11.4.3** makes `/AS` the array by which a processor sets optional content group states from
  external factors. Remove it and the configuration's own `/BaseState`, `/ON` and `/OFF` still say
  what the document shows; what stops is the switching.
- **§12.5.3's Table 167** does *not* offer a fallback for the flags, and that asymmetry is the whole
  of why the third one is built the way it is. See section 3.

## 2. The population is the requirement's own predicate, read as a population

`pdf_archive` gains `annotations_the_flags_forbid` and
`annotations_with_extra_appearance_states` beside the `annotations_of_a_forbidden_subtype` ADR 1099
added, and `flags_permitted` / `flags_permitting` beside them. This is ADR 1099's rule rather than a
convenience: a findings list is capped where a document's annotations are not, so a rewrite driven
off findings half-finishes a file with many of them — and a rewrite driven off a *second* reading of
the clause can act on an annotation the requirement itself passed. Both readers walk
`Examination::annotations()`, which is what the predicates walk.

`flags_permitting` is written and not yet called by any rewrite. It is the value the other half of
`doc/pdf-a-mitigations.md`'s entry needs — `preserve` at the flag site, which clears the hidden bits
in place — and it is here because the two halves are one reading of Table 167 and writing it twice
is how they come to disagree.

## 3. Removal rather than un-hiding, and the clause decides it

ISO 19005-2 section 6.3.2 and ISO 19005-4 section 6.3.2 require `Print` set and `Hidden`,
`Invisible`, `NoView` and `ToggleNoView` clear. `doc/pdf-a-conversion-limits.md` section 3.7 names
the two futures — the annotation becomes visible and printable, or it goes — and makes removal the
default. The argument is the direction of the change: §12.5.3's Table 167 says of a clear `Print`
bit "If clear, never print the annotation, regardless of whether it is rendered on the screen", so
the file as the standard defines it says this annotation is not on the printed page. Writing the
other value **puts a mark where the producer put none**; removing the annotation takes one away. Of
the two, the second is the smaller claim about what the producer meant, and it is the one an
archivist can check in the report — which names each annotation, its page, its subtype, whether it
drew a mark, and the `/F` its producer wrote.

This is why `Loss::HiddenAnnotation` is a word of its own beside `Loss::AnnotationPrinting`. The
older one is the *absent* `/F`: §12.5.2's Table 166 gives the entry a default of 0, so a producer
who stated none decided nothing, and writing bit 3 fills in an absence. An operator who authorised
that has said nothing about a review comment somebody deliberately hid.

## 4. Three places for one key, and why the removal edits rather than replaces

§8.11.4.1's Table 98 lets an optional content configuration be written as an object of its own,
directly inside an `/OCProperties` that is an object, or directly inside an `/OCProperties` the
catalog itself states. `super::sites::automatic_states` finds which, and the rewrite removes the key
from whatever the dictionary **currently** holds at that point in the walk.

That is not a style choice. `Rewrite::OptionalContentOrder` replaces an `/OCProperties` dictionary
wholesale with the one its own preparation built from the source, so a removal that also worked from
a source snapshot would be undone by whichever of the two ran second on a document needing both. The
`/AS` removal therefore runs *after* `complete_order` in both the object walk and the catalog walk,
and edits its output. A configuration written directly inside a `/Configs` array that is an object of
its own is refused by name, which is where the `/Order` completion stops for the same reason: this
walk rewrites dictionaries, and an array object is not one.

## 5. Two refusals kept, and each is a clause rather than a limit

- **An appearance dictionary stating no `/N`.** Table 170's default for the two keys being removed
  *is* the `/N` entry. Where there is none the removal falls back to nothing, and the output would
  carry an appearance dictionary describing no appearance — so the document is refused with a
  sentence saying exactly that, rather than half-answered.
- **An annotation written directly into a page's `/Annots`.** §7.7.3.3's Table 31 requires indirect
  references there; files exist that do otherwise, and a rewrite acting on objects has nothing to
  take out. ADR 1099 refuses this at the subtype site and this refuses it at the flag site, in the
  same words for the same reason.

## 6. What was deliberately not built

`preserve` at the flag site — un-hiding the annotation — needs a mechanism the configuration
vocabulary has not got: `preserve` today means *keep this content somewhere else* and carries a
placement or a tool, and this one keeps the annotation exactly where it is. `keep-everything` asks
for it and still gets a note saying so, which is the honest answer. `optional-content/configuration-names`
is untouched: `doc/pdf-a-mitigations.md`'s entry makes it a `supply` with the converter as the
supplier, which is a different argument and belongs in its own.
