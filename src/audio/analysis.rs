use anyhow::{Context, Result};
use num_complex::Complex;
use realfft::{RealFftPlanner, RealToComplex};
use std::collections::VecDeque;
use std::sync::Arc;

use super::{AudioFrame, AudioSample, AudioData};
use crate::config::AudioConfig;

/// Frequency domain data from FFT analysis
#[derive(Debug, Clone)]
pub struct FrequencyData {
    /// Frequency bins (magnitude)
    pub bins: Vec<f32>,
    
    /// Frequency range for each bin (Hz)
    pub bin_frequencies: Vec<f32>,
    
    /// Peak frequency (Hz)
    pub peak_frequency: f32,
    
    /// Spectral centroid (brightness)
    pub spectral_centroid: f32,
    
    /// Spectral energy
    pub spectral_energy: f32,
}

/// Extracted audio features for visualization
#[derive(Debug, Clone)]
pub struct AudioFeatures {
    /// Overall volume (RMS)
    pub volume: f32,
    
    /// Peak amplitude
    pub peak: f32,
    
    /// Detailed frequency bands for better visualization
    pub sub_bass: f32,      // 20-60 Hz
    pub bass: f32,          // 60-250 Hz
    pub low_mid: f32,       // 250-500 Hz
    pub mid: f32,           // 500-2000 Hz
    pub high_mid: f32,      // 2000-4000 Hz
    pub presence: f32,      // 4000-6000 Hz
    pub brilliance: f32,    // 6000-20000 Hz
    
    /// Beat detection confidence (0.0 - 1.0)
    pub beat_confidence: f32,
    
    /// Estimated tempo (BPM)
    pub tempo: f32,
    
    /// Zero crossing rate (indication of pitch)
    pub zero_crossing_rate: f32,
    
    /// Spectral centroid (brightness)
    pub spectral_centroid: f32,
    
    /// Spectral rolloff (frequency below which 85% of energy is contained)
    pub spectral_rolloff: f32,
}

/// Audio analyzer with FFT processing and feature extraction
pub struct AudioAnalyzer {
    config: AudioConfig,
    
    // FFT processing
    fft_planner: RealFftPlanner<f32>,
    fft_processor: Option<Arc<dyn RealToComplex<f32>>>,
    
    // Buffers
    input_buffer: Vec<f32>,
    fft_input: Vec<f32>,
    fft_output: Vec<Complex<f32>>,
    window: Vec<f32>,
    
    // History for beat detection and smoothing
    volume_history: VecDeque<f32>,
    bass_history: VecDeque<f32>,
    
    // State
    sample_rate: f32,
    bin_frequencies: Vec<f32>,
    
    // Beat detection state
    last_beat_time: std::time::Instant,
    tempo_buffer: VecDeque<f32>,
}

impl AudioAnalyzer {
    /// Create a new audio analyzer
    pub fn new(config: &AudioConfig) -> Result<Self> {
        let mut fft_planner = RealFftPlanner::new();
        let fft_processor = fft_planner.plan_fft_forward(config.fft_size);
        
        // Pre-allocate buffers
        let input_buffer = vec![0.0; config.buffer_size];
        let fft_input = vec![0.0; config.fft_size];
        let fft_output = vec![Complex::new(0.0, 0.0); config.fft_size / 2 + 1];
        
        // Create Hann window for better frequency resolution
        let window = Self::create_hann_window(config.fft_size);
        
        // Calculate frequency bins
        let sample_rate = config.sample_rate as f32;
        let bin_frequencies = (0..=config.fft_size / 2)
            .map(|i| i as f32 * sample_rate / config.fft_size as f32)
            .collect();
        
        // Initialize history buffers
        let history_size = (sample_rate / config.buffer_size as f32 * 2.0) as usize; // ~2 seconds
        let volume_history = VecDeque::with_capacity(history_size);
        let bass_history = VecDeque::with_capacity(history_size);
        let tempo_buffer = VecDeque::with_capacity(32); // Last 32 beat intervals
        
        Ok(Self {
            config: config.clone(),
            fft_planner,
            fft_processor: Some(fft_processor),
            input_buffer,
            fft_input,
            fft_output,
            window,
            volume_history,
            bass_history,
            sample_rate,
            bin_frequencies,
            last_beat_time: std::time::Instant::now(),
            tempo_buffer,
        })
    }
    
