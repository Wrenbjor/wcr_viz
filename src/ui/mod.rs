use anyhow::Result;
use std::path::PathBuf;
use crate::preset::PresetManager;

pub mod overlay;
pub mod navigation;
pub mod renderer;

use overlay::PresetOverlay;
use navigation::PresetNavigator;

/// UI manager for preset control
#[derive(Clone)]
pub struct PresetUI {
    overlay: PresetOverlay,
    navigator: PresetNavigator,
    is_overlay_visible: bool,
    current_preset_path: Option<PathBuf>,
}

impl PresetUI {
    /// Create a new preset UI manager
    pub fn new() -> Self {
        Self {
            overlay: PresetOverlay::new(),
            navigator: PresetNavigator::new(),
            is_overlay_visible: false, // Start with overlay hidden
            current_preset_path: None,
        }
    }
    
    /// Toggle the overlay visibility
    pub fn toggle_overlay(&mut self) {
        self.is_overlay_visible = !self.is_overlay_visible;
        log::info!("🎨 UI: Overlay visibility toggled to: {}", self.is_overlay_visible);
        if self.is_overlay_visible {
            self.overlay.show();
        } else {
            self.overlay.hide();
        }
    }
    
    /// Show the overlay
    pub fn show_overlay(&mut self) {
        self.is_overlay_visible = true;
        log::info!("🎨 UI: Overlay shown");
        self.overlay.show();
    }
    
    /// Hide the overlay
    pub fn hide_overlay(&mut self) {
        self.is_overlay_visible = false;
        log::info!("🎨 UI: Overlay hidden");
        self.overlay.hide();
    }
    
    /// Check if overlay is visible
    pub fn is_overlay_visible(&self) -> bool {
        self.is_overlay_visible
    }
    
    /// Load presets from a directory
    pub fn load_presets(&mut self, path: &str) -> Result<()> {
        log::info!("UI: Loading presets from: {}", path);
        self.navigator.load_presets_from_directory(path)?;
        log::info!("UI: Loaded {} categories", self.navigator.get_categories().len());
        Ok(())
    }
    
    /// Navigate to next preset
    pub fn next_preset(&mut self, preset_manager: &mut PresetManager) {
        preset_manager.next_preset();
        self.update_current_preset_info(preset_manager);
    }
    
    /// Navigate to previous preset
    pub fn prev_preset(&mut self, preset_manager: &mut PresetManager) {
        preset_manager.prev_preset();
        self.update_current_preset_info(preset_manager);
    }
    
    /// Select a specific preset by index
    pub fn select_preset(&mut self, preset_manager: &mut PresetManager, index: usize) {
        preset_manager.switch_to_preset(index);
        self.update_current_preset_info(preset_manager);
    }
    
    /// Select a preset by name
    pub fn select_preset_by_name(&mut self, preset_manager: &mut PresetManager, name: &str) {
        for i in 0..preset_manager.preset_count() {
            if let Some(preset) = preset_manager.get_preset(i) {
                if preset.metadata.name == name {
                    preset_manager.switch_to_preset(i);
                    self.update_current_preset_info(preset_manager);
                    break;
                }
            }
        }
    }
    
    /// Update current preset information
    fn update_current_preset_info(&mut self, preset_manager: &PresetManager) {
        if let Some(preset) = preset_manager.current_preset() {
            self.overlay.update_preset_info(preset);
        }
    }
    
    /// Get current preset information for display
    pub fn get_current_preset_info(&self, preset_manager: &PresetManager) -> Option<PresetInfo> {
        preset_manager.current_preset().map(|preset| PresetInfo {
            name: preset.metadata.name.clone(),
            author: preset.metadata.author.clone(),
            rating: preset.metadata.rating,
            current_index: preset_manager.current_preset_index,
            total_presets: preset_manager.preset_count(),
            transition_progress: preset_manager.transition_progress(),
        })
    }
    
