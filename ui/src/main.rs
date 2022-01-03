use crate::glutin::dpi::LogicalPosition;
use crate::MemoryItemType::*;
use core::process::Process;
use glium::glutin;
use glium::glutin::dpi::Position;
use glium::glutin::event::{Event, WindowEvent};
use glium::glutin::event_loop::{ControlFlow, EventLoop};
use glium::glutin::platform::windows::WindowBuilderExtWindows;
use glium::glutin::window::{Fullscreen, Theme, WindowBuilder};
use glium::{Display, Surface};
use imgui::StyleColor::Button;
use imgui::{
    Condition, Context, FontConfig, FontGlyphRanges, FontSource, InputTextFlags, ItemHoveredFlags,
    MenuItem, MouseButton, PopupModal, Selectable, TreeNode, Ui, Window,
};
use imgui_glium_renderer::Renderer;
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use std::f64;
use std::fmt::format;
use std::ops::Deref;
use std::path::Path;
use std::time::Instant;

fn main() {
    let event_loop = EventLoop::new();
    let context = glutin::ContextBuilder::new().with_vsync(true);
    let builder = WindowBuilder::new()
        .with_transparent(true)
        .with_decorations(false)
        .with_resizable(true)
        .with_drag_and_drop(false)
        .with_always_on_top(false)
        .with_position(Position::Logical(LogicalPosition { x: 0.0, y: 0.0 }))
        .with_inner_size(glutin::dpi::LogicalSize::new(2560f64 / 3.0, 1440f64));
    let display =
        Display::new(builder, context, &event_loop).expect("Failed to initialize display");
    let mut imgui = Context::create();
    imgui.set_ini_filename(None);

    let mut platform = WinitPlatform::init(&mut imgui);
    {
        let gl_window = display.gl_window();
        let window = gl_window.window();

        let dpi_mode = if let Ok(factor) = std::env::var("IMGUI_EXAMPLE_FORCE_DPI_FACTOR") {
            // Allow forcing of HiDPI factor for debugging purposes
            match factor.parse::<f64>() {
                Ok(f) => HiDpiMode::Locked(f),
                Err(e) => panic!("Invalid scaling factor: {}", e),
            }
        } else {
            HiDpiMode::Default
        };

        platform.attach_window(imgui.io_mut(), window, dpi_mode);
    }

    // Fixed font size. Note imgui_winit_support uses "logical
    // pixels", which are physical pixels scaled by the devices
    // scaling factor. Meaning, 13.0 pixels should look the same size
    // on two different screens, and thus we do not need to scale this
    // value (as the scaling is handled by winit)
    let hidpi_factor = platform.hidpi_factor();
    let font_size = (13.0 * hidpi_factor) as f32;

    imgui.fonts().add_font(&[
        FontSource::TtfData {
            data: include_bytes!("../resources/Roboto-Regular.ttf"),
            size_pixels: font_size,
            config: Some(FontConfig {
                // As imgui-glium-renderer isn't gamma-correct with
                // it's font rendering, we apply an arbitrary
                // multiplier to make the font a bit "heavier". With
                // default imgui-glow-renderer this is unnecessary.
                rasterizer_multiply: 1.5,
                // Oversampling font helps improve text rendering at
                // expense of larger font atlas texture.
                oversample_h: 4,
                oversample_v: 4,
                ..FontConfig::default()
            }),
        },
        FontSource::TtfData {
            data: include_bytes!("../resources/mplus-1p-regular.ttf"),
            size_pixels: font_size,
            config: Some(FontConfig {
                // Oversampling font helps improve text rendering at
                // expense of larger font atlas texture.
                oversample_h: 4,
                oversample_v: 4,
                // Range of glyphs to rasterize
                glyph_ranges: FontGlyphRanges::japanese(),
                ..FontConfig::default()
            }),
        },
    ]);
    imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;
    let mut renderer = Renderer::init(&mut imgui, &display).expect("Failed to initialize renderer");
    let mut last_frame = Instant::now();
    let mut state = UIState::default();
    state.d3d_test_window_state = D3DTestWindowState {
        process_name: "DarkSoulsIII.exe".to_string(),
        process: Process::from_name("DarkSoulsIII.exe"),
        memory_tree: Some(MemoryTreeState {
            address: 0x7FF4AD045A58,
            item_type: MemoryItemType::Class,
            children: Box::new(vec![]),
        }),
    };

    state.run = true;
    event_loop.run(move |event, _, control_flow| match event {
        Event::NewEvents(_) => {
            let now = Instant::now();
            imgui.io_mut().update_delta_time(now - last_frame);
            last_frame = now;
        }
        Event::MainEventsCleared => {
            let gl_window = display.gl_window();
            platform
                .prepare_frame(imgui.io_mut(), gl_window.window())
                .expect("Failed to prepare frame");
            gl_window.window().request_redraw();
        }
        Event::RedrawRequested(_) => {
            // std::thread::sleep(std::time::Duration::from_millis(1000 / 60));

            let mut ui = imgui.frame();
            run_ui(&mut state, &mut ui);
            if !state.run {
                *control_flow = ControlFlow::Exit;
            }
            let gl_window = display.gl_window();
            let mut target = display.draw();
            target.clear_color_srgb(0.0, 0.0, 0.0, 0.0);
            platform.prepare_render(&ui, gl_window.window());
            let draw_data = ui.render();
            renderer
                .render(&mut target, draw_data)
                .expect("Rendering failed");
            target.finish().expect("Failed to swap buffers");
        }
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => *control_flow = ControlFlow::Exit,
        event => {
            let gl_window = display.gl_window();
            platform.handle_event(imgui.io_mut(), gl_window.window(), &event);
        }
    })
}

