# 1923 — A requirement handler this processor checks is not run, and §12.11.5 is implemented

Session 1543, a docs round of batch eighty-six. Status: **accepted** and **built**. Context:
`doc/questions/A286` (the script exclusion ends at this document and this viewer), `A193` (whose
Tier 1 list names §12.11.1 and §12.11.5), `A100`; ADRs 0896, 1602, 1904 section 3. Code:
`crates/pdf-model/src/requirements.rs` (`disabled_handlers`),
`crates/pdf-model/src/view/script_sites.rs` (`run_open_scripts`). Tests:
`requirements.rs::a_handler_is_disabled_where_its_requirement_is_one_this_processor_checks`,
`crates/pdf-script/tests/hook.rs::a_requirement_handler_this_processor_checks_is_not_run_and_the_rest_of_the_library_is`.

## 1. The premise, re-read

ADR 0896 made §12.11.5 `out-of-scope`: every requirement in it binds a processor that invokes a
handler, and this program ran none. ADR 1904 section 3 said that reasoning had decayed, and it had.
At `on`, the open sequence runs every entry of Table 32's name tree, and a requirement handler is
one of those entries. Table 276 says a `JS` handler's `/Script` names "a document-level ECMAScript
action stored in the document name dictionary". So the clause's `shall` on `/Script` is now owed:
"If the PDF processor understands the parent document requirements dictionary and can verify the
requirement specified in that dictionary, it shall disable execution of the requirement handler
identified in this dictionary." Before this change nothing read `/RH`. A document whose handler
says "this viewer cannot do X" would have run that handler in a viewer that checked X and found it
could do it.

## 2. Decision

`requirements::disabled_handlers` returns every `/Script` name to disable. `run_open_scripts`
skips the name-tree entry with that name, reports it as not run with the reason, and runs the rest.

- **"Can check" is decided by type.** `Kind::unmet` answers all twenty-five of Table 275's types,
  met or unmet. So every type in the table is one this processor checks. Table 273's `/RH` disables
  the handler "(whether or not it can satisfy that requirement)". A type outside the table,
  `Kind::Other`, cannot be checked, so its handler runs. That is the case a handler exists for.
- **`/RH` is one dictionary or an array of them**, as Table 273 types it. `NoOp` names no script.
  Any other handler type is ignored, as the clause's last sentence requires.
- **The match is on decoded text.** A `/Script` and a name tree's key are both decoded as §7.9.2.2
  text strings, so a UTF-16 `/Script` matches a PDFDocEncoding key that spells the same name.
- **Only the name tree is affected.** That is where `/Script` points. A script reached any other
  way is not a requirement handler.

## 3. The rows

**§12.11.5** moves from `out-of-scope` to **`implemented`**. Its `exclusion` field goes, and its
note states the reading above. At `off` no entry of the tree runs, so the handler is disabled by the
level as well. The row is `implemented` on §12.6.4.17's reading of `A100`. The `should` that both
the feature and the handler "should be present in the document" is addressed to a producer, and
this program writes neither. **§12.11.1's and §12.11's notes** said the handler was disabled
because the exclusion kept every script from running. Each now names `disabled_handlers`. Their
statuses do not change. §12.11.1's ordering `shall`, "shall evaluate them before execution of any
ECMAScripts", is met as before: `Viewer::open` asks §12.11.6's policy before the first present, and
the scripts run after it (ADR 1602).

**Gates.** Both tests fail with the skip disabled (trap 13) and pass with it.
