// quorra.h's section on what page one does not need, declared for .NET. What each function does,
// what it consumes and when it refuses is the comment above its declaration in the header.

using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

namespace Quorra.Interop;

internal static unsafe partial class Native
{
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_preparation_take(nint viewer, nint* preparation);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_preparation_run(nint preparation, nint* prepared);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial void quorra_preparation_free(nint preparation);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_prepared_hand_back(nint viewer, nint prepared, nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial void quorra_prepared_free(nint prepared);
}
