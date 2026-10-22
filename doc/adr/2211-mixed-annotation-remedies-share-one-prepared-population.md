# 2211 — Mixed annotation remedies share one prepared population

Session 1688, archive. Status: accepted; implementation under validation.

The as-shown flag split uses one prepared population from the validator's
annotations_the_flags_forbid predicate. It partitions object/page occurrences
into kept and removed lists before writing. AnnotationFlagsByShape applies both
existing mechanisms: retained dictionaries receive flags_permitting, and removed
object references leave each page's Annots. Other bits and appearance streams
are unchanged. The report names each removed occurrence and each retained
occurrence's original and written flags, including shared references on two pages.

Preparation refuses a direct annotation requiring this rewrite, a discarded
Widget, or a discarded annotation with Popup or Parent links. The bounded
implementation does not repair field trees or remove linked popup structures.
It does not inherit the legacy blanket-removal route's wider popup handling.
A mixed document that needs an unsupported case produces no partial output.
Already conforming annotations are outside the prepared population.

This preserves the source page at rest without inventing appearances or changing
the renderer. The independent mixed fixture holds visible nonprinting, Hidden,
NoView and already printable annotations. At PDF/A-2b and PDF/A-4 it retains the
two visible marks, removes the two hidden objects, preserves appearance bytes,
and yields the same 400-by-400 CPU raster. Separate shared-reference controls
check both output identity and occurrence-specific report pages. The external
veraPDF validator accepts the two output fixtures. Widget, linked-structure,
ambiguous-flag and malformed-value controls require refusal before output.

The interpretation and writer share neither expected pixels nor a guessed flag
policy in the tests: source/output rasters are compared, exact stream bytes are
checked, and report decisions and object references are inspected independently.
The old unqualified Stop and a partial visible-only answer remain refusal controls.