    /// Process an audio frame and return analyzed data
    pub fn process_frame(&mut self, frame: &AudioFrame) -> Result<AudioData> {
        // Convert multi-channel to mono by averaging
        let mono_samples = self.convert_to_mono(&frame.samples, frame.channels);
        
        // Update input buffer (ring buffer behavior)
        self.update_input_buffer(&mono_samples);
        
        // Prepare FFT input with windowing
        self.prepare_fft_input();
        
        // Perform FFT
        let spectrum = self.perform_fft()?;
        
        // Extract audio features
        let features = self.extract_features(&mono_samples, &spectrum);
        
        // Update history for beat detection
        self.update_history(&features);
        
        // Create processed audio data
        Ok(AudioData {
            waveform: mono_samples,
            spectrum,
            features,
            timestamp: frame.timestamp,
        })
    }
    
    /// Convert multi-channel audio to mono
    fn convert_to_mono(&self, samples: &[AudioSample], channels: u16) -> Vec<AudioSample> {
        if channels == 1 {
            return samples.to_vec();
        }
        
        let frame_count = samples.len() / channels as usize;
        let mut mono = Vec::with_capacity(frame_count);
        
        for frame in 0..frame_count {
            let mut sum = 0.0;
            for channel in 0..channels as usize {
                sum += samples[frame * channels as usize + channel];
            }
            mono.push(sum / channels as f32);
        }
        
        mono
    }
    
    /// Update the input buffer with new samples
    fn update_input_buffer(&mut self, new_samples: &[AudioSample]) {
        let buffer_size = self.input_buffer.len();
        let new_size = new_samples.len();
        
        if new_size >= buffer_size {
            // Replace entire buffer
            self.input_buffer.copy_from_slice(&new_samples[new_size - buffer_size..]);
        } else {
            // Shift existing data and append new samples
            self.input_buffer.copy_within(new_size.., 0);
            let start_idx = buffer_size - new_size;
            self.input_buffer[start_idx..].copy_from_slice(new_samples);
        }
    }
    
    /// Prepare FFT input with windowing
    fn prepare_fft_input(&mut self) {
        let buffer_size = self.input_buffer.len();
        let fft_size = self.fft_input.len();
        
        // Zero-pad if needed or take the most recent samples
        if buffer_size >= fft_size {
            let start_idx = buffer_size - fft_size;
            self.fft_input.copy_from_slice(&self.input_buffer[start_idx..]);
        } else {
            // Zero-pad at the beginning
            let pad_size = fft_size - buffer_size;
            for i in 0..pad_size {
                self.fft_input[i] = 0.0;
            }
            self.fft_input[pad_size..].copy_from_slice(&self.input_buffer);
        }
        
        // Apply Hann window
        for (_i, (sample, window_val)) in self.fft_input.iter_mut().zip(self.window.iter()).enumerate() {
            *sample *= window_val;
        }
    }
    
    /// Perform FFT and return frequency data
    fn perform_fft(&mut self) -> Result<FrequencyData> {
        // Perform FFT
        if let Some(ref fft_processor) = self.fft_processor {
            fft_processor.process(&mut self.fft_input, &mut self.fft_output)
                .context("FFT processing failed")?;
        } else {
            anyhow::bail!("FFT processor not initialized");
        }
        
        // Calculate magnitude spectrum
        let bins: Vec<f32> = self.fft_output.iter()
            .map(|c| c.norm() / self.config.fft_size as f32)
            .collect();
        
        // Find peak frequency
        let peak_bin = bins.iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0);
        let peak_frequency = self.bin_frequencies[peak_bin];
        
        // Calculate spectral centroid (brightness measure)
        let total_magnitude: f32 = bins.iter().sum();
        let spectral_centroid = if total_magnitude > 0.0 {
            bins.iter()
                .zip(self.bin_frequencies.iter())
                .map(|(mag, freq)| mag * freq)
                .sum::<f32>() / total_magnitude
        } else {
            0.0
        };
        
        // Calculate spectral energy
        let spectral_energy = bins.iter().map(|x| x * x).sum::<f32>();
        
