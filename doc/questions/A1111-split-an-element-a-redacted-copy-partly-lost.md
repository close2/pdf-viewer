Status: complete
Given: 2026-11-28, in conversation — transcribed by the round
Owes: when redacting part of a tagged Form XObject shared across pages, split the structure element so the producer's element keeps naming the intact form with its /Alt, /ActualText, and /E while a new sibling element without them names the redacted page's copy and updates that copy's parent-tree entry

> Q1111: split

Reading:
Build the split. When a redaction creates a page-specific copy of a tagged shared Form XObject and removes part of a sequence, keep the original structure element (with its `/Alt`, `/ActualText`, and `/E`) attached to the untouched pages' form, create a new sibling structure element without those descriptions for the redacted page's copy, and point the copy's parent-tree entry at the new sibling element.
