fn parse_number(input: &str) -> Result<i32, std::num::ParseIntError> {
    input.parse::<i32>()
}


fn main() -> Result <(), std::num::ParseIntError>{
    // match parse_number("42") {
    //     Ok(value) => println!("parsed: {value}"),
    //     Err(e) => println!("Failed to parse: {e}" ),
    // }

    // match parse_number("not a number") {
    //     Ok(value) => println!("parsed number: {value}"),
    //     Err(e) => println!("Failed to parse: {e:?}"),
    // }

    let value = parse_number("42")?;  // => this is success
    //let value = parse_number("not a number")?;
    println!("Parsed: {value}");

    Ok(())
}