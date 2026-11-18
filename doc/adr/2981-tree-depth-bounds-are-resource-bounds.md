# 2981 — Tree depth bounds are resource bounds; a boundary outside the medium is an empty box

Session 2075. Status: **implemented.** Context: review 2038 (findings B03, B04, B19, B48, D41), ADRs 0257 and 1496.
Clauses: ISO 32000-2 §7.7.3.1, §7.9.6, §14.7.2, §14.11.2.1, Annex C (informative).

**Depth.** §7.7.3.1 allows "any form of tree structure" and §7.9.6's node forms allow an intermediate node with one
kid, so a deep tree is not a malformed one; the comments calling every tree past the bound cyclic or hostile were too
strong. The page tree's bound rises from 64 to 1024, which the walks' recursion holds on a test thread's stack, and
its cycle test asks a set rather than scanning the path, so a deep tree does not make every visit cost its depth.
Past the bound the pages are found by the scan and `Pages::found_by_scanning` says so, as before. The structure tree's
walk now reports `Reading::truncated` at both of its own bounds — an element at the depth bound that still has
children, and a `/K` array longer than `MAX_CHILDREN` — where it had cut silently. The name and number trees keep
their bound of 64, and their comment now says what it is: a cut a caller cannot see, with Annex C the standard's only
word on limits.

**An empty intersection.** §14.11.2.1: a box extending outside the media box shall be treated "as its intersection
with the media box". A nondegenerate box wholly outside the medium fell back to the crop box or the medium and showed
content the clause clips. The clip is now the empty intersection. The *area* a page is shown at cannot be empty, so
there the box gives way as before, and the page shows nothing. A stated box with no area of its own (`[0 0 0 0]`)
is still read as absent.
