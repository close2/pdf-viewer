# 2184 — Descriptor transfer is Unix-only, and the C# compiler check travels with the host

Session 1674. Status: accepted; validation recorded in session 1674. Context: ADRs 2140, 2141, 2168;
CI run 39555931043 on `5ded4916`.

## The platform boundary

The confined transform broker can send a file descriptor only on Unix. The descriptor answer's
receiver and the broker's descriptor branch are compiled only there. A descriptor answer received
elsewhere is an explicit `Unsupported` error; the byte answer retains its existing framing and
budget. This does not build a Windows worker: the Windows CLI already announces its in-process
route. Imports used only by the Unix route follow that same compilation boundary.

The large-grid encoder test checks the confinement the platform actually installs. Linux must
report enforcement. Other platforms must report no enforcement and no address-space ceiling;
they still encode the admitted grid and refuse the next one. Testing a budget does not establish
that the operating system confines the worker.

## A portable compiler check

`tools/dotnet-check.sh` finds an SDK through `DOTNET_ROOT`, the saved local SDK or `PATH`, reads the
held project files and compiles the host against their actual package references. Temporary
projects disable packaging and replace only XAML-generated fields and initialization methods.
The check never substitutes an SDK type. Its temporary files are removed on exit.

The conformance test probes for the SDK: an absent optional SDK is reported, while a discovered
SDK whose build fails fails the test. The command is also directly runnable. XML parsing and
C# compilation cannot validate XAML semantics, layout, native loading or interaction; Windows
CI and a Windows drive remain the instruments for those questions.
