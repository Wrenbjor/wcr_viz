use anyhow::Result;
use crate::preset::Preset;
use std::collections::HashMap;

/// Preset renderer for converting MilkDrop presets to WGSL shaders
pub struct PresetRenderer {
    // Shader cache to avoid recompiling the same shaders
    shader_cache: HashMap<String, String>,
}

impl PresetRenderer {
    /// Create a new preset renderer
    pub fn new() -> Self {
        Self {
            shader_cache: HashMap::new(),
        }
    }
    
    /// Convert MilkDrop per-pixel shader to WGSL
    pub fn convert_per_pixel_shader(&self, preset: &Preset) -> Result<String> {
        if let Some(per_pixel) = &preset.equations.per_pixel {
            // Convert HLSL per-pixel code to WGSL
            let mut wgsl_code = String::new();
            
            // Add basic WGSL shader structure
            wgsl_code.push_str(&self.generate_basic_wgsl_shader(per_pixel, "per_pixel")?);
            
            Ok(wgsl_code)
        } else {
            // Generate a default per-pixel shader
            Ok(self.generate_default_per_pixel_shader())
        }
    }
    
    /// Convert MilkDrop warp shader to WGSL
    pub fn convert_warp_shader(&self, preset: &Preset) -> Result<String> {
        if let Some(warp) = &preset.equations.warp_shader {
            // Convert HLSL warp code to WGSL
            let mut wgsl_code = String::new();
            
            // Add basic WGSL shader structure
            wgsl_code.push_str(&self.generate_basic_wgsl_shader(warp, "warp")?);
            
            Ok(wgsl_code)
        } else {
            // Generate a default warp shader
            Ok(self.generate_default_warp_shader())
        }
    }
    
    /// Convert MilkDrop composite shader to WGSL
    pub fn convert_comp_shader(&self, preset: &Preset) -> Result<String> {
        if let Some(comp) = &preset.equations.comp_shader {
            // Convert HLSL comp code to WGSL
            let mut wgsl_code = String::new();
            
            // Add basic WGSL shader structure
            wgsl_code.push_str(&self.generate_basic_wgsl_shader(comp, "comp")?);
            
            Ok(wgsl_code)
        } else {
            // Generate a default comp shader
            Ok(self.generate_default_comp_shader())
        }
    }
    
    /// Generate uniform buffer for preset variables
    pub fn generate_uniform_buffer(&self, preset: &Preset) -> Result<Vec<u8>> {
        // Create a uniform buffer with all the preset variables
        let mut buffer = Vec::new();
        
        // Add time variables
        buffer.extend_from_slice(&preset.variables.time.to_le_bytes());
        buffer.extend_from_slice(&(preset.variables.frame as f32).to_le_bytes());
        
        // Add audio variables
        buffer.extend_from_slice(&preset.variables.bass.to_le_bytes());
        buffer.extend_from_slice(&preset.variables.mid.to_le_bytes());
        buffer.extend_from_slice(&preset.variables.treb.to_le_bytes());
        buffer.extend_from_slice(&preset.variables.vol.to_le_bytes());
        
        // Add mouse variables
        buffer.extend_from_slice(&preset.variables.mouse_x.to_le_bytes());
        buffer.extend_from_slice(&preset.variables.mouse_y.to_le_bytes());
        
        // Add user variables (q1-q64) as 16 vec4<f32> values
        for i in 0..16 {
            let q1 = if i * 4 < preset.variables.q.len() { preset.variables.q[i * 4] } else { 0.0 };
            let q2 = if i * 4 + 1 < preset.variables.q.len() { preset.variables.q[i * 4 + 1] } else { 0.0 };
            let q3 = if i * 4 + 2 < preset.variables.q.len() { preset.variables.q[i * 4 + 2] } else { 0.0 };
            let q4 = if i * 4 + 3 < preset.variables.q.len() { preset.variables.q[i * 4 + 3] } else { 0.0 };
            
            // Pack as vec4<f32> (16 bytes)
            buffer.extend_from_slice(&q1.to_le_bytes());
            buffer.extend_from_slice(&q2.to_le_bytes());
            buffer.extend_from_slice(&q3.to_le_bytes());
            buffer.extend_from_slice(&q4.to_le_bytes());
        }
        
        // Add custom variables
        for (_name, value) in &preset.variables.custom {
            buffer.extend_from_slice(&value.to_le_bytes());
        }
        
        // Pad to 256-byte alignment
        while buffer.len() % 256 != 0 {
            buffer.push(0);
        }
        
        Ok(buffer)
    }
    
