// src/graphics/renderer.rs

use anyhow::Result;
use std::time::Instant;
use wgpu::{util::DeviceExt, Buffer, Device, Queue, RenderPipeline, SurfaceConfiguration};
use crate::audio::AudioData;
use crate::ui::UIRenderer;
use crate::preset::{Preset, PresetManager, renderer::PresetRenderer};
use fontdue::{Font, FontSettings};


/// Vertex structure for waveform lines
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
    tex_coords: [f32; 2], // Add texture coordinates for preset rendering
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 6]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

/// UI element for rendering
#[derive(Debug, Clone)]
struct UIElement {
    element_type: UIElementType,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: [f32; 4],
    text: Option<String>,
}

#[derive(Debug, Clone)]
enum UIElementType {
    Rectangle,
    Text,
    Line { x2: f32, y2: f32 },
}

/// Waveform line data
#[derive(Debug, Clone)]
struct WaveformLine {
    frequency_band: String,
    color: [f32; 4],
    values: Vec<f32>,
    max_points: usize,
}

impl WaveformLine {
    fn new(frequency_band: &str, color: [f32; 4], max_points: usize) -> Self {
        Self {
            frequency_band: frequency_band.to_string(),
            color,
            values: vec![0.0; max_points],
            max_points,
        }
    }
    
    fn update(&mut self, new_value: f32) {
        // Shift all values to the left
        self.values.rotate_left(1);
        // Set the last value to the new value
        if let Some(last) = self.values.last_mut() {
            *last = new_value;
        }
    }
    
    fn create_vertices(&self, _width: f32, _height: f32, audio_data: Option<&AudioData>) -> Vec<Vertex> {
        let mut vertices = Vec::new();
        let vertical_scale = 2.0; // Much larger amplitude swing
        let wave_thickness = 0.03; // Slightly thicker for better visibility
    
        // Use .map() to directly access the waveform field from the Option
        if let Some(audio) = audio_data.map(|data| &data.waveform) {
            let num_samples = audio.len();
            if num_samples == 0 {
                // Create a smooth straight line when no samples
                for i in 0..100 {
                    let x = (i as f32 / 99.0) * 2.0 - 1.0;
                    for ribbon_layer in 0..7 {
                        let layer_offset = (ribbon_layer as f32 - 3.0) * (wave_thickness / 6.0);
                        let y = layer_offset;
                        let alpha = 1.0 - (ribbon_layer as f32 * 0.15).abs();
                        let alpha = alpha.max(0.2);
                        vertices.push(Vertex {
                            position: [x, y],
                            color: [0.5, 0.5, 0.5, alpha], // Gray color for no signal
                            tex_coords: [0.0, 0.0], // No texture coordinates for waveform
                        });
                    }
                }
                return vertices;
            }
    
            for i in 0..num_samples {
                let x = (i as f32 / (num_samples - 1) as f32) * 2.0 - 1.0;
                let y_base = audio[i] * vertical_scale;
    
                // Create color gradient based on position and amplitude
                let t = i as f32 / num_samples as f32;
                let amplitude_factor = audio[i].abs();
                
                // Color gradient from blue to red based on position
                let r = t;
                let g = 0.3 + amplitude_factor * 0.4;
                let b = 1.0 - t;
                
                // Create ribbon effect with multiple layers
                for ribbon_layer in 0..7 {
                    let layer_offset = (ribbon_layer as f32 - 3.0) * (wave_thickness / 6.0);
                    let y = y_base + layer_offset;
                    
                    // Alpha decreases for outer layers
                    let alpha = 1.0 - (ribbon_layer as f32 * 0.15).abs();
                    let alpha = alpha.max(0.2);
                    
                    vertices.push(Vertex {
                        position: [x, y],
                        color: [r, g, b, alpha],
                        tex_coords: [0.0, 0.0], // No texture coordinates for waveform
                    });
                }
            }
        }
        
        vertices
    }
}

pub struct Renderer {
    device: Device,
    queue: Queue,
    pipeline: RenderPipeline,
    ui_pipeline: RenderPipeline, // Separate pipeline for UI triangles
    preset_pipeline: RenderPipeline, // Pipeline for preset rendering
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    // Dedicated UI buffers to avoid conflicts
    ui_vertex_buffer: Buffer,
    ui_index_buffer: Buffer,
    current_audio_data: Option<AudioData>,
    start_time: Instant,
    
    // Waveform lines for different frequency bands
    waveform_lines: Vec<WaveformLine>,
    
    // UI elements to render
    ui_elements: Vec<UIElement>,
    
