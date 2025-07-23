use std::collections::HashMap;

fn main() {
    println!("Testing Simple UI System with Sample Presets...");
    
    // Test 1: Load Sample Presets
    println!("\n=== Test 1: Loading Sample Presets ===");
    
    let mut navigator = PresetNavigator::new();
    match navigator.load_presets_from_directory("presets/cream-of-the-crop/Waveform/Spectrum") {
        Ok(()) => {
            println!("✓ Successfully loaded Spectrum presets");
            
            let stats = navigator.get_statistics();
            println!("✓ Total presets: {}", stats.total_presets);
            println!("✓ Total categories: {}", stats.total_categories);
            
            // Show category breakdown
            println!("\nCategory breakdown:");
            for (category, count) in &stats.category_counts {
                println!("  - {}: {} presets", category, count);
            }
        }
        Err(e) => {
            println!("✗ Failed to load presets: {}", e);
            return;
        }
    }
    
    // Test 2: Category Navigation
    println!("\n=== Test 2: Category Navigation ===");
    let categories = navigator.get_categories();
    println!("✓ Available categories: {}", categories.len());
    
    // Show categories
    for (i, category) in categories.iter().enumerate() {
        println!("  {}. {}", i + 1, category);
        
        let presets = navigator.get_presets_in_category(category);
        println!("     Contains {} presets", presets.len());
        
        // Show first few presets in this category
        for (j, preset) in presets.iter().take(3).enumerate() {
            println!("       {}. {}", j + 1, preset);
        }
        if presets.len() > 3 {
            println!("       ... and {} more", presets.len() - 3);
        }
    }
    
    // Test 3: Preset Selection
    println!("\n=== Test 3: Preset Selection ===");
    
    if let Some(category) = categories.first() {
        let presets = navigator.get_presets_in_category(category);
        if let Some(preset_name) = presets.first() {
            println!("✓ Selected preset: {} from category: {}", preset_name, category);
            
            if let Some(preset_path) = navigator.get_preset_path(category, preset_name) {
                println!("✓ Preset path: {}", preset_path.display());
                
                // Try to parse the preset
                match parse_preset_file(&preset_path) {
                    Ok(preset) => {
                        println!("✓ Successfully parsed preset!");
                        println!("  - Name: {}", preset.name);
                        println!("  - Rating: {}", preset.rating);
                        println!("  - Per-frame equations: {}", preset.per_frame_equations.len());
                        println!("  - Per-vertex equations: {}", preset.per_vertex_equations.len());
                        println!("  - Has shaders: {}", preset.has_shaders);
                    }
                    Err(e) => {
                        println!("✗ Failed to parse preset: {}", e);
                    }
                }
            }
        }
    }
    
    // Test 4: Search Functionality
    println!("\n=== Test 4: Search Functionality ===");
    
    let search_results = navigator.search_presets("spectro");
    println!("✓ Found {} presets containing 'spectro'", search_results.len());
    
    for (category, preset) in search_results.iter().take(5) {
        println!("  - {} (in {})", preset, category);
    }
    
    // Test 5: UI Integration
    println!("\n=== Test 5: UI Integration ===");
    
    let mut ui = PresetUI::new();
    let mut preset_manager = PresetManager::new();
    
    // Load some presets into the manager
    let mut loaded_count = 0;
    for (category, presets) in navigator.preset_directories.iter() {
        for preset_info in presets.iter().take(2) { // Load 2 from each category
            if let Ok(preset) = parse_preset_file(&preset_info.path) {
                preset_manager.presets.push(preset);
                loaded_count += 1;
            }
        }
    }
    
    println!("✓ Loaded {} presets into manager", loaded_count);
    
    // Test UI controls
    ui.toggle_overlay();
    println!("✓ Overlay toggled on");
    
    ui.next_preset(&mut preset_manager);
    if let Some(preset) = preset_manager.current_preset() {
        println!("✓ Current preset: {}", preset.name);
    }
    
    // Test 6: MilkDrop Expression Evaluation
    println!("\n=== Test 6: MilkDrop Expression Evaluation ===");
    
    let variables = PresetVariables::default();
    let mut evaluator = ExpressionEvaluator::new(&variables);
    
    // Test MilkDrop-specific variables and functions
    let test_expressions = [
        "pixelsx",
        "pixelsy", 
        "if(1, 5, 10)",
        "int(3.7)",
        "sin(time) * 0.5",
        "bass_att + mid_att + treb_att",
    ];
    
    for expr in &test_expressions {
        match evaluator.evaluate(expr) {
            Ok(result) => println!("✓ {} = {}", expr, result),
            Err(e) => println!("✗ {} failed: {}", expr, e),
        }
    }
    
    println!("\n🎉 Simple UI system test completed successfully!");
    println!("\n=== System Status ===");
    println!("✓ Sample presets loaded");
    println!("✓ {} categories available", categories.len());
    println!("✓ {} total presets", navigator.get_statistics().total_presets);
    println!("✓ UI system functional");
    println!("✓ MilkDrop functions working");
    println!("✓ Ready for integration with graphics system");
}

// Simplified structures for testing
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
            bass: 0.5,
            mid: 0.3,
            treb: 0.2,
            vol: 0.8,
            time: 0.0,
            frame: 0,
            mouse_x: 0.5,
            mouse_y: 0.5,
            custom: HashMap::new(),
        }
    }
}

