use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, Stream, SampleRate, StreamConfig};
use crossbeam_channel::{Receiver, Sender};
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Instant;

use super::AudioFrame;
use crate::config::{AudioConfig, AudioCaptureMode};

/// Audio capture system supporting multiple input sources
pub struct AudioCaptureSystem {
    config: AudioConfig,
    host: Host,
    current_device: Arc<RwLock<Option<Device>>>,
    current_stream: Arc<RwLock<Option<Stream>>>,
    frame_sender: Arc<RwLock<Option<Sender<AudioFrame>>>>,
    is_capturing: Arc<RwLock<bool>>,
}

// Make AudioCaptureSystem Send + Sync
unsafe impl Send for AudioCaptureSystem {}
unsafe impl Sync for AudioCaptureSystem {}

impl AudioCaptureSystem {
    /// Create a new audio capture system
    pub fn new(config: &AudioConfig) -> Result<Self> {
        let host = cpal::default_host();
        log::info!("Using audio host: {:?}", host.id());
        
        Ok(Self {
            config: config.clone(),
            host,
            current_device: Arc::new(RwLock::new(None)),
            current_stream: Arc::new(RwLock::new(None)),
            frame_sender: Arc::new(RwLock::new(None)),
            is_capturing: Arc::new(RwLock::new(false)),
        })
    }
    
    /// Start audio capture and return a receiver for audio frames
    pub async fn start(&self) -> Result<Receiver<AudioFrame>> {
        log::info!("Starting audio capture");
        
        // Create communication channel
        let (sender, receiver) = crossbeam_channel::unbounded();
        *self.frame_sender.write() = Some(sender);
        
        // Get the appropriate device based on capture mode
        let device = self.get_capture_device()?;
        *self.current_device.write() = Some(device.clone());
        
        // Get device configuration
        let config = self.get_device_config(&device)?;
        log::info!("Using audio config: {:?}", config);
        
        // Create and start the audio stream
        let stream = self.create_audio_stream(&device, &config)?;
        stream.play().context("Failed to start audio stream")?;
        
        *self.current_stream.write() = Some(stream);
        *self.is_capturing.write() = true;
        
        log::info!("Audio capture started successfully");
        Ok(receiver)
    }
    
    /// Stop audio capture
    pub async fn stop(&self) -> Result<()> {
        log::info!("Stopping audio capture");
        
        *self.is_capturing.write() = false;
        
        // Stop and drop the stream
        *self.current_stream.write() = None;
        
        // Clear the device
        *self.current_device.write() = None;
        
        // Clear the sender
        *self.frame_sender.write() = None;
        
        log::info!("Audio capture stopped");
        Ok(())
    }
    
    /// Set the audio device to use
    pub async fn set_device(&self, _device_name: Option<String>) -> Result<()> {
        // This will be used when restarting capture
        // For now, just update the config
        // TODO: Store device name for next start() call
        Ok(())
    }
    
    /// Check if currently capturing
    pub fn is_capturing(&self) -> bool {
        *self.is_capturing.read()
    }
    
    /// Get the appropriate capture device based on configuration
    fn get_capture_device(&self) -> Result<Device> {
        match self.config.capture_mode {
            AudioCaptureMode::Input => self.get_input_device(),
            AudioCaptureMode::Loopback => self.get_loopback_device(),
            AudioCaptureMode::Both => {
                // For now, default to loopback. Later we'll implement mixing
                self.get_loopback_device()
            }
        }
    }
    
    /// Get an input device (microphone)
    fn get_input_device(&self) -> Result<Device> {
        if let Some(device_name) = &self.config.device_name {
            // Find device by name
            for device in self.host.input_devices()? {
                if let Ok(name) = device.name() {
                    if name == *device_name {
                        log::info!("Using specified input device: {}", name);
                        return Ok(device);
                    }
                }
            }
            log::warn!("Specified device '{}' not found, using default", device_name);
        }
        
        // Use default input device
        let device = self.host.default_input_device()
            .ok_or_else(|| anyhow::anyhow!("No default input device available"))?;
            
        if let Ok(name) = device.name() {
            log::info!("Using default input device: {}", name);
        }
        
        Ok(device)
    }
    
