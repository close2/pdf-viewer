// Effective library state and the pane's user choice stay separate: hiding navigation controls
// does not close an outline, and hiding all panels temporarily must not forget an open pane.
namespace Quorra.App;

internal readonly record struct WindowChrome(bool MenuBar, bool ToolBar, bool WindowUI, bool OtherWindows);
internal readonly record struct ChromeLayout(bool Menu, bool Navigation, bool Status, bool Panels, bool PaneOpen);

internal sealed class ChromeState
{
    private bool panelsAllowed = true;
    private bool openBeforeHiding;

    internal ChromeLayout Apply(WindowChrome chrome, bool paneOpen, int? panel = null)
    {
        if (panelsAllowed && !chrome.OtherWindows) openBeforeHiding = paneOpen;
        if (!panelsAllowed && chrome.OtherWindows) paneOpen = openBeforeHiding;
        panelsAllowed = chrome.OtherWindows;
        if (chrome.OtherWindows && panel is int requested) paneOpen = requested >= 0;
        // The reader's Permissions menu stays available despite HideMenubar (ADR 1145),
        // as in the other hosts; presentation hides it. This host has no toolbar.
        return new(chrome.OtherWindows, chrome.WindowUI && chrome.OtherWindows,
            chrome.WindowUI, chrome.OtherWindows, chrome.OtherWindows && paneOpen);
    }
}