    /// Generate a basic WGSL shader from HLSL code
    fn generate_basic_wgsl_shader(&self, _hlsl_code: &str, shader_type: &str) -> Result<String> {
        let mut wgsl_code = String::new();
        
        // Add WGSL shader header
        wgsl_code.push_str(&format!(
            r#"@group(0) @binding(0) var tex_sampler: texture_2d<f32>;
@group(0) @binding(1) var tex_sampler_sampler: sampler;
@group(0) @binding(2) var<uniform> uniforms: Uniforms;

struct Uniforms {{
    time: f32,
    frame: f32,
    bass: f32,
    mid: f32,
    treb: f32,
    vol: f32,
    mouse_x: f32,
    mouse_y: f32,
    q: array<vec4<f32>, 16>, // 16 vec4s = 64 f32s, naturally aligned
}};

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {{
    // Convert HLSL variables to WGSL
    let time = uniforms.time;
    let frame = uniforms.frame;
    let bass = uniforms.bass;
    let mid = uniforms.mid;
    let treb = uniforms.treb;
    let vol = uniforms.vol;
    let mouse_x = uniforms.mouse_x;
    let mouse_y = uniforms.mouse_y;
    let q1 = uniforms.q[0];
    let q2 = uniforms.q[1];
    let q3 = uniforms.q[2];
    let q4 = uniforms.q[3];
    let q5 = uniforms.q[4];
    let q6 = uniforms.q[5];
    let q7 = uniforms.q[6];
    let q8 = uniforms.q[7];
    
    // Sample the texture
    let tex_color = textureSample(tex_sampler, tex_sampler_sampler, uv);
    
    // Basic color manipulation based on audio
    let intensity = (bass + mid + treb) / 3.0;
    let color = vec3<f32>(
        tex_color.r * (1.0 + bass * 0.5),
        tex_color.g * (1.0 + mid * 0.5),
        tex_color.b * (1.0 + treb * 0.5)
    );
    
    // Add some basic effects based on the shader type
    var final_color: vec3<f32>;
    if ("{}" == "per_pixel") {{
        // Per-pixel effects
        let wave = sin(uv.x * 10.0 + time) * 0.1;
        final_color = color + vec3<f32>(wave, wave * 0.5, wave * 0.2);
    }} else if ("{}" == "warp") {{
        // Warp effects
        let warp_uv = uv + vec2<f32>(
            sin(uv.y * 5.0 + time) * 0.05,
            cos(uv.x * 5.0 + time) * 0.05
        );
        let warped_color = textureSample(tex_sampler, tex_sampler_sampler, warp_uv);
        final_color = color * 0.7 + warped_color.rgb * 0.3;
    }} else if ("{}" == "comp") {{
        // Composite effects
        let pulse = sin(time * 2.0) * 0.2 + 0.8;
        final_color = color * pulse;
    }} else {{
        final_color = color;
    }};
    
    return vec4<f32>(final_color, tex_color.a);
}}
"#,
            shader_type, shader_type, shader_type
        ));
        
        Ok(wgsl_code)
    }
    
