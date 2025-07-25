// src/graphics/renderer.rs

use anyhow::Result;
use std::time::Instant;
use wgpu::{util::DeviceExt, Buffer, Device, Queue, RenderPipeline, SurfaceConfiguration};
use crate::audio::AudioData;
use crate::ui::UIRenderer;
use crate::preset::{Preset, PresetManager, renderer::PresetRenderer};
use swash::{FontRef, zeno};
use swash::scale::ScaleContext;

use std::collections::HashMap;
use std::convert::AsRef;




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

    // Font rendering
    font_data: Vec<u8>,
    font_texture: wgpu::Texture,
    font_texture_view: wgpu::TextureView,
    font_sampler: wgpu::Sampler,
    char_data: HashMap<char, ([f32; 4], f32, f32)>, // UV coords, advance_width, height
    scale_context: ScaleContext,
    
    // UI Projection
    projection_buffer: Buffer,

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
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("UI Pipeline Layout"),
                bind_group_layouts: &[
                    &device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        label: Some("UI Bind Group Layout"),
                        entries: &[
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
                            wgpu::BindGroupLayoutEntry {
                                binding: 1,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 2,
                                visibility: wgpu::ShaderStages::VERTEX,
                                ty: wgpu::BindingType::Buffer {
                                    ty: wgpu::BufferBindingType::Uniform,
                                    has_dynamic_offset: false,
                                    min_binding_size: None,
                                },
                                count: None,
                            },
                        ],
                    }),
                ],
                push_constant_ranges: &[],
            })),
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

        // Create projection uniform buffer (will be updated in render_ui)
        let projection_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Projection Buffer"),
            contents: bytemuck::cast_slice(&[0.0f32; 16]), // Placeholder, will be updated
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Note: Projection bind group is now part of the UI bind group (binding 2)
        // This is handled in the render_ui function

        // Font loading and texture atlas creation - try Arial first, fallback to Inconsolata
        let font_data = std::fs::read("C:/Windows/Fonts/arial.ttf")
            .or_else(|_| std::fs::read("C:/Windows/Fonts/Arial.ttf"))
            .or_else(|_| std::fs::read("assets/fonts/Inconsolata-Regular.ttf"))?;

        // Create Swash scale context
        let mut scale_context = ScaleContext::new();
        
        // Parse font with Swash
        let font_ref = FontRef::from_index(&font_data, 0).ok_or_else(|| anyhow::anyhow!("Failed to parse font"))?;
        
        // Create a font texture atlas using Swash
        let font_size = 32.0; // Adjust font size as needed
        let mut font_atlas_width = 0;
        let mut font_atlas_height = 0;
        let mut char_metrics: HashMap<char, ([f32; 4], f32, f32)> = HashMap::new();

        // Create scaler
        let mut scaler = scale_context
            .builder(font_ref)
            .size(font_size)
            .hint(true)
            .build();

        // First pass to determine atlas size and collect metrics
        let mut max_height = 0;
        let mut glyph_data = Vec::new();
        
        for char_code in 32..127 { // ASCII characters
            let character = std::char::from_u32(char_code).unwrap();
            let glyph_id = font_ref.charmap().map(character);
            
            if glyph_id != 0 { // 0 means no glyph for this character
                let outline = scaler.scale_outline(glyph_id);
                let advance = font_size * 0.6; // Simple advance for now
                
                if let Some(outline) = outline {
                    // Convert outline to bitmap using zeno
                    let bounds = outline.bounds();
                    let width = (bounds.max.x - bounds.min.x).ceil() as usize;
                    let height = (bounds.max.y - bounds.min.y).ceil() as usize;
                    
                    font_atlas_width += width + 1; // +1 for padding
                    max_height = max_height.max(height);
                    
                    glyph_data.push((character, outline, advance, width, height));
                } else {
                    // Fallback for characters without outlines (like spaces)
                    let char_width = advance.max(8.0) as usize;
                    font_atlas_width += char_width + 1; // minimum width for spaces
                    glyph_data.push((character, outline, advance, char_width, 16));
                }
            }
        }

        let font_atlas_width = font_atlas_width.max(1); // Ensure at least 1 pixel
        let font_atlas_height = max_height.max(1); // Use consistent height for all characters
        
        log::info!("Font atlas: {}x{} pixels, {} characters", font_atlas_width, font_atlas_height, glyph_data.len());

        let mut font_atlas_data = vec![0u8; font_atlas_width * font_atlas_height];
        let mut current_x = 0;

        // Second pass to rasterize and copy to atlas using Swash
        for (character, outline_opt, advance, width, height) in glyph_data {
            // Place all characters at the baseline (bottom of atlas)
            let y_offset = font_atlas_height - height;
            
            if let Some(outline) = outline_opt {
                // Render outline to bitmap using zeno
                let bounds = outline.bounds();
                let mut bitmap = vec![0u8; width * height];
                
                // Create a simple rasterizer using zeno
                zeno::Mask::new(&outline)
                    .size(width, height)
                    .offset(-bounds.min.x, -bounds.min.y)
                    .render_into(&mut bitmap, None);
                
                // Copy bitmap to atlas
                for y in 0..height {
                    for x in 0..width {
                        let atlas_idx = (current_x + x) + ((y + y_offset) * font_atlas_width);
                        let bitmap_idx = x + (y * width);
                        if atlas_idx < font_atlas_data.len() && bitmap_idx < bitmap.len() {
                            font_atlas_data[atlas_idx] = bitmap[bitmap_idx];
                        }
                    }
                }
            }

            // Calculate UV coordinates as [x1, y1, x2, y2] (corners, not x/y/w/h)
            let uv_x1 = current_x as f32 / font_atlas_width as f32;
            let uv_y1 = y_offset as f32 / font_atlas_height as f32;
            let uv_x2 = (current_x + width) as f32 / font_atlas_width as f32;
            let uv_y2 = (y_offset + height) as f32 / font_atlas_height as f32;

            char_metrics.insert(character, ([uv_x1, uv_y1, uv_x2, uv_y2], advance, height as f32));
            current_x += width + 1;
        }

        let font_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Font Atlas Texture"),
            size: wgpu::Extent3d {
                width: font_atlas_width as u32,
                height: font_atlas_height as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm, // Single channel for font
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &font_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &font_atlas_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(font_atlas_width as u32),
                rows_per_image: Some(font_atlas_height as u32),
            },
            font_texture.size(),
        );

        let font_texture_view = font_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let font_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Font Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Font loading and texture atlas creation
        let font_bytes = std::fs::read("assets/fonts/Inconsolata-Regular.ttf")?;
        let font = Font::from_bytes(font_bytes, FontSettings::default()).map_err(|e| anyhow::anyhow!("Error loading font: {}", e))?;

        // Create a font texture atlas
        let font_size = 32.0; // Adjust font size as needed
        let mut font_atlas_width = 0;
        let mut font_atlas_height = 0;
        let mut char_data = HashMap::new();

        // First pass to determine atlas size and collect metrics
        for char_code in 32..127 { // ASCII characters
            let character = std::char::from_u32(char_code).unwrap();
            let (metrics, _bitmap) = font.rasterize(character, font_size);
            font_atlas_width += metrics.width + 1; // +1 for padding
            font_atlas_height = font_atlas_height.max(metrics.height);
        }

        let font_atlas_width = font_atlas_width.max(1); // Ensure at least 1 pixel
        let font_atlas_height = font_atlas_height.max(1);

        let mut font_atlas_data = vec![0u8; font_atlas_width * font_atlas_height];
        let mut current_x = 0;

        // Second pass to rasterize and copy to atlas
        for char_code in 32..127 {
            let character = std::char::from_u32(char_code).unwrap();
            let (metrics, bitmap) = font.rasterize(character, font_size);

            for y in 0..metrics.height {
                for x in 0..metrics.width {
                    let atlas_idx = (current_x + x) + (y * font_atlas_width);
                    let bitmap_idx = x + (y * metrics.width);
                    font_atlas_data[atlas_idx] = bitmap[bitmap_idx];
                }
            }

            let uv_x = current_x as f32 / font_atlas_width as f32;
            let uv_y = 0.0;
            let uv_w = metrics.width as f32 / font_atlas_width as f32;
            let uv_h = metrics.height as f32 / font_atlas_height as f32;

            char_data.insert(character, (metrics, [uv_x, uv_y, uv_w, uv_h]));
            current_x += metrics.width + 1;
        }

        let font_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Font Atlas Texture"),
            size: wgpu::Extent3d {
                width: font_atlas_width as u32,
                height: font_atlas_height as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm, // Single channel for font
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &font_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &font_atlas_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(font_atlas_width as u32),
                rows_per_image: Some(font_atlas_height as u32),
            },
            font_texture.size(),
        );

        let font_texture_view = font_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let font_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Font Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Font loading and texture atlas creation
        let font_bytes = std::fs::read("assets/fonts/Inconsolata-Regular.ttf")?;
        let font = Font::from_bytes(font_bytes, FontSettings::default()).map_err(|e| anyhow::anyhow!("Error loading font: {}", e))?;

        // Create a font texture atlas
        let font_size = 32.0; // Adjust font size as needed
        let mut font_atlas_width = 0;
        let mut font_atlas_height = 0;
        let mut char_data = HashMap::new();

        // First pass to determine atlas size and collect metrics
        for char_code in 32..127 { // ASCII characters
            let character = std::char::from_u32(char_code).unwrap();
            let (metrics, _bitmap) = font.rasterize(character, font_size);
            font_atlas_width += metrics.width + 1; // +1 for padding
            font_atlas_height = font_atlas_height.max(metrics.height);
        }

        let font_atlas_width = font_atlas_width.max(1); // Ensure at least 1 pixel
        let font_atlas_height = font_atlas_height.max(1);

        let mut font_atlas_data = vec![0u8; font_atlas_width * font_atlas_height];
        let mut current_x = 0;

        // Second pass to rasterize and copy to atlas
        for char_code in 32..127 {
            let character = std::char::from_u32(char_code).unwrap();
            let (metrics, bitmap) = font.rasterize(character, font_size);

            for y in 0..metrics.height {
                for x in 0..metrics.width {
                    let atlas_idx = (current_x + x) + (y * font_atlas_width);
                    let bitmap_idx = x + (y * metrics.width);
                    font_atlas_data[atlas_idx] = bitmap[bitmap_idx];
                }
            }

            let uv_x = current_x as f32 / font_atlas_width as f32;
            let uv_y = 0.0;
            let uv_w = metrics.width as f32 / font_atlas_width as f32;
            let uv_h = metrics.height as f32 / font_atlas_height as f32;

            char_data.insert(character, (metrics, [uv_x, uv_y, uv_w, uv_h]));
            current_x += metrics.width + 1;
        }

        let font_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Font Atlas Texture"),
            size: wgpu::Extent3d {
                width: font_atlas_width as u32,
                height: font_atlas_height as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm, // Single channel for font
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &font_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &font_atlas_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(font_atlas_width as u32),
                rows_per_image: Some(font_atlas_height as u32),
            },
            font_texture.size(),
        );

        let font_texture_view = font_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let font_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Font Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
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
        
            window_width: config.width,
            window_height: config.height,
            max_vertices,
            max_indices,
            current_vertex_count: 0,
            current_index_count: 0,
            max_ui_vertices,
            max_ui_indices,
            font_data,
            font_texture,
            font_texture_view,
            font_sampler,
            char_data,
            scale_context,
            projection_buffer,
            preset_renderer: PresetRenderer::new(),
            preset_manager: None,
            uniform_buffer: Some(uniform_buffer),
            texture: Some(texture),
            texture_view: Some(texture_view),
            sampler: Some(sampler),
        })
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

        self.render_with_encoder(view, &mut encoder)?;

        self.queue.submit(std::iter::once(encoder.finish()));
        
        Ok(())
    }

    pub fn render_with_encoder(&mut self, view: &wgpu::TextureView, encoder: &mut wgpu::CommandEncoder) -> Result<()> {
        // Check if we have a preset to render and clone it to avoid borrow conflicts
        let preset_to_render = if let Some(ref preset_manager) = self.preset_manager {
            preset_manager.current_preset().cloned()
        } else {
            None
        };

        // Render based on what we have
        if let Some(preset) = preset_to_render {
            // Render preset instead of waveform
            self.render_preset(encoder, view, &preset)?;
        } else {
            // Fall back to waveform rendering
            self.render_waveform(encoder, view)?;
        }
        
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
                    // Use screen coordinates directly
                    let x1 = element.x;
                    let y1 = element.y;
                    let x2 = element.x + element.width;
                    let y2 = element.y + element.height;
                    
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
                        start_idx as u32, (start_idx + 1) as u32, (start_idx + 2) as u32,
                        (start_idx + 1) as u32, (start_idx + 3) as u32, (start_idx + 2) as u32
                    ]);
                }
                UIElementType::Text => {
                    if let Some(text) = &element.text {
                        let mut x_offset = 0.0;
                        
                        for ch in text.chars() {
                            if let Some((uv_rect, advance_width, char_height)) = self.char_data.get(&ch) {
                                let char_x = element.x + x_offset;
                                let char_y = element.y;
                                
                                let x1 = char_x;
                                let y1 = char_y;
                                let x2 = char_x + advance_width;
                                let y2 = char_y + char_height;
                                
                                let start_idx = ui_vertices.len() as u16;
                                ui_vertices.extend_from_slice(&[
                                    Vertex { position: [x1, y1], color: element.color, tex_coords: [uv_rect[0], uv_rect[1]] },
                                    Vertex { position: [x2, y1], color: element.color, tex_coords: [uv_rect[2], uv_rect[1]] },
                                    Vertex { position: [x1, y2], color: element.color, tex_coords: [uv_rect[0], uv_rect[3]] },
                                    Vertex { position: [x2, y2], color: element.color, tex_coords: [uv_rect[2], uv_rect[3]] },
                                ]);
                                
                                ui_indices.extend_from_slice(&[
                                    start_idx as u32, (start_idx + 1) as u32, (start_idx + 2) as u32,
                                    (start_idx + 1) as u32, (start_idx + 3) as u32, (start_idx + 2) as u32
                                ]);
                                
                                x_offset += advance_width;
                            }
                        }
                    }
                }
                UIElementType::Line { x2, y2 } => {
                    // Use screen coordinates directly
                    let x1_norm = element.x;
                    let y1_norm = element.y;
                    let x2_norm = *x2;
                    let y2_norm = *y2;
                    
                    let start_idx = ui_vertices.len();
                    ui_vertices.extend_from_slice(&[
                        Vertex { position: [x1_norm, y1_norm], color: element.color, tex_coords: [0.0, 0.0] },
                        Vertex { position: [x2_norm, y2_norm], color: element.color, tex_coords: [1.0, 1.0] },
                    ]);
                    
                    ui_indices.extend_from_slice(&[start_idx as u32, (start_idx + 1) as u32]);
                }
            }
        }

        // Update buffers with UI data
        if !ui_vertices.is_empty() {
            log::info!("Rendering UI: {} vertices, {} indices", ui_vertices.len(), ui_indices.len());
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
            
            // Update projection matrix for screen coordinates with correct Y-axis
            let projection_matrix = cgmath::ortho(0.0, self.window_width as f32, 0.0, self.window_height as f32, -1.0, 1.0);
            self.queue.write_buffer(&self.projection_buffer, 0, bytemuck::cast_slice(<cgmath::Matrix4<f32> as AsRef<[f32; 16]>>::as_ref(&projection_matrix)));

            // Create UI bind group with all three bindings (texture, sampler, projection)
            let ui_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("UI Bind Group"),
                layout: &self.ui_pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&self.font_texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.font_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.projection_buffer.as_entire_binding(),
                    },
                ],
            });
            render_pass.set_bind_group(0, &ui_bind_group, &[]);

            render_pass.draw_indexed(0..ui_indices.len() as u32, 0, 0..1);
        }
        
        Ok(())
    }

    /// Resize the renderer
    pub fn resize(&mut self, width: u32, height: u32) {
        self.window_width = width;
        self.window_height = height;
    }
    
    /// Render overlay text on top of the scene
    pub fn render_overlay_text(&mut self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView, overlay_text: &[String]) -> Result<()> {
        if overlay_text.is_empty() {
            return Ok(());
        }

        // Calculate text positions (simple layout from top-left)
        let start_x = 20.0; // 20px from left edge
        let start_y = 20.0; // 20px from top edge
        let line_height = 30.0; // 30px between lines to accommodate larger font
        let font_size = 32.0; // Much larger font size for readability

        // Create UI vertices from overlay text - IMPORTANT: use u16 to match buffer type
        let mut ui_vertices = Vec::new();
        let mut ui_indices: Vec<u16> = Vec::new();
        
        let mut current_y = start_y;
        
        // Render each line of text using proper font rendering
        for line in overlay_text {
            if !line.is_empty() {
                self.add_text_to_buffers(&mut ui_vertices, &mut ui_indices, start_x, current_y, line, font_size, [1.0, 1.0, 1.0, 0.9]); // White text with slight transparency
            }
            current_y += line_height;
        }

        // Upload UI vertex and index data and render
        if !ui_vertices.is_empty() {
            log::debug!("Rendering overlay text: {} vertices, {} indices", ui_vertices.len(), ui_indices.len());

            // Upload vertex and index data
            self.queue.write_buffer(&self.ui_vertex_buffer, 0, bytemuck::cast_slice(&ui_vertices));
            self.queue.write_buffer(&self.ui_index_buffer, 0, bytemuck::cast_slice(&ui_indices));

            // Create projection matrix for UI coordinates
            let projection_matrix = [
                [2.0 / self.window_width as f32, 0.0, 0.0, 0.0],
                [0.0, -2.0 / self.window_height as f32, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [-1.0, 1.0, 0.0, 1.0],
            ];

            // Update the projection buffer
            self.queue.write_buffer(
                &self.projection_buffer,
                0,
                bytemuck::cast_slice(&projection_matrix),
            );

            // Create bind group for UI rendering
            let ui_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("UI Bind Group"),
                layout: &self.ui_pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&self.font_texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.font_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.projection_buffer.as_entire_binding(),
                    },
                ],
            });

            // Start render pass for UI
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Overlay Text Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Don't clear, draw over existing content
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.ui_pipeline);
            render_pass.set_bind_group(0, &ui_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.ui_vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.ui_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..ui_indices.len() as u32, 0, 0..1);
        }

        Ok(())
    }

    /// Add text characters to the UI rendering buffers
    fn add_text_to_buffers(&self, ui_vertices: &mut Vec<Vertex>, ui_indices: &mut Vec<u16>, x: f32, y: f32, text: &str, font_size: f32, color: [f32; 4]) {
        let mut current_x = x;
        
        for character in text.chars() {
            if let Some((uv_coords, advance_width, height)) = self.char_data.get(&character) {
                let scale_factor = font_size / 32.0; // Atlas font size is 32.0
                let char_width = advance_width * scale_factor;
                let char_height = height * scale_factor;
                
                // Create quad for this character
                let base_index = ui_vertices.len() as u16;
                
                // Add vertices for character quad
                ui_vertices.extend_from_slice(&[
                    Vertex { position: [current_x, y], tex_coords: [uv_coords[0], uv_coords[1]], color },
                    Vertex { position: [current_x + char_width, y], tex_coords: [uv_coords[2], uv_coords[1]], color },
                    Vertex { position: [current_x + char_width, y + char_height], tex_coords: [uv_coords[2], uv_coords[3]], color },
                    Vertex { position: [current_x, y + char_height], tex_coords: [uv_coords[0], uv_coords[3]], color },
                ]);
                
                // Add indices for character quad (two triangles)
                ui_indices.extend_from_slice(&[
                    base_index, base_index + 1, base_index + 2,
                    base_index, base_index + 2, base_index + 3,
                ]);
                
                current_x += char_width;
            } else if character == ' ' {
                // Handle spaces
                current_x += font_size * 0.25;
            } else {
                // Handle unknown characters (space them appropriately)
                current_x += font_size * 0.5;
            }
        }
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
            width: 0.0, // Width will be calculated per character
            height: 0.0, // Height will be calculated per character
            color,
            text: Some(text.to_string()),
        });
        
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
        
        Ok(())
    }

    fn get_window_dimensions(&self) -> (u32, u32) {
        (self.window_width, self.window_height)
    }
}