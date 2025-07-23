use regex::Regex;

fn main() {
    let re = Regex::new(r"test").unwrap();
    println!("Regex test successful!");
} 