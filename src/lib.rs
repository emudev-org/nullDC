use std::sync::Arc;

#[cfg(target_arch = "wasm32")]
use console_error_panic_hook;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures;
#[cfg(target_arch = "wasm32")]
use wasm_logger;
#[cfg(target_arch = "wasm32")]
use wgpu::web_sys;

#[cfg(any(target_os = "windows", target_os = "macos"))]
use muda::{Menu, MenuItem};

use winit::window::Window;
use winit::window::WindowAttributes;
use winit::{
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
};

use git_version::git_version;

pub use dreamcast;
use dreamcast::{Dreamcast, present_for_texture, run_slice_dreamcast};

mod debugger_core;

#[cfg(target_arch = "wasm32")]
mod debugger_html5_broadcast_server;
#[cfg(target_arch = "wasm32")]
use debugger_html5_broadcast_server::BroadcastDebugServer;

#[cfg(not(target_arch = "wasm32"))]
mod debugger_websocket_server;
#[cfg(not(target_arch = "wasm32"))]
pub use debugger_websocket_server::start_debugger_server;

const GIT_HASH: &str = git_version!();

struct State {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    uniform_bind_group_layout: wgpu::BindGroupLayout,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    framebuffer_texture: wgpu::Texture,
    framebuffer_bind_group: wgpu::BindGroup,
    framebuffer_width: u32,
    framebuffer_height: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    window_width: f32,
    window_height: f32,
    fb_width: f32,
    fb_height: f32,
}

// Framebuffer dimensions - default Dreamcast resolution
const DEFAULT_FB_WIDTH: u32 = 640;
const DEFAULT_FB_HEIGHT: u32 = 480;

// In your initialization code (probably in main or new)
#[cfg(target_arch = "wasm32")]
fn get_canvas_size() -> winit::dpi::PhysicalSize<u32> {
    use wasm_bindgen::JsCast;

    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let canvas = document
        .get_element_by_id("egui_canvas")
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .unwrap();

    winit::dpi::PhysicalSize::new(canvas.width(), canvas.height())
}

