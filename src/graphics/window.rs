use anyhow::Result;
use log::info;
use std::sync::Arc;
use wgpu::{Adapter, Device, Queue, Surface, SurfaceConfiguration, Instance};
use winit::{
    event_loop::ActiveEventLoop,
    window::{Window, WindowAttributes},
};

use super::GraphicsConfig;

/// Window and surface manager
pub struct WindowManager {
    pub window: Arc<Window>,
    pub surface: Surface<'static>,
    pub device: Device,
    pub queue: Queue,
    pub config: SurfaceConfiguration,
}

impl WindowManager {
    /// Create a new window manager (legacy - use new_with_event_loop for winit 0.30)
    pub async fn new(_config: &GraphicsConfig) -> Result<WindowManager> {
        // This method is kept for compatibility but shouldn't be used with winit 0.30
        Err(anyhow::anyhow!("Use new_with_event_loop instead for winit 0.30"))
    }
    
    /// Create a new window manager with active event loop (winit 0.30 style)
    pub async fn new_with_event_loop(
        config: &GraphicsConfig, 
        event_loop: &ActiveEventLoop
    ) -> Result<WindowManager> {
        info!("Creating window: {}x{}", config.width, config.height);
        
        // Create window attributes (new API)
        let window_attributes = WindowAttributes::default()
            .with_title(&config.window_title)
            .with_inner_size(winit::dpi::LogicalSize::new(config.width, config.height))
            .with_resizable(true)
            .with_decorations(true)
            .with_transparent(false)
            .with_position(winit::dpi::LogicalPosition::new(100.0, 100.0)) // Position window
            .with_visible(true); // Ensure window is visible
        
        // Create window with active event loop and wrap in Arc
        let window = Arc::new(event_loop.create_window(window_attributes)?);
        
        // Create wgpu instance
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            flags: wgpu::InstanceFlags::default(),
            ..Default::default()
        });
        
        // Create surface (using Arc<Window>)
        let surface = instance.create_surface(window.clone())?;
        
        // Get adapter (new API)
        let adapter = Self::request_adapter(&instance, &surface).await?;
        
        info!("Using GPU: {}", adapter.get_info().name);
        
        // Create device and queue (updated with trace field)
        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("RustDrop Device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: Default::default(),
            trace: wgpu::Trace::Off,
        }).await?;
        
        // Configure surface
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        
        let config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: config.width,
            height: config.height,
            present_mode: if config.vsync {
                wgpu::PresentMode::Fifo
            } else {
                wgpu::PresentMode::Immediate
            },
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        
        surface.configure(&device, &config);
        
        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
        })
    }
    
    /// Request a suitable GPU adapter (updated API)
    async fn request_adapter(instance: &Instance, surface: &Surface<'_>) -> Result<Adapter> {
        let adapter_options = wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(surface),
        };
        
        instance.request_adapter(&adapter_options).await
            .map_err(|e| anyhow::anyhow!("Failed to find suitable GPU adapter: {}", e))
    }
    
    /// Resize the surface
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
        }
    }
    
    /// Get the current surface configuration
    pub fn config(&self) -> &SurfaceConfiguration {
        &self.config
    }
    
    /// Get the device
    pub fn device(&self) -> &Device {
        &self.device
    }
    
    /// Get the queue
    pub fn queue(&self) -> &Queue {
        &self.queue
    }
    
    /// Get the surface
    pub fn surface(&self) -> &Surface {
        &self.surface
    }
}