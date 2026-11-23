# 3413 — One interaction gate, which every route to an annotation asks

Session 2178. Status: accepted. Context: audit 2175 (B02, B03), ADRs 1369, 1603, 2814; trap 5.

## Decision

1. **Every route a person reaches an annotation by asks one function**,
   `pdf_model::view::annotations::passes`, with one of two questions (`Gate`). The routes found:
   the hover and press of `annotation_at`; a click's activation, a hover's cursor and a host's
   `Query::LinkAt`, now all `view::link_at`; `field_at` and `field_at_in_view` (a host's
   `Query::FieldAt`, which `viewer_host::form::clicked` asks); the caret on a widget and on a free
   text; the unreachable-press report; the Tab key's focus step; the accessibility tree. Before,
   only `annotation_at` and the tree asked, so a hidden link was followed by a click and `field_at`
   named a hidden widget.
2. **`Gate::Responds` is `annotation::interacts`**: Table 167's `Hidden`, `NoView`, a non-widget's
   `ReadOnly`, a watermark, and for a widget its field's Table 227 bit 1 — bit 7 "shall be ignored
   for widget annotations; its function is subsumed by the ReadOnly flag of the associated form
   field". The bit is read inherited, under an FDF import's `/Ff`, `/SetFf` and `/ClrFf`, and under
   a script's `readonly`, which stands over the file's flag (ADR 1603). Asked by what the pointer
   *does*: an appearance, a trigger, a link's activation, a caret.
3. **`Gate::Present` is `annotation::present`**: the same without the field's `ReadOnly`, and it
   is Table 227's own definition that separates them — the widgets "should not interact with the
   user; that is, they should not respond to mouse clicks nor change their appearance in response
   to mouse motions". A read-only field is still read: the Tab key's focus reaches it, the
   accessibility tree offers it (a calculated total is the common case), and `field_at` names it
   so that a host refuses its click by name rather than dropping it (trap 5). Hidden and NoView
   withdraw an annotation from all of these.

## Cost

Two questions where the audit asked for one; each route's choice is named at the call. `field_at`
keeps its signature and answers from the file alone; `field_at_in_view` is the view-state answer.
A script's `readonly` costs a field-tree walk per widget only once some script has set a property.
