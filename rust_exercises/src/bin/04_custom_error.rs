fn parse_number(input: &str) -> Result<i32, std::num::ParseIntError> {
    input.parse::<i32>()
}

fn read_file_content(path: &str) -> Result<String, std::io::Error> {
    std::fs::read_to_string(path)
}

#[derive(thiserror::Error, Debug)]
enum AppError{
    #[error("failed to parse number: {0}")]
    Parse(#[from] std::num::ParseIntError),

    #[error("failed to read file: {0}")]
    Io(#[from] std::io::Error),
}

fn read_and_parse(path: &str) -> Result<i32, AppError> {
    let content = read_file_content(path)?;
    let number = parse_number(content.trim())?;
    Ok(number)
} 

fn main() {
    match read_and_parse("nonexistent.txt") {
        Ok(n) => println!("Number: {n}"),
        Err(e) => println!("Error: {e}"),
    }
}