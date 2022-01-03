use winapi::um::dwmapi::DwmExtendFrameIntoClientArea;
use winapi::um::uxtheme::MARGINS;
use winapi::um::wingdi::RGB;
use windows::core::*;
use windows::Foundation::Numerics::Matrix3x2;

use windows::Win32::Foundation::{BOOL, HWND, LPARAM, LRESULT, PWSTR, RECT, WPARAM};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT, D2D_RECT_F, D2D_SIZE_U,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1CreateFactory, ID2D1Factory1, ID2D1HwndRenderTarget, ID2D1SolidColorBrush,
    ID2D1StrokeStyle, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE, D2D1_BRUSH_PROPERTIES,
    D2D1_CAP_STYLE_ROUND, D2D1_CAP_STYLE_TRIANGLE, D2D1_DEBUG_LEVEL_INFORMATION,
    D2D1_FACTORY_OPTIONS, D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_HWND_RENDER_TARGET_PROPERTIES,
    D2D1_PRESENT_OPTIONS_IMMEDIATELY, D2D1_RENDER_TARGET_PROPERTIES,
    D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_STROKE_STYLE_PROPERTIES,
    D2D1_TEXT_ANTIALIAS_MODE_CLEARTYPE, D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE,
};

use windows::Win32::Graphics::DirectWrite::{
    DWriteCreateFactory, IDWriteFactory1, IDWriteFontCollection, IDWriteTextFormat,
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL,
    DWRITE_FONT_WEIGHT_NORMAL,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;

use crate::{game::GameData, utf16_str, utf8_str};
use windows::Win32::Graphics::Gdi::{BeginPaint, CreateSolidBrush, EndPaint, PAINTSTRUCT};
use windows::Win32::System::LibraryLoader::GetModuleHandleA;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExA, DefWindowProcA, DestroyWindow, DisableProcessWindowsGhosting,
    DispatchMessageA, GetWindowLongPtrA, GetWindowRect, LoadCursorW, PeekMessageA, PostQuitMessage,
    RegisterClassExA, SetForegroundWindow, SetLayeredWindowAttributes, SetWindowLongPtrA,
    SetWindowPos, ShowWindow, TranslateMessage, CREATESTRUCTA, CS_HREDRAW, CS_VREDRAW,
    GWLP_USERDATA, HWND_TOPMOST, IDC_ARROW, LWA_ALPHA, MSG, SW_SHOW, ULW_COLORKEY, WM_DESTROY,
    WM_DISPLAYCHANGE, WM_NCCREATE, WM_PAINT, WM_QUIT, WNDCLASSEXA, WS_EX_LAYERED, WS_EX_TOPMOST,
    WS_EX_TRANSPARENT, WS_POPUP, WS_VISIBLE,
};

pub type RenderCTX = GameData;
pub type RenderFn = fn(&mut Overlay);

pub struct Overlay {
    target: HWND,
    overlay: HWND,

    factory: Option<ID2D1Factory1>,
    dw_factory: Option<IDWriteFactory1>,
    render_target: Option<ID2D1HwndRenderTarget>,

    // cached resource
    stroke: Option<ID2D1StrokeStyle>,
    brush: Option<ID2D1SolidColorBrush>,
    dw_text_format: Option<IDWriteTextFormat>,
    render_fn: Option<RenderFn>,
    render_ctx: RenderCTX,
}

#[no_mangle]
unsafe extern "system" fn wnd_proc(
    wnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_NCCREATE {
        let cs = lparam as *const CREATESTRUCTA;
        let this = (*cs).lpCreateParams as *mut Overlay;
        (*this).overlay = wnd;
        SetWindowLongPtrA(wnd, GWLP_USERDATA, this as _);
    } else {
        let this = GetWindowLongPtrA(wnd, GWLP_USERDATA) as *mut Overlay;
        if !this.is_null() {
            return (*this).message_handler(message, wparam, lparam);
        }
    }
    DefWindowProcA(wnd, message, wparam, lparam)
}

impl Overlay {
    pub fn render_target(&self) -> &ID2D1HwndRenderTarget {
        self.render_target.as_ref().unwrap()
    }

    pub fn draw_line(&self) -> Result<()> {
        Ok(())
    }

    pub fn render_ctx_mut(&mut self) -> &mut RenderCTX {
        &mut self.render_ctx
    }

    pub fn render_ctx(&mut self) -> &RenderCTX {
        &self.render_ctx
    }

    pub fn draw_text(&mut self, text: String, x: f32, y: f32, w: f32, h: f32) -> Result<()> {
        unsafe {
            let fmt = self.dw_text_format.as_ref().unwrap().clone();
            let brush = self.brush.as_ref().unwrap();
            let target = self.render_target();
            target.DrawText(
                utf16_str!(text),
                text.len() as u32,
                fmt,
                &D2D_RECT_F {
                    left: x,
                    top: y,
                    right: x + w,
                    bottom: y + h,
                },
                brush,
                0,
                0,
            );
        }
        Ok(())
    }

