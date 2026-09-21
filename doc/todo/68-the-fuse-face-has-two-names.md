# 68 — The FUSE face is `quorrafs` on disk and `pdffs` everywhere it speaks

Status: **open**, found by round 1156 while packaging the snapshot. A 10-band defect numbered past
its band because the band is full.
Priority: **low** — nothing computes wrong; a person meets two names for one program.
Code: `crates/pdf-fuse/src/main.rs` (module docs, the usage line, every diagnostic prefix and the
`MountOption::FSName`), `crates/pdf-fuse/Cargo.toml`'s `description`. RFC 0003 names the face.

## What is wrong

The binary has been `quorrafs` since the rename to quorra (`doc/questions/A*` of 2026-09-06), but
the crate still calls itself `pdffs` in its documentation, prints `pdffs:` before every error, and
mounts under the file-system name `pdffs`, which is what `mount` and `/proc/mounts` show. A person
unpacking the snapshot runs `quorrafs`, reads an error from `pdffs`, and finds `pdffs` in the mount
table.

## What a round owes

One deliberate rename, not a drive-by: `FSName` is user-visible state a script may match on, so the
change is recorded as a change (an ADR if anything argued for keeping the old name, otherwise the
record), the docs that mention the mount name are swept (`grep -rn pdffs doc crates`), and the
`pdf-fuse` tests that assert on the prefix move with it. Round 1156 deliberately did not half-do it.
