// Public API for embedded window support (plugins, etc.)

use crate::WindowOptions;
use raw_window_handle;

/// Extensions to WindowOptions for embedded contexts
impl WindowOptions {
    /// Create window options for embedding GPUI into an existing native window
    /// 
    /// This is useful for plugin development (VST, CLAP, AU) or embedding GPUI
    /// into other applications.
    /// 
    /// # Arguments
    /// * `raw_handle` - The native window handle from `raw-window-handle` crate
    /// 
    /// # Example
    /// ```ignore
    /// use raw_window_handle::{RawWindowHandle, Win32WindowHandle};
    /// 
    /// // From your plugin host
    /// let hwnd = plugin_host.get_window_handle();
    /// 
    /// let win32_handle = Win32WindowHandle::new(
    ///     std::num::NonZeroIsize::new(hwnd as isize).unwrap()
    /// );
    /// let raw_handle = RawWindowHandle::Win32(win32_handle);
    /// 
    /// let options = WindowOptions::for_embedded_window(raw_handle);
    /// cx.open_window(options, |window, cx| {
    ///     cx.new(|_| MyPluginUI::new())
    /// });
    /// ```
    pub fn for_embedded_window(raw_handle: raw_window_handle::RawWindowHandle) -> Self {
        Self {
            raw_window_handle: Some(raw_handle),
            // These options don't matter for embedded windows as the host controls them
            focus: false,
            show: false, 
            is_movable: false,
            is_resizable: false,
            is_minimizable: false,
            titlebar: None,
            ..Default::default()
        }
    }
}
