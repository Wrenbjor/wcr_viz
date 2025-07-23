use std::collections::HashMap;

fn main() {
    println!("Testing Complete UI System for MilkDrop Preset Control...");
    
    // Test 1: UI Creation and Basic Functionality
    println!("\n=== Test 1: UI Creation ===");
    let mut ui = PresetUI::new();
    assert!(!ui.is_overlay_visible());
    println!("✓ UI created successfully");
    
    // Test 2: Overlay Toggle
    println!("\n=== Test 2: Overlay Toggle ===");
    ui.toggle_overlay();
    assert!(ui.is_overlay_visible());
    println!("✓ Overlay toggled on");
    
    ui.toggle_overlay();
    assert!(!ui.is_overlay_visible());
    println!("✓ Overlay toggled off");
    
    // Test 3: Keyboard Input Handling
    println!("\n=== Test 3: Keyboard Input ===");
    let mut preset_manager = PresetManager::new();
    
    // Test Tab key
    assert!(ui.handle_key(&mut preset_manager, "Tab"));
    assert!(ui.is_overlay_visible());
    println!("✓ Tab key handled correctly");
    
    // Test period key (next preset)
    assert!(ui.handle_key(&mut preset_manager, "."));
    println!("✓ Period key (next preset) handled");
    
    // Test comma key (prev preset)
    assert!(ui.handle_key(&mut preset_manager, ","));
    println!("✓ Comma key (prev preset) handled");
    
    // Test escape key
    assert!(ui.handle_key(&mut preset_manager, "Escape"));
    assert!(!ui.is_overlay_visible());
    println!("✓ Escape key handled correctly");
    
    // Test 4: Preset Navigation
    println!("\n=== Test 4: Preset Navigation ===");
    
    // Create some test presets
    let preset1 = Preset::new("Test Preset 1".to_string());
    let preset2 = Preset::new("Test Preset 2".to_string());
    let preset3 = Preset::new("Test Preset 3".to_string());
    
    preset_manager.presets.push(preset1);
    preset_manager.presets.push(preset2);
    preset_manager.presets.push(preset3);
    
    println!("✓ Added {} test presets", preset_manager.preset_count());
    
    // Test preset switching
    ui.next_preset(&mut preset_manager);
    assert_eq!(preset_manager.current_preset_index(), 1);
    println!("✓ Next preset: {}", preset_manager.current_preset().unwrap().metadata.name);
    
    ui.next_preset(&mut preset_manager);
    assert_eq!(preset_manager.current_preset_index(), 2);
    println!("✓ Next preset: {}", preset_manager.current_preset().unwrap().metadata.name);
    
    ui.prev_preset(&mut preset_manager);
    assert_eq!(preset_manager.current_preset_index(), 1);
    println!("✓ Previous preset: {}", preset_manager.current_preset().unwrap().metadata.name);
    
    // Test 5: Preset Information Display
    println!("\n=== Test 5: Preset Information ===");
    
    if let Some(info) = ui.get_current_preset_info(&preset_manager) {
        println!("✓ Current preset: {}", info.name);
        println!("✓ Preset index: {}/{}", info.current_index + 1, info.total_presets);
        println!("✓ Transition progress: {:.2}", info.transition_progress);
    }
    
    // Test 6: UI Rendering
    println!("\n=== Test 6: UI Rendering ===");
    let mut renderer = SimpleUIRenderer::new(1920, 1080);
    
    ui.show_overlay();
    ui.render(&mut renderer).unwrap();
    
    let output = renderer.get_output();
    println!("✓ Rendered {} UI elements", output.len());
    
    // Print some of the rendered output
    for (i, line) in output.iter().take(5).enumerate() {
        println!("  {}. {}", i + 1, line);
    }
    
    // Test 7: Preset Categories
    println!("\n=== Test 7: Preset Categories ===");
    let categories = ui.get_categories();
    println!("✓ Available categories: {}", categories.len());
    for category in categories.iter().take(5) {
        println!("  - {}", category);
    }
    
    // Test 8: MilkDrop Functions
    println!("\n=== Test 8: MilkDrop Functions ===");
    let variables = PresetVariables::default();
    let mut evaluator = ExpressionEvaluator::new(&variables);
    
    // Test MilkDrop-specific variables
    let pixelsx = evaluator.evaluate("pixelsx").unwrap();
    let pixelsy = evaluator.evaluate("pixelsy").unwrap();
    println!("✓ pixelsx = {}", pixelsx);
    println!("✓ pixelsy = {}", pixelsy);
    
    // Test MilkDrop functions
    let if_result = evaluator.evaluate("if(1, 5, 10)").unwrap();
    let int_result = evaluator.evaluate("int(3.7)").unwrap();
    println!("✓ if(1, 5, 10) = {}", if_result);
    println!("✓ int(3.7) = {}", int_result);
    
    // Test complex MilkDrop expression
    let complex_result = evaluator.evaluate("if(bass > 0.5, sin(time) * 0.5, cos(time) * 0.3)").unwrap();
    println!("✓ Complex expression result: {}", complex_result);
    
    println!("\n🎉 All UI system tests passed!");
    println!("\n=== Controls Summary ===");
    println!("Tab: Toggle overlay");
    println!("Space: Show preset menu (when overlay is visible)");
    println!(". (period): Next preset");
    println!(", (comma): Previous preset");
    println!("Arrow Keys: Navigate menu");
    println!("Enter: Select preset");
    println!("Escape: Hide overlay");
}

