use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod parser;
pub mod evaluator;
pub mod renderer;

#[cfg(test)]
mod test;

use parser::PresetParser;
use evaluator::ExpressionEvaluator;

/// MilkDrop preset metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetMetadata {
    /// Preset name
    pub name: String,
    
    /// Preset author
    pub author: Option<String>,
    
    /// Preset rating (1-5)
    pub rating: Option<u8>,
    
    /// Preset description
    pub description: Option<String>,
    
    /// Preset tags
    pub tags: Vec<String>,
}

/// MilkDrop preset variables (q1-q64)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetVariables {
    /// User variables q1-q64
    pub q: Vec<f32>,
    
    /// Audio variables
    pub bass: f32,
    pub mid: f32,
    pub treb: f32,
    pub vol: f32,
    
    /// Time variables
    pub time: f32,
    pub frame: u32,
    
    /// Mouse variables
    pub mouse_x: f32,
    pub mouse_y: f32,
    
    /// Custom variables
    pub custom: HashMap<String, f32>,
}

impl Default for PresetVariables {
    fn default() -> Self {
        Self {
            q: vec![0.0; 64],
            bass: 0.0,
            mid: 0.0,
            treb: 0.0,
            vol: 0.0,
            time: 0.0,
            frame: 0,
            mouse_x: 0.0,
            mouse_y: 0.0,
            custom: HashMap::new(),
        }
    }
}

/// MilkDrop preset equations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetEquations {
    /// Per-frame equations (executed once per frame)
    pub per_frame: Vec<String>,
    
    /// Per-vertex equations (executed for each vertex)
    pub per_vertex: Vec<String>,
    
    /// Per-pixel equations (shader code)
    pub per_pixel: Option<String>,
    
    /// Warp shader code
    pub warp_shader: Option<String>,
    
    /// Composite shader code
    pub comp_shader: Option<String>,
}

