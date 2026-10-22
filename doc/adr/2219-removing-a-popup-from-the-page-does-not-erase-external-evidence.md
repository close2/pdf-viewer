# 2219 — Removing a popup from the page does not erase external evidence

Session 1692, archive. Status: accepted; implemented; scoped and canonical archive gates pass.

The bounded popup proof establishes ownership in the page annotation graph.
It does not establish that no other PDF object references either member.
A whole-file reference census would be unnecessary for the existing writer's
unlink operation and would imply stronger erasure semantics than the consent.

The prepared removal set filters references from each page's Annots array.
The writer's ordinary reachability walk still follows every surviving reference
and copies referenced source objects. Thus an otherwise unreferenced pair drops
out naturally, while an external reference retains its dictionaries and their
reciprocal relationship. The report means removed from the page, not erased
from the file. Known structure/action relationships remain outside this proof.

The regression fixture holds the parent through a private catalog dictionary.
After conversion it verifies both dictionaries, reciprocal links and original
flags remain readable, while neither member remains in Annots. Both PDF/A-2b
and PDF/A-4 run the control in either Annots order. A second conversion also
succeeds. Separate raster and decoded appearance-byte checks protect the kept
annotation; no writer deletion or reference-renumbering algorithm changes.

This boundary builds the page membership and relationship indexes once, avoiding
repeated whole-document scans; the graph is built only when a linked removal exists.
It does not generalise to widgets, replies, multiple parents or visible popups.
