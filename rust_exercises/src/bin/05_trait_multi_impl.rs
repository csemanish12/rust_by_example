trait Greeter {
    fn name(&self) -> String;
    fn greet(&self) -> String {
        format!("Hello, my name is {}", self.name())
    }
}

struct Person {
    name: String
}

struct Robot {
    id: u32
}

impl Greeter for Person {
    fn name(&self) -> String {
        format!("PersonName: {}",self.name)
    }
}

impl Greeter for Robot {
    fn name(&self) -> String {
        format!("Unit-{}", self.id)
    }

    fn greet(&self) -> String {
        format!("BEEP BOOP I AM {}", self.name())
    }
}

fn main(){
    let person: Person = Person{name: String::from("manish")};
    println!("{}", person.greet());

    let robot = Robot{id: 123};
    println!("{}", robot.greet());
}