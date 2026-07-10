fn main() {
    let s1 = String::from("hello");
    let s2 = s1.clone();

    // .clone() makes an explicit, deliberate deep copy, so s1 stays
    // valid too. Both prints work — but this cost a heap allocation.
    println!("s1 = {s1}");
    println!("s2 = {s2}");
}
