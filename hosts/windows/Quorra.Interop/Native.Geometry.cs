// quorra.h's section on geometry, declared for .NET. What each function does,
// what it consumes and when it refuses is the comment above its declaration in the header.

using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

namespace Quorra.Interop;

internal static unsafe partial class Native
{
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_selection_text(
        nint viewer,
        byte* @out,
        nuint cap,
        nuint* needed);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_selection_quads(nint viewer, nint* quads);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_selection_copy_text(
        nint viewer,
        byte* @out,
        nuint cap,
        nuint* needed,
        uint* order);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_highlight_quads(nint viewer, nint* quads);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial void quorra_quads_free(nint quads);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial nuint quorra_quads_len(nint quads);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_quads_get(nint quads, nuint index, float* into);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_focused_annotation(
        nint viewer,
        uint* number,
        ushort* generation,
        float* quad);
}
