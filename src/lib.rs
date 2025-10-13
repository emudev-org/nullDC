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

use winit::window::Window;
use winit::window::WindowAttributes;
use winit::{
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
};

use git_version::git_version;
use wgpu::util::DeviceExt;

pub use dreamcast;
use dreamcast::{Dreamcast, run_slice_dreamcast};

const GIT_HASH: &str = git_version!();

struct State {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    egui_renderer: egui_wgpu::Renderer,
    egui_state: egui_winit::State,
    egui_ctx: egui::Context,
    // UI state
    clear_color: [f32; 3],
    show_triangle: bool,
    framebuffer: egui::TextureHandle,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [0.0, 0.5, 0.0],
        tex_coords: [0.5, 0.0],
    },
    Vertex {
        position: [-0.5, -0.5, 0.0],
        tex_coords: [0.0, 1.0],
    },
    Vertex {
        position: [0.5, -0.5, 0.0],
        tex_coords: [1.0, 1.0],
    },
];

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
            .find(|f| f.is_srgb())
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

        // Checker texture
        let texture_size = 256u32;
        let texture_data: Vec<u8> = (0..texture_size * texture_size)
            .flat_map(|i| {
                let x = i % texture_size;
                let y = i / texture_size;
                let checker = ((x / 32) + (y / 32)) % 2 == 0;
                if checker {
                    [255, 100, 100, 255]
                } else {
                    [100, 100, 255, 255]
                }
            })
            .collect();

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: texture_size,
                height: texture_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("texture"),
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &texture_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * texture_size),
                rows_per_image: Some(texture_size),
            },
            wgpu::Extent3d {
                width: texture_size,
                height: texture_size,
                depth_or_array_layers: 1,
            },
        );

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
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
            label: Some("texture_bind_group_layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("texture_bind_group"),
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
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

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let egui_ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &*window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );

        let egui_renderer = egui_wgpu::Renderer::new(&device, config.format, None, 1, false);

        let framebuffer: egui::TextureHandle = egui_ctx.load_texture(
            "framebuffer",
            egui::ColorImage::new([640, 480], vec![egui::Color32::BLACK; 640 * 480]),
            egui::TextureOptions::NEAREST,
        );

        Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            vertex_buffer,
            bind_group,
            egui_renderer,
            egui_state,
            egui_ctx,
            clear_color: [0.1, 0.2, 0.3],
            show_triangle: true,
            framebuffer,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Begin egui frame
        let raw_input = self.egui_state.take_egui_input(&*self.window);
        let egui_output = self.egui_ctx.run(raw_input, |ctx| {
            egui::Window::new("Framebuffer").show(ctx, |ui| {
                ui.image((self.framebuffer.id(), egui::vec2(640.0, 480.0)));
            });
        });

        // Upload egui textures and meshes
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("main encoder"),
            });

        let screen_desc = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            // FIX 1: Use egui Context for ppp
            pixels_per_point: self.egui_ctx.pixels_per_point(),
        };

        for (id, image_delta) in &egui_output.textures_delta.set {
            self.egui_renderer
                .update_texture(&self.device, &self.queue, *id, image_delta);
        }

        let paint_jobs = self
            .egui_ctx
            .tessellate(egui_output.shapes, self.egui_ctx.pixels_per_point());
        self.egui_renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            &paint_jobs,
            &screen_desc,
        );

        // 1) Clear + draw triangle (if enabled)
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("triangle pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    // FIX 2: New field in wgpu
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: self.clear_color[0] as f64,
                            g: self.clear_color[1] as f64,
                            b: self.clear_color[2] as f64,
                            a: 1.0,
                        }),
                        // FIX 3: StoreOp, not bool
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            if self.show_triangle {
                rpass.set_pipeline(&self.render_pipeline);
                rpass.set_bind_group(0, &self.bind_group, &[]);
                rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                rpass.draw(0..3, 0..1);
            }
        }

        // 2) Draw egui on top (separate pass, load existing color)
        {
            let rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            let mut rpass = rpass.forget_lifetime();

            // FIX 4: render into a RenderPass, not encoder+view
            self.egui_renderer
                .render(&mut rpass, &paint_jobs, &screen_desc);
        }

        // Submit
        self.queue.submit(Some(encoder.finish()));
        output.present();

        // Cleanup egui textures
        for id in &egui_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        //     log::info!("Render");
        //     let frame = self.surface.get_current_texture().unwrap();
        // let view = frame.texture.create_view(&Default::default());

        // let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        //     label: Some("Render Encoder"),
        // });

        // {
        //     let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        //         label: Some("Clear Pass"),
        //         color_attachments: &[Some(wgpu::RenderPassColorAttachment {
        //             view: &view,
        //             resolve_target: None,
        //             depth_slice: None,
        //             ops: wgpu::Operations {
        //                 load: wgpu::LoadOp::Clear(wgpu::Color::RED), // force bright red
        //                 store: wgpu::StoreOp::Store,
        //             },
        //         })],
        //         depth_stencil_attachment: None,
        //         timestamp_writes: None,
        //         occlusion_query_set: None,
        //     });
        // }

        // self.queue.submit(Some(encoder.finish()));
        // frame.present();

        Ok(())
    }
}