/// MilkDrop preset configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetConfig {
    /// Warp settings
    pub warp: WarpConfig,
    
    /// Composite settings
    pub composite: CompositeConfig,
    
    /// Motion settings
    pub motion: MotionConfig,
    
    /// Decay settings
    pub decay: DecayConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarpConfig {
    pub enabled: bool,
    pub scale: f32,
    pub rotation: f32,
    pub translation_x: f32,
    pub translation_y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeConfig {
    pub enabled: bool,
    pub blend_mode: BlendMode,
    pub opacity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionConfig {
    pub enabled: bool,
    pub speed: f32,
    pub direction: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecayConfig {
    pub enabled: bool,
    pub decay_rate: f32,
    pub gamma: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlendMode {
    Normal,
    Add,
    Subtract,
    Multiply,
    Screen,
    Overlay,
}

/// Complete MilkDrop preset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    /// Preset metadata
    pub metadata: PresetMetadata,
    
    /// Preset configuration
    pub config: PresetConfig,
    
    /// Preset equations
    pub equations: PresetEquations,
    
    /// Preset variables (current state)
    pub variables: PresetVariables,
    
    /// Raw preset text
    pub raw_text: String,
}

impl Preset {
    /// Create a new empty preset
    pub fn new(name: String) -> Self {
        Self {
            metadata: PresetMetadata {
                name,
                author: None,
                rating: None,
                description: None,
                tags: Vec::new(),
            },
            config: PresetConfig {
                warp: WarpConfig {
                    enabled: true,
                    scale: 1.0,
                    rotation: 0.0,
                    translation_x: 0.0,
                    translation_y: 0.0,
                },
                composite: CompositeConfig {
                    enabled: true,
                    blend_mode: BlendMode::Normal,
                    opacity: 1.0,
                },
                motion: MotionConfig {
                    enabled: false,
                    speed: 1.0,
                    direction: 0.0,
                },
                decay: DecayConfig {
                    enabled: false,
                    decay_rate: 0.95,
                    gamma: 1.0,
                },
            },
            equations: PresetEquations {
                per_frame: Vec::new(),
                per_vertex: Vec::new(),
                per_pixel: None,
                warp_shader: None,
                comp_shader: None,
            },
            variables: PresetVariables::default(),
            raw_text: String::new(),
        }
    }
    
    /// Load a preset from a .milk file
    pub fn from_file(path: &str) -> Result<Self> {
        let parser = PresetParser::new();
        parser.parse_file(path)
    }
    
    /// Load a preset from text content
    pub fn from_text(text: &str) -> Result<Self> {
        let parser = PresetParser::new();
        parser.parse_text(text)
    }
    
    /// Update preset variables with audio data
    pub fn update_audio_variables(&mut self, bass: f32, mid: f32, treb: f32, vol: f32) {
        self.variables.bass = bass;
        self.variables.mid = mid;
        self.variables.treb = treb;
        self.variables.vol = vol;
    }
    
    /// Update time variables
    pub fn update_time_variables(&mut self, time: f32, frame: u32) {
        self.variables.time = time;
        self.variables.frame = frame;
    }
    
    /// Update mouse variables
    pub fn update_mouse_variables(&mut self, x: f32, y: f32) {
        self.variables.mouse_x = x;
        self.variables.mouse_y = y;
    }
    
    /// Execute per-frame equations
    pub fn execute_per_frame(&mut self) -> Result<()> {
        let mut evaluator = ExpressionEvaluator::new(&self.variables);
        
        for equation in &self.equations.per_frame {
            evaluator.evaluate(equation)?;
        }
        
        // Update our variables with the evaluator's variables
        self.variables = evaluator.get_variables().clone();
        
        Ok(())
    }
    
    /// Get a user variable (q1-q64)
    pub fn get_q(&self, index: usize) -> f32 {
        if index < 64 && index < self.variables.q.len() {
            self.variables.q[index]
        } else {
            0.0
        }
    }
    
    /// Set a user variable (q1-q64)
    pub fn set_q(&mut self, index: usize, value: f32) {
        if index < 64 {
            if index >= self.variables.q.len() {
                self.variables.q.resize(64, 0.0);
            }
            self.variables.q[index] = value;
        }
    }
    
    /// Get a custom variable
    pub fn get_custom(&self, name: &str) -> Option<f32> {
        self.variables.custom.get(name).copied()
    }
    
    /// Set a custom variable
    pub fn set_custom(&mut self, name: String, value: f32) {
        self.variables.custom.insert(name, value);
    }
}

/// Preset manager for handling multiple presets
#[derive(Clone)]
pub struct PresetManager {
    presets: Vec<Preset>,
    pub current_preset_index: usize,
    transition_time: f32,
    is_transitioning: bool,
}

impl PresetManager {
    /// Create a new preset manager
    pub fn new() -> Self {
        Self {
            presets: Vec::new(),
            current_preset_index: 0,
            transition_time: 0.0,
            is_transitioning: false,
        }
    }
    
    /// Load presets from a directory
    pub fn load_presets_from_directory(&mut self, path: &str) -> Result<()> {
        let entries = std::fs::read_dir(path)?;
        
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("milk") {
                match Preset::from_file(path.to_str().unwrap()) {
                    Ok(preset) => {
                        self.presets.push(preset);
                        log::info!("Loaded preset: {}", path.display());
                    }
                    Err(e) => {
                        log::warn!("Failed to load preset {}: {}", path.display(), e);
                    }
                }
            }
        }
        
        log::info!("Loaded {} presets", self.presets.len());
        Ok(())
    }
    
    /// Get the current preset
    pub fn current_preset(&self) -> Option<&Preset> {
        self.presets.get(self.current_preset_index)
    }
    
    /// Get the current preset mutably
    pub fn current_preset_mut(&mut self) -> Option<&mut Preset> {
        self.presets.get_mut(self.current_preset_index)
    }
    
    /// Get all presets
    pub fn get_presets(&self) -> &Vec<Preset> {
        &self.presets
    }
    
    /// Get all presets mutably
    pub fn get_presets_mut(&mut self) -> &mut Vec<Preset> {
        &mut self.presets
    }
    
    /// Add a preset
    pub fn add_preset(&mut self, preset: Preset) {
        self.presets.push(preset);
    }
    
    /// Switch to the next preset
    pub fn next_preset(&mut self) {
        if !self.presets.is_empty() {
            self.current_preset_index = (self.current_preset_index + 1) % self.presets.len();
            self.start_transition();
        }
    }
    
    /// Switch to the previous preset
    pub fn prev_preset(&mut self) {
        if !self.presets.is_empty() {
            self.current_preset_index = if self.current_preset_index == 0 {
                self.presets.len() - 1
            } else {
                self.current_preset_index - 1
            };
            self.start_transition();
        }
    }
    
    /// Switch to a specific preset by index
    pub fn switch_to_preset(&mut self, index: usize) {
        if index < self.presets.len() {
            self.current_preset_index = index;
            self.start_transition();
        }
    }
    
    /// Start a preset transition
    fn start_transition(&mut self) {
        self.is_transitioning = true;
        self.transition_time = 0.0;
    }
    
    /// Update transition state
    pub fn update_transition(&mut self, delta_time: f32) {
        if self.is_transitioning {
            self.transition_time += delta_time;
            
            // Transition duration of 2 seconds
            if self.transition_time >= 2.0 {
                self.is_transitioning = false;
            }
        }
    }
    
    /// Get transition progress (0.0 to 1.0)
    pub fn transition_progress(&self) -> f32 {
        if self.is_transitioning {
            (self.transition_time / 2.0).min(1.0)
        } else {
            1.0
        }
    }
    
    /// Get the number of loaded presets
    pub fn preset_count(&self) -> usize {
        self.presets.len()
    }
    
    /// Get preset by index
    pub fn get_preset(&self, index: usize) -> Option<&Preset> {
        self.presets.get(index)
    }
} 