    /// Ensure our window is positioned over the target window
    pub fn ensure_position(&self) {
        let rect = self.get_rect();

        unsafe {
            SetWindowPos(
                self.get_overlay(),
                HWND_TOPMOST,
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
                0,
            );
        }
    }

    /// Begin drawing loop
    fn draw(&mut self) -> Result<()> {
        {
            self.begin_drawing()?;
        }
        unsafe {
            let ctx = self.render_target.as_ref().unwrap();
            ctx.Clear(&D2D1_COLOR_F {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            });
            if let Some(f) = self.render_fn {
                f(self);
            }
        };
        self.end_drawing()
    }

    /// Set up drawing
    fn begin_drawing(&mut self) -> Result<()> {
        self.ensure_position();
        if self.render_target.is_none() {
            self.init_dx()?;
        }
        unsafe {
            self.render_target.as_ref().unwrap().BeginDraw();
        }
        Ok(())
    }

    /// Finish up drawing
    fn end_drawing(&mut self) -> Result<()> {
        unsafe {
            self.render_target
                .as_ref()
                .unwrap()
                .EndDraw(std::ptr::null_mut(), std::ptr::null_mut())?;
        }
        Ok(())
    }

    fn release_device(&mut self) {
        self.render_target = None;
    }

    /// Get rectangle of target window
    pub fn get_rect(&self) -> RECT {
        let mut rectangle = RECT {
            bottom: 0,
            left: 0,
            right: 0,
            top: 0,
        };

        // get dimensions of target window
        unsafe {
            GetWindowRect(self.get_target(), &mut rectangle);
        }

        rectangle
    }

    /// Spawn an overlay
    pub fn new(target: HWND, ctx: RenderCTX) -> Result<Self> {
        Ok(Self {
            target,
            overlay: 0,
            factory: None,
            dw_factory: None,
            render_target: None,
            stroke: None,
            brush: None,
            dw_text_format: None,
            render_fn: None,
            render_ctx: ctx,
        })
    }

    pub fn run_loop(&mut self, render_fn: RenderFn) -> Result<()> {
        let instance = unsafe { GetModuleHandleA(None) };
        let mut wc = WNDCLASSEXA {
            cbSize: std::mem::size_of::<WNDCLASSEXA>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instance,
            hIcon: 0usize as _,
            hCursor: unsafe { LoadCursorW(None, IDC_ARROW) },
            hbrBackground: unsafe { CreateSolidBrush(RGB(0, 0, 0)) },
            lpszMenuName: utf8_str!(""),
            lpszClassName: utf8_str!("win-overlay::overlay"),
            ..Default::default()
        };

        unsafe {
            DisableProcessWindowsGhosting();
        }

        // register it
        if unsafe { RegisterClassExA(&mut wc as *mut _) } == 0 {
            std::panic!("Unable to register window class!");
        }

        let rect = [0i8; std::mem::size_of::<RECT>()].as_mut_ptr() as *mut RECT;

        // get dimensions of target window
        unsafe { GetWindowRect(self.target, rect) };

        // our own style
        let styles = WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST;

        // create our own window
        let window = unsafe {
            CreateWindowExA(
                styles,
                utf8_str!("win-overlay::overlay"),
                utf8_str!(""),
                WS_POPUP | WS_VISIBLE,
                rect.read().left,
                rect.read().top,
                rect.read().right - rect.read().left,
                rect.read().bottom - rect.read().top,
                None,
                None,
                instance,
                self as *mut _ as _,
            )
        };

        // test if we actually created the window
        if window == 0 {
            panic!("Unable to create window");
        }
        self.overlay = window;

        // let's not do any stuff ourself
        let margins: *mut MARGINS =
            [0i8; std::mem::size_of::<MARGINS>()].as_mut_ptr() as *mut MARGINS;
        unsafe {
            (*margins).cxLeftWidth = rect.read().left;
            (*margins).cxRightWidth = rect.read().top;
            (*margins).cyTopHeight = rect.read().right - rect.read().left;
            (*margins).cyBottomHeight = rect.read().bottom - rect.read().top;
            DwmExtendFrameIntoClientArea(window as winapi::shared::windef::HWND, margins);

            // let is use alpha
            SetLayeredWindowAttributes(window, RGB(0, 0, 0), 255, ULW_COLORKEY | LWA_ALPHA);

            SetForegroundWindow(window);
            // show our window
            ShowWindow(window, SW_SHOW);
        }
        self.render_fn = Some(render_fn);

        let mut message = MSG::default();
        unsafe {
            loop {
                PeekMessageA(&mut message, window, 0, 0, 0);
                TranslateMessage(&mut message);
                if message.message == WM_QUIT {
                    return Ok(());
                }
                DispatchMessageA(&message);
                self.draw().unwrap();
            }
        }
    }