        Ok(FrequencyData {
            bins,
            bin_frequencies: self.bin_frequencies.clone(),
            peak_frequency,
            spectral_centroid,
            spectral_energy,
        })
    }
    
    /// Extract audio features from time and frequency domain
    fn extract_features(&mut self, waveform: &[AudioSample], spectrum: &FrequencyData) -> AudioFeatures {
        // Calculate RMS volume
        let volume = if !waveform.is_empty() {
            (waveform.iter().map(|x| x * x).sum::<f32>() / waveform.len() as f32).sqrt()
        } else {
            0.0
        };
        
        // Calculate peak amplitude
        let peak = waveform.iter()
            .map(|x| x.abs())
            .fold(0.0f32, |acc, x| acc.max(x));
        
        // Extract frequency bands
        let sub_bass = self.extract_frequency_band(spectrum, 20.0, 60.0);
        let bass = self.extract_frequency_band(spectrum, 60.0, 250.0);
        let low_mid = self.extract_frequency_band(spectrum, 250.0, 500.0);
        let mid = self.extract_frequency_band(spectrum, 500.0, 2000.0);
        let high_mid = self.extract_frequency_band(spectrum, 2000.0, 4000.0);
        let presence = self.extract_frequency_band(spectrum, 4000.0, 6000.0);
        let brilliance = self.extract_frequency_band(spectrum, 6000.0, self.sample_rate / 2.0);
        
        // Calculate zero crossing rate
        let zero_crossing_rate = self.calculate_zero_crossing_rate(waveform);
        
        // Detect beats
        let beat_confidence = self.detect_beat(volume, bass);
        
        // Estimate tempo
        let tempo = self.estimate_tempo();
        
        AudioFeatures {
            volume,
            peak,
            sub_bass,
            bass,
            low_mid,
            mid,
            high_mid,
            presence,
            brilliance,
            beat_confidence,
            tempo,
            zero_crossing_rate,
            spectral_centroid: spectrum.spectral_centroid,
            spectral_rolloff: self.calculate_spectral_rolloff(spectrum),
        }
    }
    
    /// Extract energy from a specific frequency band
    fn extract_frequency_band(&self, spectrum: &FrequencyData, low_freq: f32, high_freq: f32) -> f32 {
        let nyquist = self.sample_rate / 2.0;
        let low_bin = ((low_freq / nyquist) * spectrum.bins.len() as f32) as usize;
        let high_bin = ((high_freq / nyquist) * spectrum.bins.len() as f32) as usize;
        
        let low_bin = low_bin.min(spectrum.bins.len().saturating_sub(1));
        let high_bin = high_bin.min(spectrum.bins.len());
        
        if high_bin <= low_bin {
            return 0.0;
        }
        
        // Extract the frequency band
        let band_energy: f32 = spectrum.bins[low_bin..high_bin].iter().sum();
        let band_width = high_bin - low_bin;
        
        if band_width == 0 {
            return 0.0;
        }
        
        // Apply logarithmic scaling for better sensitivity
        let avg_energy = band_energy / band_width as f32;
        let log_energy = if avg_energy > 0.0 {
            (avg_energy + 1.0).ln() / 10.0 // Scale down and apply log
        } else {
            0.0
        };
        
        // Apply frequency-dependent weighting (lower frequencies are more important for visualization)
        let center_freq = (low_freq + high_freq) / 2.0;
        let freq_weight = if center_freq < 1000.0 {
            1.5 // Boost low frequencies
        } else if center_freq < 4000.0 {
            1.0 // Normal weight for mid frequencies
        } else {
            0.8 // Slightly reduce high frequencies
        };
        
        (log_energy * freq_weight).min(1.0)
    }
    
    /// Calculate zero crossing rate (indicates pitch/noise characteristics)
    fn calculate_zero_crossing_rate(&self, waveform: &[AudioSample]) -> f32 {
        if waveform.len() < 2 {
            return 0.0;
        }
        
        let crossings = waveform.windows(2)
            .filter(|window| window[0] * window[1] < 0.0)
            .count();
            
        crossings as f32 / (waveform.len() - 1) as f32
    }
    
    /// Simple beat detection based on energy changes
    fn detect_beat(&mut self, current_volume: f32, current_bass: f32) -> f32 {
        let mut beat_confidence = 0.0;
        
        // Check if we have enough history
        if self.volume_history.len() > 10 && self.bass_history.len() > 10 {
            // Calculate recent average
            let recent_avg: f32 = self.volume_history.iter().rev().take(5).sum::<f32>() / 5.0;
            let bass_avg: f32 = self.bass_history.iter().rev().take(5).sum::<f32>() / 5.0;
            
            // Beat detection: current energy significantly higher than recent average
            let volume_ratio = if recent_avg > 0.0 { current_volume / recent_avg } else { 1.0 };
            let bass_ratio = if bass_avg > 0.0 { current_bass / bass_avg } else { 1.0 };
            
            // Combine volume and bass energy for beat detection
            if volume_ratio > 1.5 && bass_ratio > 1.3 {
                let now = std::time::Instant::now();
                let time_since_last = now.duration_since(self.last_beat_time).as_secs_f32();
                
                // Avoid detecting beats too frequently (minimum 100ms apart)
                if time_since_last > 0.1 {
                    beat_confidence = ((volume_ratio - 1.5) + (bass_ratio - 1.3)) / 2.0;
                    beat_confidence = beat_confidence.min(1.0);
                    
                    // Record beat timing for tempo estimation
                    if beat_confidence > 0.3 {
                        self.tempo_buffer.push_back(time_since_last);
                        if self.tempo_buffer.len() > 32 {
                            self.tempo_buffer.pop_front();
                        }
                        self.last_beat_time = now;
                    }
                }
            }
        }
        
        beat_confidence
    }
    
    /// Estimate tempo from beat intervals
    fn estimate_tempo(&self) -> f32 {
        if self.tempo_buffer.len() < 4 {
            return 0.0;
        }
        
        // Calculate average beat interval
        let avg_interval: f32 = self.tempo_buffer.iter().sum::<f32>() / self.tempo_buffer.len() as f32;
        
        // Convert to BPM
        if avg_interval > 0.0 {
            60.0 / avg_interval
        } else {
            0.0
        }
    }
    
    /// Update history buffers for beat detection
    fn update_history(&mut self, features: &AudioFeatures) {
        // Add to history
        self.volume_history.push_back(features.volume);
        self.bass_history.push_back(features.bass);
        
        // Limit history size
        let max_history = 100; // Keep ~2 seconds of history at typical buffer rates
        if self.volume_history.len() > max_history {
            self.volume_history.pop_front();
        }
        if self.bass_history.len() > max_history {
            self.bass_history.pop_front();
        }
    }
    
    /// Create a Hann window for FFT
    fn create_hann_window(size: usize) -> Vec<f32> {
        (0..size)
            .map(|i| {
                let phase = 2.0 * std::f32::consts::PI * i as f32 / (size - 1) as f32;
                0.5 * (1.0 - phase.cos())
            })
            .collect()
    }

    /// Calculate spectral rolloff (frequency below which 85% of energy is contained)
    fn calculate_spectral_rolloff(&self, spectrum: &FrequencyData) -> f32 {
        let energy_threshold = spectrum.spectral_energy * 0.85; // 85% of total energy
        let mut cumulative_energy = 0.0;
        let mut rolloff_bin = 0;

        for (i, &mag) in spectrum.bins.iter().enumerate() {
            cumulative_energy += mag;
            if cumulative_energy >= energy_threshold {
                rolloff_bin = i;
                break;
            }
        }

        // If no bin reaches 85%, return the highest frequency bin
        if rolloff_bin == 0 {
            rolloff_bin = spectrum.bins.len() - 1;
        }

        spectrum.bin_frequencies[rolloff_bin]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AudioCaptureMode;
    
    #[test]
    fn test_audio_analyzer_creation() {
        let config = AudioConfig {
            device_name: None,
            sample_rate: 44100,
            buffer_size: 1024,
            fft_size: 2048,
            capture_mode: AudioCaptureMode::Input,
            enable_loopback: false,
            target_latency_ms: 50.0,
        };
        
        let analyzer = AudioAnalyzer::new(&config);
        assert!(analyzer.is_ok());
    }
    
    #[test]
    fn test_mono_conversion() {
        let config = AudioConfig {
            device_name: None,
            sample_rate: 44100,
            buffer_size: 1024,
            fft_size: 2048,
            capture_mode: AudioCaptureMode::Input,
            enable_loopback: false,
            target_latency_ms: 50.0,
        };
        
        let analyzer = AudioAnalyzer::new(&config).unwrap();
        
        // Test stereo to mono conversion
        let stereo_samples = vec![1.0, 2.0, 3.0, 4.0]; // L, R, L, R
        let mono = analyzer.convert_to_mono(&stereo_samples, 2);
        
        assert_eq!(mono.len(), 2);
        assert_eq!(mono[0], 1.5); // (1.0 + 2.0) / 2
        assert_eq!(mono[1], 3.5); // (3.0 + 4.0) / 2
    }
    
    #[test]
    fn test_hann_window() {
        let window = AudioAnalyzer::create_hann_window(8);
        assert_eq!(window.len(), 8);
        
        // Hann window should start and end at 0
        assert!((window[0] - 0.0).abs() < 1e-6);
        assert!((window[7] - 0.0).abs() < 1e-6);
        
        // And have maximum at the center
        let max_val = window.iter().fold(0.0f32, |acc, &x| acc.max(x));
        assert!((max_val - 1.0).abs() < 0.1);
    }
}