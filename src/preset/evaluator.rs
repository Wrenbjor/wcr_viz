use anyhow::{Result, anyhow};
use std::collections::HashMap;
use crate::preset::PresetVariables;

/// Expression evaluator for MilkDrop preset equations
pub struct ExpressionEvaluator {
    variables: PresetVariables,
    custom_vars: HashMap<String, f32>,
}

impl ExpressionEvaluator {
    /// Create a new expression evaluator
    pub fn new(variables: &PresetVariables) -> Self {
        Self {
            variables: variables.clone(),
            custom_vars: HashMap::new(),
        }
    }
    
    /// Evaluate a MilkDrop equation
    pub fn evaluate(&mut self, equation: &str) -> Result<f32> {
        let equation = equation.trim();
        
        // Handle assignment: var=expression
        if let Some(equal_pos) = equation.find('=') {
            let var_name = equation[..equal_pos].trim();
            let expression = equation[equal_pos + 1..].trim();
            
            let result = self.evaluate_expression(expression)?;
            self.set_variable(var_name, result)?;
            
            Ok(result)
        } else {
            // Just evaluate the expression
            self.evaluate_expression(equation)
        }
    }
    
    /// Evaluate a mathematical expression
    fn evaluate_expression(&self, expression: &str) -> Result<f32> {
        let tokens = self.tokenize(expression)?;
        let postfix = self.infix_to_postfix(tokens)?;
        self.evaluate_postfix(postfix)
    }
    
    /// Tokenize the expression into tokens
    fn tokenize(&self, expression: &str) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        let mut current = String::new();
        let mut i = 0;
        
        while i < expression.len() {
            let ch = expression.chars().nth(i).unwrap();
            
            match ch {
                ' ' | '\t' | '\n' => {
                    // Skip whitespace
                    if !current.is_empty() {
                        tokens.push(self.create_token(&current)?);
                        current.clear();
                    }
                }
                '(' | ')' | '+' | '-' | '*' | '/' | '^' | ',' => {
                    // Handle operators and parentheses
                    if !current.is_empty() {
                        tokens.push(self.create_token(&current)?);
                        current.clear();
                    }
                    tokens.push(self.create_operator_token(ch)?);
                }
                _ => {
                    current.push(ch);
                }
            }
            
            i += 1;
        }
        
        // Handle any remaining token
        if !current.is_empty() {
            tokens.push(self.create_token(&current)?);
        }
        
