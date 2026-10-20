# 2060 — The eight commands the C ABI lacked reach it in C's shape

Session 1612, the first half of RFC 0010 section 6's N1 and all of section 3.1's table, for the
.NET host the owner asked for in `doc/questions/A287`. Status: **accepted** and **built**. Context:
ADRs 0247 (one entry point per command), 1160 (the clock), 1291 and 1527 (a server's answer),
1892, 1930 and 1994 (a page drawn a window at a time), 1628 (a script's question), 1762 (the keys).

## 1. What was missing

`tools/state.sh hosts` printed 37 of 45 `Command`s reaching `crates/viewer-ffi`: `CommitField`,
`Clock`, `Respond`, `Oversize`, `Keys`, `Scripts`, `AnswerScripts` and `AnswerScript`. Three more
gaps stood behind them, each making one of the eight unusable from C: a `QUORRA_EVENT_SUBMIT`
crossed with no URL, method or media type, and its body was not readable although the header said
`quorra_event_bytes` gave it; a script's question crossed only as `quorra_events_describe`'s
sentence; and a raster could be read only after the viewer held it, which a window of a listed page
never is.

## 2. Decisions

1. **One entry point per command, its arguments in C's shape** (ADR 0247 section 1):
   `quorra_commit_field`, `quorra_clock`, `quorra_respond`, `quorra_oversize`, `quorra_keys`,
   `quorra_scripts` (ADR 2061), `quorra_answer_scripts`, `quorra_answer_script`.
2. **The clock is `pdf_syntax::Date`'s fields, one argument each**, the RFC's shape: a `known`
   flag for the `Option`, and `zoned` beside `offset_minutes` because a date that states no zone
   is not the claim `Z` is. No struct crosses, so `QUORRA_ABI_VERSION` stays 2. The range is the
   clause's own grammar: the fields are spelt by the `/M` writer and read back by
   `Date::parse`, and a date is taken only where it reads back unchanged; anything else is
   `QUORRA_NUMBER_OUT_OF_RANGE` and nothing is sent.
3. **A server's answer takes its format as a number**, `QUORRA_DATA_*`, because
   `Command::Respond` does; the two decisions that choose it are every window's and are offered
   rather than restated — `quorra_data_format_of_media_type` (`viewer_host::submit`'s
   `FDF_TYPES`/`XFDF_TYPES`, with the parameter dropped as `media_type_of` drops it, six lines that
   module keeps private) and `quorra_data_format_of_name` (`pdf_model::action::data_format`).
   `quorra_event_submit`, `_submit_url` and `_submit_media_type` give the request;
   `quorra_event_bytes` gives its body, which makes the header's sentence true.
4. **`QUORRA_OVERSIZE_LISTED` keeps the request as the list.** RFC 0010 priced `Oversize` as
   coming with section 4's device; the request already crosses as an opaque handle, so the
   processor draws a window of it now: `quorra_render_request_oversized` and
   `quorra_render_request_window` are `viewer_host::oversize`'s rules (the page, and the viewport
   grown by a quarter), `quorra_render_request_rasterise_window` draws through `Window::target_of`,
   `quorra_render_ready_listed` answers `Rendered::Listed` without consuming the request, and
   `quorra_raster_info`/`_copy` read a raster the viewer does not hold. A device lane is section
   4's and changes none of these.
5. **A script's question crosses in Adobe's numbers**: `app.alert`'s `nIcon` 0–3, `nType` 0–3 and
   the button returned 1–4, so a C# host mirrors the reference rather than this tree.
   `quorra_event_script_question` also gives the button a closed dialogue answers with, ADR 1628's
   choice, so no host re-decides it; `quorra_event_script_text` gives the four strings, a title or a
   label not given being `QUORRA_NO_ANSWER` rather than "".
6. **No event kind is added.** Every event the eight produce already had a kind (22–27), so
   `QUORRA_EVENT_KIND_COUNT` stays 30 and a compiled caller's `quorra_abi_check` still passes.

## 3. Consequences

261 entry points (21 new) and 34 constants; `tools/state.sh hosts` prints 45 of 45. The C program
sets the clock and finds the `/M` it wrote, imports a server's FDF, answers a script that is not
waiting, and draws a window of a 23812 × 33676 page. Owed: a submission's `owed` sentences, which
the windows print beside the request, do not cross; the key table and its count, which N1 also
names, are not built here.
