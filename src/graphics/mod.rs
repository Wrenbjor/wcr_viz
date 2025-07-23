use anyhow::Result;
use crossbeam_channel::Receiver;
use log::{info, error};
use winit::{
    application::ApplicationHandler,
    event::{WindowEvent, ElementState},
    event_loop::{ActiveEventLoop, EventLoop},
    window::WindowId,
    keyboard::{KeyCode, PhysicalKey},
};

use crate::audio::{AudioEvent, AudioData};

pub mod renderer;
pub mod shaders;
pub mod window;

use renderer::Renderer;
use window::WindowManager;

/// Graphics system configuration
#[derive(Debug, Clone)]
pub struct GraphicsConfig {
    /// Window title
    pub window_title: String,
    
    /// Initial window width
    pub width: u32,
    
    /// Initial window height
    pub height: u32,
    
    /// Target FPS for rendering
    pub target_fps: u32,
    
    /// Enable vsync
    pub vsync: bool,
    
    /// Enable fullscreen on startup
    pub fullscreen: bool,
}

impl Default for GraphicsConfig {
    fn default() -> Self {
        Self {
            window_title: "RustDrop Visualizer".to_string(),
            width: 1280,
            height: 720,
            target_fps: 60,
            vsync: true,
            fullscreen: false,
        }
    }
}

/// Application state for the new winit 0.30 ApplicationHandler pattern
struct AppState {
    window_manager: Option<WindowManager>,
    renderer: Option<Renderer>,
    audio_receiver: Receiver<AudioEvent>,
    current_audio_data: Option<AudioData>,
    config: GraphicsConfig,
    is_running: bool,
}

impl ApplicationHandler for AppState {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window_manager.is_none() {
            // Initialize window manager and renderer when resumed
            match pollster::block_on(self.create_window_and_renderer(event_loop)) {
                Ok((window_manager, renderer)) => {
                    self.window_manager = Some(window_manager);
                    self.renderer = Some(renderer);
                    info!("Graphics system initialized successfully");
                }
                Err(e) => {
                    error!("Failed to initialize graphics: {}", e);
                    event_loop.exit();
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if let Some(window_manager) = &mut self.window_manager {
            if window_manager.window.id() != window_id {
                return;
            }
            
            match event {
                WindowEvent::CloseRequested => {
                    info!("Window close requested");
                    event_loop.exit();
                }
                WindowEvent::KeyboardInput {
                    event: winit::event::KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        state: ElementState::Pressed,
                        ..
                    },
                    ..
                } => {
                    info!("ESC pressed, exiting");
                    event_loop.exit();
                }
                WindowEvent::Resized(physical_size) => {
                    info!("Window resized to {}x{}", physical_size.width, physical_size.height);
                    window_manager.resize(physical_size.width, physical_size.height);
                    
                    if let Some(renderer) = &mut self.renderer {
                        renderer.resize(physical_size.width, physical_size.height);
                    }
                }
                WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                    info!("Scale factor changed to {}", scale_factor);
                    // Handle scale factor change
                }
                WindowEvent::RedrawRequested => {
                    // Check for audio data updates
                    while let Ok(audio_event) = self.audio_receiver.try_recv() {
                        match audio_event {
                            AudioEvent::DataReady(data) => {
                                self.current_audio_data = Some(data);
                                if let (Some(ref audio_data), Some(ref mut renderer)) = 
                                    (&self.current_audio_data, &mut self.renderer) {
                                    if let Err(e) = renderer.update_audio_data(audio_data) {
                                        error!("Failed to update audio data: {}", e);
                                    }
                                }
                            }
                            AudioEvent::Error(e) => {
                                error!("Audio error: {}", e);
                            }
                            _ => {}
                        }
                    }
                    
                    // Render frame
                    if let Some(ref mut renderer) = &mut self.renderer {
                        if let Some(ref window_manager) = &self.window_manager {
                            if let Ok(output) = window_manager.surface().get_current_texture() {
                                let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
                                if let Err(e) = renderer.render(&view) {
                                    error!("Render error: {}", e);
                                }
                                output.present();
                            }
                        }
                    }
                    
                    // Request next frame
                    if let Some(ref window_manager) = &self.window_manager {
                        window_manager.window.request_redraw();
                    }
                }
                _ => {}
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // Request redraw continuously for real-time visualization
        if let Some(window_manager) = &self.window_manager {
            window_manager.window.request_redraw();
        }
    }
}

impl AppState {
    async fn create_window_and_renderer(
        &self, 
        event_loop: &ActiveEventLoop
    ) -> Result<(WindowManager, Renderer)> {
        // Create window manager with the active event loop
        let window_manager = WindowManager::new_with_event_loop(&self.config, event_loop).await?;
        
        // Bring window to front and focus it
        window_manager.window.focus_window();
        
        // Create renderer with the new API
        let device = window_manager.device().clone();
        let queue = window_manager.queue().clone();
        let config = window_manager.config();
        let renderer = Renderer::new(device, queue, config)?;
        
        Ok((window_manager, renderer))
    }
}

/// Main graphics system coordinator
pub struct GraphicsSystem {
    config: GraphicsConfig,
    audio_receiver: Receiver<AudioEvent>,
}

impl GraphicsSystem {
    /// Create a new graphics system
    pub async fn new(
        config: GraphicsConfig,
        audio_receiver: Receiver<AudioEvent>,
    ) -> Result<Self> {
        info!("Initializing graphics system");
        
        Ok(Self {
            config,
            audio_receiver,
        })
    }
    
    /// Start the graphics system and run the main loop
    pub async fn run(&mut self) -> Result<()> {
        info!("Starting graphics system");
        
        // Create event loop
        let event_loop = EventLoop::new()?;
        
        // Create application state
        let mut app_state = AppState {
            window_manager: None,
            renderer: None,
            audio_receiver: self.audio_receiver.clone(),
            current_audio_data: None,
            config: self.config.clone(),
            is_running: true,
        };
        
        // Run the event loop with the new ApplicationHandler pattern
        event_loop.run_app(&mut app_state)?;
        
        Ok(())
    }
}