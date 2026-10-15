# 1961 — Table 286's temporary file is a restriction, and the clip is handed to the host

Session 1562, slot 5 of batch eighty-nine. Status: **accepted, built**. Context: `CLAUDE.md` principle
3's *A document's restrictions are the reader's to set*; ADRs 0803, 1144, 1203; RFC 0009 §3.5 and §11
question 3; `doc/questions/A254`; ISO 32000-2 §7.6.4.2 Table 22, §13.2.4.2 Table 286.

## 1. Two levels, and only one of them is built

Two different statements apply to handing a clip to a player.

- **What the document asserts.** Table 286's /TF is "permissions that control the use of the media
  data", stated by the author over the reader, and it governs "a temporary file in order to play a
  media clip". That is principle 3's subject. It is asked as a new operation,
  `restriction::Operation::HandOverMedia`, written `media` on a command line. `restriction::media`
  reads its four values, and `Level::verdict` decides once, `off` by default as every other operation
  is. TEMPEXTRACT and TEMPACCESS both read §7.6.4.2's bit 5, because Table 22 says readers "shall
  ignore" bit 10. Only embedded data asks the question: a URL or a named file is handed over without
  any file being written.
- **What the reader allows this machine to start.** This is A254's `Media` level, an answer to RFC
  0009 §11 question 3: `refuse|ask|warn|open`, `ask` by default, in each window's menu. **It is not
  built.** No window starts a player, so the level would have nothing to decide yet. It is owed the
  day a window starts one, and `doc/todo/65`'s bucket 1 says so.

One level for both was rejected. It would make a reader who obeys a document's /TF also decide whether
their own player may start, and those are ADR 1144's two subjects under one control.

## 2. The hand-off

`viewer_core::Event::Media` carries the content type and the data. An embedded clip's data is decoded
on this side, for `Event::Extracted`'s reason. A URL and a named file travel as words. At *ask*, the
decoded clip is held as `Held::Media` until `Command::Answer`. The confined window's wire carries the
event (`MEDIA`, 28) and the operation (8). The C ABI gives it `QUORRA_EVENT_MEDIA` (28,
`QUORRA_EVENT_KIND_COUNT` 29). Every window says `viewer_host::handed_media`'s sentence, which ends
"this window starts no player yet, so nothing plays".

## 3. The cost

The operation is a ninth entry in `RestrictionPolicy::OPERATIONS`, so every surface that enumerates
the levels (the wire, the C ABI and the windows' restriction menus) grows one row with no extra code.
The event is a new variant that every exhaustive host match had to handle.