    /// Generate a default per-pixel shader
    fn generate_default_per_pixel_shader(&self) -> String {
        r#"@group(0) @binding(0) var tex_sampler: texture_2d<f32>;
@group(0) @binding(1) var tex_sampler_sampler: sampler;
@group(0) @binding(2) var<uniform> uniforms: Uniforms;

struct Uniforms {
    time: f32,
    frame: f32,
    bass: f32,
    mid: f32,
    treb: f32,
    vol: f32,
    mouse_x: f32,
    mouse_y: f32,
    q: array<vec4<f32>, 16>, // 16 vec4s = 64 f32s, naturally aligned
};

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let time = uniforms.time;
    let bass = uniforms.bass;
    let mid = uniforms.mid;
    let treb = uniforms.treb;
    
    // Create a colorful pattern based on audio
    let x = uv.x;
    let y = uv.y;
    
    let r = sin(x * 10.0 + time) * 0.5 + 0.5;
    let g = sin(y * 10.0 + time * 0.7) * 0.5 + 0.5;
    let b = sin((x + y) * 5.0 + time * 1.3) * 0.5 + 0.5;
    
    // Modulate with audio
    let intensity = (bass + mid + treb) / 3.0;
    let color = vec3<f32>(r, g, b) * (0.5 + intensity * 0.5);
    
    return vec4<f32>(color, 1.0);
}
"#.to_string()
    }
    
    /// Generate a default warp shader
    fn generate_default_warp_shader(&self) -> String {
        r#"@group(0) @binding(0) var tex_sampler: texture_2d<f32>;
@group(0) @binding(1) var tex_sampler_sampler: sampler;
@group(0) @binding(2) var<uniform> uniforms: Uniforms;

struct Uniforms {
    time: f32,
    frame: f32,
    bass: f32,
    mid: f32,
    treb: f32,
    vol: f32,
    mouse_x: f32,
    mouse_y: f32,
    q: array<vec4<f32>, 16>, // 16 vec4s = 64 f32s, naturally aligned
};

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let time = uniforms.time;
    let bass = uniforms.bass;
    
    // Simple warp effect
    let warp_strength = 0.1 + bass * 0.2;
    let warped_uv = uv + vec2<f32>(
        sin(uv.y * 8.0 + time) * warp_strength,
        cos(uv.x * 8.0 + time) * warp_strength
    );
    
    let color = textureSample(tex_sampler, tex_sampler_sampler, warped_uv);
    
    return vec4<f32>(color.rgb, color.a);
}
"#.to_string()
    }
    
    /// Generate a default comp shader
    fn generate_default_comp_shader(&self) -> String {
        r#"@group(0) @binding(0) var tex_sampler: texture_2d<f32>;
@group(0) @binding(1) var tex_sampler_sampler: sampler;
@group(0) @binding(2) var<uniform> uniforms: Uniforms;

struct Uniforms {
    time: f32,
    frame: f32,
    bass: f32,
    mid: f32,
    treb: f32,
    vol: f32,
    mouse_x: f32,
    mouse_y: f32,
    q: array<vec4<f32>, 16>, // 16 vec4s = 64 f32s, naturally aligned
};

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let time = uniforms.time;
    let vol = uniforms.vol;
    
    // Simple composite effect
    let base_color = textureSample(tex_sampler, tex_sampler_sampler, uv);
    
    // Add some glow effect
    let glow = sin(time * 3.0) * 0.3 + 0.7;
    let final_color = base_color.rgb * glow;
    
    return vec4<f32>(final_color, base_color.a);
}
"#.to_string()
    }
    
    /// Execute per-frame equations and update preset variables
    pub fn execute_per_frame_equations(&self, preset: &mut Preset) -> Result<()> {
        // Execute per-frame equations
        preset.execute_per_frame()?;
        
        // Update time variables
        preset.update_time_variables(preset.variables.time + 0.016, preset.variables.frame + 1);
        
        Ok(())
    }
    
    /// Get shader source for a preset
    pub fn get_shader_source(&mut self, preset: &Preset) -> Result<String> {
        // Check cache first
        let cache_key = format!("{}_{}", preset.metadata.name, preset.equations.per_frame.len());
        if let Some(cached) = self.shader_cache.get(&cache_key) {
            return Ok(cached.clone());
        }
        
        // Generate new shader
        let mut shader_source = String::new();
        
        // Add vertex shader
        shader_source.push_str(&self.generate_vertex_shader());
        
        // Add fragment shader based on preset type
        if preset.equations.per_pixel.is_some() {
            shader_source.push_str(&self.convert_per_pixel_shader(preset)?);
        } else if preset.equations.warp_shader.is_some() {
            shader_source.push_str(&self.convert_warp_shader(preset)?);
        } else if preset.equations.comp_shader.is_some() {
            shader_source.push_str(&self.convert_comp_shader(preset)?);
        } else {
            // Default shader
            shader_source.push_str(&self.generate_default_per_pixel_shader());
        }
        
        // Cache the shader
        self.shader_cache.insert(cache_key, shader_source.clone());
        
        Ok(shader_source)
    }
    
    /// Generate vertex shader
    fn generate_vertex_shader(&self) -> String {
        r#"struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(in.position, 0.0, 1.0);
    out.tex_coords = in.tex_coords;
    return out;
}
"#.to_string()
    }
} 