        Ok(tokens)
    }
    
    /// Create a token from a string
    fn create_token(&self, s: &str) -> Result<Token> {
        // Check if it's a number
        if let Ok(num) = s.parse::<f32>() {
            return Ok(Token::Number(num));
        }
        
        // Check if it's a variable
        if s.starts_with('q') && s.len() > 1 {
            if let Ok(index) = s[1..].parse::<usize>() {
                if index >= 1 && index <= 64 {
                    return Ok(Token::Variable(format!("q{}", index)));
                }
            }
        }
        
        // Check if it's a built-in variable
        match s {
            "time" | "frame" | "bass" | "mid" | "treb" | "vol" | "mouse_x" | "mouse_y" |
            "pixelsx" | "pixelsy" | "bass_att" | "mid_att" | "treb_att" | "vol_att" => {
                Ok(Token::Variable(s.to_string()))
            }
            // Check if it's a function
            "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "sinh" | "cosh" | "tanh" |
            "log" | "log10" | "exp" | "sqrt" | "abs" | "floor" | "ceil" | "round" |
            "min" | "max" | "pow" | "rand" | "if" | "int" => {
                Ok(Token::Function(s.to_string()))
            }
            _ => {
                // Assume it's a custom variable
                Ok(Token::Variable(s.to_string()))
            }
        }
    }
    
    /// Create an operator token
    fn create_operator_token(&self, ch: char) -> Result<Token> {
        match ch {
            '(' => Ok(Token::LeftParen),
            ')' => Ok(Token::RightParen),
            '+' => Ok(Token::Operator(Operator::Add)),
            '-' => Ok(Token::Operator(Operator::Subtract)),
            '*' => Ok(Token::Operator(Operator::Multiply)),
            '/' => Ok(Token::Operator(Operator::Divide)),
            '^' => Ok(Token::Operator(Operator::Power)),
            ',' => Ok(Token::Comma),
            _ => Err(anyhow!("Unknown operator: {}", ch)),
        }
    }
    
    /// Convert infix expression to postfix (Reverse Polish Notation)
    fn infix_to_postfix(&self, tokens: Vec<Token>) -> Result<Vec<Token>> {
        let mut output = Vec::new();
        let mut stack = Vec::new();
        
        for token in tokens {
            match token {
                Token::Number(_) | Token::Variable(_) => {
                    output.push(token);
                }
                Token::Function(_) => {
                    stack.push(token);
                }
                Token::LeftParen => {
                    stack.push(token);
                }
                Token::RightParen => {
                    while let Some(top) = stack.pop() {
                        match top {
                            Token::LeftParen => break,
                            _ => output.push(top),
                        }
                    }
                }
                Token::Operator(op) => {
                    while let Some(top) = stack.last() {
                        match top {
                            Token::Operator(top_op) if top_op.precedence() >= op.precedence() => {
                                output.push(stack.pop().unwrap());
                            }
                            Token::Function(_) => {
                                output.push(stack.pop().unwrap());
                            }
                            _ => break,
                        }
                    }
                    stack.push(Token::Operator(op));
                }
                Token::Comma => {
                    // Handle function arguments
                    while let Some(top) = stack.last() {
                        match top {
                            Token::LeftParen => break,
                            _ => output.push(stack.pop().unwrap()),
                        }
                    }
                }
            }
        }
        
        while let Some(token) = stack.pop() {
            output.push(token);
        }
        
        Ok(output)
    }
    
    /// Evaluate postfix expression
    fn evaluate_postfix(&self, tokens: Vec<Token>) -> Result<f32> {
        let mut stack = Vec::new();
        
        for token in tokens {
            match token {
                Token::Number(num) => {
                    stack.push(num);
                }
                Token::Variable(var_name) => {
                    let value = self.get_variable_value(&var_name)?;
                    stack.push(value);
                }
                Token::Operator(op) => {
                    if stack.len() < 2 {
                        return Err(anyhow!("Insufficient operands for operator"));
                    }
                    
                    let b = stack.pop().unwrap();
                    let a = stack.pop().unwrap();
                    
                    let result = match op {
                        Operator::Add => a + b,
                        Operator::Subtract => a - b,
                        Operator::Multiply => a * b,
                        Operator::Divide => {
                            if b == 0.0 {
                                return Err(anyhow!("Division by zero"));
                            }
                            a / b
                        }
                        Operator::Power => a.powf(b),
                    };
                    
                    stack.push(result);
                }
                Token::Function(func_name) => {
                    let result = self.evaluate_function(&func_name, &mut stack)?;
                    stack.push(result);
                }
                _ => {}
            }
        }
        
        if stack.len() != 1 {
            return Err(anyhow!("Invalid expression"));
        }
        
        Ok(stack.pop().unwrap())
    }
    
    /// Get the value of a variable
    fn get_variable_value(&self, var_name: &str) -> Result<f32> {
        match var_name {
            "time" => Ok(self.variables.time),
            "frame" => Ok(self.variables.frame as f32),
            "bass" => Ok(self.variables.bass),
            "mid" => Ok(self.variables.mid),
            "treb" => Ok(self.variables.treb),
            "vol" => Ok(self.variables.vol),
            "mouse_x" => Ok(self.variables.mouse_x),
            "mouse_y" => Ok(self.variables.mouse_y),
            "pixelsx" => Ok(1920.0), // Default resolution, should be configurable
            "pixelsy" => Ok(1080.0), // Default resolution, should be configurable
            "bass_att" => Ok(self.variables.bass), // For now, same as bass
            "mid_att" => Ok(self.variables.mid),   // For now, same as mid
            "treb_att" => Ok(self.variables.treb), // For now, same as treb
            "vol_att" => Ok(self.variables.vol),   // For now, same as vol
            _ => {
                // Check if it's a q variable
                if var_name.starts_with('q') && var_name.len() > 1 {
                    if let Ok(index) = var_name[1..].parse::<usize>() {
                        if index >= 1 && index <= 64 && index - 1 < self.variables.q.len() {
                            return Ok(self.variables.q[index - 1]);
                        }
                    }
                }
                
                // Check custom variables
                if let Some(value) = self.variables.custom.get(var_name) {
                    return Ok(*value);
                }
                
                // Return 0 for undefined variables (MilkDrop behavior)
                Ok(0.0)
            }
        }
    }
    
    /// Set a variable value
    fn set_variable(&mut self, var_name: &str, value: f32) -> Result<()> {
        match var_name {
            "time" => self.variables.time = value,
            "frame" => self.variables.frame = value as u32,
            "bass" => self.variables.bass = value,
            "mid" => self.variables.mid = value,
            "treb" => self.variables.treb = value,
            "vol" => self.variables.vol = value,
            "mouse_x" => self.variables.mouse_x = value,
            "mouse_y" => self.variables.mouse_y = value,
            _ => {
                // Check if it's a q variable
                if var_name.starts_with('q') && var_name.len() > 1 {
                    if let Ok(index) = var_name[1..].parse::<usize>() {
                        if index >= 1 && index <= 64 {
                            if index - 1 >= self.variables.q.len() {
                                self.variables.q.resize(64, 0.0);
                            }
                            self.variables.q[index - 1] = value;
                            return Ok(());
                        }
                    }
                }
                
                // Set as custom variable
                self.variables.custom.insert(var_name.to_string(), value);
            }
        }
        
        Ok(())
    }
    
    /// Evaluate a function
    fn evaluate_function(&self, func_name: &str, stack: &mut Vec<f32>) -> Result<f32> {
        match func_name {
            "sin" => {
                if stack.is_empty() {
                    return Err(anyhow!("Insufficient arguments for sin"));
                }
                Ok(stack.pop().unwrap().sin())
            }
            "cos" => {
                if stack.is_empty() {
                    return Err(anyhow!("Insufficient arguments for cos"));
                }
                Ok(stack.pop().unwrap().cos())
            }
            "tan" => {
                if stack.is_empty() {
                    return Err(anyhow!("Insufficient arguments for tan"));
                }
                Ok(stack.pop().unwrap().tan())
            }
            "asin" => {
                if stack.is_empty() {
                    return Err(anyhow!("Insufficient arguments for asin"));
                }
                Ok(stack.pop().unwrap().asin())
            }
            "acos" => {
                if stack.is_empty() {
                    return Err(anyhow!("Insufficient arguments for acos"));
                }
                Ok(stack.pop().unwrap().acos())
            }
            "atan" => {
                if stack.is_empty() {
                    return Err(anyhow!("Insufficient arguments for atan"));
                }
                Ok(stack.pop().unwrap().atan())
            }
            "sinh" => {
                if stack.is_empty() {
                    return Err(anyhow!("Insufficient arguments for sinh"));
                }
                Ok(stack.pop().unwrap().sinh())
            }
            "cosh" => {
                if stack.is_empty() {
                    return Err(anyhow!("Insufficient arguments for cosh"));
                }
                Ok(stack.pop().unwrap().cosh())
            }
            "tanh" => {
                if stack.is_empty() {
                    return Err(anyhow!("Insufficient arguments for tanh"));
                }
                Ok(stack.pop().unwrap().tanh())
            }
            "log" => {
                if stack.is_empty() {
                    return Err(anyhow!("Insufficient arguments for log"));
                }
                Ok(stack.pop().unwrap().ln())
            }
            "log10" => {
                if stack.is_empty() {
                    return Err(anyhow!("Insufficient arguments for log10"));
                }
                Ok(stack.pop().unwrap().log10())
            }
            "exp" => {
                if stack.is_empty() {
                    return Err(anyhow!("Insufficient arguments for exp"));
                }
                Ok(stack.pop().unwrap().exp())
            }
            "sqrt" => {
                if stack.is_empty() {
                    return Err(anyhow!("Insufficient arguments for sqrt"));
                }
                let x = stack.pop().unwrap();
                if x < 0.0 {
                    return Err(anyhow!("Square root of negative number"));
                }
                Ok(x.sqrt())
            }
            "abs" => {
                if stack.is_empty() {
                    return Err(anyhow!("Insufficient arguments for abs"));
                }
                Ok(stack.pop().unwrap().abs())
            }
            "floor" => {
                if stack.is_empty() {
                    return Err(anyhow!("Insufficient arguments for floor"));
                }
                Ok(stack.pop().unwrap().floor())
            }
            "ceil" => {
                if stack.is_empty() {
                    return Err(anyhow!("Insufficient arguments for ceil"));
                }
                Ok(stack.pop().unwrap().ceil())
            }
            "round" => {
                if stack.is_empty() {
                    return Err(anyhow!("Insufficient arguments for round"));
                }
                Ok(stack.pop().unwrap().round())
            }
            "min" => {
                if stack.len() < 2 {
                    return Err(anyhow!("Insufficient arguments for min"));
                }
                let b = stack.pop().unwrap();
                let a = stack.pop().unwrap();
                Ok(a.min(b))
            }
            "max" => {
                if stack.len() < 2 {
                    return Err(anyhow!("Insufficient arguments for max"));
                }
                let b = stack.pop().unwrap();
                let a = stack.pop().unwrap();
                Ok(a.max(b))
            }
            "pow" => {
                if stack.len() < 2 {
                    return Err(anyhow!("Insufficient arguments for pow"));
                }
                let b = stack.pop().unwrap();
                let a = stack.pop().unwrap();
                Ok(a.powf(b))
            }
            "rand" => {
                // Simple random number generator (0.0 to 1.0)
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                use std::time::SystemTime;
                
                let mut hasher = DefaultHasher::new();
                SystemTime::now().hash(&mut hasher);
                let hash = hasher.finish();
                Ok((hash as f32) / (u64::MAX as f32))
            }
            "if" => {
                // MilkDrop if function: if(condition, true_value, false_value)
                if stack.len() < 3 {
                    return Err(anyhow!("Insufficient arguments for if"));
                }
                let false_value = stack.pop().unwrap();
                let true_value = stack.pop().unwrap();
                let condition = stack.pop().unwrap();
                Ok(if condition != 0.0 { true_value } else { false_value })
            }
            "int" => {
                // MilkDrop int function: converts float to integer
                if stack.is_empty() {
                    return Err(anyhow!("Insufficient arguments for int"));
                }
                Ok(stack.pop().unwrap().floor())
            }
            _ => Err(anyhow!("Unknown function: {}", func_name)),
        }
    }
    
    /// Get the current variables
    pub fn get_variables(&self) -> &PresetVariables {
        &self.variables
    }
    
    /// Get mutable access to variables
    pub fn get_variables_mut(&mut self) -> &mut PresetVariables {
        &mut self.variables
    }
}

