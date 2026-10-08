# OpenType's script tag registry, and where this list came from

`script-tags.txt` is the registry of OpenType script tags, one tag and the registry's name for the
script per line. `crates/pdf-font/src/pairs.rs` chooses a program's `GPOS` script table by the tag
registered for the script a run of rich text is written in (ADR 1696), and the test
`every_script_reaches_a_registered_tag` reads every code `Scripts.txt` names against this list, so
the mapping in `pairs::tags` cannot name a tag the registry does not hold.

## Where it came from

<https://learn.microsoft.com/en-us/typography/opentype/spec/scripttags>, the page titled "Script
tags (OpenType 1.9.1)", fetched on 2026-10-08. Its table's first two columns are kept, in its own
order; the third, remarks, is not. The registry prints the tag `'yi  '` with no-break spaces "to
ensure that two space characters are displayed" and says the tag's bytes are `0x79 0x69 0x20 0x20`,
so every tag here is written with ordinary spaces.

    sha256  0fa89c4e9ffca684b80bb6c846f70851943e3173689c5b3de42511e6d0224923  script-tags.txt

Only a test reads it; nothing is compiled in from it.

# OpenType's language system tag registry

`language-tags.txt` is the registry of OpenType language system tags: one tag per line, the ISO 639
codes the registry gives it, whether the registry marks it deprecated, and its name for the language
system. `crates/pdf-font/build.rs` compiles it into the table `pairs::Language` selects a script
table's language system by, from the natural language §14.9.2 gives a run's text (ADR 1708), and the
test `every_registered_code_is_reached_by_its_own_tag` reads every row back through that table.

## Where it came from

<https://learn.microsoft.com/en-us/typography/opentype/spec/languagetags>, the page titled "Language
system tags (OpenType 1.9.1)", fetched on 2026-10-08. Its table's three columns are kept, in its own
order: the name moved last, the tag with its quotes and with the registry's "(deprecated)" taken out
into a column of its own, and of the third column only its leading ISO 639 codes — the remarks after
them (`See also`, a URL, a BCP 47 variant subtag) are not kept, and the eight rows giving no code have
an empty column. The registry writes four tags without quotes (`BAD0`, `APPH`…); every tag here is
quoted.

    sha256  c7e88689da94c91fc5d7ab43e29395112363914ddb7fc9cac114cad2f0106a79  language-tags.txt
