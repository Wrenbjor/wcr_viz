use anyhow::Result;
use std::sync::Arc;
use winit::window::Window;

use crate::preset::PresetManager;

#[derive(Debug, Clone)]
pub enum Message {
    ToggleOverlay,
    HideOverlay,
    NextPreset,
    PreviousPreset,
}

pub struct IcedIntegration {
    is_overlay_visible: bool,
    current_preset_index: usize,
    preset_count: usize,
}

impl IcedIntegration {
    pub fn new() -> Result<Self> {
        Ok(Self {
            is_overlay_visible: false,
            current_preset_index: 0,
            preset_count: 0,
        })
    }

    pub fn is_overlay_visible(&self) -> bool {
        self.is_overlay_visible
    }

    pub fn toggle_overlay(&mut self) {
        self.is_overlay_visible = !self.is_overlay_visible;
        log::info!("🎨 Simple Overlay: Visibility toggled to {}", self.is_overlay_visible);
    }

    pub fn hide_overlay(&mut self) {
        self.is_overlay_visible = false;
        log::info!("🎨 Simple Overlay: Hidden");
    }

    pub fn show_overlay(&mut self) {
        self.is_overlay_visible = true;
        log::info!("🎨 Simple Overlay: Shown");
    }

    pub fn update_preset_info(&mut self, preset_manager: &PresetManager) {
        self.current_preset_index = preset_manager.current_preset_index;
        self.preset_count = preset_manager.preset_count();
    }

    pub fn handle_input(&mut self, _window: &Arc<Window>, _event: &winit::event::WindowEvent) -> bool {
        // For this simple implementation, we'll handle input in the main event loop
        false
    }

    pub fn render_ui(&mut self, preset_manager: &PresetManager) {
        self.update_preset_info(preset_manager);
        
        if self.is_overlay_visible {
            log::debug!("🎨 Simple Overlay: Updated with preset {} of {}", 
                       self.current_preset_index + 1, 
                       self.preset_count.max(1));
        }
    }

    pub fn get_overlay_text(&self) -> Vec<String> {
        if !self.is_overlay_visible {
            return vec![];
        }

        // Very simple overlay text for testing
        vec![
            "OVERLAY ON".to_string(),
            "TAB = TOGGLE".to_string(),
            "ESC = HIDE".to_string(),
            format!("PRESET {}/{}", self.current_preset_index + 1, self.preset_count),
        ]
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::ToggleOverlay => {
                self.toggle_overlay();
            }
            Message::HideOverlay => {
                self.hide_overlay();
            }
            Message::NextPreset => {
                if self.preset_count > 0 {
                    self.current_preset_index = (self.current_preset_index + 1) % self.preset_count;
                    log::info!("🎵 Next preset: {}", self.current_preset_index);
                }
            }
            Message::PreviousPreset => {
                if self.preset_count > 0 {
                    self.current_preset_index = if self.current_preset_index == 0 {
                        self.preset_count - 1
                    } else {
                        self.current_preset_index - 1
                    };
                    log::info!("🎵 Previous preset: {}", self.current_preset_index);
                }
            }
        }
    }
} 