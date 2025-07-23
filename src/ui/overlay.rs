use anyhow::Result;
use crate::preset::Preset;
use crate::ui::{UIRenderer, navigation::PresetNavigator};

/// Overlay for displaying preset information and controls
#[derive(Clone)]
pub struct PresetOverlay {
    is_visible: bool,
    current_preset_info: Option<PresetInfo>,
    show_menu: bool,
    pub selected_category: usize,
    selected_preset: usize,
    pub categories: Vec<String>,
    presets_in_category: Vec<String>,
}

#[derive(Debug, Clone)]
struct PresetInfo {
    name: String,
    author: Option<String>,
    rating: Option<u8>,
    equations_count: usize,
    has_shaders: bool,
}

impl PresetOverlay {
    /// Create a new preset overlay
    pub fn new() -> Self {
        Self {
            is_visible: true, // Always visible for testing
            current_preset_info: None,
            show_menu: true, // Always show menu for testing
            selected_category: 0,
            selected_preset: 0,
            categories: Vec::new(),
            presets_in_category: Vec::new(),
        }
    }
    
    /// Show the overlay
    pub fn show(&mut self) {
        self.is_visible = true;
        log::info!("🎨 Overlay: Show called, is_visible: {}", self.is_visible);
    }
    
    /// Hide the overlay
    pub fn hide(&mut self) {
        self.is_visible = false;
        self.show_menu = false;
        log::info!("🎨 Overlay: Hide called, is_visible: {}, show_menu: {}", self.is_visible, self.show_menu);
    }
    
    /// Check if overlay is visible
    pub fn is_visible(&self) -> bool {
        self.is_visible
    }
    
    /// Update preset information
    pub fn update_preset_info(&mut self, preset: &Preset) {
        self.current_preset_info = Some(PresetInfo {
            name: preset.metadata.name.clone(),
            author: preset.metadata.author.clone(),
            rating: preset.metadata.rating,
            equations_count: preset.equations.per_frame.len() + preset.equations.per_vertex.len(),
            has_shaders: preset.equations.per_pixel.is_some() || 
                        preset.equations.warp_shader.is_some() || 
                        preset.equations.comp_shader.is_some(),
        });
    }
    
    /// Show preset selection menu
    pub fn show_preset_menu(&mut self, navigator: &PresetNavigator) {
        self.show_menu = true;
        self.categories = navigator.get_categories();
        
        log::info!("🎨 Overlay: Showing preset menu with {} categories", self.categories.len());
        
        if !self.categories.is_empty() {
            self.selected_category = 0;
            self.selected_preset = 0;
            self.update_presets_for_category(navigator);
        } else {
            log::warn!("🎨 Overlay: No categories available for preset menu");
            self.presets_in_category.clear();
        }
    }
    
    /// Navigate menu with arrow keys
    pub fn navigate_menu(&mut self, direction: MenuDirection, navigator: &PresetNavigator) {
        let old_selection = (self.selected_category, self.selected_preset);
        
        match direction {
            MenuDirection::Up => {
                if self.selected_preset > 0 {
                    self.selected_preset -= 1;
                } else if self.selected_preset == 0 && !self.presets_in_category.is_empty() {
                    // Wrap to bottom of preset list
                    self.selected_preset = self.presets_in_category.len() - 1;
                }
            }
            MenuDirection::Down => {
                if self.selected_preset < self.presets_in_category.len().saturating_sub(1) {
                    self.selected_preset += 1;
                } else if self.selected_preset == self.presets_in_category.len().saturating_sub(1) && !self.presets_in_category.is_empty() {
                    // Wrap to top of preset list
                    self.selected_preset = 0;
                }
            }
            MenuDirection::Left => {
                if self.selected_category > 0 {
                    self.selected_category -= 1;
                    self.selected_preset = 0;
                    self.update_presets_for_category(navigator);
                }
            }
            MenuDirection::Right => {
                if self.selected_category < self.categories.len().saturating_sub(1) {
                    self.selected_category += 1;
                    self.selected_preset = 0;
                    self.update_presets_for_category(navigator);
                }
            }
        }
        
        let new_selection = (self.selected_category, self.selected_preset);
        if old_selection != new_selection {
            log::info!("🎨 Overlay: Navigation changed from {:?} to {:?}", old_selection, new_selection);
            log::info!("🎨 Overlay: Current category: '{}', presets: {}", 
                      self.categories.get(self.selected_category).unwrap_or(&"None".to_string()),
                      self.presets_in_category.len());
        }
    }
    
