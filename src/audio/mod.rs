use anyhow::Result;
use crossbeam_channel::{Receiver, Sender};
use parking_lot::RwLock;
use std::sync::Arc;

pub mod capture;
pub mod analysis;
pub mod input;

pub use capture::AudioCaptureSystem;
pub use analysis::{AudioAnalyzer, FrequencyData, AudioFeatures};

use crate::config::AudioConfig;

/// Raw audio sample data
pub type AudioSample = f32;

/// Audio frame containing samples for all channels
#[derive(Debug, Clone)]
pub struct AudioFrame {
    pub samples: Vec<AudioSample>,
    pub timestamp: std::time::Instant,
    pub sample_rate: u32,
    pub channels: u16,
}

/// Processed audio data ready for visualization
#[derive(Debug, Clone)]
pub struct AudioData {
    /// Time domain samples (waveform)
    pub waveform: Vec<AudioSample>,
    
    /// Frequency domain data (spectrum)
    pub spectrum: FrequencyData,
    
    /// Extracted audio features
    pub features: AudioFeatures,
    
    /// Timestamp when this data was captured
    pub timestamp: std::time::Instant,
}

/// Audio system events
#[derive(Debug, Clone)]
pub enum AudioEvent {
    /// New audio data is available
    DataReady(AudioData),
    
    /// Audio device changed
    DeviceChanged(String),
    
    /// Audio processing error
    Error(String),
    
    /// System going to sleep/wake
    SystemSuspend,
    SystemResume,
}

/// Main audio system coordinator
pub struct AudioSystem {
    config: AudioConfig,
    capture_system: Arc<AudioCaptureSystem>,
    analyzer: Arc<RwLock<AudioAnalyzer>>,
    
    // Communication channels
    event_sender: Sender<AudioEvent>,
    event_receiver: Receiver<AudioEvent>,
    
    // State
    is_running: Arc<RwLock<bool>>,
    current_device: Arc<RwLock<Option<String>>>,
}

impl AudioSystem {
    /// Create a new audio system
    pub fn new(config: &AudioConfig) -> Result<Self> {
        let (event_sender, event_receiver) = crossbeam_channel::unbounded();
        
        let capture_system = Arc::new(AudioCaptureSystem::new(config)?);
        let analyzer = Arc::new(RwLock::new(AudioAnalyzer::new(config)?));
        
        Ok(Self {
            config: config.clone(),
            capture_system,
            analyzer,
            event_sender,
            event_receiver,
            is_running: Arc::new(RwLock::new(false)),
            current_device: Arc::new(RwLock::new(None)),
        })
    }
    
    /// Start the audio system
    pub async fn start(&self) -> Result<()> {
        log::info!("Starting audio system");
        
        *self.is_running.write() = true;
        
        // Start audio capture
        let _capture_handle = {
            let capture_system = Arc::clone(&self.capture_system);
            let analyzer = Arc::clone(&self.analyzer);
            let event_sender = self.event_sender.clone();
            let is_running = Arc::clone(&self.is_running);
            
            tokio::spawn(async move {
                Self::audio_processing_loop(capture_system, analyzer, event_sender, is_running).await
            })
        };
        
        log::info!("Audio system started successfully");
        Ok(())
    }
    
    /// Stop the audio system
    pub async fn stop(&self) -> Result<()> {
        log::info!("Stopping audio system");
        
        *self.is_running.write() = false;
        
        // Stop capture system
        self.capture_system.stop().await?;
        
        log::info!("Audio system stopped");
        Ok(())
    }
    
    /// Get the event receiver for audio events
    pub fn event_receiver(&self) -> Receiver<AudioEvent> {
        self.event_receiver.clone()
    }
    
    /// Check if the audio system is running
    pub fn is_running(&self) -> bool {
        *self.is_running.read()
    }
    
    /// Get the current audio device name
    pub fn current_device(&self) -> Option<String> {
        self.current_device.read().clone()
    }
    
    /// Change the audio input device
    pub async fn change_device(&self, device_name: Option<String>) -> Result<()> {
        log::info!("Changing audio device to: {:?}", device_name);
        
        // Stop current capture
        self.capture_system.stop().await?;
        
        // Update device
        self.capture_system.set_device(device_name.clone()).await?;
        
        // Restart capture
        self.capture_system.start().await?;
        
        *self.current_device.write() = device_name.clone();
        
        // Notify listeners
        let device_name = device_name.unwrap_or_else(|| "Default".to_string());
        let _ = self.event_sender.send(AudioEvent::DeviceChanged(device_name));
        
        Ok(())
    }
    
    /// Main audio processing loop
    async fn audio_processing_loop(
        capture_system: Arc<AudioCaptureSystem>,
        analyzer: Arc<RwLock<AudioAnalyzer>>,
        event_sender: Sender<AudioEvent>,
        is_running: Arc<RwLock<bool>>,
    ) {
        let frame_receiver = match capture_system.start().await {
            Ok(receiver) => receiver,
            Err(e) => {
                log::error!("Failed to start audio capture: {}", e);
                let _ = event_sender.send(AudioEvent::Error(format!("Capture failed: {}", e)));
                return;
            }
        };
        
        log::info!("Audio processing loop started");
        
        while *is_running.read() {
            // Receive audio frames from capture system
            match frame_receiver.recv() {
                Ok(frame) => {
                    // Process the audio frame
                    let audio_data = {
                        let mut analyzer = analyzer.write();
                        match analyzer.process_frame(&frame) {
                            Ok(data) => data,
                            Err(e) => {
                                log::warn!("Audio analysis failed: {}", e);
                                continue;
                            }
                        }
                    };
                    
                    // Send processed data to listeners
                    if let Err(e) = event_sender.send(AudioEvent::DataReady(audio_data)) {
                        log::warn!("Failed to send audio data: {}", e);
                    }
                }
                Err(e) => {
                    if *is_running.read() {
                        log::error!("Audio frame receive error: {}", e);
                        let _ = event_sender.send(AudioEvent::Error(format!("Receive error: {}", e)));
                    }
                    break;
                }
            }
        }
        
        log::info!("Audio processing loop stopped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AudioCaptureMode;
    
    #[tokio::test]
    async fn test_audio_system_creation() {
        let config = AudioConfig {
            device_name: None,
            sample_rate: 44100,
            buffer_size: 1024,
            fft_size: 2048,
            capture_mode: AudioCaptureMode::Loopback,
            enable_loopback: true,
            target_latency_ms: 50.0,
        };
        
        let audio_system = AudioSystem::new(&config);
        assert!(audio_system.is_ok());
    }
}