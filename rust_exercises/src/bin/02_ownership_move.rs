fn main() {
    let s1 = String::from("hello");
    let s2 = s1;

    // s1 is no longer valid here — uncomment the next line to see the
    // compiler reject it with error[E0382]: borrow of moved value: `s1`
    // println!("{s1}");

    println!("{s2}");
}