#[derive(Debug, Default)]
struct UIState {
    run: bool,
    bar_state: MainBarState,
    d3d_test_window_state: D3DTestWindowState,
    rtti_window_state: RTTISearchWindowState,
}

fn run_ui(state: &mut UIState, ui: &mut Ui) {
    main_bar(&mut state.bar_state, ui);
    d3d_test_window(
        state.bar_state.d3d_test,
        &mut state.d3d_test_window_state,
        ui,
    );
    state.rtti_window_state.process_name = state.d3d_test_window_state.process_name.clone();
    state.rtti_window_state.process =
        Process::from_name(state.rtti_window_state.process_name.as_str());
    rtti_search_window(
        state.bar_state.rtti_viewer,
        &mut state.rtti_window_state,
        ui,
    );
    ui.show_demo_window(&mut state.run);
}

#[derive(Debug, Clone, Default)]
struct MainBarState {
    d3d_test: bool,
    memory_viewer: bool,
    rtti_viewer: bool,
}

fn main_bar(state: &mut MainBarState, ui: &mut Ui) {
    if let Some(bar) = ui.begin_main_menu_bar() {
        if let Some(d3d_menu) = ui.begin_menu("D3D Tools") {
            MenuItem::new("D3D Test").build_with_ref(ui, &mut state.d3d_test);

            d3d_menu.end();
        }

        if let Some(d3d_menu) = ui.begin_menu("Cheat Tools") {
            MenuItem::new("Memory Viewer").build_with_ref(ui, &mut state.memory_viewer);
            MenuItem::new("RTTI Viewer").build_with_ref(ui, &mut state.rtti_viewer);
            d3d_menu.end();
        }
        bar.end();
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(packed)]
pub struct Vector2 {
    pub a1: f32,
    pub a2: f32,
}

#[derive(Debug, Clone, Copy)]
#[repr(packed)]
pub struct Vector3 {
    pub a1: f32,
    pub a2: f32,
    pub a3: f32,
}

#[derive(Debug, Clone, Copy)]
#[repr(packed)]
pub struct Vector4 {
    pub a1: f32,
    pub a2: f32,
    pub a3: f32,
    pub a4: f32,
}

#[derive(Debug, Clone, Copy)]
#[repr(packed)]
pub struct Vector3x3 {
    pub a11: f32,
    pub a12: f32,
    pub a13: f32,
    pub a21: f32,
    pub a22: f32,
    pub a23: f32,
    pub a31: f32,
    pub a32: f32,
    pub a33: f32,
}

#[derive(Debug, Clone, Copy)]
#[repr(packed)]
pub struct Vector3x4 {
    pub a11: f32,
    pub a12: f32,
    pub a13: f32,
    pub a14: f32,
    pub a21: f32,
    pub a22: f32,
    pub a23: f32,
    pub a24: f32,
    pub a31: f32,
    pub a32: f32,
    pub a33: f32,
    pub a34: f32,
}

#[derive(Debug, Clone, Copy)]
#[repr(packed)]
pub struct Vector4x4 {
    pub a11: f32,
    pub a12: f32,
    pub a13: f32,
    pub a14: f32,
    pub a21: f32,
    pub a22: f32,
    pub a23: f32,
    pub a24: f32,
    pub a31: f32,
    pub a32: f32,
    pub a33: f32,
    pub a34: f32,
    pub a41: f32,
    pub a42: f32,
    pub a43: f32,
    pub a44: f32,
}

#[derive(Debug, Default)]
struct D3DTestWindowState {
    pub process_name: String,
    pub process: Option<Process>,
    pub memory_tree: Option<MemoryTreeState>,
}

#[derive(PartialEq, Debug, Clone)]
enum MemoryItemType {
    Hex64,
    Hex32,
    Hex16,
    Hex8,
    Int64,
    Int32,
    Int16,
    Int8,
    UInt64,
    UInt32,
    UInt16,
    UInt8,
    Bool,
    Float,
    Double,
    Pointer,
    Class,
    Utf8Text,
    Utf8TextPtr,
    Utf16Text,
    Utf16TextPtr,
    Vector1x2,
    Vector1x3,
    Vector1x4,
    Vector3x3,
    Vector3x4,
    Vector4x4,
}

impl Default for MemoryItemType {
    fn default() -> MemoryItemType {
        MemoryItemType::Hex64
    }
}

#[derive(Debug, Default)]
struct MemoryTreeState {
    address: usize,
    item_type: MemoryItemType,
    children: Box<Vec<MemoryTreeState>>,
}

impl MemoryTreeState {
    pub fn popup_menu_id(&self) -> String {
        format!("popup.memory.{:p}", self)
    }
}

fn memory_tree_node<'a>(
    ps: &mut Process,
    offset: Option<usize>,
    state: &'a mut MemoryTreeState,
    ui: &Ui,
) -> usize {
    let unknown: String = "xxxx".to_owned();
    let mut value_size: usize = 0;
    let mut value_bytes: Vec<u8> = usize::default().to_le_bytes().to_vec();
    let value = match state.item_type {
        MemoryItemType::Hex64 => {
            value_size = std::mem::size_of::<u64>();
            if let Ok(value) = ps.read::<u64>(state.address) {
                value_bytes = value.to_le_bytes().to_vec();
                format!("0x{:08X}", value)
            } else {
                unknown
            }
        }
        MemoryItemType::Hex32 => {
            value_size = std::mem::size_of::<u32>();
            if let Ok(value) = ps.read::<u32>(state.address) {
                value_bytes = value.to_le_bytes().to_vec();
                format!("0x{:04X}", value)
            } else {
                unknown
            }
        }
        MemoryItemType::Hex16 => {
            value_size = std::mem::size_of::<u16>();
            if let Ok(value) = ps.read::<u16>(state.address) {
                value_bytes = value.to_le_bytes().to_vec();
                format!("0x{:02X}", value)
            } else {
                unknown
            }
        }
        MemoryItemType::Hex8 => {
            value_size = std::mem::size_of::<u8>();
            if let Ok(value) = ps.read::<u8>(state.address) {
                value_bytes = value.to_le_bytes().to_vec();
                format!("0x{:01X}", value)
            } else {
                unknown
            }
        }

        MemoryItemType::UInt64 => {
            value_size = std::mem::size_of::<u64>();
            if let Ok(value) = ps.read::<u64>(state.address) {
                value_bytes = value.to_le_bytes().to_vec();
                format!("{}", value)
            } else {
                unknown
            }
        }
        MemoryItemType::UInt32 => {
            value_size = std::mem::size_of::<u32>();
            if let Ok(value) = ps.read::<u32>(state.address) {
                value_bytes = value.to_le_bytes().to_vec();
                format!("{}", value)
            } else {
                unknown
            }
        }
        MemoryItemType::UInt16 => {
            value_size = std::mem::size_of::<u16>();
            if let Ok(value) = ps.read::<u16>(state.address) {
                value_bytes = value.to_le_bytes().to_vec();
                format!("{}", value)
            } else {
                unknown
            }
        }
        MemoryItemType::UInt8 => {
            value_size = std::mem::size_of::<u8>();
            if let Ok(value) = ps.read::<u8>(state.address) {
                value_bytes = value.to_le_bytes().to_vec();
                format!("{}", value)
            } else {
                unknown
            }
        }

        MemoryItemType::Int64 => {
            value_size = std::mem::size_of::<i64>();
            if let Ok(value) = ps.read::<i64>(state.address) {
                value_bytes = value.to_le_bytes().to_vec();
                format!("{}", value)
            } else {
                unknown
            }
        }
        MemoryItemType::Int32 => {
            value_size = std::mem::size_of::<i32>();
            if let Ok(value) = ps.read::<i32>(state.address) {
                value_bytes = value.to_le_bytes().to_vec();
                format!("{}", value)
            } else {
                unknown
            }
        }
        MemoryItemType::Int16 => {
            value_size = std::mem::size_of::<i16>();
            if let Ok(value) = ps.read::<i16>(state.address) {
                value_bytes = value.to_le_bytes().to_vec();
                format!("{}", value)
            } else {
                unknown
            }
        }
        MemoryItemType::Int8 => {
            value_size = std::mem::size_of::<i8>();
            if let Ok(value) = ps.read::<i8>(state.address) {
                value_bytes = value.to_le_bytes().to_vec();
                format!("{}", value)
            } else {
                unknown
            }
        }

        Bool => {
            value_size = std::mem::size_of::<bool>();
            if let Ok(value) = ps.read::<bool>(state.address) {
                value_bytes = Vec::new();
                value_bytes.push(value as u8);
                format!("{}", value)
            } else {
                unknown
            }
        }
        Float => {
            value_size = std::mem::size_of::<f32>();
            if let Ok(value) = ps.read::<f32>(state.address) {
                value_bytes = value.to_le_bytes().to_vec();
                format!("{:.4}", value)
            } else {
                unknown
            }
        }
        Double => {
            value_size = std::mem::size_of::<f64>();
            if let Ok(value) = ps.read::<f64>(state.address) {
                value_bytes = value.to_le_bytes().to_vec();
                format!("{:.4}", value)
            } else {
                unknown
            }
        }
        Pointer => {
            value_size = std::mem::size_of::<usize>();
            if let Ok(value) = ps.read::<usize>(state.address) {
                value_bytes = value.to_le_bytes().to_vec();
                if std::mem::size_of::<usize>() == 32 {
                    format!("{:08X}", value)
                } else {
                    format!("{:016X}", value)
                }
            } else {
                unknown
            }
        }
        Class => {
            value_size = std::mem::size_of::<usize>();
            if let Ok(value) = ps.read::<usize>(state.address) {
                value_bytes = value.to_le_bytes().to_vec();
            }
            if std::mem::size_of::<usize>() == 32 {
                format!("{:08X}", state.address)
            } else {
                format!("{:016X}", state.address)
            }
        }

        Vector1x2 => {
            value_size = std::mem::size_of::<Vector2>();
            let mut bytes: [u8; std::mem::size_of::<Vector2>()] =
                [0u8; std::mem::size_of::<Vector2>()];
            if let Ok(value) = ps.read::<Vector2>(state.address) {
                bytes = unsafe { std::mem::transmute_copy(&value) };
                value_bytes = bytes.to_vec();
                format!("[{:.4} , {:.4}]", value.a1, value.a2)
            } else {
                unknown
            }
        }

        Vector1x3 => {
            value_size = std::mem::size_of::<Vector3>();
            let mut bytes: [u8; std::mem::size_of::<Vector3>()] =
                [0u8; std::mem::size_of::<Vector3>()];
            if let Ok(value) = ps.read::<Vector3>(state.address) {
                bytes = unsafe { std::mem::transmute_copy(&value) };
                value_bytes = bytes.to_vec();
                format!("[{:.4} , {:.4}, {:.4}]", value.a1, value.a2, value.a3)
            } else {
                unknown
            }
        }

        Vector1x4 => {
            value_size = std::mem::size_of::<Vector4>();
            let mut bytes: [u8; std::mem::size_of::<Vector4>()] =
                [0u8; std::mem::size_of::<Vector4>()];
            if let Ok(value) = ps.read::<Vector4>(state.address) {
                bytes = unsafe { std::mem::transmute_copy(&value) };
                value_bytes = bytes.to_vec();
                format!(
                    "[{:.4} , {:.4}, {:.4}, {:.4}]",
                    value.a1, value.a2, value.a3, value.a4
                )
            } else {
                unknown
            }
        }
        _ => unknown,
    };
    assert_eq!(value_size, value_bytes.len());
    let int_string;
    let float_string;
    match value_size {
        1 => {
            let mut bytes: [u8; 1] = [0u8; 1];
            bytes.copy_from_slice(value_bytes.as_slice());
            int_string = i8::from_le_bytes(bytes).to_string();
            float_string = "0.0000".to_owned();
        }

        2 => {
            let mut bytes: [u8; 2] = [0u8; 2];
            bytes.copy_from_slice(value_bytes.as_slice());
            int_string = i16::from_le_bytes(bytes).to_string();
            float_string = "0.0000".to_owned();
        }

        4 => {
            let mut bytes: [u8; 4] = [0u8; 4];
            bytes.copy_from_slice(value_bytes.as_slice());
            int_string = i32::from_le_bytes(bytes).to_string();
            float_string = format!("{:.4}", f32::from_le_bytes(bytes));
        }

        8 => {
            let mut bytes: [u8; 8] = [0u8; 8];
            bytes.copy_from_slice(value_bytes.as_slice());
            int_string = i64::from_le_bytes(bytes).to_string();
            float_string = format!("{:.4}", f64::from_le_bytes(bytes) as f32);
        }
        _ => {
            int_string = "0".to_owned();
            float_string = "0.0000".to_owned();
        }
    }

    let address;
    if std::mem::size_of::<usize>() == 32 {
        address = format!("{:08X}", state.address);
    } else {
        address = format!("{:016X}", state.address);
    }
    if MemoryItemType::Class == state.item_type {
        if TreeNode::new(address)
            .default_open(true)
            .build(ui, || {
                ui.same_line();
                if ui.small_button("+") {
                    ui.open_popup(state.popup_menu_id());
                }
                memory_tree_item_menu(ps, state, ui);
                let mut index: usize = 0;
                for child in state.children.as_mut() {
                    index += memory_tree_node(ps, Some(index), child, ui);
                }
            })
            .is_some()
        {
            if ui.is_item_clicked_with_button(MouseButton::Right) {
                ui.open_popup(state.popup_menu_id());
            }
        } else {
            if ui.is_item_clicked_with_button(MouseButton::Right) {
                ui.open_popup(state.popup_menu_id());
            }
        }

        memory_tree_item_menu(ps, state, ui);
    } else {
        if let Some(off) = offset {
            ui.text_colored(
                [245.0 / 255.0, 40.0 / 255.0, 145.0 / 255.0, 0.8],
                format!("{:04X}", off),
            );
            ui.same_line();
        }
        ui.same_line();
        ui.text_colored([42.0 / 202.0, 40.0 / 74.0, 145.0 / 255.0, 1.0], address);
        let mut hex_string = String::with_capacity(value_bytes.len());
        for b in value_bytes.iter() {
            hex_string.push_str(format!("{:02X}", b).as_str());
            hex_string.push(' ');
        }

        let mut char_string = String::with_capacity(value_bytes.len());
        for b in value_bytes {
            let char = char::from(b);
            if !char.is_ascii_graphic() {
                char_string.push('.');
            } else {
                char_string.push(char);
            }
            char_string.push(' ');
        }

        ui.same_line();
        ui.text_colored([42.0 / 202.0, 40.0 / 74.0, 145.0 / 255.0, 1.0], hex_string);
        ui.same_line();
        ui.text_colored([42.0 / 202.0, 40.0 / 74.0, 145.0 / 255.0, 1.0], char_string);
        match state.item_type {
            Vector1x2 | Vector1x3 | Vector1x4 | Vector3x3 | Vector3x4 | Vector4x4 => {
                ui.text_colored([42.0 / 202.0, 40.0 / 74.0, 145.0 / 255.0, 1.0], value);
            }
            _ => {
                ui.same_line();
                ui.text_colored(
                    [42.0 / 202.0, 40.0 / 74.0, 145.0 / 255.0, 1.0],
                    float_string,
                );
                ui.same_line();
                ui.text_colored([42.0 / 202.0, 40.0 / 74.0, 145.0 / 255.0, 1.0], int_string);
                ui.same_line();
                ui.text_colored([42.0 / 202.0, 40.0 / 74.0, 145.0 / 255.0, 1.0], value);
            }
        }

        if ui.is_item_clicked_with_button(MouseButton::Right) {
            ui.open_popup(state.popup_menu_id());
        }
        memory_tree_item_menu(ps, state, ui);
    }

    value_size
}

