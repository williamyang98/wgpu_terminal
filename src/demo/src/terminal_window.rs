use std::sync::{Arc,Mutex};
use std::io::Read;
use cgmath::{Vector2,Vector4,ElementWise};
use glyph_grid::{
    glyph_cache::GlyphCache,
    glyph_grid::GlyphGrid,
};
use glyph_grid_renderer::renderer::Renderer;
use terminal::{
    terminal::Terminal,
    primitives::Cell,
};
use winit::{
    event::{Event, WindowEvent, MouseScrollDelta, ElementState},
    keyboard::{KeyCode,PhysicalKey,Key},
    event_loop::EventLoopWindowTarget,
    window::Window,
};
use crate::{
    frame_counter::FrameCounter,
    terminal_target::TerminalTarget,
};

pub struct TerminalWindow<'a, T> {
    terminal: Arc<Mutex<Terminal>>, 
    terminal_target: &'a mut T,
    terminal_cells: Vec<Cell>,
    glyph_grid: GlyphGrid,
    glyph_cache: GlyphCache,
    winit_window: &'a Window,
    wgpu_config: wgpu::SurfaceConfiguration,
    wgpu_surface: wgpu::Surface<'a>,
    wgpu_device: wgpu::Device,
    wgpu_queue: wgpu::Queue,
    renderer: Renderer,
    current_frame: usize,
    frame_counter: FrameCounter,
    is_scrollback_buffer: bool,
    scrollback_buffer_line: usize,
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
        let glyph_grid = GlyphGrid::new(initial_grid_size);
        let glyph_cache = GlyphCache::new(font, font_size);
        terminal_target.set_size(initial_grid_size)?;

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
                required_features: 
                    wgpu::Features::TEXTURE_BINDING_ARRAY | 
                    wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
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
            terminal_cells: Vec::new(),
            glyph_grid,
            glyph_cache,
            winit_window,
            wgpu_config,
            wgpu_surface,
            wgpu_device,
            wgpu_queue,
            renderer,
            current_frame: 0,
            frame_counter: FrameCounter::default(),
            is_scrollback_buffer: false,
            scrollback_buffer_line: 0,
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
        if !self.is_scrollback_buffer {
            return;
        }
        use winit::event::MouseScrollDelta as Delta;
        match delta {
            Delta::LineDelta(x, y) => {
                if y > 0.0 {
                    self.scrollback_buffer_line = self.scrollback_buffer_line.max(1) - 1;
                } else {
                    self.scrollback_buffer_line += 1;
                }
            },
            Delta::PixelDelta(delta) => {
                if delta.y > 0.0 {
                    self.scrollback_buffer_line = self.scrollback_buffer_line.max(1) - 1;
                } else {
                    self.scrollback_buffer_line += 1;
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
        let glyph_size = self.glyph_cache.get_glyph_size();
        let new_grid_size = new_size.div_element_wise(glyph_size);
        let new_grid_size = Vector2::new(new_grid_size.x.max(1), new_grid_size.y.max(1));
        let actual_render_size = new_grid_size.mul_element_wise(glyph_size);
        let new_render_scale = actual_render_size.cast::<f32>().unwrap().div_element_wise(new_size.cast::<f32>().unwrap());
        // update gpu
        let params = self.renderer.get_render_params();
        params.set_render_scale(&self.wgpu_queue, new_render_scale);
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
        // self.frame_counter.update();
        let mut terminal_size = None;
        if let Ok(terminal) = self.terminal.try_lock() {
            let viewport = terminal.get_viewport();
            let size = viewport.get_size();
            let total_cells = size.x*size.y;
            self.terminal_cells.resize(total_cells, Cell::default());
            self.terminal_cells.fill(Cell::default());

            if !self.is_scrollback_buffer {
                for y in 0..size.y {
                    let (src_row, status) = viewport.get_row(y);
                    let dst_index = y*size.x;
                    let dst_row = &mut self.terminal_cells[dst_index..(dst_index+size.x)];
                    assert!(status.length <= size.x);
                    for x in 0..status.length {
                        dst_row[x] = src_row[x];
                    }
                    for x in status.length..size.x {
                        dst_row[x] = Cell::default();
                    }
                }
            } else {
                let scrollback_buffer = viewport.get_scrollback_buffer();
                let lines = scrollback_buffer.get_lines();
                self.scrollback_buffer_line = self.scrollback_buffer_line.min(lines.len());
                let lines = &lines[self.scrollback_buffer_line..];
                let mut cursor: Vector2<usize> = Vector2::new(0,0);
                for line in lines {
                    let row = scrollback_buffer.get_row(line);
                    for cell in row {
                        let dst_index = cursor.y*size.x + cursor.x;
                        self.terminal_cells[dst_index] = *cell;
                        cursor.x += 1;
                        if cursor.x >= size.x {
                            cursor.x = 0;
                            cursor.y += 1;
                        }
                        if cursor.y >= size.y {
                            break;
                        }
                    }
                    if cursor.y >= size.y {
                        break;
                    }
                    cursor.x = 0;
                    cursor.y += 1;
                    if cursor.y >= size.y {
                        break;
                    }
                }
            }
            terminal_size = Some(size);
        };
        if let Some(terminal_size) = terminal_size { 
            // update glyph data if possible
            self.glyph_grid.resize(terminal_size);
            let dst_grid = self.glyph_grid.get_mut_view();
            for (dst, src) in dst_grid.data.iter_mut().zip(self.terminal_cells.iter()) {
                let location =  self.glyph_cache.get_glyph_location(src.character, self.current_frame);
                dst.set_page_index(location.page_index);
                dst.set_glyph_position(location.glyph_position);
                dst.set_foreground_colour(Vector4::new(
                    src.foreground_colour.r,
                    src.foreground_colour.g,
                    src.foreground_colour.b,
                    255,
                ));
                dst.set_background_colour(Vector4::new(
                    src.background_colour.r,
                    src.background_colour.g,
                    src.background_colour.b,
                    255,
                ));
            }
        }
        {
            // update glyph grid
            let cpu_grid = self.glyph_grid.get_view();
            let gpu_grid = self.renderer.get_glyph_grid();
            gpu_grid.update_grid(&self.wgpu_queue, &self.wgpu_device, cpu_grid);
        }
        {
            // update glyph atlases page sizes
            let gpu_pages = self.renderer.get_glyph_atlas();
            let cpu_pages = self.glyph_cache.get_mut_pages();
            let page_sizes: Vec<Vector2<usize>> = cpu_pages
                .iter()
                .map(|page| page.get_atlas().get_texture_size())
                .collect();
            gpu_pages.create_pages_if_changed(&self.wgpu_device, page_sizes.as_slice());
            // upload glyph atlas data to gpu if dirtied
            for (i, page) in cpu_pages.iter_mut().enumerate() {
                let total_changes = page.get_total_changes();
                if total_changes > 0 {
                    page.clear_total_changes();
                    let atlas = page.get_atlas();
                    let grid_size = atlas.get_grid_size();
                    let texture_view = atlas.get_texture_view();
                    gpu_pages.update_page(&self.wgpu_queue, i, texture_view, grid_size);
                }
            }
        }
    }

    fn on_keyboard_input(&mut self, event: winit::event::KeyEvent) {
        if event.state == ElementState::Pressed {
            if let PhysicalKey::Code(KeyCode::F1) = event.physical_key {
                self.is_scrollback_buffer = !self.is_scrollback_buffer;
                return;
            }
            if !self.is_scrollback_buffer {
                if let PhysicalKey::Code(code) = event.physical_key {
                    if let Some(data) = convert_keycode_to_bytes(code) {
                        let _ = self.terminal_target.write_data(data);
                        return;
                    }
                }
                if let Key::Character(string) = event.logical_key {
                    let _ = self.terminal_target.write_data(string.as_bytes());
                    return;
                }
            } else {
                if let PhysicalKey::Code(code) = event.physical_key {
                    const LARGE_JUMP: usize = 4096;
                    const SMALL_JUMP: usize = 16;
                    match code {
                        KeyCode::End => {
                            self.scrollback_buffer_line += LARGE_JUMP;
                        },
                        KeyCode::Home => {
                            self.scrollback_buffer_line = self.scrollback_buffer_line.max(LARGE_JUMP) - LARGE_JUMP;
                        },
                        KeyCode::PageDown => {
                            self.scrollback_buffer_line += SMALL_JUMP;
                        },
                        KeyCode::PageUp => {
                            self.scrollback_buffer_line = self.scrollback_buffer_line.max(SMALL_JUMP) - SMALL_JUMP;
                        },
                        _ => {},
                    }
                }
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
