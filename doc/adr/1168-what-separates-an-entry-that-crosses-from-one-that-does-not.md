# ADR 1168 — What separates an entry that crosses from one that does not

Status: accepted, 2026-09-22. Session 1165, the batch's host-UI round.

Table 153's `/Sort` is obeyed in all three windows and reaches the C ABI, §12.3.6's layouts reach
it too, and the property ADR 0576 stated for dropping four entries is replaced with one that
holds. Amends ADR 0576 section 3's last paragraph; the rest of that ADR stands.

## 1. The owner's note, tested

`doc/adr_revisit/0576-the-c-abis-other-half.md` argues that "a look belongs to the platform" cannot
be the property, because the same ADR carries Table 29's look entries. Its claims, checked against
the ADR, the clause and the tree:

- **"The same ADR carries Table 29's look entries" — holds.** ADR 0576 section 1 names
  `/PageMode` and `/PageLayout` as two of its eleven, and its own C gate prints them.
- **"The dropped entries' premise has expired" — holds, and not for the reason given.** The note
  says `/Sort` is "the thumbnail panel's ordering" and `/Navigator` "whether to open it", and that
  all three hosts have had that panel since ADR 0564, five sessions before 0576. The panel those
  entries describe is the *files* panel, not §12.3.4's thumbnails, and it reached the two native
  hosts in ADR 0711 — session 772, which is *after* 0576 rather than before it. The premise
  expired; it expired later and elsewhere than the note says.
- **"Some other property separates them, and the ADR never states it" — holds**, and that is the
  part worth fixing.

## 2. The property

**An entry crosses the boundary when it decides something about the document's own content that no
caller can derive, and stays out when it dictates the appearance of a surface the platform owns.**

That is what separates the carried from the dropped in every case ADR 0576 decided, once the four
are reconsidered under it:

| entry | decides | crosses |
|---|---|---|
| `/PageLayout`, `/PageMode` | which pages and which panel the producer asked for | yes, and did |
| §12.3.5.1's `/D` | which file is presented first, resolved against the name tree | yes, and did |
| Table 153's `/Sort` | the order the items stand in, from values in each file's `/CI` | yes, now |
| §12.3.6's `/Layout` | which presentations the producer prefers, in order | yes, now |
| Table 157's `/Colors` | the colours of a navigator's background, cards and text | no |
| Table 158's `/Split` | the orientation and position of a splitter bar | no |

The first four are facts about the file that a host cannot work out for itself. The last two name
furniture: `/Colors` is "a suggested set of colours" whose NOTE only recommends their use, and
§12.3.5.1 says a splitter's "visibility, orientation or position may be interactively adjusted by
user action" — so both describe a widget none of the three windows has, and a window that has one
will paint it in its own platform's colours. They are not dropped in silence:
`viewer_host::panel::unsupported_presentation` already tells a person what the document asked for.

**`/Sort` is also the only one of the four carrying a `shall`** — "the order in which items in the
collection shall be sorted in the user interface" — which is the second reason it is first.

## 3. Computed once, below the hosts

The ordering could not be a `sort_by` in a panel, and that is the interesting half. Two of Table
155's three item subtypes read §7.11.6's collection item on each file specification's `/CI`, which
only a `Document` can reach; the other subtypes read the file specification, which
`Answer::Attachments` already carries. Three windows and a C caller pairing those up would be four
readings of one clause.

So `pdf_model::collection::sorted_keys` takes the document and the attachments and answers the
`/EmbeddedFiles` keys in order; `Answer::Collection` carries that answer, the confined codec
carries it, `viewer_host::panel::in_sort_order` applies it for all three windows, and
`quorra_collection_ordered` hands it to a C caller. **The answer crosses and the arithmetic does
not** — the same division §12.3.5.1's `/D` has taken since ADR 0231.

Table 156 stops before three questions, and each is decided here as a decision:

- **Lexical order is this reader's**, on the clause's own NOTE 3: "Lexical ordering is an
  implementation dependency for interactive PDF processors." What is used is the decoded text
  string's Unicode scalar values.
- **A member with no value for a field sorts after every member that has one**, in both
  directions, so that `/A` reorders what the document stated rather than shuffling what it did
  not.
- **The name tree's order breaks a tie the named fields could not.** Table 156 stops at "until the
  named fields are exhausted"; the tree's order is the one other order the document stated, so
  nothing is invented.

Table 47's `/P` is not consulted, because its own entry says not to: "This entry is ignored when an
interactive PDF processor sorts the items in the collection."

## 4. The layouts cross as a list, not as a choice

§12.3.6 asks a processor for "the first one it is capable of displaying", and what a caller of
`viewer-ffi` is capable of displaying is that caller's fact. So `quorra_collection_layouts` and
`quorra_collection_layout` hand over Table 160's `/Layout` whole, in the document's own order of
preference, with `QUORRA_NAVIGATOR_CUSTOM` carrying the name for a registered layout this table
does not define. The three windows keep `Navigator::preferred` against
`viewer_host::panel::DRAWN_LAYOUTS`, because there the processor *is* this program.

Errata Collection 3's Issue #477 renumbers §12.3.6 to §12.3.5.3; every citation here follows the
published text in `doc/md/`, as the rest of the tree does, and §12.3.6's own ledger row records the
move.

`QUORRA_ABI_VERSION` does not move: four entry points added and eight constants are what an old
caller never calls.