fn memory_tree_item_menu(ps: &Process, state: &mut MemoryTreeState, ui: &Ui) {
    ui.popup(state.popup_menu_id(), || {
        if let Some(menu) = ui.begin_menu("Change Type") {
            for el in [
                Hex64,
                Hex32,
                Hex16,
                Hex8,
                Int64,
                Int32,
                Int16,
                Int8,
                UInt64,
                UInt32,
                UInt16,
                UInt8,
                Bool,
                Float,
                Double,
                Pointer,
                Class,
                Utf8Text,
                Utf8TextPtr,
                Utf16Text,
                Utf16TextPtr,
                Vector1x2,
                Vector1x3,
                Vector1x4,
                Vector3x3,
                Vector3x4,
                Vector4x4,
            ] {
                if state.item_type != el {
                    if MenuItem::new(format!("{:?}", el)).build(ui) {
                        if el == Pointer {
                            if let Ok(ptr) = ps.read::<usize>(state.address) {
                                state.address = ptr;
                                state.children.clear();
                            }
                        }
                        state.item_type = el;
                        if state.item_type == Class {
                            state.children.clear();
                        }
                        println!("{:?}", state);
                    }
                }
            }
            menu.end();
        }
        if Class == state.item_type || Pointer == state.item_type {
            if let Some(menu) = ui.begin_menu("Add Bytes") {
                let ptr_size = std::mem::size_of::<usize>();
                for i in 1..7 {
                    let label = format!(
                        "{}ptr {}bytes",
                        ptr_size.pow(i - 1 as u32),
                        ptr_size.pow(i as u32)
                    );
                    if MenuItem::new(label).build(ui) {
                        for _ in 0..ptr_size.pow(i - 1 as u32) {
                            let index = state.children.len();
                            state.children.push(MemoryTreeState {
                                address: state.address + index * ptr_size,
                                item_type: MemoryItemType::Hex64,
                                children: Box::new(vec![]),
                            });
                        }
                    }
                }
                menu.end();
            }
        }
    });
}

