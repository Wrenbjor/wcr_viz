use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Navigator for browsing preset directories and categories
#[derive(Clone)]
pub struct PresetNavigator {
    preset_directories: HashMap<String, Vec<PresetInfo>>,
    categories: Vec<String>,
    current_directory: Option<PathBuf>,
    // New: Track nested structure
    nested_categories: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct PresetInfo {
    pub name: String,
    pub path: PathBuf,
    pub category: String,
    pub subcategory: Option<String>,
    pub full_path: String, // Full relative path for display
}

impl PresetNavigator {
    /// Create a new preset navigator
    pub fn new() -> Self {
        Self {
            preset_directories: HashMap::new(),
            categories: Vec::new(),
            current_directory: None,
            nested_categories: HashMap::new(),
        }
    }
    
    /// Load presets from a directory
    pub fn load_presets_from_directory(&mut self, path: &str) -> Result<()> {
        let path = Path::new(path);
        if !path.exists() {
            return Err(anyhow::anyhow!("Directory does not exist: {}", path.display()));
        }
        
        self.current_directory = Some(path.to_path_buf());
        self.preset_directories.clear();
        self.categories.clear();
        self.nested_categories.clear();
        
        self.scan_directory(path, None)?;
        
        log::info!("🎯 Navigator: Loaded {} categories with nested structure", self.categories.len());
        for (category, subcats) in &self.nested_categories {
            log::info!("🎯 Navigator: Category '{}' has {} subcategories", category, subcats.len());
        }
        
        Ok(())
    }
    
    /// Scan a directory for presets with support for nested structure
    fn scan_directory(&mut self, path: &Path, parent_category: Option<&str>) -> Result<()> {
        if path.is_dir() {
            let current_category = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string();
            
            let mut _has_subdirs = false;
            let mut has_presets = false;
            let mut subcategories = Vec::new();
            
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let entry_path = entry.path();
                
                if entry_path.is_dir() {
                    _has_subdirs = true;
                    let subcategory_name = entry_path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Unknown")
                        .to_string();
                    subcategories.push(subcategory_name.clone());
                    
                    // Recursively scan subdirectories
                    self.scan_directory(&entry_path, Some(&current_category))?;
                } else if entry_path.extension().and_then(|s| s.to_str()) == Some("milk") {
                    has_presets = true;
                    // Add preset to current directory's category
                    self.add_preset(&entry_path, path, parent_category)?;
                }
            }
            
            // Store nested structure
            if !subcategories.is_empty() {
                self.nested_categories.insert(current_category.clone(), subcategories);
            }
            
            // Only add to categories if this directory has presets (leaf nodes)
            if has_presets {
                let full_path = if let Some(parent) = parent_category {
                    format!("{}/{}", parent, current_category)
                } else {
                    current_category.clone()
                };
                
                if !self.categories.contains(&full_path) {
                    self.categories.push(full_path);
                }
            }
        }
        
        Ok(())
    }
    
    /// Add a preset to the navigator with full path tracking
    fn add_preset(&mut self, preset_path: &Path, category_path: &Path, parent_category: Option<&str>) -> Result<()> {
        let preset_name = preset_path.file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();
        
        let category_name = category_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();
        
        // Create full relative path for display (this should match the categories list)
        let full_path = if let Some(parent) = parent_category {
            format!("{}/{}", parent, category_name)
        } else {
            category_name.clone()
        };
        
        let preset_info = PresetInfo {
            name: preset_name,
            path: preset_path.to_path_buf(),
            category: category_name.clone(),
            subcategory: parent_category.map(|s| s.to_string()),
            full_path: full_path.clone(),
        };
        
        // Add to category using full path as key
        self.preset_directories.entry(full_path.clone())
            .or_insert_with(Vec::new)
            .push(preset_info);
        
        Ok(())
    }
    
    /// Get all available categories (including nested ones)
    pub fn get_categories(&self) -> Vec<String> {
        self.categories.clone()
    }
    
    /// Get presets in a specific category
    pub fn get_presets_in_category(&self, category: &str) -> Vec<String> {
        if let Some(presets) = self.preset_directories.get(category) {
            presets.iter().map(|p| p.name.clone()).collect()
        } else {
            Vec::new()
        }
    }
    
    /// Get preset path by name and category
    pub fn get_preset_path(&self, category: &str, preset_name: &str) -> Option<PathBuf> {
        if let Some(presets) = self.preset_directories.get(category) {
            for preset in presets {
                if preset.name == preset_name {
                    return Some(preset.path.clone());
                }
            }
        }
        None
    }
    
    /// Get preset info by name and category
    pub fn get_preset_info(&self, category: &str, preset_name: &str) -> Option<&PresetInfo> {
        if let Some(presets) = self.preset_directories.get(category) {
            presets.iter().find(|p| p.name == preset_name)
        } else {
            None
        }
    }
    
    /// Get all presets across all categories
    pub fn get_all_presets(&self) -> Vec<(String, String)> {
        let mut all_presets = Vec::new();
        for (category, presets) in &self.preset_directories {
            for preset in presets {
                all_presets.push((category.clone(), preset.name.clone()));
            }
        }
        all_presets
    }
    
    /// Search presets by name
    pub fn search_presets(&self, query: &str) -> Vec<(String, String)> {
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
    
    /// Get preset statistics
    pub fn get_statistics(&self) -> PresetStatistics {
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
    
    /// Get nested categories for a parent category
    pub fn get_nested_categories(&self, parent_category: &str) -> Vec<String> {
        self.nested_categories.get(parent_category)
            .cloned()
            .unwrap_or_default()
    }
}

/// Statistics about loaded presets
#[derive(Debug, Clone)]
pub struct PresetStatistics {
    pub total_presets: usize,
    pub total_categories: usize,
    pub category_counts: HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigator_creation() {
        let navigator = PresetNavigator::new();
        assert_eq!(navigator.get_categories().len(), 0);
    }

    #[test]
    fn test_category_management() {
        let mut navigator = PresetNavigator::new();
        // Test would require actual directory structure
        assert_eq!(navigator.get_categories().len(), 0);
    }
} 