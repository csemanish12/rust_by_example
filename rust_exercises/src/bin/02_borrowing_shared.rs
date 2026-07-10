fn print_it(s: &String) {
    println!("{s}");
}

fn main(){
    let s1 = String::from("hello");

    print_it(&s1);
    print_it(&s1);

    println!("s1 is {s1}");
}