    /// Get an output device configured for loopback capture
    fn get_loopback_device(&self) -> Result<Device> {
        #[cfg(target_os = "windows")]
        {
            // For loopback, we need to use input devices that can capture system audio
            // First, try to find Stereo Mix or similar loopback devices
            for device in self.host.input_devices()? {
                if let Ok(name) = device.name() {
                    let name_lower = name.to_lowercase();
                    if name_lower.contains("stereo mix") || name_lower.contains("what u hear") || 
                       name_lower.contains("monitor") || name_lower.contains("loopback") {
                        log::info!("Using loopback device: {}", name);
                        return Ok(device);
                    }
                }
            }
            
            // If no loopback device found, try to use the specified device
            if let Some(device_name) = &self.config.device_name {
                // Find device by name in input devices
                for device in self.host.input_devices()? {
                    if let Ok(name) = device.name() {
                        if name == *device_name {
                            log::info!("Using specified input device for loopback: {}", name);
                            return Ok(device);
                        }
                    }
                }
                log::warn!("Specified input device '{}' not found, using default", device_name);
            }
            
            // Fall back to default input device
            let device = self.host.default_input_device()
                .ok_or_else(|| anyhow::anyhow!("No default input device available"))?;
                
            if let Ok(name) = device.name() {
                log::info!("Using default input device for loopback: {}", name);
            }
            
            Ok(device)
        }
        
        #[cfg(not(target_os = "windows"))]
        {
            // On other platforms, try to find a monitor/loopback device
            for device in self.host.input_devices()? {
                if let Ok(name) = device.name() {
                    let name_lower = name.to_lowercase();
                    if name_lower.contains("monitor") || name_lower.contains("loopback") {
                        log::info!("Using loopback device: {}", name);
                        return Ok(device);
                    }
                }
            }
            
            log::warn!("No loopback device found, falling back to default input");
            self.get_input_device()
        }
    }
    
    /// Get the configuration for the specified device
    fn get_device_config(&self, device: &Device) -> Result<StreamConfig> {
        let default_config = device.default_input_config()
            .context("Failed to get default input config")?;
        
        // Create a config with our preferred settings
        let config = StreamConfig {
            channels: default_config.channels(),
            sample_rate: SampleRate(self.config.sample_rate),
            buffer_size: cpal::BufferSize::Fixed(self.config.buffer_size as u32),
        };
        
        log::debug!("Device config: {:?}", config);
        Ok(config)
    }
    
    /// Create an audio stream for the given device and configuration
    fn create_audio_stream(&self, device: &Device, config: &StreamConfig) -> Result<Stream> {
        let frame_sender = Arc::clone(&self.frame_sender);
        let capture_mode = self.config.capture_mode.clone();
        let sample_rate = config.sample_rate.0;
        let channels = config.channels;
        
        // Create the error callback
        let error_callback = |err| {
            log::error!("Audio stream error: {}", err);
        };
        
        // Build the appropriate stream type
        let stream = match capture_mode {
            AudioCaptureMode::Loopback => {
                #[cfg(target_os = "windows")]
                {
                    // For loopback, try to find a monitor device first
                    let data_callback = move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        let timestamp = Instant::now();
                        
                        let frame = AudioFrame {
                            samples: data.to_vec(),
                            timestamp,
                            sample_rate,
                            channels,
                        };
                        
                        if let Some(sender) = frame_sender.read().as_ref() {
                            if let Err(_) = sender.try_send(frame) {
                                // Channel is full, which is normal under high load
                                // We can either drop frames or implement a different strategy
                            }
                        }
                    };
                    
                    // Try to build an input stream (for Stereo Mix, etc.)
                    match device.build_input_stream(config, data_callback, error_callback, None) {
                        Ok(stream) => {
                            log::info!("Successfully created loopback input stream");
                            stream
                        }
                        Err(e) => {
                            log::warn!("Failed to create loopback input stream: {}", e);
                            log::info!("Falling back to monitor stream");
                            self.create_monitor_stream(config, sample_rate, channels)?
                        }
                    }
                }
                
                #[cfg(not(target_os = "windows"))]
                {
                    let data_callback = move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        let timestamp = Instant::now();
                        
                        let frame = AudioFrame {
                            samples: data.to_vec(),
                            timestamp,
                            sample_rate,
                            channels,
                        };
                        
                        if let Some(sender) = frame_sender.read().as_ref() {
                            if let Err(_) = sender.try_send(frame) {
                                // Channel is full, which is normal under high load
                                // We can either drop frames or implement a different strategy
                            }
                        }
                    };
                    
                    device.build_input_stream(config, data_callback, error_callback, None)
                        .context("Failed to build input stream")?
                }
            }
            _ => {
                let data_callback = move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let timestamp = Instant::now();
                    
                    let frame = AudioFrame {
                        samples: data.to_vec(),
                        timestamp,
                        sample_rate,
                        channels,
                    };
                    
                    if let Some(sender) = frame_sender.read().as_ref() {
                        if let Err(_) = sender.try_send(frame) {
                            // Channel is full, which is normal under high load
                            // We can either drop frames or implement a different strategy
                        }
                    }
                };
                
