use winapi::um::dwmapi::DwmExtendFrameIntoClientArea;
//use winapi::um::dwmapi::DwmExtendFrameIntoClientArea;
use winapi::um::uxtheme::MARGINS;
use winapi::um::wingdi::RGB;
use windows::core::*;
use windows::Foundation::Numerics::Matrix3x2;
use windows::Win32::Foundation::{DXGI_STATUS_OCCLUDED, HINSTANCE};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, PSTR, RECT, WPARAM};
use windows::Win32::Graphics::Direct2D::Common::{D2D1_ALPHA_MODE_IGNORE, D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_ALPHA_MODE_STRAIGHT, D2D1_ALPHA_MODE_UNKNOWN, D2D1_COLOR_F, D2D1_PIXEL_FORMAT, D2D_POINT_2F, D2D_SIZE_U};
use windows::Win32::Graphics::Direct2D::{D2D1CreateFactory, ID2D1Device, ID2D1DeviceContext, ID2D1Factory1, ID2D1SolidColorBrush, ID2D1StrokeStyle, D2D1_BITMAP_OPTIONS_CANNOT_DRAW, D2D1_BITMAP_OPTIONS_TARGET, D2D1_BITMAP_PROPERTIES1, D2D1_BRUSH_PROPERTIES, D2D1_CAP_STYLE_ROUND, D2D1_CAP_STYLE_TRIANGLE, D2D1_DEBUG_LEVEL_INFORMATION, D2D1_DEVICE_CONTEXT_OPTIONS_NONE, D2D1_FACTORY_OPTIONS, D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_STROKE_STYLE_PROPERTIES, D2D1_UNIT_MODE_DIPS, D2D1_DIRECTIONALBLUR_OPTIMIZATION_QUALITY, D2D1_RENDER_TARGET_PROPERTIES, D2D1_HWND_RENDER_TARGET_PROPERTIES, D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_PRESENT_OPTIONS_IMMEDIATELY, ID2D1HwndRenderTarget, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE};
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
use windows::Win32::Graphics::Direct3D11::{D3D11CreateDevice, ID3D11Device, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION, D3D11_CREATE_DEVICE_DEBUG, D3D11_CREATE_DEVICE_SINGLETHREADED};
use windows::Win32::Graphics::DirectWrite::{
    DWriteCreateFactory, IDWriteFactory1, IDWriteTextFormat, DWRITE_FACTORY_TYPE_SHARED,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_ALPHA_MODE_STRAIGHT, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_UNKNOWN, DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::{
    IDXGIDevice, IDXGIFactory2, IDXGISurface, IDXGISwapChain1, DXGI_PRESENT_TEST,
    DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL, DXGI_USAGE_RENDER_TARGET_OUTPUT,
};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateSolidBrush, EndPaint, ValidateRect, PAINTSTRUCT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleA;
use windows::Win32::UI::WindowsAndMessaging::{CreateWindowExA, DefWindowProcA, DestroyWindow, DisableProcessWindowsGhosting, DispatchMessageA, GetMessageA, GetWindowLongPtrA, GetWindowRect, LoadCursorW, PeekMessageA, PostQuitMessage, RegisterClassExA, SetLayeredWindowAttributes, SetWindowLongPtrA, SetWindowPos, ShowWindow, CREATESTRUCTA, CS_HREDRAW, CS_VREDRAW, GWLP_USERDATA, HWND_TOPMOST, IDC_ARROW, LWA_ALPHA, MSG, PM_REMOVE, SIZE_MINIMIZED, SW_SHOW, ULW_COLORKEY, WM_ACTIVATE, WM_DESTROY, WM_DISPLAYCHANGE, WM_NCCREATE, WM_PAINT, WM_QUIT, WM_SIZE, WM_USER, WNDCLASSEXA, WS_EX_LAYERED, WS_EX_TRANSPARENT, WS_OVERLAPPEDWINDOW, WS_POPUP, WS_VISIBLE, WS_EX_TOPMOST};

#[macro_export]
macro_rules! native_str {
    ($str: expr) => {
        PSTR(format!("{}\0", $str).as_ptr() as _)
    };
}

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
    render_fn: Option<Box<&'static dyn FnMut(&mut Overlay)>>,
    dpi: f32,
    frequency: i64,
    occlusion: u32,
    visible: bool,
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

            let f = self.render_fn.as_ref().unwrap();

            // if let Some(f) = f_opt {
            //     f(self);
            // }
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
    pub fn new(target: HWND) -> Result<Self> {
        Ok(Self {
            target,
            overlay: 0,
            factory: None,
            dw_factory: None,
            render_target: None,
            stroke: None,
            brush: None,
            dw_text_format: None,
            dpi: 0.0,
            frequency: 0,
            occlusion: 0,
            visible: false,
            render_fn: None,
        })
    }

    pub fn run_loop<F: FnMut(&mut Overlay) + 'static>(&mut self, render_fn:&'static F) -> Result<()>  {
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
            lpszMenuName: native_str!(""),
            lpszClassName: native_str!("win-overlay::overlay"),
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
                native_str!("win-overlay::overlay"),
                native_str!(""),
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

            // show our window
            ShowWindow(window, SW_SHOW);
        }
        self.render_fn = Some(Box::new(render_fn));

        let mut message = MSG::default();
        unsafe {
            loop {
                GetMessageA(&mut message, None, 0, 0);
                if message.message == WM_QUIT {
                    return Ok(());
                }
                DispatchMessageA(&message);
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
                WM_SIZE => {
                    if wparam != SIZE_MINIMIZED as usize {
                        self.release_device();
                    }
                    0
                }
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
        unsafe {d2d_factory.GetDesktopDpi(&mut dpi_x, &mut dpi_y)};

        let rt_props = D2D1_RENDER_TARGET_PROPERTIES {
            r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
            pixelFormat: D2D1_PIXEL_FORMAT{
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED
            },
            dpiX: dpi_x,
            dpiY: dpi_y,
            usage: 0,
            minLevel: 0
        };

        let rect = self.get_rect();
        let hw_rt_props = D2D1_HWND_RENDER_TARGET_PROPERTIES {
            hwnd: window,
            pixelSize: D2D_SIZE_U {
                width: (rect.right - rect.left) as u32,
                height: (rect.bottom - rect.top)  as u32
            },
            presentOptions: D2D1_PRESENT_OPTIONS_IMMEDIATELY
        };
        let render_target: ID2D1HwndRenderTarget = unsafe {d2d_factory.CreateHwndRenderTarget(&rt_props,&hw_rt_props)?};
        unsafe {render_target.SetAntialiasMode(D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);}

        let dw_factory: IDWriteFactory1 = unsafe {
            DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED, &IDWriteFactory1::IID)
                .map(|factory| std::mem::transmute(factory))?
        };

        let props = D2D1_STROKE_STYLE_PROPERTIES {
            startCap: D2D1_CAP_STYLE_ROUND,
            endCap: D2D1_CAP_STYLE_TRIANGLE,
            ..Default::default()
        };

        let stroke_style = unsafe { d2d_factory.CreateStrokeStyle(&props, std::ptr::null(), 0)? };

        let brush_color = D2D1_COLOR_F {
            r: 0.92,
            g: 0.38,
            b: 0.208,
            a: 1.0,
        };

        let brush_props = D2D1_BRUSH_PROPERTIES {
            opacity: 0.8,
            transform: Matrix3x2::identity(),
        };
        let brush = unsafe { render_target.CreateSolidColorBrush(&brush_color, &brush_props)? };

        self.factory = Some(d2d_factory);
        self.render_target = Some(render_target);
        self.dw_factory = Some(dw_factory);
        self.stroke = Some(stroke_style);
        self.brush = Some(brush);

        Ok(())
    }

    pub fn get_overlay(&self) -> HWND {
        self.overlay
    }

    pub fn get_target(&self) -> HWND {
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
