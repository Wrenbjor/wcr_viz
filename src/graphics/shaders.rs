use anyhow::Result;
use log::info;
use wgpu::{Device, ShaderModule, ShaderModuleDescriptor};

/// Shader manager for loading and compiling WGSL shaders
pub struct ShaderManager {
    device: Device,
}

impl ShaderManager {
    /// Create a new shader manager
    pub fn new(device: Device) -> Self {
        Self { device }
    }
    
    /// Load and compile a shader from WGSL source
    pub fn load_shader(&self, label: &str, source: &str) -> Result<ShaderModule> {
        info!("Loading shader: {}", label);
        
        let shader = self.device.create_shader_module(ShaderModuleDescriptor {
            label: Some(label),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        
        Ok(shader)
    }
    
    /// Get the basic vertex shader for 2D rendering
    pub fn basic_vertex_shader(&self) -> Result<ShaderModule> {
        const VERTEX_SHADER: &str = r#"
            struct VertexInput {
                @location(0) position: vec2<f32>,
                @location(1) color: vec4<f32>,
            };
            
            struct VertexOutput {
                @builtin(position) position: vec4<f32>,
                @location(0) color: vec4<f32>,
            };
            
            @vertex
            fn vertex_main(input: VertexInput) -> VertexOutput {
                var output: VertexOutput;
                output.position = vec4<f32>(input.position, 0.0, 1.0);
                output.color = input.color;
                return output;
            }
        "#;
        
        self.load_shader("basic_vertex", VERTEX_SHADER)
    }
    
    /// Get the basic fragment shader
    pub fn basic_fragment_shader(&self) -> Result<ShaderModule> {
        const FRAGMENT_SHADER: &str = r#"
            @fragment
            fn fragment_main(@location(0) color: vec4<f32>) -> @location(0) vec4<f32> {
                return color;
            }
        "#;
        
        self.load_shader("basic_fragment", FRAGMENT_SHADER)
    }
    
    /// Get the audio-reactive fragment shader for frequency bars
    pub fn frequency_bars_fragment_shader(&self) -> Result<ShaderModule> {
        const FRAGMENT_SHADER: &str = r#"
            struct Uniforms {
                audio_data: array<f32, 256>,
                time: f32,
                volume: f32,
                bass: f32,
                mid: f32,
                treble: f32,
            };
            
            @group(0) @binding(0) var<uniform> uniforms: Uniforms;
            
            @fragment
            fn fragment_main(@location(0) color: vec4<f32>) -> @location(0) vec4<f32> {
                // Create a pulsing effect based on bass
                let pulse = sin(uniforms.time * 10.0) * 0.1 + 0.9;
                let bass_boost = uniforms.bass * 2.0;
                
                // Enhance color with audio reactivity
                let enhanced_color = color * vec4<f32>(pulse + bass_boost, pulse, pulse, 1.0);
                
                return enhanced_color;
            }
        "#;
        
        self.load_shader("frequency_bars_fragment", FRAGMENT_SHADER)
    }
    
    /// Get the waveform fragment shader
    pub fn waveform_fragment_shader(&self) -> Result<ShaderModule> {
        const FRAGMENT_SHADER: &str = r#"
            struct Uniforms {
                waveform_data: array<f32, 1024>,
                time: f32,
                volume: f32,
            };
            
            @group(0) @binding(0) var<uniform> uniforms: Uniforms;
            
            @fragment
            fn fragment_main(@location(0) color: vec4<f32>) -> @location(0) vec4<f32> {
                // Create a flowing effect based on volume
                let flow = sin(uniforms.time * 5.0 + uniforms.volume * 10.0) * 0.2 + 0.8;
                
                // Enhance color with volume reactivity
                let enhanced_color = color * vec4<f32>(flow, flow * 0.8, flow * 1.2, 1.0);
                
                return enhanced_color;
            }
        "#;
        
        self.load_shader("waveform_fragment", FRAGMENT_SHADER)
    }
    
    /// Get the pulsing circle fragment shader
    pub fn pulsing_circle_fragment_shader(&self) -> Result<ShaderModule> {
        const FRAGMENT_SHADER: &str = r#"
            struct Uniforms {
                time: f32,
                bass: f32,
                mid: f32,
                treble: f32,
                volume: f32,
            };
            
            @group(0) @binding(0) var<uniform> uniforms: Uniforms;
            
            @fragment
            fn fragment_main(@location(0) color: vec4<f32>) -> @location(0) vec4<f32> {
                // Create multiple pulsing rings based on different frequency bands
                let bass_pulse = sin(uniforms.time * 8.0) * uniforms.bass * 0.5 + 0.5;
                let mid_pulse = sin(uniforms.time * 12.0) * uniforms.mid * 0.3 + 0.7;
                let treble_pulse = sin(uniforms.time * 16.0) * uniforms.treble * 0.2 + 0.8;
                
                // Combine pulses for a rich visual effect
                let combined_pulse = (bass_pulse + mid_pulse + treble_pulse) / 3.0;
                
                // Create a color gradient based on frequency content
                let bass_color = vec3<f32>(1.0, 0.2, 0.2) * bass_pulse;
                let mid_color = vec3<f32>(0.2, 1.0, 0.2) * mid_pulse;
                let treble_color = vec3<f32>(0.2, 0.2, 1.0) * treble_pulse;
                
                let final_color = (bass_color + mid_color + treble_color) * combined_pulse;
                
                return vec4<f32>(final_color, 1.0);
            }
        "#;
        
        self.load_shader("pulsing_circle_fragment", FRAGMENT_SHADER)
    }
}

/// Predefined shader collection
pub struct Shaders {
    pub basic_vertex: ShaderModule,
    pub basic_fragment: ShaderModule,
    pub frequency_bars_fragment: ShaderModule,
    pub waveform_fragment: ShaderModule,
    pub pulsing_circle_fragment: ShaderModule,
}

impl Shaders {
    /// Load all shaders
    pub fn load_all(device: Device) -> Result<Self> {
        let shader_manager = ShaderManager::new(device);
        
        Ok(Self {
            basic_vertex: shader_manager.basic_vertex_shader()?,
            basic_fragment: shader_manager.basic_fragment_shader()?,
            frequency_bars_fragment: shader_manager.frequency_bars_fragment_shader()?,
            waveform_fragment: shader_manager.waveform_fragment_shader()?,
            pulsing_circle_fragment: shader_manager.pulsing_circle_fragment_shader()?,
        })
    }
} 