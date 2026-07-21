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
        format!("Person:{}", self.name)
    }
}

impl Greeter for Robot {
    fn name(&self) -> String {
        format!("unit-{}", self.id)
    }

    fn greet(&self) -> String {
        format!("BEEP BOOP I AM {}", self.name())
    }
}

fn announce<T: Greeter>(g: &T) {
    println!("Announcing: {}", g.greet());
}

fn main(){
    let person: Person = Person{name: String::from("Manish")};
    let robot: Robot = Robot{id: 123};

    announce(&person);
    announce(&robot);

}