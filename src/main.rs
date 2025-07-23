use anyhow::Result;
use clap::Parser;
use log::{info, error};
use std::sync::Arc;

mod audio;
mod config;
mod graphics;

use audio::AudioSystem;
use config::Config;
use graphics::{GraphicsSystem, GraphicsConfig};

#[derive(Parser)]
#[command(name = "wcr-viz")]
#[command(about = "A modern MilkDrop music visualizer clone written in Rust")]
struct Args {
    /// Configuration file path
    #[arg(short, long, default_value = "config.toml")]
    config: String,
    
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
    
    /// List available audio devices
    #[arg(long)]
    list_devices: bool,
    
    /// Audio device to use (default: system default)
    #[arg(long)]
    device: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize logging
    if args.verbose {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    } else {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }
    
    info!("Starting RustDrop Music Visualizer v{}", env!("CARGO_PKG_VERSION"));
    
    // Load configuration
    let mut config = Config::load(&args.config)?;
    info!("Configuration loaded from: {}", args.config);
    
    // Override device if specified
    if let Some(device_name) = args.device {
        config.audio.device_name = Some(device_name);
        info!("Using specified audio device: {:?}", config.audio.device_name);
    }
    
    // List audio devices if requested
    if args.list_devices {
        list_audio_devices()?;
        return Ok(());
    }
    
    // Initialize audio system
    let audio_system = Arc::new(AudioSystem::new(&config.audio)?);
    info!("Audio system initialized");
    
    // Get audio event receiver
    let audio_receiver = audio_system.event_receiver();
    
    // Start audio system
    let audio_handle = {
        let audio_system = Arc::clone(&audio_system);
        tokio::spawn(async move {
            if let Err(e) = audio_system.start().await {
                error!("Audio system failed: {}", e);
            }
        })
    };
    
    // Create graphics configuration from config
    let graphics_config = GraphicsConfig {
        window_title: "RustDrop Visualizer".to_string(),
        width: config.graphics.window_width,
        height: config.graphics.window_height,
        target_fps: config.graphics.target_fps,
        vsync: config.graphics.vsync,
        fullscreen: config.graphics.start_fullscreen,
    };
    
    // Initialize graphics system
    let mut graphics_system = GraphicsSystem::new(graphics_config, audio_receiver).await?;
    info!("Graphics system initialized");
    
    // Run graphics system (this will block until window is closed)
    info!("Starting visualization...");
    graphics_system.run().await?;
    
    // Cleanup
    audio_handle.abort();
    info!("RustDrop shutdown complete");
    
    Ok(())
}

fn list_audio_devices() -> Result<()> {
    use cpal::traits::{DeviceTrait, HostTrait};
    
    info!("Available audio devices:");
    
    let host = cpal::default_host();
    
    // List input devices
    println!("\nInput Devices:");
    for device in host.input_devices()? {
        let name = device.name().unwrap_or_else(|_| "Unknown".to_string());
        println!("  - {}", name);
        
        if let Ok(config) = device.default_input_config() {
            println!("    Default config: {:?}", config);
        }
    }
    
    // List output devices (for loopback)
    println!("\nOutput Devices (available for loopback):");
    for device in host.output_devices()? {
        let name = device.name().unwrap_or_else(|_| "Unknown".to_string());
        println!("  - {}", name);
        
        if let Ok(config) = device.default_output_config() {
            println!("    Default config: {:?}", config);
        }
    }
    
    // Show default devices
    if let Some(device) = host.default_input_device() {
        if let Ok(name) = device.name() {
            println!("\nDefault input device: {}", name);
        }
    }
    
    if let Some(device) = host.default_output_device() {
        if let Ok(name) = device.name() {
            println!("Default output device: {}", name);
        }
    }
    
    Ok(())
}