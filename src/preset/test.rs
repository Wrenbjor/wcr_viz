#[cfg(test)]
mod tests {
    use super::*;
    use crate::preset::{Preset, PresetParser, ExpressionEvaluator, PresetVariables};

    #[test]
    fn test_preset_creation() {
        let preset = Preset::new("Test Preset".to_string());
        assert_eq!(preset.metadata.name, "Test Preset");
        assert_eq!(preset.variables.q.len(), 64);
    }

    #[test]
    fn test_expression_evaluator() {
        let variables = PresetVariables::default();
        let mut evaluator = ExpressionEvaluator::new(&variables);
        
        // Test simple arithmetic
        assert_eq!(evaluator.evaluate("2+3").unwrap(), 5.0);
        assert_eq!(evaluator.evaluate("5-2").unwrap(), 3.0);
        assert_eq!(evaluator.evaluate("4*3").unwrap(), 12.0);
        assert_eq!(evaluator.evaluate("10/2").unwrap(), 5.0);
        
        // Test variable assignment
        evaluator.evaluate("q1=5").unwrap();
        assert_eq!(evaluator.get_variables().q[0], 5.0);
        
        evaluator.evaluate("q2=q1+3").unwrap();
        assert_eq!(evaluator.get_variables().q[1], 8.0);
    }

    #[test]
    fn test_preset_parser() {
        let preset_text = r#"
[preset00]
name="Simple Test Preset"
author="Test Author"
rating=4

per_frame_1=q1=q1+0.1
per_frame_2=q2=sin(time)*0.5

[per_pixel]
ret=ret*0.95
"#;
        
        let parser = PresetParser::new();
        let preset = parser.parse_text(preset_text).unwrap();
        
        assert_eq!(preset.metadata.name, "Simple Test Preset");
        assert_eq!(preset.metadata.author, Some("Test Author".to_string()));
        assert_eq!(preset.metadata.rating, Some(4));
        assert_eq!(preset.equations.per_frame.len(), 2);
        assert!(preset.equations.per_pixel.is_some());
    }

    #[test]
    fn test_preset_variables() {
        let mut preset = Preset::new("Test".to_string());
        
        // Test q variable access
        preset.set_q(0, 5.0);
        assert_eq!(preset.get_q(0), 5.0);
        
        // Test audio variable updates
        preset.update_audio_variables(0.5, 0.3, 0.2, 0.8);
        assert_eq!(preset.variables.bass, 0.5);
        assert_eq!(preset.variables.mid, 0.3);
        assert_eq!(preset.variables.treb, 0.2);
        assert_eq!(preset.variables.vol, 0.8);
        
        // Test time variable updates
        preset.update_time_variables(10.5, 100);
        assert_eq!(preset.variables.time, 10.5);
        assert_eq!(preset.variables.frame, 100);
    }
} 