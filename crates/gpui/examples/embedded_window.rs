// Example of embedding GPUI into an existing window
// This demonstrates how to use GPUI in plugin contexts (VST, CLAP, etc.)

use gpui::*;

#[cfg(target_os = "windows")]
fn main() {
    use raw_window_handle::{RawWindowHandle, Win32WindowHandle};
    use std::num::NonZeroIsize;
    use windows::Win32::Foundation::*;
    use windows::Win32::UI::WindowsAndMessaging::*;

    // For demonstration, create a simple Win32 window that will act as our "host"
    // In a real plugin, this window would be provided by the host application
    unsafe {
        let host_window = create_host_window();

        println!("Created host window: {:?}", host_window);
        println!("Starting GPUI in embedded mode...");

        // Create raw window handle from the Win32 HWND
        let win32_handle = Win32WindowHandle::new(
            NonZeroIsize::new(host_window.0 as isize).expect("HWND is null"),
        );
        let raw_handle = RawWindowHandle::Win32(win32_handle);

        // Create GPUI app
        let app = Application::new();

        app.run(move |cx| {
            // Open a window using the existing HWND
            let options = WindowOptions::for_embedded_window(raw_handle);

            cx.open_window(options, |window, cx| {
                window.set_window_title("Embedded GPUI Window");
                cx.new(|_| EmbeddedView)
            })
            .expect("Failed to open embedded window");
        });
    }
}

#[cfg(target_os = "macos")]
fn main() {
    use cocoa::appkit::{NSBackingStoreType, NSWindow, NSWindowStyleMask};
    use cocoa::base::{id, nil};
    use cocoa::foundation::{NSPoint, NSRect, NSSize, NSString};
    use objc::{msg_send, sel, sel_impl};
    use raw_window_handle::{AppKitWindowHandle, RawWindowHandle};
    use std::ffi::c_void;
    use std::ptr::NonNull;

    // Create GPUI app first - it will initialize NSApplication properly
    let app = Application::new();

    app.run(move |cx| {
        unsafe {
            // Now create a host window that will contain our embedded GPUI view
            let window = NSWindow::alloc(nil).initWithContentRect_styleMask_backing_defer_(
                NSRect::new(NSPoint::new(100.0, 100.0), NSSize::new(800.0, 600.0)),
                NSWindowStyleMask::NSTitledWindowMask
                    | NSWindowStyleMask::NSClosableWindowMask
                    | NSWindowStyleMask::NSResizableWindowMask
                    | NSWindowStyleMask::NSMiniaturizableWindowMask,
                NSBackingStoreType::NSBackingStoreBuffered,
                false as objc::runtime::BOOL,
            );

            let _: () = msg_send![window, setTitle: cocoa::foundation::NSString::alloc(nil).init_str("GPUI Embedded Host Window")];
            let _: () = msg_send![window, makeKeyAndOrderFront: nil];

            // Get the content view - this is where we'll embed GPUI
            let content_view: id = msg_send![window, contentView];

            println!("Created host window with content view: {:?}", content_view);
            println!("Starting GPUI in embedded mode...");

            // Create raw window handle from NSView
            let ns_view = NonNull::new(content_view as *mut c_void).expect("NSView is null");
            let appkit_handle = AppKitWindowHandle::new(ns_view);
            let raw_handle = RawWindowHandle::AppKit(appkit_handle);

            // Open a window using the existing NSView
            let options = WindowOptions::for_embedded_window(raw_handle);

            cx.open_window(options, |window, cx| {
                window.set_window_title("Embedded GPUI Window");
                cx.new(|_| EmbeddedView)
            })
            .expect("Failed to open embedded window");
        }
    });
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn main() {
    println!("Linux embedded window support not yet implemented in this example");
    println!("The API is available via WindowOptions::for_embedded_window()");
    println!("Pass an X11 Window or Wayland surface via raw-window-handle");
}

struct EmbeddedView;

impl Render for EmbeddedView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_center()
            .size_full()
            .bg(rgb(0x2e3440))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(
                        div()
                            .text_xl()
                            .text_color(rgb(0xeceff4))
                            .child("GPUI Embedded Window"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0xd8dee9))
                            .child("This GPUI window is rendered inside an external host window"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0xa3be8c))
                            .child("Perfect for VST/CLAP plugins!"),
                    ),
            )
    }
}

#[cfg(target_os = "windows")]
unsafe fn create_host_window() -> windows::Win32::Foundation::HWND {
    use windows::core::w;
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;

    let class_name = w!("GPUIHostWindow");

    let wc = WNDCLASSW {
        lpfnWndProc: Some(DefWindowProcW),
        hInstance: GetModuleHandleW(None).unwrap().into(),
        lpszClassName: class_name,
        ..Default::default()
    };

    RegisterClassW(&wc);

    CreateWindowExW(
        WINDOW_EX_STYLE(0),
        class_name,
        w!("GPUI Embedded Host Window"),
        WS_OVERLAPPEDWINDOW | WS_VISIBLE,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        800,
        600,
        None,
        None,
        GetModuleHandleW(None).unwrap(),
        None,
    )
    .expect("Failed to create host window")
}
