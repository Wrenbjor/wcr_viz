use anyhow::Result;
use crate::ui::UIRenderer;

/// Simple text-based UI renderer for testing
pub struct SimpleUIRenderer {
    output_buffer: Vec<String>,
    width: u32,
    height: u32,
}

impl UIRenderer for SimpleUIRenderer {
    fn draw_text(&mut self, x: f32, y: f32, text: &str, _color: [f32; 4]) -> Result<()> {
        let line = format!("Text at ({}, {}): {}", x, y, text);
        self.output_buffer.push(line);
        Ok(())
    }
    
    fn draw_rect(&mut self, x: f32, y: f32, width: f32, height: f32, _color: [f32; 4]) -> Result<()> {
        let line = format!("Rect at ({}, {}) size {}x{}", x, y, width, height);
        self.output_buffer.push(line);
        Ok(())
    }
    
    fn draw_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, _color: [f32; 4]) -> Result<()> {
        let line = format!("Line from ({}, {}) to ({}, {})", x1, y1, x2, y2);
        self.output_buffer.push(line);
        Ok(())
    }

    fn get_window_dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

impl SimpleUIRenderer {
    /// Create a new simple UI renderer
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            output_buffer: Vec::new(),
            width,
            height,
        }
    }
    
    /// Get the rendered output
    pub fn get_output(&self) -> &[String] {
        &self.output_buffer
    }
    
    /// Clear the output buffer
    pub fn clear(&mut self) {
        self.output_buffer.clear();
    }
    
    /// Print the current output
    pub fn print_output(&self) {
        for line in &self.output_buffer {
            println!("{}", line);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_simple_renderer() {
        let mut renderer = SimpleUIRenderer::new(800, 600);
        
        renderer.draw_text(10.0, 10.0, "Hello World", [1.0, 1.0, 1.0, 1.0]).unwrap();
        renderer.draw_rect(50.0, 50.0, 100.0, 50.0, [0.5, 0.5, 0.5, 1.0]).unwrap();
        
        let output = renderer.get_output();
        assert_eq!(output.len(), 2);
        assert!(output[0].contains("Hello World"));
        assert!(output[1].contains("Rect"));
    }
} 