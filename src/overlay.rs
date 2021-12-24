use winapi::um::dwmapi::DwmExtendFrameIntoClientArea;
//use winapi::um::dwmapi::DwmExtendFrameIntoClientArea;
use winapi::um::uxtheme::MARGINS;
use winapi::um::wingdi::RGB;
use windows::core::*;
use windows::Foundation::Numerics::Matrix3x2;
use windows::Win32::Foundation::{DXGI_STATUS_OCCLUDED, HINSTANCE};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, PSTR, RECT, WPARAM};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_IGNORE, D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_ALPHA_MODE_STRAIGHT,
    D2D1_ALPHA_MODE_UNKNOWN, D2D1_COLOR_F, D2D1_PIXEL_FORMAT, D2D_POINT_2F,
};
use windows::Win32::Graphics::Direct2D::{D2D1CreateFactory, ID2D1Device, ID2D1DeviceContext, ID2D1Factory1, ID2D1SolidColorBrush, ID2D1StrokeStyle, D2D1_BITMAP_OPTIONS_CANNOT_DRAW, D2D1_BITMAP_OPTIONS_TARGET, D2D1_BITMAP_PROPERTIES1, D2D1_BRUSH_PROPERTIES, D2D1_CAP_STYLE_ROUND, D2D1_CAP_STYLE_TRIANGLE, D2D1_DEBUG_LEVEL_INFORMATION, D2D1_DEVICE_CONTEXT_OPTIONS_NONE, D2D1_FACTORY_OPTIONS, D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_STROKE_STYLE_PROPERTIES, D2D1_UNIT_MODE_DIPS, D2D1_DIRECTIONALBLUR_OPTIMIZATION_QUALITY};
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
    dx_factory: Option<IDXGIFactory2>,
    dw_factory: Option<IDWriteFactory1>,
    dx_context: Option<ID2D1DeviceContext>,
    swap_chain: Option<IDXGISwapChain1>,

    // cached resource
    stroke: Option<ID2D1StrokeStyle>,
    brush: Option<ID2D1SolidColorBrush>,
    dw_text_format: Option<IDWriteTextFormat>,

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
        self.begin_drawing()?;

        //
        unsafe {
            let ctx = self.dx_context.as_ref().unwrap();
            ctx.Clear(&D2D1_COLOR_F {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            });
            ctx.DrawLine(
                D2D_POINT_2F { x: 0.0, y: 0.0 },
                D2D_POINT_2F { x: 50.0, y: 50.0 },
                self.brush.as_ref().unwrap(),
                2.0,
                self.stroke.as_ref().unwrap(),
            );
        };
        self.end_drawing()
    }

    /// Set up drawing
    fn begin_drawing(&mut self) -> Result<()> {
        self.ensure_position();
        if self.dx_context.is_none() {
            self.init_dx()?;
            self.create_swap_chain_bitmap()?;
        }
        unsafe {
            self.dx_context.as_ref().unwrap().BeginDraw();
        }
        Ok(())
    }

    /// Finish up drawing
    fn end_drawing(&mut self) -> Result<()> {
        unsafe {
            self.dx_context
                .as_ref()
                .unwrap()
                .EndDraw(std::ptr::null_mut(), std::ptr::null_mut())?;
        }
        if let Err(error) = self.present(1, 0) {
            if error.code() != DXGI_STATUS_OCCLUDED {
                self.release_device();
            }
        }
        Ok(())
    }

    fn present(&self, sync: u32, flags: u32) -> Result<()> {
        unsafe { self.swap_chain.as_ref().unwrap().Present(sync, flags) }
    }

    fn release_device(&mut self) {
        self.dx_context = None;
        self.swap_chain = None;
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
            dx_factory: None,
            dw_factory: None,
            dx_context: None,
            swap_chain: None,
            stroke: None,
            brush: None,
            dw_text_format: None,
            dpi: 0.0,
            frequency: 0,
            occlusion: 0,
            visible: false,
        })
    }

    pub fn run_loop(&mut self) -> Result<()> {
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
                        // TODO resize swap chain bitmap
                    }
                    0
                }
                WM_USER => {
                    if self.present(0, DXGI_PRESENT_TEST).is_ok() {
                        self.dx_factory
                            .as_ref()
                            .unwrap()
                            .UnregisterOcclusionStatus(self.occlusion);
                        self.occlusion = 0;
                        self.visible = true;
                    }
                    0
                }
                WM_ACTIVATE => {
                    self.visible = true;
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
        let mut flags = D3D11_CREATE_DEVICE_BGRA_SUPPORT | D3D11_CREATE_DEVICE_DEBUG;
        let mut device_opt: Option<ID3D11Device> = None;
        let mut device: ID3D11Device;
        let context: ID2D1DeviceContext;
        let dxgi_factory: IDXGIFactory2;
        let swap_chain: IDXGISwapChain1;
        unsafe {
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HINSTANCE::default(),
                flags,
                std::ptr::null(),
                0,
                D3D11_SDK_VERSION,
                &mut device_opt,
                std::ptr::null_mut(),
                &mut None,
            )?;
            device = device_opt.unwrap();

            let d2device: ID2D1Device = d2d_factory.CreateDevice(device.cast::<IDXGIDevice>()?)?;
            context = d2device.CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)?;
            context.SetUnitMode(D2D1_UNIT_MODE_DIPS);


            let dxgi_device: IDXGIDevice = device.cast::<IDXGIDevice>()?;
            dxgi_factory = dxgi_device.GetAdapter()?.GetParent()?;

            let props = DXGI_SWAP_CHAIN_DESC1 {
                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                BufferCount: 2,
                SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
                ..Default::default()
            };

            swap_chain = dxgi_factory.CreateSwapChainForHwnd(
                &device,
                window as windows::Win32::Foundation::HWND,
                &props,
                std::ptr::null(),
                None,
            )?;
        }

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
        let brush = unsafe { context.CreateSolidColorBrush(&brush_color, &brush_props)? };

        self.factory = Some(d2d_factory);
        self.dx_factory = Some(dxgi_factory);
        self.dx_context = Some(context);
        self.dw_factory = Some(dw_factory);
        self.swap_chain = Some(swap_chain);
        self.stroke = Some(stroke_style);
        self.brush = Some(brush);

        Ok(())
    }

    fn create_swap_chain_bitmap(&self) -> Result<()> {
        let surface: IDXGISurface = unsafe { self.swap_chain.as_ref().unwrap().GetBuffer(0)? };

        let props = D2D1_BITMAP_PROPERTIES1 {
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: 96.0,
            dpiY: 96.0,
            bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
            colorContext: None,
        };

        unsafe {
            let bitmap = self
                .dx_context
                .as_ref()
                .unwrap()
                .CreateBitmapFromDxgiSurface(&surface, &props)?;
            self.dx_context.as_ref().unwrap().SetTarget(bitmap);
        };

        Ok(())
    }

    fn resize_swap_chain_bitmap(&mut self) -> Result<()> {
        if let Some(ctx) = &self.dx_context {
            let swap_chain = self.swap_chain.as_ref().unwrap();
            unsafe { ctx.SetTarget(None) };

            if unsafe {
                swap_chain
                    .ResizeBuffers(0, 0, 0, DXGI_FORMAT_UNKNOWN, 0)
                    .is_ok()
            } {
                self.create_swap_chain_bitmap()?;
            } else {
                self.release_device();
            }
            self.draw()?;
        }
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
