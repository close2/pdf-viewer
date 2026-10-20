// quorra.h's section on the keyboard, declared for .NET (RFC 0010 step N1, ADR 2072). What each
// function does, what it sends and when it refuses is the comment above its declaration in the
// header.

using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

namespace Quorra.Interop;

internal static unsafe partial class Native
{
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_key_press(
        nint viewer,
        uint key,
        [MarshalAs(UnmanagedType.U1)] bool shift,
        [MarshalAs(UnmanagedType.U1)] bool command,
        uint present,
        [MarshalAs(UnmanagedType.U1)] bool warned,
        uint* act,
        float* by,
        nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_key_said(nint viewer, byte* @out, nuint cap, nuint* needed);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial uint quorra_key_count();

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial uint quorra_act_count();

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial byte* quorra_act_name(uint act);
}