    fn message_handler(&mut self, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        unsafe {
            match message {
                WM_PAINT => {
                    let mut ps = PAINTSTRUCT::default();
                    BeginPaint(self.overlay, &mut ps);
                    self.draw().unwrap();
                    EndPaint(self.overlay, &ps);
                    0
                }
                // WM_SIZE => {
                //     if wparam != SIZE_MINIMIZED as usize {
                //         self.release_device();
                //     }
                //     0
                // }
                WM_DISPLAYCHANGE => {
                    self.draw().unwrap();
                    0
                }
                WM_DESTROY => {
                    PostQuitMessage(0);
                    0
                }
                _ => DefWindowProcA(self.overlay, message, wparam, lparam),
            }
        }
    }

    fn init_dx(&mut self) -> Result<()> {
        let window = self.overlay;

        // init dx
        let mut options = D2D1_FACTORY_OPTIONS::default();
        options.debugLevel = D2D1_DEBUG_LEVEL_INFORMATION;

        let mut d2d_factory_opt: Option<ID2D1Factory1> = None;
        unsafe {
            D2D1CreateFactory(
                D2D1_FACTORY_TYPE_SINGLE_THREADED,
                &ID2D1Factory1::IID,
                &options,
                std::mem::transmute(&mut d2d_factory_opt),
            )?
        }
        let d2d_factory = d2d_factory_opt.unwrap();
        let mut dpi_x: f32 = 0.0;
        let mut dpi_y: f32 = 0.0;
        unsafe { d2d_factory.GetDesktopDpi(&mut dpi_x, &mut dpi_y) };

        let rt_props = D2D1_RENDER_TARGET_PROPERTIES {
            r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: dpi_x,
            dpiY: dpi_y,
            usage: 0,
            minLevel: 0,
        };

        let rect = self.get_rect();
        let hw_rt_props = D2D1_HWND_RENDER_TARGET_PROPERTIES {
            hwnd: window,
            pixelSize: D2D_SIZE_U {
                width: (rect.right - rect.left) as u32,
                height: (rect.bottom - rect.top) as u32,
            },
            presentOptions: D2D1_PRESENT_OPTIONS_IMMEDIATELY,
        };
        let render_target: ID2D1HwndRenderTarget =
            unsafe { d2d_factory.CreateHwndRenderTarget(&rt_props, &hw_rt_props)? };
        unsafe {
            render_target.SetAntialiasMode(D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
            render_target.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_CLEARTYPE);
        }

        let dw_factory: IDWriteFactory1 = unsafe {
            DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED, &IDWriteFactory1::IID)
                .map(|f| std::mem::transmute(f))?
        };

        let text_format: IDWriteTextFormat = unsafe {
            let mut collections: Option<IDWriteFontCollection> = None;
            dw_factory.GetSystemFontCollection(std::mem::transmute(&mut collections), BOOL(1))?;
            dw_factory.CreateTextFormat(
                PWSTR("微软雅黑".as_ptr() as *mut u16),
                collections.unwrap(),
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                13 as f32,
                PWSTR(b"".as_ptr() as *mut u16),
            )?
        };

        let props = D2D1_STROKE_STYLE_PROPERTIES {
            startCap: D2D1_CAP_STYLE_ROUND,
            endCap: D2D1_CAP_STYLE_TRIANGLE,
            ..Default::default()
        };

        let stroke_style = unsafe { d2d_factory.CreateStrokeStyle(&props, std::ptr::null(), 0)? };

        let brush_color = D2D1_COLOR_F {
            r: 255.0,
            g: 255.0,
            b: 255.0,
            a: 1.0,
        };

        let brush_props = D2D1_BRUSH_PROPERTIES {
            opacity: 1.0,
            transform: Matrix3x2::identity(),
        };
        let brush = unsafe { render_target.CreateSolidColorBrush(&brush_color, &brush_props)? };

        self.factory = Some(d2d_factory);
        self.render_target = Some(render_target);
        self.dw_factory = Some(dw_factory);
        self.stroke = Some(stroke_style);
        self.brush = Some(brush);
        self.dw_text_format = Some(text_format);

        Ok(())
    }

    fn get_overlay(&self) -> HWND {
        self.overlay
    }

    fn get_target(&self) -> HWND {
        self.target
    }
}

impl Drop for Overlay {
    fn drop(&mut self) {
        if self.overlay != 0 {
            unsafe {
                DestroyWindow(self.overlay);
            }
        }
        self.release_device();
    }
}
