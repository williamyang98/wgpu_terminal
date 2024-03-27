use std::io::Read;
use cgmath::{Vector2,Vector4,ElementWise};
use tile_renderer::{
    Renderer,
    GlyphCache,
    FontdueGlyphGenerator,
    CellData,
};
use terminal::{
    Terminal,
    TerminalUserEvent,
    terminal_renderer::TerminalRenderer,
};
use winit::{
    event::{Event, WindowEvent, ElementState, MouseButton},
    keyboard::{KeyCode,PhysicalKey,Key},
    event_loop::EventLoopWindowTarget,
    window::Window,
};
use crate::app_events::AppEvent;
use crate::frame_counter::FrameCounter;
use vt100::common::WindowAction;
use crossbeam_channel::Sender;

pub struct AppWindow<'a> {
    terminal: Terminal,
    terminal_user_events: Sender<TerminalUserEvent>, 
    terminal_renderer: TerminalRenderer,
    glyph_grid: Vec<CellData>,
    glyph_cache: GlyphCache,
    winit_window: &'a Window,
    wgpu_config: wgpu::SurfaceConfiguration,
    wgpu_surface: wgpu::Surface<'a>,
    wgpu_device: wgpu::Device,
    wgpu_queue: wgpu::Queue,
    renderer: Renderer,
    is_redraw_requested: bool,
    current_frame: usize,
    frame_counter: FrameCounter,
}

