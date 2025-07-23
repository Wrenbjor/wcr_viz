use anyhow::Result;
use std::fs;
use regex::Regex;
use crate::preset::{Preset, PresetMetadata, PresetEquations, PresetConfig, WarpConfig, CompositeConfig, MotionConfig, DecayConfig, BlendMode};

/// Parser for MilkDrop .milk preset files
pub struct PresetParser {
    // Regex patterns for parsing different sections
    preset_header_regex: Regex,
    per_frame_init_regex: Regex,
    per_frame_regex: Regex,
    per_vertex_regex: Regex,
    per_pixel_regex: Regex,
    warp_shader_regex: Regex,
    comp_shader_regex: Regex,
    metadata_regex: Regex,
}

impl PresetParser {
    /// Create a new preset parser
    pub fn new() -> Self {
        Self {
            preset_header_regex: Regex::new(r"\[preset(\d+)\]").unwrap(),
            per_frame_init_regex: Regex::new(r"per_frame_init_\d+=(.+)").unwrap(),
            per_frame_regex: Regex::new(r"per_frame_\d+=(.+)").unwrap(),
            per_vertex_regex: Regex::new(r"per_vertex_\d+=(.+)").unwrap(),
            per_pixel_regex: Regex::new(r"per_pixel_\d+=(.+)").unwrap(),
            warp_shader_regex: Regex::new(r"warp_\d+=(.+)").unwrap(),
            comp_shader_regex: Regex::new(r"comp_\d+=(.+)").unwrap(),
            metadata_regex: Regex::new(r"(\w+)=(.+)").unwrap(),
        }
    }
    
    /// Parse a .milk file from disk
    pub fn parse_file(&self, path: &str) -> Result<Preset> {
        let content = fs::read_to_string(path)?;
        self.parse_text(&content)
    }
    
    /// Parse preset content from text
    pub fn parse_text(&self, text: &str) -> Result<Preset> {
        let lines: Vec<&str> = text.lines().collect();
        
        // Extract preset name from header
        let preset_name = self.extract_preset_name(&lines)?;
        
        // Create base preset
        let mut preset = Preset::new(preset_name);
        preset.raw_text = text.to_string();
        
        // Parse metadata
        preset.metadata = self.parse_metadata(&lines)?;
        
        // Parse equations
        preset.equations = self.parse_equations(&lines)?;
        
        // Parse configuration
        preset.config = self.parse_config(&lines)?;
        
        Ok(preset)
    }
    
    /// Extract preset name from header
    fn extract_preset_name(&self, lines: &[&str]) -> Result<String> {
        for line in lines {
            if let Some(captures) = self.preset_header_regex.captures(line) {
                if let Some(preset_num) = captures.get(1) {
                    return Ok(format!("Preset {}", preset_num.as_str()));
                }
            }
        }
        
        // If no preset header found, try to extract from filename or use default
        Ok("Unnamed Preset".to_string())
    }
    
    /// Parse preset metadata
    fn parse_metadata(&self, lines: &[&str]) -> Result<PresetMetadata> {
        let mut metadata = PresetMetadata {
            name: "Unnamed Preset".to_string(),
            author: None,
            rating: None,
            description: None,
            tags: Vec::new(),
        };
        
        for line in lines {
            let line = line.trim();
            
            // Skip empty lines and comments
            if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
                continue;
            }
            
            // Parse metadata fields
            if let Some(captures) = self.metadata_regex.captures(line) {
                let key = captures.get(1).unwrap().as_str().to_lowercase();
                let value = captures.get(2).unwrap().as_str().trim_matches('"');
                
                match key.as_str() {
                    "name" => metadata.name = value.to_string(),
                    "author" => metadata.author = Some(value.to_string()),
                    "rating" => {
                        if let Ok(rating) = value.parse::<u8>() {
                            metadata.rating = Some(rating);
                        }
                    }
                    "description" => metadata.description = Some(value.to_string()),
                    "tags" => {
                        metadata.tags = value.split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                    _ => {}
                }
            }
        }
        
        Ok(metadata)
    }
    
