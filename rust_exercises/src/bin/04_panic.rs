fn main() {
    let content = std::fs::read_to_string("nonexistent.txt").unwrap();
    println!("content is: {content}");
}