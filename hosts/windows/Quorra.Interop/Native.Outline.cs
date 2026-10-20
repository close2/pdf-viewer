// quorra.h's section on §12.3.3's outline, declared for .NET. What each function does,
// what it consumes and when it refuses is the comment above its declaration in the header.

using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

namespace Quorra.Interop;

internal static unsafe partial class Native
{
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_outline_read(nint viewer, nint* outline);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial void quorra_outline_free(nint outline);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial nuint quorra_outline_len(nint outline);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_outline_title(
        nint outline,
        nuint row,
        byte* @out,
        nuint cap,
        nuint* needed);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_outline_depth(
        nint outline,
        nuint row,
        uint* depth,
        byte* expanded);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_outline_object(
        nint outline,
        nuint row,
        uint* number,
        ushort* generation);
}
