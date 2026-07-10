fn main(){
    let mut s1 = String::from("hello");

    let r1 = &s1;
    let r2 = &mut s1;

    println!("ri is {r1} and r2 is {r2}");
}