impl<'a> AppWindow<'a> {
    pub async fn new(
        winit_window: &'a Window,
        terminal: Terminal, 
        font_filename: String, font_size: f32,
    ) -> anyhow::Result<Self> 
    {
        let terminal_user_events = terminal.get_user_event_handler();
        // wgpu
        let wgpu_instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::util::backend_bits_from_env().unwrap_or_default(),
            flags: wgpu::InstanceFlags::from_build_config().with_env(),
            dx12_shader_compiler: wgpu::util::dx12_shader_compiler_from_env().unwrap_or_default(),
            gles_minor_version: wgpu::util::gles_minor_version_from_env().unwrap_or_default(),
        });
        let wgpu_surface = wgpu_instance.create_surface(winit_window)?;
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
        // glyph cache
        let mut font_file = std::fs::File::open(font_filename.clone())?;
        let mut font_data = Vec::<u8>::new();
        let total_bytes_read = font_file.read_to_end(&mut font_data)?;
        let font_data = &font_data[..total_bytes_read];
        let font_settings = fontdue::FontSettings::default();
        let font = fontdue::Font::from_bytes(font_data, font_settings).map_err(anyhow::Error::msg)?;
        let wgpu_limits = wgpu_adapter.limits();
        let max_texture_size = wgpu_limits.max_texture_dimension_2d as usize;
        let max_texture_size = Vector2::new(max_texture_size, max_texture_size);
        let glyph_generator = Box::new(FontdueGlyphGenerator::new(font, font_size));
        let glyph_cache = GlyphCache::new(glyph_generator, max_texture_size);

        Ok(Self {
            terminal,
            terminal_user_events,
            terminal_renderer: TerminalRenderer::default(),
            glyph_grid: Vec::new(),
            glyph_cache,
            winit_window,
            wgpu_config,
            wgpu_surface,
            wgpu_device,
            wgpu_queue,
            renderer,
            is_redraw_requested: false,
            current_frame: 0,
            frame_counter: FrameCounter::default(),
        })
    }

    pub fn on_winit_event(
        &mut self, event: Event<AppEvent>, target: &EventLoopWindowTarget<AppEvent>) {
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => target.exit(),
                WindowEvent::MouseWheel { delta, .. } => self.on_mouse_wheel(delta),
                WindowEvent::KeyboardInput { event, .. } => self.on_keyboard_input(event),
                WindowEvent::MouseInput { state, button, .. } => self.on_mouse_input(button, state),
                WindowEvent::Focused(is_focused) => self.on_focus(is_focused),
                WindowEvent::Resized(new_size) => {
                    let new_size = Vector2::new(new_size.width as usize, new_size.height as usize);
                    self.on_resize(new_size);
                },
                WindowEvent::CursorMoved { position, .. } => {
                    let position = Vector2::new(position.x.max(0.0) as usize, position.y.max(0.0) as usize);
                    self.on_cursor_move(position);
                },
                WindowEvent::RedrawRequested => self.on_redraw_requested(),
                _ => {
                    // log::info!("Unhandled: {:?}", event);
                },
            },
            Event::UserEvent(event) => match event {
                AppEvent::WindowAction(action) => self.on_window_action(action),
            },
            _ => {
                // log::info!("Unhandled: {:?}", event);
            },
        }
    }

    fn trigger_redraw(&mut self) {
        if !self.is_redraw_requested {
            self.is_redraw_requested = true;
            self.winit_window.request_redraw();
        }
    }

    fn on_mouse_input(&mut self, button: MouseButton, state: ElementState) {
        use vt100::encoder::{MouseButton as TMouseButton};
        let button = match button {
            MouseButton::Left => TMouseButton::LeftClick,
            MouseButton::Right => TMouseButton::RightClick,
            MouseButton::Middle => TMouseButton::MiddleClick,
            _ => return,
        };
        let event = match state {
            ElementState::Pressed => TerminalUserEvent::MousePress(button),
            ElementState::Released => TerminalUserEvent::MouseRelease(button),
        };
        self.terminal_user_events.send(event).unwrap();
    }

    fn on_window_action(&mut self, action: WindowAction) {
        match action {
            WindowAction::SetWindowTitle(title) => self.winit_window.set_title(title.as_str()),
            WindowAction::Refresh => self.trigger_redraw(),
            _ => {
                log::info!("Unhandled: {:?}", action);
            }
        }
    }

    fn on_cursor_move(&mut self, pos: Vector2<usize>) {
        self.terminal_user_events.send(TerminalUserEvent::MouseMove(pos)).unwrap();
    }

    fn on_focus(&mut self, is_focus: bool) {
        self.terminal_user_events.send(TerminalUserEvent::WindowFocus(is_focus)).unwrap();
    }

    fn on_mouse_wheel(&mut self, delta: winit::event::MouseScrollDelta) {
        use winit::event::MouseScrollDelta as Delta;
        match delta {
            Delta::LineDelta(_x, y) => {
                if y > 0.0 {
                    self.terminal_renderer.scroll_up(1);
                } else {
                    self.terminal_renderer.scroll_down(1);
                }
            },
            Delta::PixelDelta(delta) => {
                if delta.y > 0.0 {
                    self.terminal_renderer.scroll_up(1);
                } else {
                    self.terminal_renderer.scroll_down(1);
                }
            },
        }
        self.trigger_redraw();
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
        self.terminal_user_events.send(TerminalUserEvent::WindowResize(actual_render_size)).unwrap();
        self.terminal_user_events.send(TerminalUserEvent::GridResize(new_grid_size)).unwrap();
        self.trigger_redraw();
    }

    fn on_redraw_requested(&mut self) {
        self.is_redraw_requested = false;
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
        let display = self.terminal.get_display();
        self.terminal_renderer.render_display(&display);
 
        let size = self.terminal_renderer.get_size();
        let cells = self.terminal_renderer.get_cells();
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
                src.pen.foreground_colour.r,
                src.pen.foreground_colour.g,
                src.pen.foreground_colour.b,
                255,
            );
            dst.colour_background = Vector4::new(
                src.pen.background_colour.r,
                src.pen.background_colour.g,
                src.pen.background_colour.b,
                255,
            );
            dst.style_flags = 0u32;
        }
        self.renderer.update_grid(&self.wgpu_device, &self.wgpu_queue, self.glyph_grid.as_slice(), size);
        let glyph_atlas = self.glyph_cache.get_glyph_atlas_mut();
        self.renderer.update_atlas(&self.wgpu_device, &self.wgpu_queue, glyph_atlas);
    }

    fn on_keyboard_input(&mut self, event: winit::event::KeyEvent) {
        use vt100::encoder::{KeyCode as TKey, ModifierKey, ArrowKey, FunctionKey};

        let key_press = &mut |key: TKey| self.terminal_user_events.send(TerminalUserEvent::KeyPress(key)).unwrap();
        let key_release = &mut |key: TKey| self.terminal_user_events.send(TerminalUserEvent::KeyRelease(key)).unwrap();
        // modifier keys listen to press/release
        if let PhysicalKey::Code(code) = event.physical_key {
            match event.state {
                ElementState::Pressed => match code {
                    KeyCode::AltLeft      => return key_press(TKey::ModifierKey(ModifierKey::Alt)),
                    KeyCode::AltRight     => return key_press(TKey::ModifierKey(ModifierKey::Alt)),
                    KeyCode::ControlLeft  => return key_press(TKey::ModifierKey(ModifierKey::Ctrl)),
                    KeyCode::ControlRight => return key_press(TKey::ModifierKey(ModifierKey::Ctrl)),
                    KeyCode::ShiftLeft    => return key_press(TKey::ModifierKey(ModifierKey::Shift)),
                    KeyCode::ShiftRight   => return key_press(TKey::ModifierKey(ModifierKey::Shift)),
                    _ => {},
                },
                ElementState::Released => match code {
                    KeyCode::AltLeft      => return key_release(TKey::ModifierKey(ModifierKey::Alt)),
                    KeyCode::AltRight     => return key_release(TKey::ModifierKey(ModifierKey::Alt)),
                    KeyCode::ControlLeft  => return key_release(TKey::ModifierKey(ModifierKey::Ctrl)),
                    KeyCode::ControlRight => return key_release(TKey::ModifierKey(ModifierKey::Ctrl)),
                    KeyCode::ShiftLeft    => return key_release(TKey::ModifierKey(ModifierKey::Shift)),
                    KeyCode::ShiftRight   => return key_release(TKey::ModifierKey(ModifierKey::Shift)),
                    _ => {},
                },
            };
        };

        if event.state != ElementState::Pressed {
            return;
        }

        if let PhysicalKey::Code(code) = event.physical_key {
            match code {
                KeyCode::ArrowUp    => return key_press(TKey::ArrowKey(ArrowKey::Up)),
                KeyCode::ArrowDown  => return key_press(TKey::ArrowKey(ArrowKey::Down)),
                KeyCode::ArrowLeft  => return key_press(TKey::ArrowKey(ArrowKey::Left)),
                KeyCode::ArrowRight => return key_press(TKey::ArrowKey(ArrowKey::Right)),
                _ => {},
            }
        }

        if let PhysicalKey::Code(code) = event.physical_key {
            match code {
                KeyCode::Escape    => return key_press(TKey::FunctionKey(FunctionKey::Escape)),
                KeyCode::Tab       => return key_press(TKey::FunctionKey(FunctionKey::Tab)),
                KeyCode::Backspace => return key_press(TKey::FunctionKey(FunctionKey::Backspace)),
                KeyCode::Enter     => return key_press(TKey::FunctionKey(FunctionKey::Enter)),
                KeyCode::Delete    => return key_press(TKey::FunctionKey(FunctionKey::Delete)),
                _ => {},
            }
        }

        if let PhysicalKey::Code(code) = event.physical_key {
            let size = self.terminal_renderer.get_size();
            let mut is_render = true;
            match code {
                KeyCode::End      => self.terminal_renderer.scroll_to_bottom(),
                KeyCode::Home     => self.terminal_renderer.scroll_to_top(),
                KeyCode::PageDown => self.terminal_renderer.scroll_down(size.y),
                KeyCode::PageUp   => self.terminal_renderer.scroll_up(size.y),
                _ => { is_render = false; },
            }
            if is_render {
                self.trigger_redraw();
                return;
            }
        }

        if event.physical_key == PhysicalKey::Code(KeyCode::Space) {
            key_press(TKey::Char(' '));
            self.terminal_renderer.scroll_to_bottom();
            self.trigger_redraw();
            return;
        }
        if let Key::Character(string) = event.logical_key {
            for c in string.chars() {
                key_press(TKey::Char(c));
            }
            self.terminal_renderer.scroll_to_bottom();
            self.trigger_redraw();
        }
    }
}
