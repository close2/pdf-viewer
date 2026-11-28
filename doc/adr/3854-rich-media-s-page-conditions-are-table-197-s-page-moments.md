# 3854 — Rich media's page conditions are Table 197's page moments

Session 2288. Status: **accepted and built**. Code: `pdf-model`'s `media/rich_media.rs`
(`settings`, `Activation`, `Deactivation`); `viewer-core`'s `interact/rich_media.rs` (`raised`, `presented`),
`interact.rs` (`trigger`) and `viewer.rs` (`Command::Presented`). Clauses: ISO 32000-2 §13.7.2.2, Tables 334 to 336. Plan: `doc/todo/80`
section 1 step 1; ADR 3820.

## Context

Table 335's `/Condition`: `XA` "explicitly activated by a user action or script", `PO` "as soon as
the page that contains the annotation receives focus as the current page", `PV` "as soon as any
part of the page that contains the annotation becomes visible"; Table 336's `XD`, `PC` "loses focus
as the current page" and `PI` "the entire page … is no longer visible". Table 197 already raises a
screen annotation's `/PO`, `/PC`, `/PV` and `/PI` for every annotation on a page at exactly these
moments (ADRs 2814, 2986), whatever its flags.

## Decision

1. **The same moments, not a second clock**, after the first frame (see Cost).
   `interact::trigger`, which every one of the four triggers reaches for every annotation, asks
   `rich_media::raised` first: a `PO` or `PV`
   condition met activates, a `PC` or `PI` condition met deactivates, beside whatever the
   annotation's own `/AA` performs.
2. **Activation is a play as Table 218's `/OP` 4 associates one** (`or_resume`): where the players
   already hold this annotation's clip, a paused one resumes and a playing one is left alone, so a
   page shown again, or a press on a playing annotation, starts nothing twice.
3. **Deactivation is Table 218's `/OP` 1**, a stop of whatever plays associated with the
   annotation, "no effect" where nothing does. `XD` is the reader's — Escape, or a rendition
   action's stop.
4. **A condition the tables do not list is said and read as the default**, `XA` or `XD`.
5. **A page's condition activates only what plays**: a `3D` configuration, or one the players do
   not take, is not said at every page turn, because no person asked; the press says it.

## Cost

**Page one pays nothing.** The open raises page one's `/PO`, and its first `Resize` page one's
`/PV`, on the critical path. Read there, the conditions cost the launch gate's opt_demo first frame
3187.9 thousand instructions against HEAD's 3181.3, past its band's 3183: a `/Subtype` look-up per
annotation at each moment. So `rich_media::raised` does nothing before the document's first frame
is presented (`Open::presented`), and `Command::Presented` — the host's word that the first frame
is on the screen, where RFC 0008's open sequence already waits (ADR 1602) — raises the presented
pages' rich media `PO` and `PV` once (`interact::rich_media::presented`). Nothing plays before
then, so a `PC` or `PI` raised before it has nothing to stop. A reader sees the activation one
present later than the page; the clause's "as soon as" is met at the first moment the page is
shown. Deferred, opt_demo's first frame reads 3178.1 thousand on two runs, on main c8770fe7c.

## Tests

`rich_media.rs::each_condition_is_read_and_the_defaults_stand_where_none_is_stated`;
`viewer-core/tests/rich_media_activation.rs`: `page_open_and_page_visible_activate_the_annotation_on_its_page`
and `page_close_and_page_invisible_deactivate_and_explicit_leaves_it_playing`.