    /// Parse preset equations
    fn parse_equations(&self, lines: &[&str]) -> Result<PresetEquations> {
        let mut equations = PresetEquations {
            per_frame: Vec::new(),
            per_vertex: Vec::new(),
            per_pixel: None,
            warp_shader: None,
            comp_shader: None,
        };
        
        let mut per_frame_init_equations = Vec::new();
        let mut per_frame_equations = Vec::new();
        let mut per_vertex_equations = Vec::new();
        let mut per_pixel_code = Vec::new();
        let mut warp_shader_code = Vec::new();
        let mut comp_shader_code = Vec::new();
        
        let mut in_per_pixel = false;
        let mut in_warp_shader = false;
        let mut in_comp_shader = false;
        
        for line in lines {
            let line = line.trim();
            
            // Skip empty lines and comments
            if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
                continue;
            }
            
            // Check for section markers
            if line.starts_with("[per_pixel]") {
                in_per_pixel = true;
                in_warp_shader = false;
                in_comp_shader = false;
                continue;
            } else if line.starts_with("[warp]") {
                in_per_pixel = false;
                in_warp_shader = true;
                in_comp_shader = false;
                continue;
            } else if line.starts_with("[comp]") {
                in_per_pixel = false;
                in_warp_shader = false;
                in_comp_shader = true;
                continue;
            } else if line.starts_with('[') && line.ends_with(']') {
                // End of current section
                in_per_pixel = false;
                in_warp_shader = false;
                in_comp_shader = false;
                continue;
            }
            
            // Parse per_frame_init equations
            if let Some(captures) = self.per_frame_init_regex.captures(line) {
                if let Some(equation) = captures.get(1) {
                    per_frame_init_equations.push(equation.as_str().to_string());
                }
            }
            
            // Parse per_frame equations
            if let Some(captures) = self.per_frame_regex.captures(line) {
                if let Some(equation) = captures.get(1) {
                    per_frame_equations.push(equation.as_str().to_string());
                }
            }
            
            // Parse per_vertex equations
            if let Some(captures) = self.per_vertex_regex.captures(line) {
                if let Some(equation) = captures.get(1) {
                    per_vertex_equations.push(equation.as_str().to_string());
                }
            }
            
            // Parse per_pixel code
            if in_per_pixel {
                per_pixel_code.push(line.to_string());
            }
            
            // Parse warp shader code
            if in_warp_shader {
                warp_shader_code.push(line.to_string());
            }
            
            // Parse comp shader code
            if in_comp_shader {
                comp_shader_code.push(line.to_string());
            }
        }
        
        // Sort equations by their numeric index
        per_frame_init_equations.sort_by(|a, b| {
            let a_num = self.extract_equation_number(a);
            let b_num = self.extract_equation_number(b);
            a_num.cmp(&b_num)
        });
        
        per_frame_equations.sort_by(|a, b| {
            let a_num = self.extract_equation_number(a);
            let b_num = self.extract_equation_number(b);
            a_num.cmp(&b_num)
        });
        
        per_vertex_equations.sort_by(|a, b| {
            let a_num = self.extract_equation_number(a);
            let b_num = self.extract_equation_number(b);
            a_num.cmp(&b_num)
        });
        
        // Combine per_frame_init and per_frame equations
        equations.per_frame = per_frame_init_equations.into_iter()
            .chain(per_frame_equations.into_iter())
            .collect();
        equations.per_vertex = per_vertex_equations;
        
        if !per_pixel_code.is_empty() {
            equations.per_pixel = Some(per_pixel_code.join("\n"));
        }
        
        if !warp_shader_code.is_empty() {
            equations.warp_shader = Some(warp_shader_code.join("\n"));
        }
        
        if !comp_shader_code.is_empty() {
            equations.comp_shader = Some(comp_shader_code.join("\n"));
        }
        
