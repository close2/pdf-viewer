# 2960 — A header past the search window is looked for before the table is rebuilt

Session 2072. Status: **implemented.** Context: review 2038 finding B21 and T145, ISO 32000-2 §7.5.2 NOTE 1, §7.5.6,
ADRs 1066 and 1083.

## 1. The finding

§7.5.2 makes the header the origin of every offset, and its NOTE 1 allows "arbitrary bytes preceding the %PDF-"
with no bound. `xref::header_position` looks in the first 1024 bytes only (ADR 1083 measured why that search is
cheap). Past it, the chain was read from byte zero, failed, and the file was rebuilt by scanning, and the scan
found object bodies the newest section had freed: an object deleted by §7.5.6's update came back
(`a_long_header_prefix_does_not_resurrect_an_object_deleted_by_the_latest_section`, prefix 2048).

## 2. The decision

The window stays the first place a header is looked for, so an intact file pays nothing new. Only where the
window holds no header *and* the chain read from byte zero failed does `xref::read` search the rest of the file
(`header_past_window`) and read the stated chain again from that origin, before anything is rebuilt. The whole-file
read this needs is the one a rebuild would have made anyway.

## 3. What is left

`xref::sections` and `walk_revisions`, used by validators off the launch path, still take the window's answer or
zero. They are left as they are for this round and named in the record.