fn d3d_test_window(show: bool, state: &mut D3DTestWindowState, ui: &mut Ui) {
    if show {
        let mut process_name_change = false;
        Window::new("D3D Test")
            .size([512.0, 400.0], Condition::Appearing)
            .build(ui, || {
                // Render
                let label;
                if let Some(ps) = &state.process {
                    label = format!("Process: {}", ps.id);
                } else {
                    label = "Process: Not Select".to_owned();
                }
                process_name_change = ui
                    .input_text(label, &mut state.process_name)
                    .enter_returns_true(true)
                    .build();

                // Logic
                if process_name_change && !state.process_name.is_empty() {
                    state.process = Process::from_name(state.process_name.as_str());
                }

                if let Some(ps) = &mut state.process {
                    memory_tree_node(ps, None, state.memory_tree.as_mut().unwrap(), ui);
                }
            });
    }
}

#[derive(Debug, Default)]
struct RTTISearchWindowState {
    pub process_name: String,
    pub process: Option<Process>,
}

fn rtti_search_window(show: bool, state: &mut RTTISearchWindowState, ui: &mut Ui) {
    if show {
        let mut process_name_change = false;
        Window::new("RTTI Search")
            .size([512.0, 400.0], Condition::Appearing)
            .build(ui, || {
                // Render
                let label;
                if let Some(ps) = &state.process {
                    label = format!("Process: {}", ps.id);
                } else {
                    label = "Process: Not Select".to_owned();
                }
                process_name_change = ui
                    .input_text(label, &mut state.process_name)
                    .enter_returns_true(true)
                    .build();

                // Logic
                if process_name_change && !state.process_name.is_empty() {
                    state.process = Process::from_name(state.process_name.as_str());
                }

                if let Some(ps) = &mut state.process {}
            });
    }
}
