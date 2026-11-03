# 2657 — Each outside-the-tree marker is held from the round that added it

Session 1911, a single round (instruments). Status: **accepted** and **built** in
`tools/conformance/tests/evidence.rs`, `tools/state.sh`'s `outside_paths` and
`tools/conformance/tests/bounded.rs`. Context: ADR 2242 decision 4 (a path outside the tree is
never the evidence), ADR 2260 (the workplace sentence), ADR 2656 (the places file).

## The question

`evidence.rs` recognised a path outside the tree by two markers, the notebook agent's home directory
and `scratchpad/`. After ADR 2656 a new record names a place under the home or the scratch
directory, by a tilde, or by a root `tools/places.sh` defines, and neither marker sees any of them.
The records already held name the old directory and are never rewritten (`CLAUDE.md`, "Where knowledge lives"), so the
old marker must stay. One flat list cannot do both: a wider marker held against every record would
refuse ADR 2462's path to the round agent's definition in the user's configuration, written with a
tilde before any rule covered one.

## The decision

1. `OUTSIDE` pairs each marker with the first session and the first ADR held against it. The
   notebook's two keep session 1712 and ADR 2258; `/home/`, `/scratch/`, `~/`, `$HOME/` and the
   roots `tools/places.sh` defines (`$PDF_VIEWER_SCRATCH/`, `$PDF_VIEWER_AGENT/`,
   `$PDF_VIEWER_BUILDS/`, `$BATCH_ARMS_ROOT/`) start at session 1911 and ADR 2656. `${NAME}` is
   read as `$NAME`; a word equal to any marker it is held against is a kind of place, not a path.
2. The calibration test plants each new spelling as evidence, holds that a record and an ADR one
   number before the start pass it, and holds every `$NAME/` marker to a name `places.sh` assigns,
   so the list cannot name a variable that does not exist (trap 25).
3. `tools/state.sh --outside-paths`, which reads a batch commit's body, uses the newest markers
   only, since every commit it reads is from now on; its line says "under a machine's own
   directories" where it named the notebook's directory.
4. The bare-`flock` reader in `bounded.rs` also refuses a lock spelled as `"$HEAVY_WALK_LOCK"`: once
   the scripts name the lock by its variable, a person's instruction can too, and the old reader,
   which wanted a path ending `heavy-walk.lock`, would have passed it.

## Cost

Ten markers in place of two, and a rule a reader has to learn: the marker set depends on the
document's number. A root outside `/home/` and `/scratch/` spelled literally is not seen; the
variable spelling is, and that is the spelling the navigational documents now use.