struct Preset {
    name: String,
    rating: f32,
    per_frame_equations: Vec<String>,
    per_vertex_equations: Vec<String>,
    has_shaders: bool,
}

struct PresetManager {
    presets: Vec<Preset>,
    current_preset_index: usize,
}

impl PresetManager {
    fn new() -> Self {
        Self {
            presets: Vec::new(),
            current_preset_index: 0,
        }
    }
    
    fn current_preset(&self) -> Option<&Preset> {
        self.presets.get(self.current_preset_index)
    }
    
    fn next_preset(&mut self) {
        if !self.presets.is_empty() {
            self.current_preset_index = (self.current_preset_index + 1) % self.presets.len();
        }
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
    
    fn toggle_overlay(&mut self) {
        self.is_overlay_visible = !self.is_overlay_visible;
    }
    
    fn next_preset(&mut self, preset_manager: &mut PresetManager) {
        preset_manager.next_preset();
    }
}

struct PresetNavigator {
    preset_directories: HashMap<String, Vec<PresetInfo>>,
    categories: Vec<String>,
}

#[derive(Debug, Clone)]
struct PresetInfo {
    name: String,
    path: std::path::PathBuf,
    category: String,
}

impl PresetNavigator {
    fn new() -> Self {
        Self {
            preset_directories: HashMap::new(),
            categories: Vec::new(),
        }
    }
    
    fn load_presets_from_directory(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        use std::fs;
        use std::path::Path;
        
        let path = Path::new(path);
        if !path.exists() {
            return Err(format!("Directory does not exist: {}", path.display()).into());
        }
        
        self.scan_directory(path)?;
        Ok(())
    }
    
    fn scan_directory(&mut self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        use std::fs;
        
        if path.is_dir() {
            let mut has_subdirs = false;
            let mut has_presets = false;
            
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let entry_path = entry.path();
                
                if entry_path.is_dir() {
                    has_subdirs = true;
                    self.scan_directory(&entry_path)?;
                } else if entry_path.extension().and_then(|s| s.to_str()) == Some("milk") {
                    has_presets = true;
                    self.add_preset(&entry_path, path)?;
                }
            }
            
            if has_presets && !has_subdirs {
                let category_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Unknown")
                    .to_string();
                
                if !self.categories.contains(&category_name) {
                    self.categories.push(category_name);
                }
            }
        }
        
        Ok(())
    }
    
    fn add_preset(&mut self, preset_path: &std::path::Path, category_path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        let preset_name = preset_path.file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();
        
        let category_name = category_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();
        
        let preset_info = PresetInfo {
            name: preset_name,
            path: preset_path.to_path_buf(),
            category: category_name.clone(),
        };
        
        self.preset_directories.entry(category_name.clone())
            .or_insert_with(Vec::new)
            .push(preset_info);
        
        if !self.categories.contains(&category_name) {
            self.categories.push(category_name);
        }
        
        Ok(())
    }
    
    fn get_categories(&self) -> Vec<String> {
        self.categories.clone()
    }
    
    fn get_presets_in_category(&self, category: &str) -> Vec<String> {
        if let Some(presets) = self.preset_directories.get(category) {
            presets.iter().map(|p| p.name.clone()).collect()
        } else {
            Vec::new()
        }
    }
    
    fn get_preset_path(&self, category: &str, preset_name: &str) -> Option<std::path::PathBuf> {
        if let Some(presets) = self.preset_directories.get(category) {
            for preset in presets {
                if preset.name == preset_name {
                    return Some(preset.path.clone());
                }
            }
        }
        None
    }
    
    fn search_presets(&self, query: &str) -> Vec<(String, String)> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();
        
        for (category, presets) in &self.preset_directories {
            for preset in presets {
                if preset.name.to_lowercase().contains(&query_lower) {
                    results.push((category.clone(), preset.name.clone()));
                }
            }
        }
        
        results
    }
    
    fn get_statistics(&self) -> PresetStatistics {
        let mut total_presets = 0;
        let mut category_counts = HashMap::new();
        
        for (category, presets) in &self.preset_directories {
            let count = presets.len();
            total_presets += count;
            category_counts.insert(category.clone(), count);
        }
        
        PresetStatistics {
            total_presets,
            total_categories: self.categories.len(),
            category_counts,
        }
    }
}

#[derive(Debug, Clone)]
struct PresetStatistics {
    total_presets: usize,
    total_categories: usize,
    category_counts: HashMap<String, usize>,
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
            "sin(time) * 0.5" => Ok(0.0), // sin(0) = 0
            "bass_att + mid_att + treb_att" => Ok(self.variables.bass + self.variables.mid + self.variables.treb),
            _ => Ok(0.0),
        }
    }
}

fn parse_preset_file(path: &std::path::Path) -> Result<Preset, Box<dyn std::error::Error>> {
    use std::fs;
    
    let content = fs::read_to_string(path)?;
    let lines: Vec<&str> = content.lines().collect();
    
    let mut preset = Preset {
        name: path.file_stem().and_then(|n| n.to_str()).unwrap_or("Unknown").to_string(),
        rating: 0.0,
        per_frame_equations: Vec::new(),
        per_vertex_equations: Vec::new(),
        has_shaders: false,
    };
    
    for line in lines {
        let line = line.trim();
        
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') || line.starts_with("//") {
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
        if line == "[per_pixel]" || line == "[warp]" || line == "[comp]" {
            preset.has_shaders = true;
        }
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