impl State {
    async fn new(window: Arc<Window>) -> Self {
        #[cfg(target_arch = "wasm32")]
        let size = get_canvas_size();

        #[cfg(not(target_arch = "wasm32"))]
        let size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("request_adapter");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::default(),
            })
            .await
            .expect("request_device");

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let present_mode = surface_caps
            .present_modes
            .iter()
            .copied()
            .find(|m| matches!(m, wgpu::PresentMode::AutoVsync))
            .unwrap_or(surface_caps.present_modes[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        log::info!("Surface size: {}x{}", config.width, config.height);
        surface.configure(&device, &config);

        // Create framebuffer texture
        let framebuffer_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: DEFAULT_FB_WIDTH,
                height: DEFAULT_FB_HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("framebuffer_texture"),
            view_formats: &[],
        });

        let framebuffer_view = framebuffer_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("framebuffer_bind_group_layout"),
        });

        let framebuffer_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&framebuffer_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("framebuffer_bind_group"),
        });

        // Create uniform buffer for window/framebuffer dimensions
        let uniforms = Uniforms {
            window_width: size.width as f32,
            window_height: size.height as f32,
            fb_width: DEFAULT_FB_WIDTH as f32,
            fb_height: DEFAULT_FB_HEIGHT as f32,
        };

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Uniform Buffer"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        queue.write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        let uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("uniform_bind_group_layout"),
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("uniform_bind_group"),
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout, &uniform_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            bind_group_layout,
            uniform_bind_group_layout,
            uniform_buffer,
            uniform_bind_group,
            framebuffer_texture,
            framebuffer_bind_group,
            framebuffer_width: DEFAULT_FB_WIDTH,
            framebuffer_height: DEFAULT_FB_HEIGHT,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            // Update uniforms with new window size
            let uniforms = Uniforms {
                window_width: new_size.width as f32,
                window_height: new_size.height as f32,
                fb_width: self.framebuffer_width as f32,
                fb_height: self.framebuffer_height as f32,
            };
            self.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Single render pass - draw fullscreen quad with framebuffer texture
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Framebuffer Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            rpass.set_pipeline(&self.render_pipeline);
            rpass.set_bind_group(0, &self.framebuffer_bind_group, &[]);
            rpass.set_bind_group(1, &self.uniform_bind_group, &[]);
            rpass.draw(0..6, 0..1); // Draw 6 vertices for fullscreen quad (2 triangles)
        }

        self.queue.submit(Some(encoder.finish()));
        output.present();

        Ok(())
    }

    fn update_framebuffer(&mut self, rgba_data: &[u8], width: usize, height: usize) {
        // If framebuffer size changed, recreate texture and bind group
        if width as u32 != self.framebuffer_width || height as u32 != self.framebuffer_height {
            self.framebuffer_width = width as u32;
            self.framebuffer_height = height as u32;

            self.framebuffer_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                size: wgpu::Extent3d {
                    width: self.framebuffer_width,
                    height: self.framebuffer_height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                label: Some("framebuffer_texture"),
                view_formats: &[],
            });

            let framebuffer_view = self.framebuffer_texture.create_view(&wgpu::TextureViewDescriptor::default());
            let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                ..Default::default()
            });

            self.framebuffer_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&framebuffer_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
                label: Some("framebuffer_bind_group"),
            });

            // Update uniforms with new framebuffer size
            let uniforms = Uniforms {
                window_width: self.config.width as f32,
                window_height: self.config.height as f32,
                fb_width: self.framebuffer_width as f32,
                fb_height: self.framebuffer_height as f32,
            };
            self.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
        }

        // Write framebuffer data to texture
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.framebuffer_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * self.framebuffer_width),
                rows_per_image: Some(self.framebuffer_height),
            },
            wgpu::Extent3d {
                width: self.framebuffer_width,
                height: self.framebuffer_height,
                depth_or_array_layers: 1,
            },
        );
    }
}

struct App {
    state: Option<State>,
    dreamcast: *mut Dreamcast,
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    menu: Option<Menu>,
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    devtools_item: Option<MenuItem>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            state: None,
            dreamcast: std::ptr::null_mut(),
            #[cfg(any(target_os = "windows", target_os = "macos"))]
            menu: None,
            #[cfg(any(target_os = "windows", target_os = "macos"))]
            devtools_item: None,
        }
    }
}

// Import ApplicationHandler trait from winit
use std::cell::RefCell;
use std::rc::Rc;
use winit::application::ApplicationHandler;
use winit::window::WindowId;

struct AppHandle(Rc<RefCell<App>>);

impl AppHandle {
    fn new(dreamcast: *mut Dreamcast) -> Self {
        AppHandle(Rc::new(RefCell::new(App {
            state: None,
            dreamcast: dreamcast,
            #[cfg(any(target_os = "windows", target_os = "macos"))]
            menu: None,
            #[cfg(any(target_os = "windows", target_os = "macos"))]
            devtools_item: None,
        })))
    }
}

impl ApplicationHandler for AppHandle {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes: WindowAttributes = {
            let mut attrs = Window::default_attributes().with_title(format!("nullDC {}", GIT_HASH));

            #[cfg(target_arch = "wasm32")]
            {
                use wasm_bindgen::JsCast;
                use winit::platform::web::WindowAttributesExtWebSys;

                let document = web_sys::window().unwrap().document().unwrap();
                let canvas = document
                    .get_element_by_id("egui_canvas")
                    .unwrap()
                    .dyn_into::<web_sys::HtmlCanvasElement>()
                    .unwrap();

                attrs = attrs.with_canvas(Some(canvas))
            }

            #[cfg(not(target_arch = "wasm32"))]
            {
                attrs = attrs.with_inner_size(winit::dpi::Size::Physical(
                    winit::dpi::PhysicalSize::new(1024, 1024),
                ));
            }