    /// Update presets for the currently selected category
    fn update_presets_for_category(&mut self, navigator: &PresetNavigator) {
        if let Some(category) = self.categories.get(self.selected_category) {
            self.presets_in_category = navigator.get_presets_in_category(category);
            log::info!("🎨 Overlay: Updated presets for category '{}': {} presets", 
                      category, self.presets_in_category.len());
        } else {
            self.presets_in_category.clear();
            log::warn!("🎨 Overlay: No category found at index {}", self.selected_category);
        }
    }
    
    /// Get selected preset name
    pub fn get_selected_preset(&self) -> Option<String> {
        if self.show_menu && !self.presets_in_category.is_empty() {
            self.presets_in_category.get(self.selected_preset).cloned()
        } else {
            None
        }
    }
    
    /// Render the overlay
    pub fn render(&self, renderer: &mut dyn UIRenderer) -> Result<()> {
        log::info!("🎨 Overlay: Starting render, visible: {}, show_menu: {}", self.is_visible, self.show_menu);
        
        if !self.is_visible {
            log::info!("🎨 Overlay: Not visible, skipping render");
            return Ok(());
        }
        
        // Draw background overlay
        log::info!("🎨 Overlay: Drawing background");
        renderer.draw_rect(0.0, 0.0, 1920.0, 1080.0, [0.0, 0.0, 0.0, 0.7])?;
        
        if self.show_menu {
            log::info!("🎨 Overlay: Rendering menu");
            self.render_menu(renderer)?;
        } else {
            log::info!("🎨 Overlay: Rendering info panel");
            self.render_info_panel(renderer)?;
        }
        
        // Draw controls help
        log::info!("🎨 Overlay: Rendering controls help");
        self.render_controls_help(renderer)?;
        
        log::info!("🎨 Overlay: Render complete");
        Ok(())
    }
    
    /// Render the info panel
    fn render_info_panel(&self, renderer: &mut dyn UIRenderer) -> Result<()> {
        let x = 50.0;
        let mut y = 50.0;
        let line_height = 30.0;
        
        // Draw info panel background
        renderer.draw_rect(x - 10.0, y - 10.0, 400.0, 200.0, [0.1, 0.1, 0.1, 0.9])?;
        
        // Title
        renderer.draw_text(x, y, "PRESET INFO", [1.0, 1.0, 1.0, 1.0])?;
        y += line_height * 1.5;
        
        if let Some(info) = &self.current_preset_info {
            // Preset name
            renderer.draw_text(x, y, &format!("Name: {}", info.name), [1.0, 1.0, 0.0, 1.0])?;
            y += line_height;
            
            // Author
            if let Some(author) = &info.author {
                renderer.draw_text(x, y, &format!("Author: {}", author), [0.8, 0.8, 0.8, 1.0])?;
                y += line_height;
            }
            
            // Rating
            if let Some(rating) = info.rating {
                let stars = "★".repeat(rating as usize) + &"☆".repeat(5 - rating as usize);
                renderer.draw_text(x, y, &format!("Rating: {}", stars), [1.0, 0.8, 0.0, 1.0])?;
                y += line_height;
            }
            
            // Equations count
            renderer.draw_text(x, y, &format!("Equations: {}", info.equations_count), [0.7, 0.9, 1.0, 1.0])?;
            y += line_height;
            
            // Shaders
            renderer.draw_text(x, y, &format!("Shaders: {}", if info.has_shaders { "Yes" } else { "No" }), [0.7, 0.9, 1.0, 1.0])?;
        } else {
            renderer.draw_text(x, y, "No preset loaded", [0.7, 0.7, 0.7, 1.0])?;
        }
        
        Ok(())
    }
    
