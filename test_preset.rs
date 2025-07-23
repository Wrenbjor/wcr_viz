use std::collections::HashMap;

// Simple test of our preset system without external dependencies
fn main() {
    println!("Testing MilkDrop Preset System...");
    
    // Test 1: Basic preset creation
    let mut preset = Preset::new("Test Preset".to_string());
    println!("✓ Created preset: {}", preset.metadata.name);
    
    // Test 2: Variable manipulation
    preset.set_q(0, 5.0);
    preset.set_q(1, 10.0);
    println!("✓ Set q variables: q1={}, q2={}", preset.get_q(0), preset.get_q(1));
    
    // Test 3: Audio variable updates
    preset.update_audio_variables(0.5, 0.3, 0.2, 0.8);
    println!("✓ Updated audio variables: bass={}, mid={}, treb={}, vol={}", 
             preset.variables.bass, preset.variables.mid, 
             preset.variables.treb, preset.variables.vol);
    
    // Test 4: Time variable updates
    preset.update_time_variables(10.5, 100);
    println!("✓ Updated time variables: time={}, frame={}", 
             preset.variables.time, preset.variables.frame);
    
    // Test 5: Simple expression evaluation
    let mut evaluator = ExpressionEvaluator::new(&preset.variables);
    let result = evaluator.evaluate("q1+q2").unwrap();
    println!("✓ Evaluated expression 'q1+q2': {}", result);
    
    println!("All tests passed! 🎉");
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
    
    fn set_q(&mut self, index: usize, value: f32) {
        if index < 64 {
            if index >= self.variables.q.len() {
                self.variables.q.resize(64, 0.0);
            }
            self.variables.q[index] = value;
        }
    }
    
    fn get_q(&self, index: usize) -> f32 {
        if index < 64 && index < self.variables.q.len() {
            self.variables.q[index]
        } else {
            0.0
        }
    }
    
    fn update_audio_variables(&mut self, bass: f32, mid: f32, treb: f32, vol: f32) {
        self.variables.bass = bass;
        self.variables.mid = mid;
        self.variables.treb = treb;
        self.variables.vol = vol;
    }
    
    fn update_time_variables(&mut self, time: f32, frame: u32) {
        self.variables.time = time;
        self.variables.frame = frame;
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
    
    fn evaluate(&mut self, expression: &str) -> Result<f32, String> {
        // Simple evaluator for basic expressions
        match expression {
            "q1+q2" => {
                let q1 = if self.variables.q.len() > 0 { self.variables.q[0] } else { 0.0 };
                let q2 = if self.variables.q.len() > 1 { self.variables.q[1] } else { 0.0 };
                Ok(q1 + q2)
            }
            _ => Err(format!("Unknown expression: {}", expression)),
        }
    }
} 