            attrs
        };

        #[cfg(target_arch = "wasm32")]
        {
            use web_sys::window;
            window()
                .unwrap()
                .document()
                .unwrap()
                .set_title(format!("nullDC {}", GIT_HASH).as_str());
        }

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        #[cfg(any(target_os = "windows", target_os = "macos"))]
        {
            // Create menu
            let menu = Menu::new();
            let devtools_item = MenuItem::new("DevTools", true, None);

            menu.append(&devtools_item).unwrap();

            // Initialize menu for the window
            #[cfg(target_os = "windows")]
            {
                use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
                if let Ok(handle) = window.window_handle() {
                    if let RawWindowHandle::Win32(win32_handle) = handle.as_ref() {
                        unsafe {
                            menu.init_for_hwnd(win32_handle.hwnd.get() as _).unwrap();
                        }
                    }
                }
            }

            let state = pollster::block_on(State::new(window.clone()));
            let mut app = self.0.borrow_mut();
            app.state = Some(state);
            app.menu = Some(menu);
            app.devtools_item = Some(devtools_item);
        }

        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "windows"), not(target_os = "macos")))]
        {
            let state = pollster::block_on(State::new(window));
            let mut app = self.0.borrow_mut();
            app.state = Some(state);
        }

        #[cfg(target_arch = "wasm32")]
        {
            use std::cell::RefCell;
            use std::rc::Rc;
            use std::rc::Weak;
            use wasm_bindgen_futures::spawn_local;

            let this: Weak<RefCell<App>> = Rc::downgrade(&self.0);
            let window_clone = window.clone();

            spawn_local(async move {
                if let Some(app_rc) = this.upgrade() {
                    let state = State::new(window_clone.clone()).await;
                    app_rc.borrow_mut().state = Some(state);
                    window_clone.request_redraw();
                }
            });
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let mut app = self.0.borrow_mut();

        if let Some(_state) = app.state.as_mut() {
            match event {
                WindowEvent::CloseRequested
                | WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            state: ElementState::Pressed,
                            physical_key: PhysicalKey::Code(KeyCode::Escape),
                            ..
                        },
                    ..
                } => {
                    event_loop.exit();
                }

                WindowEvent::Resized(size) => {
                    if let Some(state) = app.state.as_mut() {
                        state.resize(size);
                    }
                }

                WindowEvent::RedrawRequested => {
                    // Run emulator slice
                    if !app.dreamcast.is_null() {
                        let dreamcast = app.dreamcast;
                        run_slice_dreamcast(dreamcast);
                    }

                    // Get framebuffer from emulator and update texture
                    if let Some((rgba, width, height)) = present_for_texture() {
                        let pixel_count = width.saturating_mul(height);
                        if pixel_count > 0 && rgba.len() >= pixel_count * 4 {
                            if let Some(state) = app.state.as_mut() {
                                state.update_framebuffer(&rgba, width, height);
                            }
                        }
                    }

                    // Render
                    if let Some(state) = app.state.as_mut() {
                        match state.render() {
                            Ok(()) => {}
                            Err(wgpu::SurfaceError::Lost) | Err(wgpu::SurfaceError::Outdated) => {
                                state.resize(state.size);
                            }
                            Err(wgpu::SurfaceError::OutOfMemory) => {
                                event_loop.exit();
                            }
                            Err(wgpu::SurfaceError::Timeout) => {
                                // skip frame
                            }
                            Err(_) => {}
                        }
                    }
                }

                _ => {}
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let app = self.0.borrow_mut();

        // Handle menu events (Windows and macOS only)
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        {
            if let Some(ref devtools_item) = app.devtools_item {
                if let Ok(event) = muda::MenuEvent::receiver().try_recv() {
                    if event.id == devtools_item.id() {
                        // Open DevTools URL in default browser
                        if let Err(e) = open::that("http://127.0.0.1:55543") {
                            log::error!("Failed to open DevTools URL: {}", e);
                        } else {
                            log::info!("Opened DevTools in default browser");
                        }
                    }
                }
            }
        }

        if let Some(state) = app.state.as_ref() {
            state.window.request_redraw();
        }
    }
}

