use anyhow::Context;

fn parse_number(input: &str) -> Result<i32, std::num::ParseIntError> {
    input.parse::<i32>()
}

fn read_file_content(path: &str) -> Result<String, std::io::Error> {
    std::fs::read_to_string(path)
}

fn read_and_parse(path: &str) -> anyhow::Result<i32> {
    let content = read_file_content(path).context("failed to read input file")?;
    let number = parse_number(content.trim())?;
    Ok(number)
}

fn main() {
    match read_and_parse("nonexistent.txt") {
        Ok(n) => println!("Number: {n}"),
        Err(e) => println!("Error: {e:?}"),
    }
}