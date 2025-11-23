// Support for embedding GPUI windows into existing native window handles
// This is useful for plugins (VST, CLAP, AU) and other host applications

use super::*;
use crate::*;
use anyhow::{Context as _, Result};
use raw_window_handle as rwh;
use std::rc::Rc;
use std::cell::RefCell;
use std::sync::Arc;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::*;

impl WindowsWindow {
    /// Create a WindowsWindow that attaches to an existing HWND instead of creating a new window
    pub(crate) fn new_embedded(
        handle: AnyWindowHandle,
        params: WindowParams,
        creation_info: WindowCreationInfo,
        raw_handle: rwh::RawWindowHandle,
    ) -> Result<Self> {
        // Extract HWND from the raw window handle
        let hwnd = match raw_handle {
            rwh::RawWindowHandle::Win32(win32_handle) => {
                HWND(win32_handle.hwnd.get() as isize)
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Expected Win32 window handle for Windows platform, got {:?}",
                    raw_handle
                ))
            }
        };

        // Validate that the HWND is valid
        if hwnd.0 == 0 || unsafe { !IsWindow(hwnd).as_bool() } {
            return Err(anyhow::anyhow!("Invalid HWND provided: {:?}", hwnd));
        }

        let WindowCreationInfo {
            executor,
            current_cursor,
            windows_version,
            drop_target_helper,
            validation_number,
            main_receiver,
            platform_window_handle,
            disable_direct_composition,
            directx_devices,
            invalidate_devices,
            ..
        } = creation_info;

        // Get the display (monitor) for this HWND
        let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTOPRIMARY) };
        let display = WindowsDisplay::new_with_handle(monitor);
        let appearance = system_appearance().log_err().unwrap_or_default();

        // Get the client rect to determine initial size
        let mut rect = RECT::default();
        unsafe {
            GetClientRect(hwnd, &mut rect)
                .context("Failed to get client rect for embedded window")?;
        }

        // Create a synthetic CREATESTRUCTW for WindowsWindowState::new
        let cs = CREATESTRUCTW {
            lpCreateParams: std::ptr::null_mut(),
            hInstance: HINSTANCE(0),
            hMenu: HMENU(0),
            hwndParent: HWND(0),
            cy: rect.bottom - rect.top,
            cx: rect.right - rect.left,
            y: 0,
            x: 0,
            style: 0,
            lpszName: PCWSTR::null(),
            lpszClass: PCWSTR::null(),
            dwExStyle: 0,
        };

        let state = RefCell::new(WindowsWindowState::new(
            hwnd,
            &directx_devices,
            &cs,
            current_cursor,
            display,
            params.window_min_size,
            appearance,
            disable_direct_composition,
            invalidate_devices.clone(),
        )?);

        let inner = Rc::new(WindowsWindowInner {
            hwnd,
            drop_target_helper,
            state,
            handle,
            hide_title_bar: false, // Embedded windows don't control the title bar
            is_movable: false,     // Host controls movement
            executor,
            windows_version,
            validation_number,
            main_receiver,
            platform_window_handle,
            system_settings: RefCell::new(WindowsSystemSettings::new(display)),
        });

        // Set up the window state pointer so events can find our state
        // This allows the window procedure to access our WindowsWindowInner
        unsafe {
            set_window_long(hwnd, GWLP_USERDATA, Rc::as_ptr(&inner) as isize);
        }

        // Register for drag and drop
        unsafe {
            if let Err(e) = RegisterDragDrop(hwnd, &DropTarget(Rc::downgrade(&inner))) {
                log::error!("Failed to register drag and drop for embedded window: {}", e);
            }
        }

        // Increment the reference count since we're storing it in GWLP_USERDATA
        // This prevents the Rc from being dropped while the HWND is still alive
        Rc::increment_strong_count(Rc::as_ptr(&inner));

        // Note: We do NOT call ShowWindow or SetFocus here, as the host controls visibility and focus

        Ok(Self(inner))
    }
}

/// Helper methods for embedded windows
impl WindowsWindow {
    /// Notify an embedded window of resize events from the host
    /// This should be called by the host application when the window size changes
    pub fn notify_host_resize(&self, new_size: Size<Pixels>) {
        let mut state = self.0.state.borrow_mut();
        state.logical_size = new_size;
        
        if let Some(callback) = &mut state.callbacks.resize {
            callback(new_size, state.scale_factor);
        }
    }

    /// Check if this is an embedded window (attached to an external HWND)
    /// This can be used to skip certain operations that don't apply to embedded windows
    pub fn is_embedded(&self) -> bool {
        // In a full implementation, you might want to store this as a flag
        // For now, we can check if we're movable (embedded windows are not)
        !self.0.is_movable
    }
}
