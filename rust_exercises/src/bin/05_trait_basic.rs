trait Greeter {
    fn greet(&self) -> String;
}

struct Person {
    name: String,
}

impl Greeter for Person {
    fn greet(&self) -> String {
        format!("Hello, my name is {}", self.name)
    }
}

fn main(){
    let persion: Person = Person {name: String::from("Manish")};
    println!("the message is: {}",persion.greet());
}