                device.build_input_stream(config, data_callback, error_callback, None)
                    .context("Failed to build input stream")?
            }
        };
        
        Ok(stream)
    }
    
    /// Create a monitor stream for Windows loopback capture
    #[cfg(target_os = "windows")]
    fn create_monitor_stream(
        &self,
        config: &StreamConfig,
        sample_rate: u32,
        channels: u16,
    ) -> Result<Stream> {
        let frame_sender = Arc::clone(&self.frame_sender);
        
        // Create the data callback
        let data_callback = move |data: &[f32], _: &cpal::InputCallbackInfo| {
            let timestamp = Instant::now();
            
            let frame = AudioFrame {
                samples: data.to_vec(),
                timestamp,
                sample_rate,
                channels,
            };
            
            if let Some(sender) = frame_sender.read().as_ref() {
                if let Err(_) = sender.try_send(frame) {
                    // Channel is full, which is normal under high load
                    // We can either drop frames or implement a different strategy
                }
            }
        };
        
        // Create the error callback
        let error_callback = |err| {
            log::error!("Audio stream error: {}", err);
        };
        
        // Try to find a monitor device
        for device in self.host.input_devices()? {
            if let Ok(name) = device.name() {
                let name_lower = name.to_lowercase();
                if name_lower.contains("monitor") || name_lower.contains("stereo mix") || name_lower.contains("what u hear") {
                    log::info!("Using monitor device: {}", name);
                    return device.build_input_stream(config, data_callback, error_callback, None)
                        .context("Failed to build monitor input stream");
                }
            }
        }
        
        // If no monitor device found, fall back to default input
        log::warn!("No monitor device found, falling back to default input device");
        let default_device = self.host.default_input_device()
            .ok_or_else(|| anyhow::anyhow!("No default input device available"))?;
            
        default_device.build_input_stream(config, data_callback, error_callback, None)
            .context("Failed to build fallback input stream")
    }
    
    /// List all available audio devices
    pub fn list_devices(&self) -> Result<()> {
        log::info!("Available audio devices:");
        
        println!("Input devices:");
        for device in self.host.input_devices()? {
            if let Ok(name) = device.name() {
                println!("  {}", name);
                
                if let Ok(config) = device.default_input_config() {
                    println!("    Sample rate: {} Hz", config.sample_rate().0);
                    println!("    Channels: {}", config.channels());
                    println!("    Sample format: {:?}", config.sample_format());
                }
            }
        }
        
        println!("\nOutput devices (for loopback):");
        for device in self.host.output_devices()? {
            if let Ok(name) = device.name() {
                println!("  {}", name);
                
                if let Ok(config) = device.default_output_config() {
                    println!("    Sample rate: {} Hz", config.sample_rate().0);
                    println!("    Channels: {}", config.channels());
                    println!("    Sample format: {:?}", config.sample_format());
                }
            }
        }
        
        Ok(())
    }
}

// Helper functions for different platforms
#[cfg(target_os = "windows")]
mod windows_audio {
    use super::*;
    
    /// Enhanced Windows-specific audio capture using WASAPI directly
    /// This can be implemented later for more precise control
    pub struct WasapiCapture {
        // TODO: Direct WASAPI implementation for advanced features
    }
    
    impl WasapiCapture {
        pub fn new() -> Result<Self> {
            // TODO: Implement direct WASAPI loopback capture
            // This would give us more control over the capture process
            // and potentially better performance
            todo!("Direct WASAPI implementation")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AudioCaptureMode;
    
    #[test]
    fn test_audio_capture_creation() {
        let config = AudioConfig {
            device_name: None,
            sample_rate: 44100,
            buffer_size: 1024,
            fft_size: 2048,
            capture_mode: AudioCaptureMode::Loopback,
            enable_loopback: true,
            target_latency_ms: 50.0,
        };
        
        let capture_system = AudioCaptureSystem::new(&config);
        assert!(capture_system.is_ok());
    }
    
    #[tokio::test]
    async fn test_device_listing() {
        let config = AudioConfig {
            device_name: None,
            sample_rate: 44100,
            buffer_size: 1024,
            fft_size: 2048,
            capture_mode: AudioCaptureMode::Input,
            enable_loopback: false,
            target_latency_ms: 50.0,
        };
        
        let capture_system = AudioCaptureSystem::new(&config).unwrap();
        
        // This should not panic
        let result = capture_system.list_devices();
        // Note: might fail in CI environments without audio devices
        // assert!(result.is_ok());
    }
}