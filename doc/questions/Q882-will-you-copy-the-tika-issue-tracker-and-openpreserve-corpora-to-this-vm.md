# Q882 — Will you copy the `tika-issue-tracker` and `openpreserve` corpora to this VM?

Source: the post-merge train's first run on main (fe09fccb, 2026-11-17). Nothing waits on the answer: rounds
compare against HEAD, and the gate is known to fail here.

## What was found

`pdf-script`'s `script_corpus` (Tier 1 in the engine, ADR 1625) fails on this VM. Its log says the opposite of what
happened. It asks for both ceilings to be lowered ("threw fell from 9315 to 9192", "unparsed fell from 8 to 0").
But the engine ran 13 879 times, against a floor of 21 101. Fewer scripts threw because fewer files were walked, not
because the code improved. The Tier 0 walk over the same corpus says why: "200 held and not on disk". Every one of
those 200 is under `corpus-cache/tika-issue-tracker/`. That corpus (39 GB on the notebook) and `openpreserve`
(702 MB) never came to the VM: they have no manifest to fetch them from. `safedocs` was refetched and verified on
2026-11-06.

Lowering the ceilings to this VM's numbers would loosen the ratchet over files the gate can no longer see.

## The question

You offered to copy the directory later. The options:

- **(a) Recommended: copy both directories** from the notebook into `~/scratch/corpus-cache/` (the `corpus-cache`
  link in the checkout already points there), for example
  `rsync -a cl@<notebook>:/home/cl/projects/pdf-viewer/corpus-cache/{tika-issue-tracker,openpreserve} ~/scratch/corpus-cache/`.
  The train's next daily run then re-runs the walk. If the counts still differ, that is a real finding.
- **(b) Restate the floors and ceilings to the safedocs-only population.** Not recommended: 200 held documents
  would then be checked by no gate on any machine.