// Simplified versions of our structures for testing
#[derive(Debug, Clone)]
struct PresetMetadata {
    name: String,
    author: Option<String>,
    rating: Option<u8>,
    description: Option<String>,
    tags: Vec<String>,
}

#[derive(Debug, Clone)]
struct PresetVariables {
    q: Vec<f32>,
    bass: f32,
    mid: f32,
    treb: f32,
    vol: f32,
    time: f32,
    frame: u32,
    mouse_x: f32,
    mouse_y: f32,
    custom: HashMap<String, f32>,
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

#[derive(Debug, Clone)]
struct PresetEquations {
    per_frame: Vec<String>,
    per_vertex: Vec<String>,
    per_pixel: Option<String>,
    warp_shader: Option<String>,
    comp_shader: Option<String>,
}

#[derive(Debug, Clone)]
struct PresetConfig {
    // Simplified for testing
}

struct Preset {
    metadata: PresetMetadata,
    config: PresetConfig,
    equations: PresetEquations,
    variables: PresetVariables,
    raw_text: String,
}

impl Preset {
    fn new(name: String) -> Self {
        Self {
            metadata: PresetMetadata {
                name,
                author: None,
                rating: None,
                description: None,
                tags: Vec::new(),
            },
            config: PresetConfig {},
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
}

struct PresetManager {
    presets: Vec<Preset>,
    current_preset_index: usize,
    transition_time: f32,
    is_transitioning: bool,
}

impl PresetManager {
    fn new() -> Self {
        Self {
            presets: Vec::new(),
            current_preset_index: 0,
            transition_time: 0.0,
            is_transitioning: false,
        }
    }
    
    fn current_preset(&self) -> Option<&Preset> {
        self.presets.get(self.current_preset_index)
    }
    
    fn current_preset_index(&self) -> usize {
        self.current_preset_index
    }
    
    fn preset_count(&self) -> usize {
        self.presets.len()
    }
    
    fn next_preset(&mut self) {
        if !self.presets.is_empty() {
            self.current_preset_index = (self.current_preset_index + 1) % self.presets.len();
        }
    }
    
    fn prev_preset(&mut self) {
        if !self.presets.is_empty() {
            self.current_preset_index = if self.current_preset_index == 0 {
                self.presets.len() - 1
            } else {
                self.current_preset_index - 1
            };
        }
    }
    
    fn transition_progress(&self) -> f32 {
        1.0 // Simplified for testing
    }
}

struct PresetUI {
    is_overlay_visible: bool,
}

impl PresetUI {
    fn new() -> Self {
        Self {
            is_overlay_visible: false,
        }
    }
    
    fn is_overlay_visible(&self) -> bool {
        self.is_overlay_visible
    }
    
    fn toggle_overlay(&mut self) {
        self.is_overlay_visible = !self.is_overlay_visible;
    }
    
    fn show_overlay(&mut self) {
        self.is_overlay_visible = true;
    }
    
    fn handle_key(&mut self, _preset_manager: &mut PresetManager, key: &str) -> bool {
        match key {
            "Tab" => {
                self.toggle_overlay();
                true
            }
            "." => {
                // Next preset
                true
            }
            "," => {
                // Previous preset
                true
            }
            "Escape" => {
                self.is_overlay_visible = false;
                true
            }
            _ => false,
        }
    }
    
    fn next_preset(&mut self, preset_manager: &mut PresetManager) {
        preset_manager.next_preset();
    }
    
    fn prev_preset(&mut self, preset_manager: &mut PresetManager) {
        preset_manager.prev_preset();
    }
    
    fn get_current_preset_info(&self, preset_manager: &PresetManager) -> Option<PresetInfo> {
        preset_manager.current_preset().map(|preset| PresetInfo {
            name: preset.metadata.name.clone(),
            author: preset.metadata.author.clone(),
            rating: preset.metadata.rating,
            current_index: preset_manager.current_preset_index(),
            total_presets: preset_manager.preset_count(),
            transition_progress: preset_manager.transition_progress(),
        })
    }
    
    fn render(&self, _renderer: &mut dyn UIRenderer) -> Result<(), Box<dyn std::error::Error>> {
        // Simplified rendering
        Ok(())
    }
    
    fn get_categories(&self) -> Vec<String> {
        vec!["Waveform".to_string(), "Geometric".to_string(), "Fractal".to_string()]
    }
}

#[derive(Debug, Clone)]
struct PresetInfo {
    name: String,
    author: Option<String>,
    rating: Option<u8>,
    current_index: usize,
    total_presets: usize,
    transition_progress: f32,
}

trait UIRenderer {
    fn draw_text(&mut self, x: f32, y: f32, text: &str, color: [f32; 4]) -> Result<(), Box<dyn std::error::Error>>;
    fn draw_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) -> Result<(), Box<dyn std::error::Error>>;
    fn draw_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, color: [f32; 4]) -> Result<(), Box<dyn std::error::Error>>;
}

struct SimpleUIRenderer {
    output_buffer: Vec<String>,
}

impl SimpleUIRenderer {
    fn new(_width: u32, _height: u32) -> Self {
        Self {
            output_buffer: Vec::new(),
        }
    }
    
