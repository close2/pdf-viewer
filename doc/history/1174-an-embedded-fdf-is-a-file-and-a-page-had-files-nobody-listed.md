# 1174 — An embedded FDF is a file, and a page had files nobody listed

Four rows on forms data and associated files, three moved and one held with its reason rewritten.

## §12.7.8.3.1 — `partial` → `departed`

Table 246's `/EmbeddedFDFs` is built. The cell states no deprecation; Table 247's
`/EncryptionRevision` does, and Errata Collection 3 Issue #173 rewrites the ambiguous prose above
it so the deprecation is FDF *encryption*'s. Each element is §7.11's specification over §7.11.4's
embedded file stream, decoded and handed to `FormsData::read` — a file, kept whole with its own
`/Encoding` (which decides how *that* file's strings become characters) and its own `/Status`.
`FormsData::files` is the nesting in the order an import applies it; `match_to_document`,
`ViewState::append_templates` and `FormsData::statuses` all walk it. Three bounds share one budget
across the whole nesting — files, depth, decoded bytes — and reaching one is reported. The
encrypted form is refused by name, the departure ADR 1185 prices.

## §12.7.8.3.2 — `partial`, one entry fewer owed

Table 249's `/IF` is applied: Table 250's dictionary is four names, numbers and a boolean, so it
crosses to the target document whole, and Table 192's `/IF` in the widget's `/MK` is the
corresponding entry the replacing sentence replaces. `Import::icon_fit` →
`AnnotationView::icon_fit` → `IconFit::read`. The other four stay owed and divide by *where the
value lives*: `/AP`'s streams are objects of the FDF file (§12.7.8.3.4's open second-`Document`
question), `/APRef` names an external PDF file (§12.7.6.4's hazard, a host question first, ADR
1155), `/A` and `/AA` carry references across object spaces. ADR 1186 section 1.

## §14.13.4 — `implemented`, with a consumer and two tests

Round 1167 was right: `attachment::associated` read any dictionary's `/AF` and nothing handed it a
page. `viewer_core::attachments_in_view` does now, for the page the reader is on — not every page,
because `Query::Attachments` is asked at open and `CLAUDE.md` principle 2 forbids the walk there.
ADR 1186 section 2 records the cost that choice carries.

## One line owed

`FormsData::conforms_to` re-implemented `max(header, newest catalog)`; it is `Document::version()`
now, so an FDF written as a chain of updates reports the version the last revision reached (ADR
1171), tested on a two-revision file whose later catalog drops the entry.