#[derive(Default)]
struct App {
    state: Option<State>,
    dreamcast: *mut Dreamcast,
}

// Import ApplicationHandler trait from winit
use std::cell::RefCell;
use std::rc::Rc;
use winit::application::ApplicationHandler;
use winit::window::WindowId;

pub fn rgb565_to_color32(buf: &[u16], w: usize, h: usize) -> egui::ColorImage {
    let mut pixels = Vec::with_capacity(w * h);
    for &px in buf {
        let r = ((px >> 11) & 0x1F) as u8;
        let g = ((px >> 5) & 0x3F) as u8;
        let b = (px & 0x1F) as u8;
        // Expand to 8-bit
        let r = (r << 3) | (r >> 2);
        let g = (g << 2) | (g >> 4);
        let b = (b << 3) | (b >> 2);
        pixels.push(egui::Color32::from_rgb(r, g, b));
    }
    egui::ColorImage {
        size: [w, h],
        pixels,
        source_size: egui::vec2(w as f32, h as f32),
    }
}

struct AppHandle(Rc<RefCell<App>>);

impl AppHandle {
    fn new(dreamcast: *mut Dreamcast) -> Self {
        AppHandle(Rc::new(RefCell::new(App {
            state: None,
            dreamcast: dreamcast,
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

        #[cfg(not(target_arch = "wasm32"))]
        {
            let state = pollster::block_on(State::new(window));
            self.0.borrow_mut().state = Some(state);
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

        if let Some(state) = app.state.as_mut() {
            if !state
                .egui_state
                .on_window_event(&state.window, &event)
                .consumed
            {
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
                        let mut image: Option<egui::ColorImage> = None;

                        if !app.dreamcast.is_null() {
                            let dreamcast = app.dreamcast;
                            unsafe {
                                run_slice_dreamcast(dreamcast);

                                let base_u8: *const u8 =
                                    (*dreamcast).video_ram.as_ptr().add(0x0000); // add offset if needed

                                let base_u16 = base_u8.cast::<u16>();
                                let len_u16 = 640 * 480;
                                let buf: &[u16] = core::slice::from_raw_parts(base_u16, len_u16);

                                image = Some(rgb565_to_color32(buf, 640, 480));
                            }
                        }

                        if let Some(state) = app.state.as_mut() {
                            // Update framebuffer texture
                            if let Some(image) = image {
                                state.framebuffer.set(image, egui::TextureOptions::NEAREST);
                            }
                            // Render
                            match state.render() {
                                Ok(()) => {}
                                Err(wgpu::SurfaceError::Lost)
                                | Err(wgpu::SurfaceError::Outdated) => {
                                    state.resize(state.size);
                                }
                                Err(wgpu::SurfaceError::OutOfMemory) => {
                                    // event_loop.exit();
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
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let app = self.0.borrow_mut();
        if let Some(state) = app.state.as_ref() {
            state.window.request_redraw();
        }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run(dreamcast: *mut Dreamcast) {
    // Setup logging and panic hooks for wasm
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
        wasm_logger::init(wasm_logger::Config::default());
    }

    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let mut app = AppHandle::new(dreamcast);
    event_loop.run_app(&mut app).unwrap();
}
