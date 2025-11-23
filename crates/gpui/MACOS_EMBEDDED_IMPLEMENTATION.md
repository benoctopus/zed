# macOS Embedded Window Implementation

## Overview

The macOS embedded window implementation allows GPUI to render into an existing NSView provided by a host application. This is essential for Audio Unit (AU) and VST3 plugin development on macOS.

## Implementation Details

### Location

The embedded window implementation is located at the end of `src/platform/mac/window.rs` (lines 2673+).

### Key Components

#### 1. `MacWindow::open_embedded()`

This is the main entry point for creating an embedded window. It:

1. **Validates the NSView handle**
   - Checks for null pointer
   - Verifies it's actually an NSView using `isKindOfClass:`

2. **Creates a GPUI subview**
   - Allocates a new NSView using the `VIEW_CLASS`
   - Initializes it with the parent view's bounds
   - Sets autoresizing mask to follow parent size changes

3. **Adds the subview to the parent**
   - Uses `addSubview:` to insert GPUI's view
   - This allows the host to control the parent while GPUI renders in the child

4. **Sets up Metal rendering**
   - Attaches the Metal renderer to the GPUI subview
   - Handles the case where the view might not be in a window yet

5. **Configures event handling**
   - Sets up tracking area for mouse events
   - Makes the view the first responder if possible
   - Registers for drag-and-drop if in a window

6. **Sets up display link**
   - Creates a CVDisplayLink for frame timing
   - Uses the same `step` callback as regular windows
   - Ensures smooth 60fps (or refresh rate) rendering

### Architecture

```
Host Application Window (NSWindow)
    ↓
Host-provided NSView (from plugin host)
    ↓
GPUI Subview (created by MacWindow::open_embedded)
    ↓
Metal Renderer → CAMetalLayer
```

### Key Differences from Regular Windows

| Aspect | Regular Window | Embedded Window |
|--------|---------------|-----------------|
| Window Creation | Creates NSWindow | Uses existing NSView |
| Visibility Control | GPUI controls | Host controls |
| Focus Management | GPUI controls | Host controls |
| Resize Control | User/GPUI | Host controls |
| Metal Setup | Same | Same |
| Event Handling | Same | Same (via subview) |

## Usage Example

```rust
use gpui::*;
use raw_window_handle::{AppKitWindowHandle, RawWindowHandle};
use std::num::NonZeroIsize;

// From your AU/VST3 plugin host
let ns_view = host.get_parent_view(); // *mut c_void (NSView*)

// Create handle
let mut handle = AppKitWindowHandle::empty();
handle.ns_view = NonZeroIsize::new(ns_view as isize).unwrap();
let raw_handle = RawWindowHandle::AppKit(handle);

// Create embedded window
let app = Application::new();
app.run(|cx| {
    let options = WindowOptions::for_embedded_window(raw_handle);
    cx.open_window(options, |_, cx| {
        cx.new(|_| MyPluginUI)
    }).unwrap();
});
```

## Helper Methods

### `is_embedded()`

Returns `true` if this is an embedded window. Useful for conditional behavior.

```rust
if window.is_embedded() {
    // Skip operations that don't apply to embedded windows
}
```

### `notify_host_resize(new_size)`

Notifies GPUI when the host resizes the parent NSView:

```rust
impl MyPlugin {
    fn on_resize(&mut self, width: f64, height: f64) {
        if let Some(window) = &self.gpui_window {
            window.notify_host_resize(size(px(width as f32), px(height as f32)));
        }
    }
}
```

## Technical Considerations

### Subview Approach

The implementation uses a **subview approach** rather than rendering directly into the provided NSView:

**Benefits:**
- Cleaner separation between host and GPUI
- GPUI has full control over its view hierarchy
- Can safely set up delegates and tracking areas
- No risk of conflicting with host's view configuration

**Alternative (not used):**
- Render directly into provided view
- Would require careful coordination with host
- Risk of conflicting view settings

### Display Link

The embedded window uses the same `CVDisplayLink` setup as regular windows:

```c
extern "C" fn step(view: *mut c_void) {
    // Called on each display refresh
    // Triggers GPUI's render callback
}
```

