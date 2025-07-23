use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{Device, SampleFormat};
use log::{info, error};
use std::sync::Arc;

use crate::config::AudioConfig;

/// Microphone and line input handling
pub struct AudioInputManager {
    config: AudioConfig,
    host: cpal::Host,
    current_device: Option<Device>,
}

impl AudioInputManager {
    /// Create a new audio input manager
    pub fn new(config: &AudioConfig) -> Result<Self> {
        let host = cpal::default_host();
        
        Ok(Self {
            config: config.clone(),
            host,
            current_device: None,
        })
    }
    
    /// Get the best available input device
    pub fn get_input_device(&self, device_name: Option<&str>) -> Result<Device> {
        if let Some(name) = device_name {
            // Find device by name
            for device in self.host.input_devices()? {
                if let Ok(device_name) = device.name() {
                    if device_name == name {
                        info!("Using specified input device: {}", device_name);
                        return Ok(device);
                    }
                }
            }
            
            error!("Specified input device '{}' not found, using default", name);
        }
        
        // Use default input device
        let device = self.host.default_input_device()
            .ok_or_else(|| anyhow::anyhow!("No default input device available"))?;
            
        if let Ok(name) = device.name() {
            info!("Using default input device: {}", name);
        }
        
        Ok(device)
    }
    
    /// List all available input devices with their capabilities
    pub fn list_input_devices(&self) -> Result<Vec<InputDeviceInfo>> {
        let mut devices = Vec::new();
        
        for device in self.host.input_devices()? {
            let name = device.name().unwrap_or_else(|_| "Unknown Device".to_string());
            
            let mut supported_configs = Vec::new();
            
            // Get supported configurations
            if let Ok(configs) = device.supported_input_configs() {
                for config in configs {
                    supported_configs.push(InputConfigInfo {
                        channels: config.channels(),
                        min_sample_rate: config.min_sample_rate().0,
                        max_sample_rate: config.max_sample_rate().0,
                        sample_format: config.sample_format(),
                    });
                }
            }
            
            // Get default configuration
            let default_config = device.default_input_config().ok();
            
            devices.push(InputDeviceInfo {
                name,
                supported_configs,
                default_config,
                is_default: false, // We'll mark the default separately
            });
        }
        
        // Mark the default device
        if let Some(default_device) = self.host.default_input_device() {
            if let Ok(default_name) = default_device.name() {
                for device in &mut devices {
                    if device.name == default_name {
                        device.is_default = true;
                        break;
                    }
                }
            }
        }
        
        Ok(devices)
    }
    
    /// Test input device capabilities
    pub fn test_device(&self, device: &Device) -> Result<DeviceTestResult> {
        let name = device.name().unwrap_or_else(|_| "Unknown".to_string());
        
        // Test default configuration
        let default_config = device.default_input_config()
            .context("Failed to get default input configuration")?;
            
        // Check if we can build a stream with our preferred settings
        let preferred_config = cpal::StreamConfig {
            channels: default_config.channels(),
            sample_rate: cpal::SampleRate(self.config.sample_rate),
            buffer_size: cpal::BufferSize::Fixed(self.config.buffer_size as u32),
        };
        
        let stream_test = device.build_input_stream(
            &preferred_config,
            |_data: &[f32], _info| {}, // Dummy callback
            |_err| {},
            None,
        );
        
        let can_use_preferred = stream_test.is_ok();
        
        // Get latency information if available
        let latency = if can_use_preferred {
            // This is platform-specific and may not be available
            None
        } else {
            None
        };
        
        Ok(DeviceTestResult {
            name,
            default_config: default_config.into(),
            can_use_preferred_config: can_use_preferred,
            estimated_latency_ms: latency,
            last_tested: std::time::Instant::now(),
        })
    }
    
    /// Get input level (for testing microphone)
    pub fn get_input_level(&self, _device: &Device) -> Result<f32> {
        // This would require actually capturing some audio
        // For now, return a placeholder
        // TODO: Implement actual level detection
        Ok(0.0)
    }
    