        Ok(equations)
    }
    
    /// Extract equation number for sorting
    fn extract_equation_number(&self, equation: &str) -> i32 {
        // Look for per_frame_XX or per_vertex_XX pattern
        if let Some(captures) = Regex::new(r"per_(frame|vertex)_(\d+)").unwrap().captures(equation) {
            if let Some(num_str) = captures.get(2) {
                return num_str.as_str().parse::<i32>().unwrap_or(0);
            }
        }
        0
    }
    
    /// Parse preset configuration
    fn parse_config(&self, lines: &[&str]) -> Result<PresetConfig> {
        // Default configuration
        let mut config = PresetConfig {
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
        };
        
        // Parse MilkDrop-specific parameters
        for line in lines {
            let line = line.trim();
            
            // Skip empty lines and comments
            if line.is_empty() || line.starts_with(';') || line.starts_with('#') || line.starts_with("//") {
                continue;
            }
            
            // Parse configuration parameters
            if let Some(captures) = self.metadata_regex.captures(line) {
                let key = captures.get(1).unwrap().as_str().to_lowercase();
                let value = captures.get(2).unwrap().as_str();
                
                match key.as_str() {
                    // Warp parameters
                    "warp" => {
                        if let Ok(scale) = value.parse::<f32>() {
                            config.warp.scale = scale;
                        }
                    }
                    "fwarpscale" => {
                        if let Ok(scale) = value.parse::<f32>() {
                            config.warp.scale = scale;
                        }
                    }
                    "rot" => {
                        if let Ok(rotation) = value.parse::<f32>() {
                            config.warp.rotation = rotation;
                        }
                    }
                    "dx" => {
                        if let Ok(tx) = value.parse::<f32>() {
                            config.warp.translation_x = tx;
                        }
                    }
                    "dy" => {
                        if let Ok(ty) = value.parse::<f32>() {
                            config.warp.translation_y = ty;
                        }
                    }
                    
                    // Composite parameters
                    "fvideoechoalpha" => {
                        if let Ok(opacity) = value.parse::<f32>() {
                            config.composite.opacity = opacity;
                        }
                    }
                    
                    // Decay parameters
                    "fdecay" => {
                        if let Ok(rate) = value.parse::<f32>() {
                            config.decay.decay_rate = rate;
                            config.decay.enabled = rate != 1.0;
                        }
                    }
                    "fgammaadj" => {
                        if let Ok(gamma) = value.parse::<f32>() {
                            config.decay.gamma = gamma;
                        }
                    }
                    
                    // Motion parameters
                    "fwarpanimspeed" => {
                        if let Ok(speed) = value.parse::<f32>() {
                            config.motion.speed = speed;
                            config.motion.enabled = speed != 0.0;
                        }
                    }
                    
                    _ => {}
                }
            }
        }
        
        for line in lines {
            let line = line.trim();
            
            // Skip empty lines and comments
            if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
                continue;
            }
            
            // Parse configuration parameters
            if let Some(captures) = self.metadata_regex.captures(line) {
                let key = captures.get(1).unwrap().as_str().to_lowercase();
                let value = captures.get(2).unwrap().as_str();
                
                match key.as_str() {
                    "warp_scale" => {
                        if let Ok(scale) = value.parse::<f32>() {
                            config.warp.scale = scale;
                        }
                    }
                    "warp_rotation" => {
                        if let Ok(rotation) = value.parse::<f32>() {
                            config.warp.rotation = rotation;
                        }
                    }
                    "warp_translation_x" => {
                        if let Ok(tx) = value.parse::<f32>() {
                            config.warp.translation_x = tx;
                        }
                    }
                    "warp_translation_y" => {
                        if let Ok(ty) = value.parse::<f32>() {
                            config.warp.translation_y = ty;
                        }
                    }
                    "comp_opacity" => {
                        if let Ok(opacity) = value.parse::<f32>() {
                            config.composite.opacity = opacity;
                        }
                    }
                    "comp_blend_mode" => {
                        config.composite.blend_mode = match value.to_lowercase().as_str() {
                            "add" => BlendMode::Add,
                            "subtract" => BlendMode::Subtract,
                            "multiply" => BlendMode::Multiply,
                            "screen" => BlendMode::Screen,
                            "overlay" => BlendMode::Overlay,
                            _ => BlendMode::Normal,
                        };
                    }
                    "motion_speed" => {
                        if let Ok(speed) = value.parse::<f32>() {
                            config.motion.speed = speed;
                            config.motion.enabled = speed != 0.0;
                        }
                    }
                    "motion_direction" => {
                        if let Ok(direction) = value.parse::<f32>() {
                            config.motion.direction = direction;
                        }
                    }
                    "decay_rate" => {
                        if let Ok(rate) = value.parse::<f32>() {
                            config.decay.decay_rate = rate;
                            config.decay.enabled = rate != 1.0;
                        }
                    }
                    "decay_gamma" => {
                        if let Ok(gamma) = value.parse::<f32>() {
                            config.decay.gamma = gamma;
                        }
                    }
                    _ => {}
                }
            }
        }
        
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_simple_preset() {
        let preset_text = r#"
[preset00]
name="Simple Test Preset"
author="Test Author"
rating=4

per_frame_1=q1=q1+0.1
per_frame_2=q2=sin(time)*0.5

[per_pixel]
ret=ret*0.95
"#;
        
        let parser = PresetParser::new();
        let preset = parser.parse_text(preset_text).unwrap();
        
        assert_eq!(preset.metadata.name, "Simple Test Preset");
        assert_eq!(preset.metadata.author, Some("Test Author".to_string()));
        assert_eq!(preset.metadata.rating, Some(4));
        assert_eq!(preset.equations.per_frame.len(), 2);
        assert!(preset.equations.per_pixel.is_some());
    }
    
    #[test]
    fn test_parse_metadata() {
        let preset_text = r#"
[preset00]
name="Test Preset"
author="Test Author"
rating=5
description="A test preset"
tags=test,simple,colorful
"#;
        
        let parser = PresetParser::new();
        let preset = parser.parse_text(preset_text).unwrap();
        
        assert_eq!(preset.metadata.name, "Test Preset");
        assert_eq!(preset.metadata.author, Some("Test Author".to_string()));
        assert_eq!(preset.metadata.rating, Some(5));
        assert_eq!(preset.metadata.description, Some("A test preset".to_string()));
        assert_eq!(preset.metadata.tags, vec!["test", "simple", "colorful"]);
    }
} 