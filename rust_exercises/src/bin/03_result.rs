fn safe_divide(a: f64, b: f64) -> Result<f64, String> {
    if b == 0.0 {
        Err(String::from("cannot divide by zero"))
    } else {
        Ok(a/b)
    }
}

fn main() {
    let results = [safe_divide(10.0, 5.0), safe_divide(5.0, 0.0)];

    for result in &results {
        match result {
            Err(message) => println!("error: {message}"),
            Ok(value) => println!("Result: {value}"),
        }
    }
}