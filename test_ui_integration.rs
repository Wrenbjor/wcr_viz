use wcr_viz::preset::PresetManager;
use wcr_viz::ui::PresetUI;

fn main() {
    // Initialize logging
    env_logger::init();
    
    println!("Testing UI Integration...");
    
    // Create preset manager and UI
    let mut preset_manager = PresetManager::new();
    let mut preset_ui = PresetUI::new();
    
    // Load presets
    match preset_ui.load_presets("presets/cream-of-the-crop") {
        Ok(_) => println!("✅ Presets loaded successfully"),
        Err(e) => println!("❌ Failed to load presets: {}", e),
    }
    
    // Test overlay toggle
    println!("Testing overlay toggle...");
    preset_ui.toggle_overlay();
    println!("Overlay visible: {}", preset_ui.is_overlay_visible());
    
    // Test key handling
    println!("Testing key handling...");
    let keys = ["Tab", "Space", "Period", "Comma"];
    for key in keys {
        let handled = preset_ui.handle_key(&mut preset_manager, key);
        println!("Key '{}' handled: {}", key, handled);
    }
    
    println!("UI Integration test complete!");
} 