    /// Handle keyboard input
    pub fn handle_key(&mut self, preset_manager: &mut PresetManager, key: &str) -> bool {
        log::info!("UI: Key pressed: {}", key);
        match key {
            "Tab" => {
                log::info!("UI: Toggling overlay");
                self.toggle_overlay();
                true
            }
            "Space" => {
                if self.is_overlay_visible {
                    // Show preset selection menu
                    self.overlay.show_preset_menu(&self.navigator);
                }
                true
            }
            "Period" | "." => {
                self.next_preset(preset_manager);
                true
            }
            "Comma" | "," => {
                self.prev_preset(preset_manager);
                true
            }
            "Return" | "Enter" => {
                // Enter key - select current preset from menu
                if self.is_overlay_visible {
                    if let Some(selected_preset) = self.overlay.get_selected_preset() {
                        if let Some(preset_path) = self.navigator.get_preset_path(&self.overlay.categories[self.overlay.selected_category], &selected_preset) {
                            match crate::preset::Preset::from_file(&preset_path.to_string_lossy()) {
                                Ok(preset) => {
                                    preset_manager.add_preset(preset);
                                    let presets = preset_manager.get_presets();
                                    preset_manager.switch_to_preset(presets.len() - 1);
                                    log::info!("UI: Loaded preset: {}", selected_preset);
                                    self.hide_overlay(); // Hide overlay after selection
                                }
                                Err(e) => {
                                    log::error!("UI: Failed to load preset {}: {}", selected_preset, e);
                                }
                            }
                        }
                    }
                }
                true
            }
            "Escape" => {
                if self.is_overlay_visible {
                    self.hide_overlay();
                }
                true
            }
            "Up" => {
                if self.is_overlay_visible {
                    self.overlay.navigate_menu(overlay::MenuDirection::Up, &self.navigator);
                }
                true
            }
            "Down" => {
                if self.is_overlay_visible {
                    self.overlay.navigate_menu(overlay::MenuDirection::Down, &self.navigator);
                }
                true
            }
            "Left" => {
                if self.is_overlay_visible {
                    self.overlay.navigate_menu(overlay::MenuDirection::Left, &self.navigator);
                }
                true
            }
            "Right" => {
                if self.is_overlay_visible {
                    self.overlay.navigate_menu(overlay::MenuDirection::Right, &self.navigator);
                }
                true
            }
            _ => false,
        }
    }
    
    /// Render the UI overlay
    pub fn render(&self, renderer: &mut dyn UIRenderer) -> Result<()> {
        log::info!("🎨 UI: Rendering overlay, visible: {}", self.is_overlay_visible);
        if self.is_overlay_visible {
            log::info!("🎨 UI: Calling overlay.render()");
            self.overlay.render(renderer)?;
            log::info!("🎨 UI: Overlay render complete");
        } else {
            log::info!("🎨 UI: Overlay not visible, skipping render");
        }
        Ok(())
    }
    
    /// Render the overlay (alias for render)
    pub fn render_overlay(&self, renderer: &mut dyn UIRenderer) -> Result<()> {
        self.render(renderer)
    }
    
    /// Get available preset categories
    pub fn get_categories(&self) -> Vec<String> {
        self.navigator.get_categories()
    }
    
    /// Get presets in a category
    pub fn get_presets_in_category(&self, category: &str) -> Vec<String> {
        self.navigator.get_presets_in_category(category)
    }
}

/// Information about the current preset for display
#[derive(Debug, Clone)]
pub struct PresetInfo {
    pub name: String,
    pub author: Option<String>,
    pub rating: Option<u8>,
    pub current_index: usize,
    pub total_presets: usize,
    pub transition_progress: f32,
}

/// Trait for UI rendering
pub trait UIRenderer {
    fn draw_text(&mut self, x: f32, y: f32, text: &str, color: [f32; 4]) -> Result<()>;
    fn draw_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) -> Result<()>;
    fn draw_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, color: [f32; 4]) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_preset_ui_creation() {
        let ui = PresetUI::new();
        assert!(!ui.is_overlay_visible());
    }
    
    #[test]
    fn test_overlay_toggle() {
        let mut ui = PresetUI::new();
        assert!(!ui.is_overlay_visible());
        
        ui.toggle_overlay();
        assert!(ui.is_overlay_visible());
        
        ui.toggle_overlay();
        assert!(!ui.is_overlay_visible());
    }
} 