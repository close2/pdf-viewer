// quorra.h's section on the commands, declared for .NET. What each function does,
// what it consumes and when it refuses is the comment above its declaration in the header.

using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

namespace Quorra.Interop;

internal static unsafe partial class Native
{
    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_open(
        nint viewer,
        ulong document,
        byte* bytes,
        nuint len,
        string? password,
        string? fragment,
        nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_close(nint viewer, ulong document, nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_focus(nint viewer, ulong document, nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_resize(
        nint viewer,
        uint width,
        uint height,
        float scale,
        nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_go_to_page(
        nint viewer,
        uint target,
        long argument,
        nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_zoom(nint viewer, uint zoom, float scale, nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_scroll(nint viewer, float dx, float dy, nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_set_view(nint viewer, quorra_viewing view, nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_find_start(
        nint viewer,
        string? needle,
        int backward,
        nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_find_continue(nint viewer, nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_find_stop(nint viewer, nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_activate(
        nint viewer,
        uint number,
        ushort generation,
        nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_render_ready_raster(
        nint viewer,
        nint request,
        nint raster,
        nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_render_ready_failed(
        nint viewer,
        nint request,
        string? why,
        nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_pointer(
        nint viewer,
        float x,
        float y,
        uint action,
        nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_select(nint viewer, uint what, nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_focused(nint viewer, uint direction, nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_set_field_text(
        nint viewer,
        string? field,
        string? text,
        nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_set_field_options(
        nint viewer,
        string? field,
        nuint* options,
        nuint count,
        nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_clear_field(nint viewer, string? field, nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_markup(
        nint viewer,
        uint kind,
        float red,
        float green,
        float blue,
        nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_free_text(
        nint viewer,
        float from_x,
        float from_y,
        float to_x,
        float to_y,
        float red,
        float green,
        float blue,
        nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_set_free_text(
        nint viewer,
        uint number,
        ushort generation,
        string? text,
        nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_set_note_text(
        nint viewer,
        uint number,
        ushort generation,
        string? text,
        nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_attach(
        nint viewer,
        byte* bytes,
        nuint len,
        string? name,
        string? description,
        string? mime,
        uint home,
        float x,
        float y,
        nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_detach(nint viewer, string? name, nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_answer(
        nint viewer,
        ulong document,
        [MarshalAs(UnmanagedType.U1)] bool proceed,
        nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_undo(nint viewer, nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_redo(nint viewer, nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_save(nint viewer, nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_extract(nint viewer, string? name, nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_document_report(nint viewer, nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_presented(nint viewer, nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_supply(
        nint viewer,
        uint purpose,
        byte* bytes,
        nuint len,
        nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_set_group(
        nint viewer,
        uint number,
        ushort generation,
        [MarshalAs(UnmanagedType.U1)] bool on,
        nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_tick(nint viewer, uint millis, nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_present(nint viewer, uint mode, nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_layout(nint viewer, uint layout, nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_restrict(nint viewer, uint level, nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_restrict_operation(
        nint viewer,
        uint operation,
        uint level,
        nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_restrict_document_operation(
        nint viewer,
        uint operation,
        uint level,
        nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_copy(nint viewer, nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_print(nint viewer, float* media, float scale, nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_print_placed(
        nint viewer,
        float* media,
        float scale,
        float page_scale,
        nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_event_printing(
        nint events,
        nuint index,
        nuint* pages,
        byte* degraded);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_print_finish(nint viewer, nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_print_page(
        nint viewer,
        nuint page,
        uint* width,
        uint* height,
        nuint* bytes);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_print_page_copy(
        nint viewer,
        nuint page,
        byte* into,
        nuint cap,
        nuint* written);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_printed_reports(nint viewer, nuint* count);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_printed_report(
        nint viewer,
        nuint index,
        byte* @out,
        nuint cap,
        nuint* needed);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_archive_profiles(nuint* count);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_archive_profile(
        nuint index,
        byte* @out,
        nuint cap,
        nuint* needed);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_archive_profile_sentence(
        nuint index,
        byte* @out,
        nuint cap,
        nuint* needed);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_archive(
        nint viewer,
        string? target,
        string? profile,
        nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_archived(nint viewer, nuint* bytes, nuint* lines);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_archived_copy(
        nint viewer,
        byte* into,
        nuint cap,
        nuint* written);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_archived_line(
        nint viewer,
        nuint index,
        byte* @out,
        nuint cap,
        nuint* needed);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_archived_json(
        nint viewer,
        byte* @out,
        nuint cap,
        nuint* needed);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_event_copied(
        nint events,
        nuint index,
        byte* logical,
        byte* @out,
        nuint cap,
        nuint* needed);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_delegate(nint viewer, uint appearances, nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_trust_anchors(
        nint viewer,
        byte** certificates,
        nuint* lengths,
        byte** names,
        nuint count,
        string? source,
        long at,
        uint acceptance,
        nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_reference_files(
        nint viewer,
        byte** files,
        nuint* lengths,
        byte** names,
        nuint count,
        string? source,
        nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_audience(
        nint viewer,
        byte** names,
        nuint name_count,
        byte** titles,
        nuint title_count,
        byte** organisations,
        nuint organisation_count,
        string? language,
        nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_separations(
        nint viewer,
        [MarshalAs(UnmanagedType.U1)] bool simulate,
        nint* events);

    [LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int quorra_beside(
        nint viewer,
        [MarshalAs(UnmanagedType.U1)] bool reserved,
        ulong name,
        nint* events);
}