// WASM initialization - sets up logging only, does not start emulation
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn wasm_init() {
    // Just set up panic hook and logging
    // The actual emulation starts when wasm_main_with_bios() is called from JavaScript
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());
    log::info!("nullDC WASM initialized. Waiting for BIOS files...");
}

// WASM entry point that accepts BIOS data from JavaScript
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn wasm_main_with_bios(bios_rom: Vec<u8>, bios_flash: Vec<u8>) {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());

    log::info!("Received BIOS ROM: {} bytes, BIOS Flash: {} bytes", bios_rom.len(), bios_flash.len());

    // Validate sizes
    if bios_rom.len() != 2 * 1024 * 1024 {
        log::error!("Invalid BIOS ROM size: {} (expected 2MB)", bios_rom.len());
        return;
    }
    if bios_flash.len() != 128 * 1024 {
        log::error!("Invalid BIOS Flash size: {} (expected 128KB)", bios_flash.len());
        return;
    }

    // Create and initialize Dreamcast with provided BIOS
    let dc = Box::into_raw(Box::new(Dreamcast::default()));
    dreamcast::init_dreamcast(dc, &bios_rom, &bios_flash);
    let debug_server = BroadcastDebugServer::new(dc);

    // Start broadcast debug server for WASM
    match debug_server {
        Ok(mut server) => {
            if let Err(e) = server.start() {
                log::error!("Failed to start broadcast debug server: {:?}", e);
            } else {
                log::info!("Broadcast debug server started successfully");
                // Keep the server alive by forgetting it (leak it intentionally)
                std::mem::forget(server);
            }
        }
        Err(e) => {
            log::error!("Failed to create broadcast debug server: {:?}", e);
        }
    }

    run(Some(dc)).await;
}

// WASM entry point to boot with an ELF file (no BIOS required)
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn wasm_main_with_elf(elf_data: Vec<u8>) {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());

    log::info!("Received BIOS ROM: {} bytes, BIOS Flash: {} bytes, ELF: {} bytes",
               bios_rom.len(), bios_flash.len(), elf_data.len());

    // Create and initialize Dreamcast with provided BIOS
    let dc = Box::into_raw(Box::new(Dreamcast::default()));

    // Load ELF
    log::info!("Loading ELF file: {} bytes", elf_data.len());
    match dreamcast::init_dreamcast_with_elf(dc, &elf_data) {
        Ok(()) => {
            log::info!("ELF file loaded successfully");
        }
        Err(e) => {
            log::error!("Failed to load ELF: {}", e);
            return;
        }
    }

    let debug_server = BroadcastDebugServer::new(dc);

    // Start broadcast debug server for WASM
    match debug_server {
        Ok(mut server) => {
            if let Err(e) = server.start() {
                log::error!("Failed to start broadcast debug server: {:?}", e);
            } else {
                log::info!("Broadcast debug server started successfully");
                // Keep the server alive by forgetting it (leak it intentionally)
                std::mem::forget(server);
            }
        }
        Err(e) => {
            log::error!("Failed to create broadcast debug server: {:?}", e);
        }
    }

    run(Some(dc)).await;
}

// Main run function that works for both native and WASM
pub async fn run(dreamcast: Option<*mut Dreamcast>) {
    // Setup logging and panic hooks for wasm
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
        wasm_logger::init(wasm_logger::Config::default());
    }

    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init();

    // For native builds with no Dreamcast provided, we can't continue
    let dreamcast_ptr = match dreamcast {
        Some(ptr) => ptr,
        None => {
            log::error!("No Dreamcast instance provided and no BIOS files available");
            return;
        }
    };

    let event_loop = EventLoop::new().unwrap();
    let mut app = AppHandle::new(dreamcast_ptr);
    event_loop.run_app(&mut app).unwrap();
}
