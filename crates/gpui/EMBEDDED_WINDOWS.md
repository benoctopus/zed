## Embedded Window Support

GPUI now supports rendering into existing native window handles, which is essential for plugin development (VST, CLAP, AU, etc.) and embedding GPUI into other applications.

### Overview

Instead of creating its own OS window, GPUI can attach to an existing window handle provided by a host application. The rendering pipeline works exactly the same, but window creation and lifecycle management are skipped.

### Usage

```rust
use gpui::*;
use raw_window_handle::{RawWindowHandle, Win32WindowHandle};

// Get the native window handle from your plugin host
let hwnd = plugin_host.get_window_handle(); // Platform-specific

// Create a raw window handle
#[cfg(target_os = "windows")]
let raw_handle = {
    use std::num::NonZeroIsize;
    let win32_handle = Win32WindowHandle::new(
        NonZeroIsize::new(hwnd as isize).unwrap()
    );
    RawWindowHandle::Win32(win32_handle)
};

// Create window options for embedded mode
let options = WindowOptions::for_embedded_window(raw_handle);

// Open the window - it will attach to the existing handle
app.run(|cx| {
    cx.open_window(options, |window, cx| {
        cx.new(|_| MyPluginUI::new())
    }).expect("Failed to create embedded window");
});
```

### Platform-Specific Notes

#### Windows

- Pass a valid `HWND` via `Win32WindowHandle`
- The window must remain valid for the lifetime of the GPUI window
- DirectX will render directly into the provided HWND's client area
- Host controls window visibility, focus, and size
- To notify GPUI of size changes from the host, you'll need to trigger resize events

#### macOS

- Pass an `NSView*` via `AppKitWindowHandle`
- GPUI creates a subview within the provided NSView
- Metal renderer attaches to the GPUI subview
- Automatically handles retina/high-DPI displays
- Display link is set up for proper frame timing
- Supports `notify_host_resize()` for host-driven size changes
- Autoresizing mask is set so the view follows parent resizing

#### Linux (X11/Wayland)

- Pass an X11 Window ID or Wayland surface
- Blade/Vulkan will attach to the existing surface

### Plugin Development Example

Here's a minimal VST3 plugin using GPUI:

```rust
use vst3::*;
use gpui::*;
use raw_window_handle::RawWindowHandle;

struct MyPlugin {
    app: Option<Application>,
    window_handle: Option<WindowHandle<MyPluginUI>>,
}

impl Plugin for MyPlugin {
    fn create_view(&mut self, parent_handle: RawWindowHandle) -> Result<()> {
        let app = Application::new();
        
        let options = WindowOptions::for_embedded_window(parent_handle);
        
        let handle = app.run(|cx| {
            cx.open_window(options, |_, cx| {
                cx.new(|_| MyPluginUI::new())
            })
        })?;
        
        self.app = Some(app);
        self.window_handle = Some(handle);
        Ok(())
    }
    
    fn on_size(&mut self, width: i32, height: i32) {
        // Notify GPUI of size changes from the host
        if let Some(handle) = &self.window_handle {
            handle.update(|_, window, _| {
                window.resize(size(px(width as f32), px(height as f32)));
            });
        }
    }
}
```

### Differences from Normal Windows

When using embedded windows:

1. **No window creation**: GPUI doesn't call `CreateWindowExW`, `NSWindow`, etc.
2. **No window control**: Title bar, resize, minimize, maximize are controlled by the host
3. **No focus management**: Host controls keyboard focus
4. **Event routing**: Events come from the host's message pump
5. **Lifecycle**: The window exists only as long as the host keeps it alive

### Implementation Details

The implementation adds a `raw_window_handle` field to `WindowOptions` and `WindowParams`:

```rust
pub struct WindowOptions {
    // ... existing fields
    
    /// An existing native window handle to attach to instead of creating a new window
    pub raw_window_handle: Option<rwh::RawWindowHandle>,
}
```

Platform implementations check this field in `Platform::open_window()`:

```rust
fn open_window(&self, handle: AnyWindowHandle, params: WindowParams) 
    -> Result<Box<dyn PlatformWindow>> 
{
    if let Some(raw_handle) = params.raw_window_handle {
        // Embedded mode: attach to existing handle
        WindowsWindow::new_embedded(handle, params, info, raw_handle)
    } else {
        // Normal mode: create new window
        WindowsWindow::new(handle, params, info)
    }
}
```

The `new_embedded` implementation:
1. Validates the handle
2. Skips window creation
3. Attaches the renderer directly to the existing handle
4. Sets up all callbacks and state as normal
5. Skips visibility and focus operations

### Current Status

- ✅ Windows: Fully implemented
- ✅ macOS: Fully implemented
- ⚠️  Linux: API ready, implementation needed

### Testing

Run the embedded window example:

```bash
cargo run --example embedded_window
```

This creates a simple Win32 window and embeds GPUI into it, demonstrating the basic flow.

### Future Enhancements

- [x] macOS NSView embedding
- [ ] Linux X11/Wayland embedding  
- [ ] Better event handling for embedded contexts
- [x] Host-to-GPUI size/scale change notifications (via `notify_host_resize()`)
- [ ] Better focus management in embedded mode
- [ ] Example CLAP plugin implementation
- [ ] Example AU plugin implementation
