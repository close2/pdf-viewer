# 3836 — `Order` is checked against every group the file references, and `/OCGs` with it

Session 2283. Status: **accepted**. Context: review 2282 finding B05; ISO 19005-2 section 6.9,
ISO 19005-4 section 6.10 (paraphrased); ISO 32000-2 §8.11.2.1, §8.11.3, §8.11.4.2 Table 98,
§8.11.4.3.

## The finding

`optional-content/order-lists-every-group` took the file's groups from `/OCProperties /OCGs`. A
group the page's content selects through `/OC` that `/OCGs` and every `Order` both leave out passed,
and no row reported Table 98's "Every optional content group shall be included in this array."

## What changed

1. **The census** is `pdf_archive::optional_content_groups`: the groups `/OCGs` lists, then every
   object stating `/Type /OCG` that a reference names **where a group stands** — an `/OC` entry
   (content, XObjects, annotations), an entry of a resource dictionary's `/Properties`, an optional
   content membership dictionary's `/OCGs`, a configuration's `/ON`, `/OFF`, `/Order`, `/RBGroups`
   and `/Locked`. A group nothing references is not counted: it is no part of what the document's
   structure reaches. A direct group dictionary is not counted: `Order` can reference only an
   indirect object.
2. **Why by position.** The first census counted every referenced object stating `/Type /OCG`.
   `pdf.js`'s `issue9949.pdf` (Illustrator 14) writes its group's `/Usage` dictionary with `/Type
   /OCG` and no `/Name`; that census put the usage dictionary into `/OCGs` and `Order` of all four
   outputs, a nameless layer in every panel, and veraPDF failed them on 6.9 test 3 (evidence). A
   dictionary standing as a `/Usage` value is a usage dictionary whatever its `/Type` says.
3. **The same row reports a group `/OCGs` leaves out**, placed at the file and naming the group.
   It is a base-standard rule, and it stands under this row by choice: the `Order` rule is read
   over `/OCGs`'s population, and the conversion that completes `Order` is the one that can
   complete `/OCGs` in the same step. Section 5.1's row reads the repair log, and a finding there
   with no repair class behind it is one the converter could not answer (ADR 3835's reason).
4. **The converter** (`sites::completed_orders`) reads the same census, appends the groups
   `/OCGs` left out to it, and completes each configuration's `Order` from the census, so the two
   arrays and the census agree in the output.
