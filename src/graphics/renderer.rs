// src/graphics/renderer.rs

use anyhow::Result;
use std::time::Instant;
use wgpu::{util::DeviceExt, Buffer, Device, Queue, RenderPipeline, SurfaceConfiguration};
use crate::audio::AudioData;

/// Vertex structure for waveform lines
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
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
            ],
        }
    }
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
    
    // src/graphics/renderer.rs

    fn create_vertices(&self, _width: f32, _height: f32, audio_data: Option<&AudioData>) -> Vec<Vertex> {
        let mut vertices = Vec::new();
        let vertical_scale = 2.0; // Much larger amplitude swing
        let wave_thickness = 0.03; // Slightly thicker for better visibility
    
        // --- FIX IS ON THIS LINE ---
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
                let amplitude_factor = (audio[i] * 2.0 + 1.0).clamp(0.0, 1.0);
                
                // Multi-color gradient: red -> orange -> yellow -> green -> blue -> purple
                let color = if t < 0.16 {
                    // Red to orange
                    let local_t = t / 0.16;
                    [1.0, local_t * 0.5, 0.0, 1.0]
                } else if t < 0.33 {
                    // Orange to yellow
                    let local_t = (t - 0.16) / 0.17;
                    [1.0, 0.5 + local_t * 0.5, local_t * 1.0, 1.0]
                } else if t < 0.5 {
                    // Yellow to green
                    let local_t = (t - 0.33) / 0.17;
                    [1.0 - local_t * 1.0, 1.0, local_t * 1.0, 1.0]
                } else if t < 0.67 {
                    // Green to blue
                    let local_t = (t - 0.5) / 0.17;
                    [0.0, 1.0 - local_t * 1.0, 1.0, 1.0]
                } else if t < 0.83 {
                    // Blue to purple
                    let local_t = (t - 0.67) / 0.16;
                    [local_t * 0.5, 0.0, 1.0, 1.0]
                } else {
                    // Purple to red
                    let local_t = (t - 0.83) / 0.17;
                    [0.5 + local_t * 0.5, 0.0, 1.0 - local_t * 1.0, 1.0]
                };
                
                // Add amplitude-based brightness
                let brightness = 0.3 + amplitude_factor * 0.7;
                let final_color = [
                    color[0] * brightness,
                    color[1] * brightness,
                    color[2] * brightness,
                    color[3]
                ];
    
                for ribbon_layer in 0..7 { // More layers for thicker ribbon
                    let layer_offset = (ribbon_layer as f32 - 3.0) * (wave_thickness / 6.0);
                    let y = y_base + layer_offset;
                     
                    let alpha = 1.0 - (ribbon_layer as f32 * 0.15).abs();
                    let alpha = alpha.max(0.2); // Higher minimum visibility
                    
                    vertices.push(Vertex {
                        position: [x, y],
                        color: [final_color[0], final_color[1], final_color[2], final_color[3] * alpha],
                    });
                }
            }
        } else {
            // Create a smooth straight line when no audio data
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
                    });
                }
            }
        }
        
        vertices
    }

}

/// Renderer for audio visualizations
pub struct Renderer {
    device: Device,
    queue: Queue,
    pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    current_audio_data: Option<AudioData>,
    start_time: Instant,
    
    // Waveform lines for different frequency bands
    waveform_lines: Vec<WaveformLine>,
    
    // Configuration
    window_width: u32,
    window_height: u32,
}

impl Renderer {
    /// Create a new renderer
    pub fn new(device: Device, queue: Queue, config: &SurfaceConfiguration) -> Result<Self> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/shader.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
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
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineStrip,
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

        // Create initial vertex and index buffers (will be updated dynamically)
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice::<Vertex, u8>(&[]),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice::<u16, u8>(&[]),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });

        // Create a single elegant sine wave
        let waveform_lines = vec![
            WaveformLine::new("sine_wave", [1.0, 1.0, 1.0, 1.0], 200), // Single white wave
        ];

        // Initialize with a default value to ensure vertices are generated
        let mut renderer = Self {
            device,
            queue,
            pipeline,
            vertex_buffer,
            index_buffer,
            current_audio_data: None,
            start_time: Instant::now(),
            waveform_lines,
            window_width: config.width,
            window_height: config.height,
        };
        
        // Add a default value to ensure vertices are generated
        renderer.waveform_lines[0].update(0.1);
        
        Ok(renderer)
    }

    /// Update with new audio data
    pub fn update_audio_data(&mut self, audio_data: &AudioData) -> Result<()> {
        self.current_audio_data = Some(audio_data.clone());

        if let Some(ref audio) = self.current_audio_data {
            // Update the single sine wave with combined audio data
            let features = &audio.features;
            
            // Combine all frequency bands into a single value for the sine wave
            let combined_audio = (features.sub_bass + features.bass + features.low_mid + 
                                features.mid + features.high_mid + features.presence + 
                                features.brilliance) / 7.0;
            
            // Apply much more aggressive scaling for maximum sensitivity
            let scaled_value = (combined_audio * 10.0).min(1.0); // 3x more sensitive than before
            self.waveform_lines[0].update(scaled_value);
        }

        Ok(())
    }

    /// Render the current frame
    pub fn render(&mut self, view: &wgpu::TextureView) -> Result<()> {
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

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

        // Update vertex buffer
        self.vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&all_vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        // Update index buffer
        self.index_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&all_indices),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
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

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..all_indices.len() as u32, 0, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }

    /// Resize the renderer
    pub fn resize(&mut self, width: u32, height: u32) {
        self.window_width = width;
        self.window_height = height;
    }
}