# 2319 — A push button's single appearance stream is the one entry of a subdictionary its state selects

Session 1742, the archive slot of batch one hundred and thirteen. Status: implemented. Context: ADR
0955 and 0957 (the `/AS` half of `annotations/normal-appearance-shape`), `doc/questions/A48`,
`TechNote 0010`'s A012 and A023, the census `the_shipped_profiles_list_what_real_producers_files_refuse_on`.

## What the census said, and what the files hold

After the composite fonts nobody named a program for (Q570, open to the owner), the largest refusal
of the pdf.js documents under `as-if-printed`, at 2b and at 4 alike, was
`annotations/normal-appearance-shape`, under the site's sentence for an appearance subdictionary with
no `/AS`. The bytes said otherwise: in every one of those documents each failing annotation is a
push button widget (`/FT /Btn`, field flags bit 17) whose `/AP` holds one key, `/N`, and whose `/N`
is a single stream. The site's refusal sentence was one sentence for every shape the requirement
fails in, so the census printed the one shape none of them had.

## The reading

ISO 19005-2 and ISO 19005-4 section 6.3.3 require a button widget's `N` to be an appearance
subdictionary, and `TechNote 0010`'s A012 resolves that a push button's is one even where it holds a
single entry. §12.5.2's Table 166 makes `/AS` required where `/AP` holds a subdictionary and what
selects the applicable stream from it. §12.5.5 draws a single stream whatever the annotation's
state. So a subdictionary holding the producer's one stream alone, under a state the new `/AS`
names, draws exactly the stream the page drew: a statement, `Stated` under the same rewrite as the
`/AS` half (`NormalAppearanceFromState`), reported under its own shape, `push-button-stream`.

**The state's name is a choice, and it decides nothing.** §12.7.5.2.2 gives a push button no value,
and no clause names its states; nothing but the new `/AS` refers to the entry, and `Off`, which
§12.7.5.2.3 reserves for a check box's off state, is avoided. The name written is `Normal`, §12.5.5's
own word for the appearance it holds. That is not A48's forbidden half: the file states one
appearance and the conversion picks none among several.

**A check box's or a radio button's single stream stays refused**, now in a sentence of its own:
§12.7.5.2.3 makes such a field's states `Off` and the name of its on state, so the one name a
subdictionary would hold says which value the field has, and the file states no appearance for that
value. The no-`/AS` subdictionary and the missing `N` keep the site's first sentence.

## What it moved

Drawn at 100 dpi before and after (`js-buttons`, `bug1675139`, `resetform`, `evaljs`, page by
page): every push button drawn where it was, the pages within one level of the source (the output
intent's), and looked at. veraPDF 1.30.3: `js-buttons`, `bug1675139`, `resetform`, `evaljs` at 2b
and `js-buttons`, `listbox_actions`, `form_two_pages` at 4, all PASS. The census against HEAD is the
session's record (`doc/history/1742`).

What would reopen it: a clause or a resolution that names a push button's appearance state.
