fn main() {
    let x = 5;
    let y = x;

    // Both x and y stay valid — i32 implements the Copy trait, so
    // assignment copies the value instead of moving it. No move error
    // here, unlike the String case in 02_ownership_move.rs.
    println!("x = {x}");
    println!("y = {y}");
}