    // Font system
    font: Option<Font>,
    
    // Configuration
    window_width: u32,
    window_height: u32,
    
    // Buffer management
    max_vertices: usize,
    max_indices: usize,
    current_vertex_count: usize,
    current_index_count: usize,
    // UI buffer management
    max_ui_vertices: usize,
    max_ui_indices: usize,
    
    // Preset rendering
    preset_renderer: PresetRenderer,
    preset_manager: Option<PresetManager>,
    uniform_buffer: Option<Buffer>,
    texture: Option<wgpu::Texture>,
    texture_view: Option<wgpu::TextureView>,
    sampler: Option<wgpu::Sampler>,
}

impl Renderer {
    pub fn new(device: Device, queue: Queue, config: &SurfaceConfiguration) -> Result<Self> {
        // Create shader modules
        let shader_source = include_str!("shaders/shader.wgsl");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Basic Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        // Create render pipeline for waveform lines (line strip)
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Waveform Pipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineStrip,
                strip_index_format: Some(wgpu::IndexFormat::Uint16),
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        // Create dedicated UI shader
        let ui_shader_source = include_str!("shaders/ui_shader.wgsl");
        let ui_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("UI Shader"),
            source: wgpu::ShaderSource::Wgsl(ui_shader_source.into()),
        });

        // Create render pipeline for UI elements (triangles) with dedicated shader
        let ui_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("UI Pipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &ui_shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &ui_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        // Create preset shader
        let preset_shader_source = r#"
@group(0) @binding(0) var tex_sampler: texture_2d<f32>;
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

struct VertexInput {
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

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let time = uniforms.time;
    let bass = uniforms.bass;
    let mid = uniforms.mid;
    let treb = uniforms.treb;
    let vol = uniforms.vol;
    
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
"#;

        let preset_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Preset Shader"),
            source: wgpu::ShaderSource::Wgsl(preset_shader_source.into()),
        });

        // Create render pipeline for preset rendering
        let preset_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Preset Pipeline"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Preset Pipeline Layout"),
                bind_group_layouts: &[&device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Preset Bind Group Layout"),
                    entries: &[
                        // Texture
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        // Sampler
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                        // Uniforms
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                })],
                push_constant_ranges: &[],
            })),
            vertex: wgpu::VertexState {
                module: &preset_shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &preset_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        // Create reusable buffers with sufficient capacity
        let max_vertices = 200000; // Increased for UI text rendering
        let max_indices = 200000;
        
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice::<Vertex, u8>(&vec![Vertex { position: [0.0, 0.0], color: [0.0, 0.0, 0.0, 0.0], tex_coords: [0.0, 0.0] }; max_vertices]),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice::<u16, u8>(&vec![0u16; max_indices]),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });

        // Create dedicated UI buffers
        let max_ui_vertices = 50000; // Dedicated capacity for UI elements
        let max_ui_indices = 50000;
        
        let ui_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("UI Vertex Buffer"),
            contents: bytemuck::cast_slice::<Vertex, u8>(&vec![Vertex { position: [0.0, 0.0], color: [0.0, 0.0, 0.0, 0.0], tex_coords: [0.0, 0.0] }; max_ui_vertices]),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let ui_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("UI Index Buffer"),
            contents: bytemuck::cast_slice::<u16, u8>(&vec![0u16; max_ui_indices]),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });

        // Initialize waveform lines
        let waveform_lines = vec![
            WaveformLine::new("Low", [1.0, 0.0, 0.0, 1.0], 1000),    // Red for bass
            WaveformLine::new("Mid", [0.0, 1.0, 0.0, 1.0], 1000),    // Green for mid
            WaveformLine::new("High", [0.0, 0.0, 1.0, 1.0], 1000),   // Blue for treble
        ];

        // Try to load Arial font
        let font = Self::load_arial_font();

        // Create a simple texture for preset rendering
        let texture_size: u32 = 256;
        let _texture_data = vec![255u8; (texture_size * texture_size * 4) as usize]; // RGBA texture
        
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Preset Texture"),
            size: wgpu::Extent3d {
                width: texture_size,
                height: texture_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Preset Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create uniform buffer
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: &vec![0u8; 288], // 288 bytes for uniforms (8 f32s + 16 vec4s)
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        Ok(Self {
            device,
            queue,
            pipeline,
            ui_pipeline,
            preset_pipeline,
            vertex_buffer,
            index_buffer,
            ui_vertex_buffer,
            ui_index_buffer,
            current_audio_data: None,
            start_time: Instant::now(),
            waveform_lines,
            ui_elements: Vec::new(),
            font,
            window_width: config.width,
            window_height: config.height,
            max_vertices,
            max_indices,
            current_vertex_count: 0,
            current_index_count: 0,
            max_ui_vertices,
            max_ui_indices,
            preset_renderer: PresetRenderer::new(),
            preset_manager: None,
            uniform_buffer: Some(uniform_buffer),
            texture: Some(texture),
            texture_view: Some(texture_view),
            sampler: Some(sampler),
        })
    }

    /// Load Arial font from Windows system fonts
    fn load_arial_font() -> Option<Font> {
        // Common Windows font paths
        let font_paths = [
            "C:\\Windows\\Fonts\\arial.ttf",
            "C:\\Windows\\Fonts\\Arial.ttf",
            "C:\\Windows\\Fonts\\ARIAL.TTF",
        ];

        for path in &font_paths {
            if let Ok(font_data) = std::fs::read(path) {
                if let Ok(font) = Font::from_bytes(font_data, FontSettings::default()) {
                    log::info!("✅ Loaded Arial font from: {}", path);
                    return Some(font);
                }
            }
        }

        // Fallback: create a simple bitmap font
        log::warn!("⚠️ Could not load Arial font, using fallback");
        None
    }

    /// Get bitmap pattern for a character (8x12 pixel font)
    fn get_char_pattern(ch: char) -> [u8; 12] {
        match ch {
            'A' => [0x18, 0x24, 0x42, 0x42, 0x7E, 0x42, 0x42, 0x42, 0x42, 0x42, 0x00, 0x00],
            'B' => [0x7C, 0x42, 0x42, 0x42, 0x7C, 0x42, 0x42, 0x42, 0x42, 0x7C, 0x00, 0x00],
            'C' => [0x3C, 0x42, 0x42, 0x40, 0x40, 0x40, 0x40, 0x42, 0x42, 0x3C, 0x00, 0x00],
            'D' => [0x78, 0x44, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x44, 0x78, 0x00, 0x00],
            'E' => [0x7E, 0x40, 0x40, 0x40, 0x7C, 0x40, 0x40, 0x40, 0x40, 0x7E, 0x00, 0x00],
            'F' => [0x7E, 0x40, 0x40, 0x40, 0x7C, 0x40, 0x40, 0x40, 0x40, 0x40, 0x00, 0x00],
            'G' => [0x3C, 0x42, 0x42, 0x40, 0x40, 0x4E, 0x42, 0x42, 0x42, 0x3C, 0x00, 0x00],
            'H' => [0x42, 0x42, 0x42, 0x42, 0x7E, 0x42, 0x42, 0x42, 0x42, 0x42, 0x00, 0x00],
            'I' => [0x3E, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x3E, 0x00, 0x00],
            'J' => [0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x42, 0x42, 0x3C, 0x00, 0x00],
            'K' => [0x42, 0x44, 0x48, 0x50, 0x60, 0x50, 0x48, 0x44, 0x42, 0x42, 0x00, 0x00],
            'L' => [0x40, 0x40, 0x40, 0x40, 0x40, 0x40, 0x40, 0x40, 0x40, 0x7E, 0x00, 0x00],
            'M' => [0x42, 0x66, 0x5A, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x00, 0x00],
            'N' => [0x42, 0x62, 0x52, 0x4A, 0x46, 0x42, 0x42, 0x42, 0x42, 0x42, 0x00, 0x00],
            'O' => [0x3C, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x3C, 0x00, 0x00],
            'P' => [0x7C, 0x42, 0x42, 0x42, 0x7C, 0x40, 0x40, 0x40, 0x40, 0x40, 0x00, 0x00],
            'Q' => [0x3C, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x4A, 0x44, 0x3A, 0x00, 0x00],
            'R' => [0x7C, 0x42, 0x42, 0x42, 0x7C, 0x48, 0x44, 0x42, 0x42, 0x42, 0x00, 0x00],
            'S' => [0x3C, 0x42, 0x42, 0x40, 0x30, 0x0C, 0x02, 0x42, 0x42, 0x3C, 0x00, 0x00],
            'T' => [0x7F, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x00, 0x00],
            'U' => [0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x3C, 0x00, 0x00],
            'V' => [0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x24, 0x18, 0x00, 0x00, 0x00],
            'W' => [0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x5A, 0x66, 0x42, 0x00, 0x00],
            'X' => [0x42, 0x42, 0x24, 0x18, 0x18, 0x18, 0x24, 0x42, 0x42, 0x42, 0x00, 0x00],
            'Y' => [0x42, 0x42, 0x24, 0x18, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x00, 0x00],
            'Z' => [0x7E, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x42, 0x42, 0x7E, 0x00, 0x00],
            'a' => [0x00, 0x00, 0x3C, 0x02, 0x3E, 0x42, 0x42, 0x42, 0x3E, 0x00, 0x00, 0x00],
            'b' => [0x40, 0x40, 0x7C, 0x42, 0x42, 0x42, 0x42, 0x42, 0x7C, 0x00, 0x00, 0x00],
            'c' => [0x00, 0x00, 0x3C, 0x42, 0x40, 0x40, 0x40, 0x42, 0x3C, 0x00, 0x00, 0x00],
            'd' => [0x02, 0x02, 0x3E, 0x42, 0x42, 0x42, 0x42, 0x42, 0x3E, 0x00, 0x00, 0x00],
            'e' => [0x00, 0x00, 0x3C, 0x42, 0x42, 0x7E, 0x40, 0x42, 0x3C, 0x00, 0x00, 0x00],
            'f' => [0x0C, 0x12, 0x10, 0x7C, 0x10, 0x10, 0x10, 0x10, 0x10, 0x00, 0x00, 0x00],
            'g' => [0x00, 0x00, 0x3E, 0x42, 0x42, 0x42, 0x3E, 0x02, 0x42, 0x3C, 0x00, 0x00],
            'h' => [0x40, 0x40, 0x7C, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x00, 0x00, 0x00],
            'i' => [0x08, 0x00, 0x38, 0x08, 0x08, 0x08, 0x08, 0x08, 0x3E, 0x00, 0x00, 0x00],
            'j' => [0x04, 0x00, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x04, 0x44, 0x38, 0x00, 0x00],
            'k' => [0x40, 0x40, 0x42, 0x44, 0x48, 0x70, 0x48, 0x44, 0x42, 0x00, 0x00, 0x00],
            'l' => [0x38, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x3E, 0x00, 0x00, 0x00],
            'm' => [0x00, 0x00, 0x76, 0x49, 0x49, 0x49, 0x49, 0x49, 0x49, 0x00, 0x00, 0x00],
            'n' => [0x00, 0x00, 0x7C, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x00, 0x00, 0x00],
            'o' => [0x00, 0x00, 0x3C, 0x42, 0x42, 0x42, 0x42, 0x42, 0x3C, 0x00, 0x00, 0x00],
            'p' => [0x00, 0x00, 0x7C, 0x42, 0x42, 0x42, 0x7C, 0x40, 0x40, 0x40, 0x00, 0x00],
            'q' => [0x00, 0x00, 0x3E, 0x42, 0x42, 0x42, 0x3E, 0x02, 0x02, 0x02, 0x00, 0x00],
            'r' => [0x00, 0x00, 0x7C, 0x42, 0x40, 0x40, 0x40, 0x40, 0x40, 0x00, 0x00, 0x00],
            's' => [0x00, 0x00, 0x3E, 0x40, 0x3C, 0x02, 0x02, 0x42, 0x3C, 0x00, 0x00, 0x00],
            't' => [0x10, 0x10, 0x7C, 0x10, 0x10, 0x10, 0x10, 0x12, 0x0C, 0x00, 0x00, 0x00],
            'u' => [0x00, 0x00, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x3E, 0x00, 0x00, 0x00],
            'v' => [0x00, 0x00, 0x42, 0x42, 0x42, 0x42, 0x24, 0x18, 0x00, 0x00, 0x00, 0x00],
            'w' => [0x00, 0x00, 0x49, 0x49, 0x49, 0x49, 0x49, 0x49, 0x36, 0x00, 0x00, 0x00],
            'x' => [0x00, 0x00, 0x42, 0x24, 0x18, 0x18, 0x24, 0x42, 0x42, 0x00, 0x00, 0x00],
            'y' => [0x00, 0x00, 0x42, 0x42, 0x42, 0x42, 0x3E, 0x02, 0x42, 0x3C, 0x00, 0x00],
            'z' => [0x00, 0x00, 0x7E, 0x04, 0x08, 0x10, 0x20, 0x40, 0x7E, 0x00, 0x00, 0x00],
            '0' => [0x3C, 0x42, 0x42, 0x46, 0x4A, 0x52, 0x62, 0x42, 0x42, 0x3C, 0x00, 0x00],
            '1' => [0x08, 0x18, 0x28, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x3E, 0x00, 0x00],
            '2' => [0x3C, 0x42, 0x42, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x7E, 0x00, 0x00],
            '3' => [0x3C, 0x42, 0x42, 0x02, 0x0C, 0x02, 0x02, 0x42, 0x42, 0x3C, 0x00, 0x00],
            '4' => [0x04, 0x0C, 0x14, 0x24, 0x44, 0x7E, 0x04, 0x04, 0x04, 0x04, 0x00, 0x00],
            '5' => [0x7E, 0x40, 0x40, 0x7C, 0x02, 0x02, 0x02, 0x42, 0x42, 0x3C, 0x00, 0x00],
            '6' => [0x3C, 0x42, 0x40, 0x40, 0x7C, 0x42, 0x42, 0x42, 0x42, 0x3C, 0x00, 0x00],
            '7' => [0x7E, 0x02, 0x04, 0x08, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x00, 0x00],
            '8' => [0x3C, 0x42, 0x42, 0x42, 0x3C, 0x42, 0x42, 0x42, 0x42, 0x3C, 0x00, 0x00],
            '9' => [0x3C, 0x42, 0x42, 0x42, 0x3E, 0x02, 0x02, 0x42, 0x42, 0x3C, 0x00, 0x00],
            '!' => [0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00, 0x00, 0x18, 0x18, 0x00, 0x00],
            '?' => [0x3C, 0x42, 0x42, 0x02, 0x04, 0x08, 0x08, 0x00, 0x08, 0x08, 0x00, 0x00],
            '.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00, 0x00],
            ',' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x30, 0x00, 0x00],
            ':' => [0x00, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00],
            ';' => [0x00, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00, 0x18, 0x18, 0x30, 0x00, 0x00],
            '-' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x7E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            '_' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x7E, 0x00, 0x00],
            '=' => [0x00, 0x00, 0x00, 0x7E, 0x00, 0x00, 0x7E, 0x00, 0x00, 0x00, 0x00, 0x00],
            '+' => [0x00, 0x00, 0x08, 0x08, 0x08, 0x7F, 0x08, 0x08, 0x08, 0x00, 0x00, 0x00],
            '*' => [0x00, 0x00, 0x08, 0x2A, 0x1C, 0x7F, 0x1C, 0x2A, 0x08, 0x00, 0x00, 0x00],
            '/' => [0x00, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x00, 0x00, 0x00, 0x00],
            '\\' => [0x00, 0x80, 0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x00, 0x00, 0x00, 0x00],
            '(' => [0x0C, 0x10, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x10, 0x0C, 0x00, 0x00],
            ')' => [0x30, 0x08, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x08, 0x30, 0x00, 0x00],
            '[' => [0x3E, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x3E, 0x00, 0x00],
            ']' => [0x7C, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x7C, 0x00, 0x00],
            '{' => [0x0E, 0x10, 0x10, 0x10, 0x60, 0x10, 0x10, 0x10, 0x10, 0x0E, 0x00, 0x00],
            '}' => [0x70, 0x08, 0x08, 0x08, 0x06, 0x08, 0x08, 0x08, 0x08, 0x70, 0x00, 0x00],
            '<' => [0x00, 0x06, 0x18, 0x60, 0x80, 0x60, 0x18, 0x06, 0x00, 0x00, 0x00, 0x00],
            '>' => [0x00, 0xC0, 0x30, 0x0C, 0x02, 0x0C, 0x30, 0xC0, 0x00, 0x00, 0x00, 0x00],
            '|' => [0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x00, 0x00],
            '&' => [0x3C, 0x42, 0x42, 0x42, 0x3C, 0x42, 0x42, 0x42, 0x3C, 0x02, 0x00, 0x00],
            '@' => [0x3C, 0x42, 0x42, 0x4E, 0x52, 0x4E, 0x40, 0x42, 0x3C, 0x00, 0x00, 0x00],
            '#' => [0x12, 0x12, 0x12, 0x7F, 0x12, 0x12, 0x7F, 0x12, 0x12, 0x12, 0x00, 0x00],
            '$' => [0x08, 0x3E, 0x49, 0x48, 0x3E, 0x09, 0x09, 0x49, 0x3E, 0x08, 0x00, 0x00],
            '%' => [0x61, 0x92, 0x94, 0x68, 0x08, 0x10, 0x16, 0x29, 0x49, 0x86, 0x00, 0x00],
            '^' => [0x18, 0x24, 0x42, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            '~' => [0x00, 0x00, 0x00, 0x32, 0x4C, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            '`' => [0x20, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            '\'' => [0x10, 0x10, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            '"' => [0x24, 0x24, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            _ => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // Unknown character
        }
    }

    /// Set the preset manager for rendering presets
    pub fn set_preset_manager(&mut self, preset_manager: PresetManager) {
        self.preset_manager = Some(preset_manager);
    }

    /// Update preset with audio data
    pub fn update_preset_audio(&mut self, audio_data: &AudioData) -> Result<()> {
        if let Some(ref mut preset_manager) = self.preset_manager {
            if let Some(ref mut preset) = preset_manager.current_preset_mut() {
                // Update audio variables
                preset.update_audio_variables(
                    audio_data.features.bass,
                    audio_data.features.mid,
                    audio_data.features.presence,
                    audio_data.features.volume
                );
                
                // Execute per-frame equations
                self.preset_renderer.execute_per_frame_equations(preset)?;
                
                // Update uniform buffer
                if let Some(ref uniform_buffer) = self.uniform_buffer {
                    let uniform_data = self.preset_renderer.generate_uniform_buffer(preset)?;
                    self.queue.write_buffer(uniform_buffer, 0, &uniform_data);
                }
            }
        }
        
        Ok(())
    }

    pub fn update_audio_data(&mut self, audio_data: &AudioData) -> Result<()> {
        self.current_audio_data = Some(audio_data.clone());
        
        // Update waveform lines with frequency data
        let features = &audio_data.features;
        self.waveform_lines[0].update(features.bass);      // Low frequency
        self.waveform_lines[1].update(features.mid);       // Mid frequency  
        self.waveform_lines[2].update(features.presence);  // High frequency
        
        // Update preset with audio data
        self.update_preset_audio(audio_data)?;
        
        Ok(())
    }

    pub fn render(&mut self, view: &wgpu::TextureView) -> Result<()> {
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // Check if we have a preset to render and clone it to avoid borrow conflicts
        let preset_to_render = if let Some(ref preset_manager) = self.preset_manager {
            preset_manager.current_preset().cloned()
        } else {
            None
        };

        // Render based on what we have
        if let Some(preset) = preset_to_render {
            // Render preset instead of waveform
            self.render_preset(&mut encoder, view, &preset)?;
        } else {
            // Fall back to waveform rendering
            self.render_waveform(&mut encoder, view)?;
        }

        // Render UI overlay
        self.render_ui(&mut encoder, view)?;

        self.queue.submit(std::iter::once(encoder.finish()));
        
        // Clear UI elements for next frame and pre-allocate capacity
        self.ui_elements.clear();
        self.ui_elements.reserve(100); // Pre-allocate space for typical UI element count
        
        Ok(())
    }

    /// Render a preset
    fn render_preset(&mut self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView, _preset: &Preset) -> Result<()> {
        // Create bind group for preset rendering
        if let (Some(texture_view), Some(sampler), Some(uniform_buffer)) = 
            (&self.texture_view, &self.sampler, &self.uniform_buffer) {
            
            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Preset Bind Group"),
                layout: &self.preset_pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: uniform_buffer.as_entire_binding(),
                    },
                ],
            });

            // Create full-screen quad vertices
            let vertices = vec![
                Vertex { position: [-1.0, -1.0], color: [1.0, 1.0, 1.0, 1.0], tex_coords: [0.0, 0.0] },
                Vertex { position: [ 1.0, -1.0], color: [1.0, 1.0, 1.0, 1.0], tex_coords: [1.0, 0.0] },
                Vertex { position: [-1.0,  1.0], color: [1.0, 1.0, 1.0, 1.0], tex_coords: [0.0, 1.0] },
                Vertex { position: [ 1.0,  1.0], color: [1.0, 1.0, 1.0, 1.0], tex_coords: [1.0, 1.0] },
            ];
            
            let indices = vec![0, 1, 2, 1, 3, 2];

            // Update vertex and index buffers
            self.queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
            self.queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&indices));

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Preset Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.preset_pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
        }
        
        Ok(())
    }

    /// Render waveform (fallback)
    fn render_waveform(&mut self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) -> Result<()> {
        // Create vertices for all waveform lines
        let mut all_vertices = Vec::new();
        let mut all_indices = Vec::new();

        for line in &self.waveform_lines {
            let vertices = line.create_vertices(self.window_width as f32, self.window_height as f32, self.current_audio_data.as_ref());
            let vertices_len = vertices.len();
            let start_index = all_vertices.len();
            
            all_vertices.extend(vertices);
            
            // For LineStrip, we just need sequential indices
            for i in 0..vertices_len {
                all_indices.push((start_index + i) as u16);
            }
        }

        // Update buffer contents
        self.queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&all_vertices));
        self.queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&all_indices));
        
        // Update current counts
        self.current_vertex_count = all_vertices.len();
        self.current_index_count = all_indices.len();

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Waveform Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        // Render waveform with line strip pipeline
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        
        // Draw waveform
        if !all_indices.is_empty() {
            render_pass.draw_indexed(0..all_indices.len() as u32, 0, 0..1);
        }
        
        Ok(())
    }

    /// Render UI overlay
    fn render_ui(&mut self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) -> Result<()> {
        // Only render UI if there are elements to render
        if self.ui_elements.is_empty() {
            return Ok(());
        }

        // Create UI vertices from collected UI elements
        let mut ui_vertices = Vec::new();
        let mut ui_indices = Vec::new();
        
        for element in &self.ui_elements {
            match &element.element_type {
                UIElementType::Rectangle => {
                    // Convert screen coordinates to normalized device coordinates
                    let screen_width = self.window_width as f32;
                    let screen_height = self.window_height as f32;
                    
                    let x1 = (element.x / screen_width) * 2.0 - 1.0;
                    let y1 = 1.0 - (element.y / screen_height) * 2.0; // Flip Y coordinate
                    let x2 = ((element.x + element.width) / screen_width) * 2.0 - 1.0;
                    let y2 = 1.0 - ((element.y + element.height) / screen_height) * 2.0;
                    
                    // Create rectangle vertices (two triangles)
                    let start_idx = ui_vertices.len();
                    ui_vertices.extend_from_slice(&[
                        Vertex { position: [x1, y1], color: element.color, tex_coords: [0.0, 0.0] }, // Top-left
                        Vertex { position: [x2, y1], color: element.color, tex_coords: [1.0, 0.0] }, // Top-right
                        Vertex { position: [x1, y2], color: element.color, tex_coords: [0.0, 1.0] }, // Bottom-left
                        Vertex { position: [x2, y2], color: element.color, tex_coords: [1.0, 1.0] }, // Bottom-right
                    ]);
                    
                    // Create indices for two triangles
                    ui_indices.extend_from_slice(&[
                        start_idx, start_idx + 1, start_idx + 2,
                        start_idx + 1, start_idx + 3, start_idx + 2
                    ]);
                }
                UIElementType::Text => {
                    // Render text using a simple bitmap font approach for readability
                    if let Some(text) = &element.text {
                        let screen_width = self.window_width as f32;
                        let screen_height = self.window_height as f32;
                        
                        // Simple bitmap font - each character is 8x12 pixels
                        let char_width = 8.0;
                        let char_height = 12.0;
                        let char_spacing = 2.0;
                        
                        let mut x_offset = 0.0;
                        
                        for ch in text.chars() {
                            if ch == ' ' {
                                x_offset += char_width * 0.5;
                                continue;
                            }
                            
                            let char_x = element.x + x_offset;
                            let char_y = element.y;
                            
                            // Render each character as a single quad (much more efficient)
                            let x1 = (char_x / screen_width) * 2.0 - 1.0;
                            let y1 = 1.0 - (char_y / screen_height) * 2.0;
                            let x2 = ((char_x + char_width) / screen_width) * 2.0 - 1.0;
                            let y2 = 1.0 - ((char_y + char_height) / screen_height) * 2.0;
                            
                            let start_idx = ui_vertices.len();
                            ui_vertices.extend_from_slice(&[
                                Vertex { position: [x1, y1], color: element.color, tex_coords: [0.0, 0.0] },
                                Vertex { position: [x2, y1], color: element.color, tex_coords: [1.0, 0.0] },
                                Vertex { position: [x1, y2], color: element.color, tex_coords: [0.0, 1.0] },
                                Vertex { position: [x2, y2], color: element.color, tex_coords: [1.0, 1.0] },
                            ]);
                            
                            ui_indices.extend_from_slice(&[
                                start_idx, start_idx + 1, start_idx + 2,
                                start_idx + 1, start_idx + 3, start_idx + 2
                            ]);
                            
                            x_offset += char_width + char_spacing;
                        }
                    }
                }
                UIElementType::Line { x2, y2 } => {
                    // Convert to line strip vertices
                    let screen_width = self.window_width as f32;
                    let screen_height = self.window_height as f32;
                    
                    let x1_norm = (element.x / screen_width) * 2.0 - 1.0;
                    let y1_norm = 1.0 - (element.y / screen_height) * 2.0;
                    let x2_norm = (*x2 / screen_width) * 2.0 - 1.0;
                    let y2_norm = 1.0 - (*y2 / screen_height) * 2.0;
                    
                    let start_idx = ui_vertices.len();
                    ui_vertices.extend_from_slice(&[
                        Vertex { position: [x1_norm, y1_norm], color: element.color, tex_coords: [0.0, 0.0] },
                        Vertex { position: [x2_norm, y2_norm], color: element.color, tex_coords: [1.0, 1.0] },
                    ]);
                    
                    ui_indices.extend_from_slice(&[start_idx, start_idx + 1]);
                }
            }
        }

        // Update buffers with UI data
        if !ui_vertices.is_empty() {
            // Check if we need to resize UI buffers
            if ui_vertices.len() > self.max_ui_vertices {
                log::warn!("UI vertex count ({}) exceeds buffer capacity ({}), truncating", 
                          ui_vertices.len(), self.max_ui_vertices);
                ui_vertices.truncate(self.max_ui_vertices);
            }
            if ui_indices.len() > self.max_ui_indices {
                log::warn!("UI index count ({}) exceeds buffer capacity ({}), truncating", 
                          ui_indices.len(), self.max_ui_indices);
                ui_indices.truncate(self.max_ui_indices);
            }
            
            // Use dedicated UI buffers
            self.queue.write_buffer(&self.ui_vertex_buffer, 0, bytemuck::cast_slice(&ui_vertices));
            self.queue.write_buffer(&self.ui_index_buffer, 0, bytemuck::cast_slice(&ui_indices));
            
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("UI Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            // Render UI with dedicated UI pipeline and buffers
            render_pass.set_pipeline(&self.ui_pipeline);
            render_pass.set_vertex_buffer(0, self.ui_vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.ui_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..ui_indices.len() as u32, 0, 0..1);
        }
        
        Ok(())
    }

    /// Resize the renderer
    pub fn resize(&mut self, width: u32, height: u32) {
        self.window_width = width;
        self.window_height = height;
    }
    
    /// Get memory usage statistics
    pub fn get_memory_stats(&self) -> (usize, usize, usize, usize) {
        (
            self.current_vertex_count,
            self.max_vertices,
            self.current_index_count,
            self.max_indices
        )
    }
    
    /// Resize buffers if needed (called when buffer overflow is detected)
    fn resize_buffers_if_needed(&mut self, required_vertices: usize, required_indices: usize) {
        let new_max_vertices = (required_vertices * 2).max(self.max_vertices);
        let new_max_indices = (required_indices * 2).max(self.max_indices);
        
        if new_max_vertices > self.max_vertices || new_max_indices > self.max_indices {
            log::info!("🔄 Resizing buffers: Vertices {}->{}, Indices {}->{}", 
                      self.max_vertices, new_max_vertices, self.max_indices, new_max_indices);
            
            // Create new larger buffers
            self.vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice::<Vertex, u8>(&vec![Vertex { position: [0.0, 0.0], color: [0.0, 0.0, 0.0, 0.0], tex_coords: [0.0, 0.0] }; new_max_vertices]),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });

            self.index_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice::<u16, u8>(&vec![0u16; new_max_indices]),
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            });
            
            self.max_vertices = new_max_vertices;
            self.max_indices = new_max_indices;
        }
    }
}