    /// Render the preset selection menu
    fn render_menu(&self, renderer: &mut dyn UIRenderer) -> Result<()> {
        let x = 100.0;
        let mut y = 100.0;
        let line_height = 25.0;
        let category_width = 400.0; // Increased width for nested paths
        let preset_width = 500.0;   // Increased width for preset names
        let max_visible_items = 20; // Limit visible items to prevent overflow
        
        log::info!("🎨 Menu: Rendering with {} categories, {} presets, selection: ({}, {})", 
                  self.categories.len(), self.presets_in_category.len(), 
                  self.selected_category, self.selected_preset);
        
        // Draw menu background with better depth
        renderer.draw_rect(x - 10.0, y - 10.0, category_width + preset_width + 30.0, 600.0, [0.1, 0.1, 0.1, 0.95])?;
        
        // Title with better contrast
        renderer.draw_text(x, y, "PRESET SELECTION", [1.0, 1.0, 0.0, 1.0])?;
        y += line_height * 2.0;
        
        // Categories section
        renderer.draw_text(x, y, "CATEGORIES:", [1.0, 1.0, 1.0, 1.0])?;
        y += line_height;
        
        // Calculate visible range for categories
        let start_category = self.selected_category.saturating_sub(max_visible_items / 2);
        let end_category = (start_category + max_visible_items).min(self.categories.len());
        
        for i in start_category..end_category {
            let display_y = y + (i - start_category) as f32 * line_height;
            
            // Skip if outside visible area
            if display_y > y + max_visible_items as f32 * line_height {
                break;
            }
            
            let category = &self.categories[i];
            let color = if i == self.selected_category {
                [1.0, 1.0, 0.0, 1.0] // Bright yellow for selected
            } else {
                [0.9, 0.9, 0.9, 1.0] // Bright white for normal
            };
            
            // Truncate long category names
            let display_text = if category.len() > 35 {
                format!("{}...", &category[..32])
            } else {
                category.clone()
            };
            
            renderer.draw_text(x, display_y, &display_text, color)?;
        }
        
        // Presets section
        y = 100.0 + line_height * 2.0;
        renderer.draw_text(x + category_width + 20.0, y, "PRESETS:", [1.0, 1.0, 1.0, 1.0])?;
        y += line_height;
        
        // Calculate visible range for presets
        let start_preset = self.selected_preset.saturating_sub(max_visible_items / 2);
        let end_preset = (start_preset + max_visible_items).min(self.presets_in_category.len());
        
        for i in start_preset..end_preset {
            let display_y = y + (i - start_preset) as f32 * line_height;
            
            // Skip if outside visible area
            if display_y > y + max_visible_items as f32 * line_height {
                break;
            }
            
            let preset = &self.presets_in_category[i];
            let color = if i == self.selected_preset {
                [1.0, 1.0, 0.0, 1.0] // Bright yellow for selected
            } else {
                [0.9, 0.9, 0.9, 1.0] // Bright white for normal
            };
            
            // Truncate long preset names
            let display_text = if preset.len() > 40 {
                format!("{}...", &preset[..37])
            } else {
                preset.clone()
            };
            
            renderer.draw_text(x + category_width + 20.0, display_y, &display_text, color)?;
        }
        
        // Add scroll indicators if needed
        if start_category > 0 {
            renderer.draw_text(x, 100.0 + line_height * 2.0, "↑", [0.7, 0.7, 0.7, 1.0])?;
        }
        if end_category < self.categories.len() {
            renderer.draw_text(x, 100.0 + (max_visible_items + 2) as f32 * line_height, "↓", [0.7, 0.7, 0.7, 1.0])?;
        }
        
        if start_preset > 0 {
            renderer.draw_text(x + category_width + 20.0, 100.0 + line_height * 2.0, "↑", [0.7, 0.7, 0.7, 1.0])?;
        }
        if end_preset < self.presets_in_category.len() {
            renderer.draw_text(x + category_width + 20.0, 100.0 + (max_visible_items + 2) as f32 * line_height, "↓", [0.7, 0.7, 0.7, 1.0])?;
        }
        
        Ok(())
    }
    
    /// Render controls help
    fn render_controls_help(&self, renderer: &mut dyn UIRenderer) -> Result<()> {
        let x = 50.0;
        let mut y = 900.0;
        let line_height = 25.0;
        
        // Draw help background
        renderer.draw_rect(x - 10.0, y - 10.0, 500.0, 150.0, [0.1, 0.1, 0.1, 0.8])?;
        
        // Title
        renderer.draw_text(x, y, "CONTROLS:", [1.0, 1.0, 0.0, 1.0])?;
        y += line_height;
        
        // Control list
        let controls = [
            ("Tab", "Toggle overlay"),
            ("Space", "Show preset menu"),
            (".", "Next preset"),
            (",", "Previous preset"),
            ("Arrow Keys", "Navigate menu"),
            ("Enter", "Select preset"),
            ("Escape", "Hide overlay"),
        ];
        
        for (key, description) in controls.iter() {
            renderer.draw_text(x, y, &format!("{}: {}", key, description), [0.8, 0.8, 0.8, 1.0])?;
            y += line_height;
        }
        
        Ok(())
    }
}

/// Menu navigation direction
#[derive(Debug, Clone)]
pub enum MenuDirection {
    Up,
    Down,
    Left,
    Right,
} 