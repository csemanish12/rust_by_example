fn safe_divide(a: f64, b: f64) -> Option<f64> {
    if b == 0.0 {
        None
    } else {
        Some(a / b)
    }

}

fn main() {
    let results = [safe_divide(10.0, 2.0), safe_divide(5.0, 0.0)];

    for result in &results {
        match result {
            Some(value) => println!("Result is: {value}"),
            None => println!("Cannot divide by zero"),
        }
    }
}