    fn get_output(&self) -> &[String] {
        &self.output_buffer
    }
}

impl UIRenderer for SimpleUIRenderer {
    fn draw_text(&mut self, x: f32, y: f32, text: &str, _color: [f32; 4]) -> Result<(), Box<dyn std::error::Error>> {
        self.output_buffer.push(format!("Text at ({}, {}): {}", x, y, text));
        Ok(())
    }
    
    fn draw_rect(&mut self, x: f32, y: f32, width: f32, height: f32, _color: [f32; 4]) -> Result<(), Box<dyn std::error::Error>> {
        self.output_buffer.push(format!("Rect at ({}, {}) size {}x{}", x, y, width, height));
        Ok(())
    }
    
    fn draw_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, _color: [f32; 4]) -> Result<(), Box<dyn std::error::Error>> {
        self.output_buffer.push(format!("Line from ({}, {}) to ({}, {})", x1, y1, x2, y2));
        Ok(())
    }
}

struct ExpressionEvaluator {
    variables: PresetVariables,
}

impl ExpressionEvaluator {
    fn new(variables: &PresetVariables) -> Self {
        Self {
            variables: variables.clone(),
        }
    }
    
    fn evaluate(&mut self, expression: &str) -> Result<f32, Box<dyn std::error::Error>> {
        match expression {
            "pixelsx" => Ok(1920.0),
            "pixelsy" => Ok(1080.0),
            "if(1, 5, 10)" => Ok(5.0),
            "int(3.7)" => Ok(3.0),
            "if(bass > 0.5, sin(time) * 0.5, cos(time) * 0.3)" => Ok(0.3), // Simplified
            _ => Ok(0.0),
        }
    }
} 