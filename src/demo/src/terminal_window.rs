use std::sync::{Arc,Mutex};
use std::io::Read;
use cgmath::{Vector2,Vector4,ElementWise};
use tile_renderer::{
    Renderer,
    GlyphCache,
    FontdueGlyphGenerator,
    CellData,
};
use terminal::terminal::Terminal;
use winit::{
    event::{Event, WindowEvent, ElementState},
    keyboard::{KeyCode,PhysicalKey,Key},
    event_loop::EventLoopWindowTarget,
    window::Window,
};
use crate::{
    frame_counter::FrameCounter,
    terminal_target::TerminalTarget,
    terminal_reader::TerminalReader,
};

pub struct TerminalWindow<'a, T> {
    terminal: Arc<Mutex<Terminal>>, 
    terminal_target: &'a mut T,
    terminal_reader: TerminalReader,
    glyph_grid: Vec<CellData>,
    glyph_cache: GlyphCache,
    winit_window: &'a Window,
    wgpu_config: wgpu::SurfaceConfiguration,
    wgpu_surface: wgpu::Surface<'a>,
    wgpu_device: wgpu::Device,
    wgpu_queue: wgpu::Queue,
    renderer: Renderer,
    current_frame: usize,
    frame_counter: FrameCounter,
}

impl<'a, T: TerminalTarget> TerminalWindow<'a, T> {
    pub async fn new(
        winit_window: &'a Window,
        terminal: Arc<Mutex<Terminal>>, 
        terminal_target: &'a mut T,
        font_filename: String, font_size: f32,
    ) -> anyhow::Result<Self> 
    {
        let mut font_file = std::fs::File::open(font_filename.clone())?;
        let mut font_data = Vec::<u8>::new();
        let total_bytes_read = font_file.read_to_end(&mut font_data)?;
        let font_data = &font_data[..total_bytes_read];
        let font = fontdue::Font::from_bytes(font_data, fontdue::FontSettings::default())
            .map_err(anyhow::Error::msg)?;
 
        let initial_grid_size = Vector2::new(1,1);
        let glyph_generator = Box::new(FontdueGlyphGenerator::new(font, font_size));
        let glyph_cache = GlyphCache::new(glyph_generator);
        let mut terminal_reader = TerminalReader::default();
        terminal_target.set_size(initial_grid_size)?;
        terminal_reader.set_size(initial_grid_size);

        // wgpu
        let wgpu_instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::util::backend_bits_from_env().unwrap_or_default(),
            flags: wgpu::InstanceFlags::from_build_config().with_env(),
            dx12_shader_compiler: wgpu::util::dx12_shader_compiler_from_env().unwrap_or_default(),
            gles_minor_version: wgpu::util::gles_minor_version_from_env().unwrap_or_default(),
        });
        let wgpu_surface = wgpu_instance.create_surface(winit_window).unwrap();
        let wgpu_adapter = wgpu_instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&wgpu_surface),
            })
            .await
            .ok_or("Failed to find valid wgpu_adapter")
            .map_err(anyhow::Error::msg)?;
        // wgpu setup
        let (wgpu_device, wgpu_queue) = wgpu_adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults().using_resolution(wgpu_adapter.limits()),
            }, None)
            .await?;
        // render pipeline
        let initial_window_size = winit_window.inner_size();
        let mut wgpu_config = wgpu_surface
            .get_default_config(&wgpu_adapter, initial_window_size.width, initial_window_size.height)
            .ok_or("Failed to get default config for wgpu surface")
            .map_err(anyhow::Error::msg)?;
        wgpu_config.present_mode = wgpu::PresentMode::AutoVsync;
        wgpu_surface.configure(&wgpu_device, &wgpu_config);
        let renderer = Renderer::new(&wgpu_config, &wgpu_device);

        Ok(Self {
            terminal,
            terminal_target,
            terminal_reader,
            glyph_grid: Vec::new(),
            glyph_cache,
            winit_window,
            wgpu_config,
            wgpu_surface,
            wgpu_device,
            wgpu_queue,
            renderer,
            current_frame: 0,
            frame_counter: FrameCounter::default(),
        })
    }

    pub fn on_winit_event(
        &mut self, event: Event<()>, target: &EventLoopWindowTarget<()>) {
        if let Event::WindowEvent { event, .. } = event {
            match event {
                WindowEvent::CloseRequested => target.exit(),
                WindowEvent::MouseWheel { delta, .. } => self.on_mouse_wheel(delta),
                WindowEvent::KeyboardInput { event, .. } => self.on_keyboard_input(event),
                WindowEvent::Resized(new_size) => {
                    let new_size = Vector2::new(new_size.width as usize, new_size.height as usize);
                    self.on_resize(new_size);
                },
                WindowEvent::RedrawRequested => {
                    self.on_redraw_requested();
                    self.winit_window.request_redraw();
                },
                _ => {},
            }
        }
    }

    fn on_mouse_wheel(&mut self, delta: winit::event::MouseScrollDelta) {
        use winit::event::MouseScrollDelta as Delta;
        match delta {
            Delta::LineDelta(_x, y) => {
                if y > 0.0 {
                    self.terminal_reader.scroll_up(1);
                } else {
                    self.terminal_reader.scroll_down(1);
                }
            },
            Delta::PixelDelta(delta) => {
                if delta.y > 0.0 {
                    self.terminal_reader.scroll_up(1);
                } else {
                    self.terminal_reader.scroll_down(1);
                }
            },
        }
    }

    pub fn on_resize(&mut self, new_size: Vector2<usize>) {
        let new_size = Vector2::new(new_size.x.max(1), new_size.y.max(1));
        self.wgpu_config.width = new_size.x as u32;
        self.wgpu_config.height = new_size.y as u32;
        self.wgpu_surface.configure(&self.wgpu_device, &self.wgpu_config);
        // calculate new terminal grid size
        let glyph_size = self.glyph_cache.get_glyph_atlas().get_glyph_size();
        let new_grid_size = new_size.div_element_wise(glyph_size);
        let new_grid_size = Vector2::new(new_grid_size.x.max(1), new_grid_size.y.max(1));
        let actual_render_size = new_grid_size.mul_element_wise(glyph_size);
        let new_render_scale = actual_render_size.cast::<f32>().unwrap().div_element_wise(new_size.cast::<f32>().unwrap());
        // update gpu
        self.renderer.update_render_scale(&self.wgpu_queue, new_render_scale);
        {
            // forcefully update terminal grid size
            let terminal = &mut self.terminal.lock().expect("Acquire terminal for size change");
            let viewport = terminal.get_viewport_mut();
            viewport.set_size(new_grid_size);
        }
        let _ = self.terminal_target.set_size(new_grid_size);
        self.winit_window.request_redraw();
    }

    fn on_redraw_requested(&mut self) {
        self.update_grid_from_terminal();
        let frame = self.wgpu_surface.get_current_texture().expect("Failed to acquire next swap chain texture");
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.wgpu_device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("render_text_commands"),
        });
        self.renderer.generate_commands(&mut encoder, &view, &self.wgpu_device);
        self.wgpu_queue.submit(Some(encoder.finish()));
        frame.present();
    }

    fn update_grid_from_terminal(&mut self) {
        self.current_frame += 1;
        self.frame_counter.update();
        if let Ok(ref terminal) = self.terminal.try_lock() {
            self.terminal_reader.read_terminal(terminal);
        };

        // update glyph data if possible
        let size = self.terminal_reader.get_size();
        let cells = self.terminal_reader.get_cells();
        let glyph_atlas = self.glyph_cache.get_glyph_atlas();
        let total_glyphs_in_block = glyph_atlas.get_total_glyphs_in_block();
        self.glyph_grid.resize(size.x*size.y, CellData::default());
        for (dst, src) in self.glyph_grid.iter_mut().zip(cells.iter()) {
            let atlas_index = self.glyph_cache.get_glyph_location(src.character, self.current_frame);
            let atlas_index = Vector2::new(
                atlas_index.block.x*total_glyphs_in_block.x + atlas_index.position.x,
                atlas_index.block.y*total_glyphs_in_block.y + atlas_index.position.y,
            );
            dst.atlas_index = atlas_index.cast::<u16>().unwrap();
            dst.colour_foreground = Vector4::new(
                src.foreground_colour.r,
                src.foreground_colour.g,
                src.foreground_colour.b,
                255,
            );
            dst.colour_background = Vector4::new(
                src.background_colour.r,
                src.background_colour.g,
                src.background_colour.b,
                255,
            );
            dst.style_flags = 0u32;
        }
        self.renderer.update_grid(&self.wgpu_device, &self.wgpu_queue, self.glyph_grid.as_slice(), size);
        let glyph_atlas = self.glyph_cache.get_glyph_atlas_mut();
        self.renderer.update_atlas(&self.wgpu_device, &self.wgpu_queue, glyph_atlas);
    }

    fn on_keyboard_input(&mut self, event: winit::event::KeyEvent) {
        if event.state != ElementState::Pressed {
            return;
        }

        if let PhysicalKey::Code(code) = event.physical_key {
            if let Some(data) = convert_keycode_to_bytes(code) {
                let _ = self.terminal_target.write_data(data);
                self.terminal_reader.scroll_to_bottom();
                return;
            }
        }

        if let Key::Character(string) = event.logical_key {
            let _ = self.terminal_target.write_data(string.as_bytes());
            self.terminal_reader.scroll_to_bottom();
            return;
        }

        if let PhysicalKey::Code(code) = event.physical_key {
            let size = self.terminal_reader.get_size();
            let mut has_code = true;
            match code {
                KeyCode::End => self.terminal_reader.scroll_to_bottom(),
                KeyCode::Home => self.terminal_reader.scroll_to_top(),
                KeyCode::PageDown => self.terminal_reader.scroll_down(size.y),
                KeyCode::PageUp => self.terminal_reader.scroll_up(size.y),
                _ => {
                    has_code = false; 
                },
            };
            if has_code {
                return;
            }
        }
    }
}

fn convert_keycode_to_bytes(code: KeyCode) -> Option<&'static [u8]> {
    match code {
        KeyCode::Enter => Some(b"\x0D\n"),
        KeyCode::Tab => Some(b"\x09"),
        KeyCode::Backspace => Some(b"\x08"),
        KeyCode::Space => Some(b" "),
        KeyCode::ArrowUp => Some(b"\x1b[A"),
        KeyCode::ArrowDown => Some(b"\x1b[B"),
        KeyCode::ArrowRight => Some(b"\x1b[C"),
        KeyCode::ArrowLeft => Some(b"\x1b[D"),
        _ => None,
    }
}
