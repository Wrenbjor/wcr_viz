use std::fs;
use std::path::Path;

fn main() {
    println!("Testing Real MilkDrop Preset Parsing...");
    
    // Test with a simple preset first
    let simple_preset_path = "presets/cream-of-the-crop/Geometric/Circles Nested/amandio c - embrace 01.milk";
    
    if Path::new(simple_preset_path).exists() {
        println!("✓ Found test preset: {}", simple_preset_path);
        
        match fs::read_to_string(simple_preset_path) {
            Ok(content) => {
                println!("✓ Successfully read preset file ({} lines)", content.lines().count());
                
                // Parse the preset
                match parse_preset(&content) {
                    Ok(preset) => {
                        println!("✓ Successfully parsed preset!");
                        println!("  - Name: {}", preset.name);
                        println!("  - Rating: {}", preset.rating);
                        println!("  - Per-frame equations: {}", preset.per_frame_equations.len());
                        println!("  - Per-vertex equations: {}", preset.per_vertex_equations.len());
                        println!("  - Has per-pixel shader: {}", preset.per_pixel_shader.is_some());
                        println!("  - Has warp shader: {}", preset.warp_shader.is_some());
                        println!("  - Has comp shader: {}", preset.comp_shader.is_some());
                        
                        // Test some specific parameters
                        println!("  - Warp scale: {}", preset.warp_scale);
                        println!("  - Decay rate: {}", preset.decay_rate);
                        println!("  - Gamma adjustment: {}", preset.gamma_adj);
                    }
                    Err(e) => {
                        println!("✗ Failed to parse preset: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("✗ Failed to read preset file: {}", e);
            }
        }
    } else {
        println!("✗ Test preset not found: {}", simple_preset_path);
    }
    
    // Test with a more complex preset
    let complex_preset_path = "presets/cream-of-the-crop/Waveform/Spectrum/suksma - spectro exp 777.milk";
    
    if Path::new(complex_preset_path).exists() {
        println!("\nTesting complex preset: {}", complex_preset_path);
        
        match fs::read_to_string(complex_preset_path) {
            Ok(content) => {
                println!("✓ Successfully read complex preset file ({} lines)", content.lines().count());
                
                match parse_preset(&content) {
                    Ok(preset) => {
                        println!("✓ Successfully parsed complex preset!");
                        println!("  - Per-frame equations: {}", preset.per_frame_equations.len());
                        println!("  - Per-vertex equations: {}", preset.per_vertex_equations.len());
                        
                        // Show some example equations
                        if !preset.per_frame_equations.is_empty() {
                            println!("  - Example per-frame equation: {}", preset.per_frame_equations[0]);
                        }
                    }
                    Err(e) => {
                        println!("✗ Failed to parse complex preset: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("✗ Failed to read complex preset file: {}", e);
            }
        }
    }
    
    println!("\n🎉 Real preset parsing test completed!");
}

// Simplified preset structure for testing
#[derive(Debug)]
struct Preset {
    name: String,
    rating: f32,
    per_frame_equations: Vec<String>,
    per_vertex_equations: Vec<String>,
    per_pixel_shader: Option<String>,
    warp_shader: Option<String>,
    comp_shader: Option<String>,
    warp_scale: f32,
    decay_rate: f32,
    gamma_adj: f32,
}

fn parse_preset(content: &str) -> Result<Preset, String> {
    let lines: Vec<&str> = content.lines().collect();
    
    let mut preset = Preset {
        name: "Unknown".to_string(),
        rating: 0.0,
        per_frame_equations: Vec::new(),
        per_vertex_equations: Vec::new(),
        per_pixel_shader: None,
        warp_shader: None,
        comp_shader: None,
        warp_scale: 1.0,
        decay_rate: 1.0,
        gamma_adj: 1.0,
    };
    
    let mut in_per_pixel = false;
    let mut in_warp = false;
    let mut in_comp = false;
    let mut per_pixel_code = Vec::new();
    let mut warp_code = Vec::new();
    let mut comp_code = Vec::new();
    
    for line in lines {
        let line = line.trim();
        
        // Skip empty lines and comments
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        
        // Parse preset header
        if line.starts_with("[preset") {
            continue;
        }
        
        // Parse metadata
        if let Some((key, value)) = parse_key_value(line) {
            match key.to_lowercase().as_str() {
                "frating" => {
                    if let Ok(rating) = value.parse::<f32>() {
                        preset.rating = rating;
                    }
                }
                "warp" => {
                    if let Ok(scale) = value.parse::<f32>() {
                        preset.warp_scale = scale;
                    }
                }
                "fdecay" => {
                    if let Ok(rate) = value.parse::<f32>() {
                        preset.decay_rate = rate;
                    }
                }
                "fgammaadj" => {
                    if let Ok(gamma) = value.parse::<f32>() {
                        preset.gamma_adj = gamma;
                    }
                }
                _ => {}
            }
        }
        
        // Parse equations
        if line.starts_with("per_frame_init_") || line.starts_with("per_frame_") {
            if let Some((_, equation)) = parse_key_value(line) {
                preset.per_frame_equations.push(equation.to_string());
            }
        }
        
        if line.starts_with("per_vertex_") {
            if let Some((_, equation)) = parse_key_value(line) {
                preset.per_vertex_equations.push(equation.to_string());
            }
        }
        
        // Parse shader sections
        if line == "[per_pixel]" {
            in_per_pixel = true;
            in_warp = false;
            in_comp = false;
            continue;
        } else if line == "[warp]" {
            in_per_pixel = false;
            in_warp = true;
            in_comp = false;
            continue;
        } else if line == "[comp]" {
            in_per_pixel = false;
            in_warp = false;
            in_comp = true;
            continue;
        } else if line.starts_with('[') && line.ends_with(']') {
            in_per_pixel = false;
            in_warp = false;
            in_comp = false;
            continue;
        }
        
        // Collect shader code
        if in_per_pixel {
            per_pixel_code.push(line.to_string());
        } else if in_warp {
            warp_code.push(line.to_string());
        } else if in_comp {
            comp_code.push(line.to_string());
        }
    }
    
    // Set shader code
    if !per_pixel_code.is_empty() {
        preset.per_pixel_shader = Some(per_pixel_code.join("\n"));
    }
    if !warp_code.is_empty() {
        preset.warp_shader = Some(warp_code.join("\n"));
    }
    if !comp_code.is_empty() {
        preset.comp_shader = Some(comp_code.join("\n"));
    }
    
    Ok(preset)
}

fn parse_key_value(line: &str) -> Option<(&str, &str)> {
    if let Some(equal_pos) = line.find('=') {
        let key = &line[..equal_pos];
        let value = &line[equal_pos + 1..];
        Some((key, value))
    } else {
        None
    }
} 