/// Token types for expression parsing
#[derive(Debug, Clone)]
enum Token {
    Number(f32),
    Variable(String),
    Function(String),
    Operator(Operator),
    LeftParen,
    RightParen,
    Comma,
}

/// Mathematical operators
#[derive(Debug, Clone)]
enum Operator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Power,
}

impl Operator {
    fn precedence(&self) -> u8 {
        match self {
            Operator::Power => 3,
            Operator::Multiply | Operator::Divide => 2,
            Operator::Add | Operator::Subtract => 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_simple_arithmetic() {
        let variables = PresetVariables::default();
        let mut evaluator = ExpressionEvaluator::new(&variables);
        
        assert_eq!(evaluator.evaluate("2+3").unwrap(), 5.0);
        assert_eq!(evaluator.evaluate("5-2").unwrap(), 3.0);
        assert_eq!(evaluator.evaluate("4*3").unwrap(), 12.0);
        assert_eq!(evaluator.evaluate("10/2").unwrap(), 5.0);
    }
    
    #[test]
    fn test_variable_assignment() {
        let variables = PresetVariables::default();
        let mut evaluator = ExpressionEvaluator::new(&variables);
        
        evaluator.evaluate("q1=5").unwrap();
        assert_eq!(evaluator.get_variables().q[0], 5.0);
        
        evaluator.evaluate("q2=q1+3").unwrap();
        assert_eq!(evaluator.get_variables().q[1], 8.0);
    }
    
    #[test]
    fn test_functions() {
        let variables = PresetVariables::default();
        let mut evaluator = ExpressionEvaluator::new(&variables);
        
        assert!((evaluator.evaluate("sin(0)").unwrap() - 0.0).abs() < 0.001);
        assert!((evaluator.evaluate("cos(0)").unwrap() - 1.0).abs() < 0.001);
        assert!((evaluator.evaluate("sqrt(4)").unwrap() - 2.0).abs() < 0.001);
    }
    
    #[test]
    fn test_complex_expression() {
        let variables = PresetVariables::default();
        let mut evaluator = ExpressionEvaluator::new(&variables);
        
        let result = evaluator.evaluate("sin(time)*0.5+cos(time)*0.3").unwrap();
        assert!(result.is_finite());
    }
} 