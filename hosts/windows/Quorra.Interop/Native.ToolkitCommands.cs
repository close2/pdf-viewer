// quorra.h's section on the commands a toolkit needs, declared for .NET. What each function does,
// what it consumes and when it refuses is the comment above its declaration in the header.

using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

namespace Quorra.Interop;

internal static unsafe partial class Native
{
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_commit_field(nint viewer, string? field, nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_clock(
        nint viewer,
        [MarshalAs(UnmanagedType.U1)] bool known,
        int year,
        byte month,
        byte day,
        byte hour,
        byte minute,
        byte second,
        [MarshalAs(UnmanagedType.U1)] bool zoned,
        short offset_minutes,
        nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_respond(
        nint viewer,
        ulong document,
        uint answers,
        string? source,
        uint format,
        byte* bytes,
        nuint len,
        nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_oversize(nint viewer, uint how, nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_keys(
        nint viewer,
        [MarshalAs(UnmanagedType.U1)] bool shift,
        [MarshalAs(UnmanagedType.U1)] bool modifier,
        [MarshalAs(UnmanagedType.U1)] bool arrows,
        nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_scripts(nint viewer, uint level, nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_answer_scripts(
        nint viewer,
        ulong document,
        [MarshalAs(UnmanagedType.U1)] bool proceed,
        nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_answer_script(
        nint viewer,
        ulong document,
        uint how,
        uint button,
        string? text,
        nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_data_format_of_media_type(string? media_type, uint* format);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_data_format_of_name(string? name, uint* format);
}
