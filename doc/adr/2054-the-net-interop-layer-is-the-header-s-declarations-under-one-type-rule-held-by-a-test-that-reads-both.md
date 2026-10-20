# 2054 — The .NET interop layer is the header's declarations under one type rule, held by a test that reads both

Session 1609, slot 3 of batch ninety-six, a "host" round. Status: **accepted**; RFC 0010's step N0,
built. Context: ADR 2048 (the .NET host joins the C ABI as a level host), RFC 0010 §3.1 and §5,
ADR 0247 (the header is hand-written and checked rather than generated).

## Decision

1. **`hosts/windows/Quorra.Interop/` holds one C# declaration for each function `quorra.h`
   declares, one constant for each `#define QUORRA_…` and one struct for each struct with a body**,
   under the header's own names (`quorra_open`, `QUORRA_OK`, `quorra_geometry`), in one file per
   section banner of the header (`Native.Commands.cs`, `Native.Events.cs`, …), plus `Structs.cs`,
   `Native.Constants.cs` and `Native.cs`, which states the rule. It is outside the Cargo workspace,
   as `kio/` is (ADR 0869).
2. **One rule maps a C type to a C# type**, written in `Native.cs` and implemented as
   `csharp_type` in `crates/viewer-ffi/tests/header_and_interop_agree.rs`: fixed-width integers,
   `size_t` and `float` to the type of the same width (`size_t` is `nuint`); a `bool` argument
   `[MarshalAs(UnmanagedType.U1)] bool`; a `bool` in memory `byte`; a `const char *` argument
   `string?` marshalled as UTF-8; every other `char *` `byte*`; an opaque handle `nint`; a struct
   its own name; any other `T *` the rule's `T` with its pointers.
3. **The test holds types, not counts.** Each function is declared once; its parameters have the
   header's names, in order; every parameter, result and field has the rule's type; every constant
   has the header's value and the signedness of its `u` suffix; every function is a cdecl
   `LibraryImport` of `Library`, which is the library this crate builds. The brief asked for names,
   argument counts and values. A count would pass a `uint` where the header says `size_t`, and in a
   64-bit process that call compiles, links, runs and reads half a word.
4. **`[LibraryImport]`, not `[DllImport]`**, as RFC 0010 §3.1 says. The marshalling is generated at
   compile time, so a declaration the generator cannot marshal fails the build rather than a call,
   and nothing waits on a runtime stub at the first call. Both would have worked, and the brief named
   `DllImport`.
5. **`net10.0`.** It is the long-term-support release in force: .NET 8's support ends in November
   2026. The library names no Windows type, so it builds wherever a .NET SDK does. The UI framework
   is still the owner's (Q501), and every one of the three candidates can reference it.
6. **Everything is `internal`.** A pointer-level surface is what this layer is, not what a window
   should program against. RFC 0010 §3.2's `quorra_window` layer is the public surface.
7. **No prose is repeated.** The header's comments are the one place an entry point is explained.
   A second copy would drift, and neither test reads prose.
8. **The C# was first written by a translation script, which is not committed**, and from now on
   it is maintained by hand, as the header is. What keeps it right is the test, not the script.
   Committing the script would have made the C# a derivative of the header that nobody reads, which
   is the thing ADR 0247 declined.

## What it costs

- **No gate compiles the C#.** The machine has no .NET SDK (`which dotnet` finds nothing), and the
  round did not install one: RFC 0010 §8.4 asks the owner whether a user-local SDK may be installed.
  Anything that only a compiler catches is caught by `dotnet build` in a CI job, which a later step
  owes: an attribute the generator rejects, a missing `using`, a nullability warning under
  `TreatWarningsAsErrors`. The C# was written to the documented LibraryImport rules (blittable
  structs only, `bool` with an explicit `MarshalAs`, `AllowUnsafeBlocks`). That is a reading of the
  rules, not a compiler's verdict on them.
- **A new entry point in the header is now two edits**, the header and the C#, and a third where it
  adds a struct. The test fails until the C# is there and names what is missing, which is the point.