    /// Check if a device supports our required configuration
    pub fn is_device_compatible(&self, device: &Device) -> bool {
        if let Ok(configs) = device.supported_input_configs() {
            for config in configs {
                // Check if our sample rate is supported
                let min_rate = config.min_sample_rate().0;
                let max_rate = config.max_sample_rate().0;
                
                if self.config.sample_rate >= min_rate && self.config.sample_rate <= max_rate {
                    // Check if we support the sample format
                    match config.sample_format() {
                        SampleFormat::F32 => return true,
                        SampleFormat::I16 => return true, // We can convert
                        SampleFormat::U16 => return true, // We can convert
                        _ => continue,
                    }
                }
            }
        }
        
        false
    }
}

/// Information about an input device
#[derive(Debug, Clone)]
pub struct InputDeviceInfo {
    pub name: String,
    pub supported_configs: Vec<InputConfigInfo>,
    pub default_config: Option<cpal::SupportedStreamConfig>,
    pub is_default: bool,
}

/// Configuration information for an input device
#[derive(Debug, Clone)]
pub struct InputConfigInfo {
    pub channels: u16,
    pub min_sample_rate: u32,
    pub max_sample_rate: u32,
    pub sample_format: SampleFormat,
}

/// Result of testing a device
#[derive(Debug, Clone)]
pub struct DeviceTestResult {
    pub name: String,
    pub default_config: cpal::StreamConfig,
    pub can_use_preferred_config: bool,
    pub estimated_latency_ms: Option<f32>,
    pub last_tested: std::time::Instant,
}

/// Input device monitoring for level detection
pub struct InputMonitor {
    device: Device,
    config: cpal::StreamConfig,
    current_level: Arc<std::sync::atomic::AtomicU32>, // Using atomic for thread-safe access
}

impl InputMonitor {
    /// Create a new input monitor
    pub fn new(device: Device, config: cpal::StreamConfig) -> Self {
        Self {
            device,
            config,
            current_level: Arc::new(std::sync::atomic::AtomicU32::new(0)),
        }
    }
    
    /// Start monitoring input levels
    pub fn start_monitoring(&self) -> Result<()> {
        let level_ref = Arc::clone(&self.current_level);
        
        let _stream = self.device.build_input_stream(
            &self.config,
            move |data: &[f32], _info| {
                // Calculate RMS level
                let rms = if !data.is_empty() {
                    let sum_squares: f32 = data.iter().map(|x| x * x).sum();
                    (sum_squares / data.len() as f32).sqrt()
                } else {
                    0.0
                };
                
                // Store as atomic (convert to u32 for atomic storage)
                let level_u32 = (rms * 1000000.0) as u32; // Scale for precision
                level_ref.store(level_u32, std::sync::atomic::Ordering::Relaxed);
            },
            |err| {
                error!("Input monitor error: {}", err);
            },
            None,
        )?;
        
        // TODO: Store stream to keep it alive
        // For now, this is just a demonstration
        
        Ok(())
    }
    
    /// Get the current input level (0.0 to 1.0)
    pub fn get_level(&self) -> f32 {
        let level_u32 = self.current_level.load(std::sync::atomic::Ordering::Relaxed);
        level_u32 as f32 / 1000000.0 // Scale back down
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AudioCaptureMode;
    
    #[test]
    fn test_input_manager_creation() {
        let config = AudioConfig {
            device_name: None,
            sample_rate: 44100,
            buffer_size: 1024,
            fft_size: 2048,
            capture_mode: AudioCaptureMode::Input,
            enable_loopback: false,
            target_latency_ms: 50.0,
        };
        
        let input_manager = AudioInputManager::new(&config);
        assert!(input_manager.is_ok());
    }
    
    #[test]
    fn test_device_listing() {
        let config = AudioConfig {
            device_name: None,
            sample_rate: 44100,
            buffer_size: 1024,
            fft_size: 2048,
            capture_mode: AudioCaptureMode::Input,
            enable_loopback: false,
            target_latency_ms: 50.0,
        };
        
        let input_manager = AudioInputManager::new(&config).unwrap();
        
        // This should not panic, but might return empty list in CI
        let devices = input_manager.list_input_devices();
        assert!(devices.is_ok());
    }
}