# What poppler sees in a file this program saved — the save round-trip's third assertion
# (ADR 0334, instrument 2 of ADR 0323).
#
# Asked exactly one question, stated so that trap 3 has nothing to bite on: "open this file
# and say what is in it" — the free text annotations of page one and the text fields of one
# named page, as poppler reads them. Nothing here renders and nothing judges; the verdict is
# the Rust side's, which knows what was written.
#
# poppler-glib rather than a poppler CLI tool, because no tool poppler ships prints an
# annotation or a field value as text: pdftotext reads page content streams, pdftoppm renders.
# The glib API is the same reader underneath, reached through python3-gobject, both of which
# the oracle machine carries.
#
# Output, one record per line, tab-separated:
#   freetext\t<contents>\t<x1>\t<y1>\t<x2>\t<y2>     (page one; area exactly as poppler reports it)
#   field\t<name>\t<value>                           (the page named by argv[3]; empty value for None)
# Exit 2 with a line on stderr when the document does not open.
#
# The area is printed raw. Measured against a raw /Rect (session 499, 160F-2019.pdf):
# poppler's annot-mapping area is the PDF rectangle translated by the crop box's own origin,
# y still upward — the Rust side does that arithmetic, so this script states no convention.

import sys

import gi

gi.require_version("Poppler", "0.18")
from gi.repository import GLib, Poppler  # noqa: E402

path = sys.argv[1]
password = sys.argv[2] if len(sys.argv) > 2 else ""
field_page = int(sys.argv[3]) if len(sys.argv) > 3 else -1

uri = GLib.filename_to_uri(path, None)
try:
    document = Poppler.Document.new_from_file(uri, password if password else None)
except GLib.Error as error:
    print(f"poppler refused the file: {error}", file=sys.stderr)
    sys.exit(2)


def cell(text):
    """One tab-separated cell: newlines and tabs would break the line protocol."""
    return (text or "").replace("\t", " ").replace("\n", " ").replace("\r", " ")


page = document.get_page(0) if document.get_n_pages() > 0 else None
if page is None:
    # The reader opened the file and cannot reach a first page — a fact the caller needs
    # apart from "no free text here", because a reference that cannot find the page cannot
    # witness an annotation on it.
    print("pageone\tmissing")
else:
    for mapping in page.get_annot_mapping():
        annotation = mapping.annot
        if annotation.get_annot_type() != Poppler.AnnotType.FREE_TEXT:
            continue
        area = mapping.area
        print(
            "freetext\t%s\t%.4f\t%.4f\t%.4f\t%.4f"
            % (cell(annotation.get_contents()), area.x1, area.y1, area.x2, area.y2)
        )

if 0 <= field_page < document.get_n_pages():
    page = document.get_page(field_page)
    for mapping in page.get_form_field_mapping() if page is not None else []:
        field = mapping.field
        if field.get_field_type() != Poppler.FormFieldType.TEXT:
            continue
        print("field\t%s\t%s" % (cell(field.get_name()), cell(field.text_get_text())))
