# WCR_Viz - Modern MilkDrop Music Visualizer

A high-performance music visualizer written in Rust, inspired by the classic MilkDrop plugin for Winamp.

## ✨ Features (Planned)

- 🎵 **Universal Audio Capture**: System audio from Spotify, YouTube, games, etc.
- 🎨 **MilkDrop Compatibility**: Load and render .milk preset files
- 🖥️ **Multi-Monitor Support**: Span visualizations across multiple displays
- ⚡ **GPU Accelerated**: Modern graphics pipeline using wgpu/WebGPU
- 🎛️ **Real-time Controls**: Keyboard shortcuts and on-screen interface
- 🥁 **Beat Detection**: Auto-preset switching based on music tempo

## 🚀 Current Status: Phase 1 - Audio Foundation

We're currently in **Phase 1** of development, focusing on:
- ✅ Project structure and build system
- ✅ Windows system audio capture (WASAPI loopback)
- ✅ Real-time FFT analysis and beat detection
- ✅ Audio feature extraction (volume, bass, treble, etc.)
- 🔄 Basic console-based audio visualization

## 🛠️ Prerequisites

1. **Rust** (latest stable): Install from [rustup.rs](https://rustup.rs/)
2. **Windows 10/11** (primary target platform)
3. **Visual Studio Build Tools** (for Windows development)

### Windows-specific Requirements

```bash
# Install Visual Studio Build Tools if not already installed
# Required for compiling Windows audio libraries
```

## 🏃‍♂️ Quick Start - Phase 1

### 1. Clone and Setup

```bash
# Create your project directory
mkdir WCR_Viz
cd WCR_Viz

# Copy the provided Cargo.toml and source files
# (Use the artifacts provided in this conversation)
```

### 2. Project Structure

Create this directory structure:
```
WCR_Viz/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── config/
│   │   └── mod.rs
│   └── audio/
│       ├── mod.rs
│       ├── capture.rs
│       ├── analysis.rs
│       └── input.rs
```

### 3. Build and Test

```bash
# Build the project
cargo build

# List available audio devices
cargo run -- --list-devices

# Run with verbose logging
cargo run -- --verbose

# Use a specific audio device
cargo run -- --device "Your Audio Device Name"
```

### 4. Configuration

The app creates a `config.toml` file on first run. Key settings:

```toml
[audio]
sample_rate = 44100
buffer_size = 1024
fft_size = 2048
capture_mode = "Loopback"  # Captures system audio
enable_loopback = true

[graphics]
target_fps = 60
window_width = 1280
window_height = 720

[ui]
show_fps = true
show_audio_levels = true
```

## 🎯 Phase 1 Goals & Testing

### Audio Capture Testing

1. **System Audio**: Play music in Spotify/YouTube and verify capture
2. **Microphone**: Test with `capture_mode = "Input"`
3. **Device Switching**: Try different audio devices

### What Should Work Now

- ✅ List all audio input/output devices
- ✅ Capture system audio (what you hear playing)
- ✅ Real-time FFT analysis
- ✅ Beat detection and tempo estimation
- ✅ Audio feature extraction (bass, mid, treble)
- ✅ Console logging of audio data

### Expected Output

When running with `--verbose`, you should see:
```
INFO  WCR_Viz] Starting WCR_Viz Music Visualizer v0.1.0
INFO  WCR_Viz::audio::capture] Using audio host: Wasapi
INFO  WCR_Viz::audio::capture] Using default output device for loopback: Speakers
INFO  WCR_Viz] Audio capture system initialized
INFO  WCR_Viz] WCR_Viz is running. Press Ctrl+C to exit.
```

And periodic audio analysis data in the logs.

## 🔧 Troubleshooting Phase 1

### Common Issues

1. **No Audio Devices Found**
   ```bash
   # Check Windows audio settings
   # Ensure your audio drivers are installed
   cargo run -- --list-devices
   ```

2. **Build Errors on Windows**
   ```bash
   # Install Visual Studio Build Tools
   # Or install Visual Studio Community with C++ workload
   ```

3. **Audio Capture Not Working**
   ```bash
   # Try different capture modes
   # Edit config.toml and change capture_mode to "Input"
   ```

4. **Permission Issues**
   ```bash
   # Run as Administrator if needed for audio access
   ```

## 📋 Phase 1 Development Tasks

### Completed ✅
- [x] Project structure and dependencies
- [x] Configuration system with TOML
- [x] Cross-platform audio capture with cpal
- [x] Windows WASAPI loopback support
- [x] Real-time FFT analysis with rustfft
- [x] Audio feature extraction
- [x] Beat detection algorithm
- [x] Logging and error handling

### In Progress 🔄
- [ ] Audio visualization in console
- [ ] Audio level meters
- [ ] Frequency spectrum display
- [ ] Beat detection visualization

### Next Phase Preview 🔮
- [ ] wgpu graphics initialization
- [ ] Basic shader pipeline
- [ ] Simple geometric visualizations
- [ ] Audio-reactive animations

## 🤝 Contributing to Phase 1

Focus areas for Phase 1:
1. **Audio Quality**: Improve FFT analysis and beat detection
2. **Performance**: Optimize real-time processing
3. **Compatibility**: Test with different audio devices
4. **Console Visualization**: Add text-based spectrum display

## 📚 Phase 1 Architecture

```
Audio Flow:
Spotify/Game → Windows Audio → WASAPI Loopback → cpal → AudioFrame
                                                           ↓
AudioAnalyzer → FFT → FrequencyData + AudioFeatures → Console Display
```

## 🔬 Testing Your Audio Setup

1. **Play music** in any application (Spotify, YouTube, etc.)
2. **Run WCR_Viz** with `cargo run -- --verbose`
3. **Check logs** for audio level and beat detection
4. **Verify** that volume/bass/treble values change with music

## 📞 Support

- **Phase 1 Issues**: Focus on audio capture and analysis problems
- **Build Problems**: Ensure you have proper Windows development tools
- **Audio Issues**: Test with `--list-devices` first

---

**Next**: Once Phase 1 is stable, we'll move to Phase 2 (Graphics Foundation) with wgpu and basic visualizations!