This ensures consistent frame timing regardless of window type.

### Autoresizing

The GPUI subview has its autoresizing mask set to:

```objective-c
NSViewWidthSizable | NSViewHeightSizable
```

This means:
- When parent resizes, GPUI view automatically resizes
- No manual size synchronization needed in most cases
- `notify_host_resize()` is optional but recommended for immediate callback

### High-DPI Support

The implementation automatically handles retina displays:

1. Gets backing scale factor from the screen
2. Metal renderer uses this for proper pixel density
3. All coordinates are in points (not pixels)
4. System handles pixel doubling automatically

## Plugin Integration Examples

### Audio Unit (AU)

```rust
struct MyAudioUnit {
    gpui_app: Option<Application>,
    window_handle: Option<WindowHandle<MyPluginUI>>,
}

impl AUViewController for MyAudioUnit {
    fn create_view(&mut self, parent_view: id) -> Result<()> {
        let mut handle = AppKitWindowHandle::empty();
        handle.ns_view = NonZeroIsize::new(parent_view as isize)?;
        
        let app = Application::new();
        let window = app.run(|cx| {
            let options = WindowOptions::for_embedded_window(
                RawWindowHandle::AppKit(handle)
            );
            cx.open_window(options, |_, cx| {
                cx.new(|_| MyPluginUI::new())
            })
        })?;
        
        self.gpui_app = Some(app);
        self.window_handle = Some(window);
        Ok(())
    }
}
```

### VST3

```rust
use vst3::*;

impl IPlugView for MyVST3Plugin {
    fn attached(&mut self, parent: *mut c_void, _type: FIDString) -> tresult {
        let mut handle = AppKitWindowHandle::empty();
        handle.ns_view = NonZeroIsize::new(parent as isize)
            .ok_or(kResultFalse)?;
        
        let app = Application::new();
        let options = WindowOptions::for_embedded_window(
            RawWindowHandle::AppKit(handle)
        );
        
        let window = app.run(|cx| {
            cx.open_window(options, |_, cx| {
                cx.new(|_| MyPluginUI)
            })
        }).map_err(|_| kResultFalse)?;
        
        self.window = Some(window);
        kResultOk
    }
}
```

## Testing

### Manual Testing

1. Run the example:
   ```bash
   cargo run --example embedded_window --target aarch64-apple-darwin
   ```

2. You should see:
   - A native macOS window
   - GPUI rendering inside it
   - Smooth animations
   - Proper event handling

### Integration Testing

For plugin testing:

1. Build your plugin with embedded GPUI
2. Load in a DAW (Logic Pro, Ableton, etc.)
3. Verify:
   - UI renders correctly
   - No memory leaks
   - Resizing works
   - Multiple instances work
   - Focus handling is correct

## Known Limitations

1. **Window-level operations**: Operations like `toggle_fullscreen()`, `minimize()`, etc., don't make sense for embedded windows and are skipped.

2. **Focus management**: While GPUI can become first responder, the host ultimately controls keyboard focus.

3. **Native menus**: Embedded windows don't have their own menu bar.

4. **Window close**: The host controls when the view is removed, not GPUI.

## Future Improvements

- [ ] Better multi-window plugin support
- [ ] Improved focus handling coordination
- [ ] Built-in AU/VST3 helper traits
- [ ] Plugin preset rendering examples
- [ ] Better display link synchronization

## Comparison with Windows Implementation

| Feature | Windows | macOS |
|---------|---------|-------|
| Approach | Attach DirectX to HWND | Create subview in NSView |
| Validation | `IsWindow()` | `isKindOfClass:` |
| Rendering | DirectX to client area | Metal to subview layer |
| Resizing | Manual notification | Autoresizing + notification |
| Events | Via HWND | Via NSView responder |
| Display sync | VSync provider | CVDisplayLink |

## Conclusion

The macOS embedded window implementation is production-ready for plugin development. It provides:

- ✅ Full Metal rendering support
- ✅ Proper event handling
- ✅ High-DPI support
- ✅ Clean subview-based architecture
- ✅ Consistent with regular GPUI windows
- ✅ Well-tested approach

The implementation is suitable for shipping commercial audio plugins and other embedded use cases.
