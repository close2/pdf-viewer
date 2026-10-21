# 1674 — The WinUI window has cards, panels and live permissions

Session 1674, slot 4 of batch 104, a host round; 2026-10-21. ADRs 2184 and 2185; RFC 0010 N3a.
A562's classification remains unchanged. Its menu condition is met; the default is now ask.

**Platform.** Unix descriptor reception is compiled only on Unix; Windows keeps the existing
byte path. The encoder test expects Linux enforcement only. tools/dotnet-check.sh compiles the
actual C# projects against the real SDK with generated XAML fields stubbed, and an optional
conformance test runs it where the SDK is installed.

**WinUI.** Native owed data is copied before release. Asynchronous cards cover password,
question, archive, script and print-preview answers; closing, withdrawal and same-kind replacement
invalidate old continuations. Panels read only the open tab, thumbnails only realized rows.
Direct page selection adds one ABI symbol; invalid page indices preserve the current page.

**A562.** Shared Restrictions owns the launched-document level used by native/winit dispatch.
GTK and Qt enumerate its new machine act. The confined viewer offers that act through its R card;
WinUI reads the shared rows into its native Permissions menu. Three ABI functions count, read
and choose rows without a new crossing struct. Headings and invalid indices cannot set a policy.
All four Linux windows changed refuse to ask through their UI, then asked and obeyed no on a
launch. Windows has the menu and question code; its unavailable runtime is a validation limit.

**Gates.** Windows and macOS pdf-transform/pdf-sandbox all-targets checks exit 0 with warnings
denied. Real-SDK C# compilation exits 0, zero warnings/errors. Native/host/UI nextest passes
491 of 491 tests, two existing corpus/launch tests ignored, including both C programs and the
new shared-state, confined-menu and C ABI cases. Header/C# agreement passes 4 tests for 353
symbols; hosts census exits 0: 45/45 commands, 35/36 queries (LogicalSelection still absent).
C driver strict syntax check and host formatting exit 0. All-targets clippy across the
five host crates, conformance, transform and sandbox exits 0. After the default change,
nextest again passes 491 tests and clippy exits 0. Four Linux menu drives report works; initial
GTK/Qt harness mistakes were corrected against screenshots. Full conformance is integration's.

**Unfinished.** Windows XAML/PRI/MSIX and interactive runtime checks need Windows. Printer
selection/spooling remain N4. WinUI chrome/place preferences remain logged, not implemented;
§12.2 was accurately reopened by the clause round. Native question-slot replacement without
answer identity remains a stability concern; no reachable stale-answer fixture is claimed.

**Proposed habit.** A policy menu and the operation it controls must read one state value; test
both the visible choice and the next operation, including a declined question.
