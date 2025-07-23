use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub audio: AudioConfig,
    pub graphics: GraphicsConfig,
    pub ui: UiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    /// Audio device name (None for default)
    pub device_name: Option<String>,
    
    /// Sample rate (Hz)
    pub sample_rate: u32,
    
    /// Buffer size for audio processing
    pub buffer_size: usize,
    
    /// FFT size for frequency analysis
    pub fft_size: usize,
    
    /// Audio capture mode
    pub capture_mode: AudioCaptureMode,
    
    /// Enable system audio loopback (Windows WASAPI)
    pub enable_loopback: bool,
    
    /// Audio latency target (milliseconds)
    pub target_latency_ms: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AudioCaptureMode {
    /// Capture from microphone/line input
    Input,
    /// Capture system audio output (loopback)
    Loopback,
    /// Capture both input and system audio
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphicsConfig {
    /// Target framerate
    pub target_fps: u32,
    
    /// Enable VSync
    pub vsync: bool,
    
    /// Window width (windowed mode)
    pub window_width: u32,
    
    /// Window height (windowed mode)
    pub window_height: u32,
    
    /// Start in fullscreen mode
    pub start_fullscreen: bool,
    
    /// Enable multi-monitor support
    pub multi_monitor: bool,
    
    /// Texture filtering mode
    pub texture_filtering: TextureFiltering,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TextureFiltering {
    Nearest,
    Linear,
    Anisotropic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// Show FPS counter
    pub show_fps: bool,
    
    /// Show audio levels
    pub show_audio_levels: bool,
    
    /// Show preset name
    pub show_preset_name: bool,
    
    /// UI scale factor
    pub scale_factor: f32,
    
    /// Hide UI after inactivity (seconds)
    pub auto_hide_ui_seconds: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            audio: AudioConfig {
                device_name: None,
                sample_rate: 44100,
                buffer_size: 1024,
                fft_size: 2048,
                capture_mode: AudioCaptureMode::Loopback,
                enable_loopback: true,
                target_latency_ms: 50.0,
            },
            graphics: GraphicsConfig {
                target_fps: 60,
                vsync: true,
                window_width: 1280,
                window_height: 720,
                start_fullscreen: false,
                multi_monitor: false,
                texture_filtering: TextureFiltering::Linear,
            },
            ui: UiConfig {
                show_fps: true,
                show_audio_levels: true,
                show_preset_name: true,
                scale_factor: 1.0,
                auto_hide_ui_seconds: 5.0,
            },
        }
    }
}

impl Config {
    /// Load configuration from a TOML file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        
        if !path.exists() {
            log::info!("Configuration file not found, creating default: {}", path.display());
            let default_config = Self::default();
            default_config.save(path)?;
            return Ok(default_config);
        }
        
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
            
        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
            
        log::debug!("Loaded configuration: {:#?}", config);
        Ok(config)
    }
    
    /// Save configuration to a TOML file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        
        let content = toml::to_string_pretty(self)
            .context("Failed to serialize configuration")?;
            
        fs::write(path, content)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;
            
        log::info!("Configuration saved to: {}", path.display());
        Ok(())
    }
    
    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Validate audio settings
        if self.audio.sample_rate == 0 {
            anyhow::bail!("Sample rate must be greater than 0");
        }
        
        if self.audio.buffer_size == 0 || !self.audio.buffer_size.is_power_of_two() {
            anyhow::bail!("Buffer size must be a power of 2 and greater than 0");
        }
        
        if self.audio.fft_size == 0 || !self.audio.fft_size.is_power_of_two() {
            anyhow::bail!("FFT size must be a power of 2 and greater than 0");
        }
        
        // Validate graphics settings
        if self.graphics.target_fps == 0 {
            anyhow::bail!("Target FPS must be greater than 0");
        }
        
        if self.graphics.window_width == 0 || self.graphics.window_height == 0 {
            anyhow::bail!("Window dimensions must be greater than 0");
        }
        
        // Validate UI settings
        if self.ui.scale_factor <= 0.0 {
            anyhow::bail!("UI scale factor must be greater than 0");
        }
        
        Ok(())
    }
}