// Implement UIRenderer trait for Renderer
impl UIRenderer for Renderer {
    fn draw_text(&mut self, x: f32, y: f32, text: &str, color: [f32; 4]) -> Result<()> {
        // Store text element for rendering
        self.ui_elements.push(UIElement {
            element_type: UIElementType::Text,
            x,
            y,
            width: text.len() as f32 * 20.0, // Better approximate width for larger font
            height: 32.0, // Font height
            color,
            text: Some(text.to_string()),
        });
        
        log::info!("🎨 UI Text: '{}' at ({}, {}) with color {:?}", text, x, y, color);
        Ok(())
    }
    
    fn draw_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) -> Result<()> {
        // Store rectangle element for rendering
        self.ui_elements.push(UIElement {
            element_type: UIElementType::Rectangle,
            x,
            y,
            width,
            height,
            color,
            text: None,
        });
        
        log::info!("🎨 UI Rect: ({}, {}) {}x{} with color {:?} - Created 4 vertices", 
                  x, y, width, height, color);
        Ok(())
    }
    
    fn draw_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, color: [f32; 4]) -> Result<()> {
        // Store line element for rendering
        self.ui_elements.push(UIElement {
            element_type: UIElementType::Line { x2, y2 },
            x: x1,
            y: y1,
            width: 0.0,
            height: 0.0,
            color,
            text: None,
        });
        
        log::info!("🎨 UI Line: ({}, {}) to ({}, {}) with color {:?}", x1, y1, x2, y2, color);
        Ok(())
    }
}