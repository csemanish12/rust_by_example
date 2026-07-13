fn main() {
    let mut s1 = String::from("hello");

    let r1 = &s1;
    println!("r1 is {r1}");

    let r2 = &mut s1;
    println!("r2 is {r2}");

    // println!("r1 is again {r1}");  // this will produce error
}