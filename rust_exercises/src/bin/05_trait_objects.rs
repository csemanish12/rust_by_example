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
        format!("Unit-{}", self.id)
    }

    fn greet(&self) -> String {
        format!("BEEP BOOP I AM {}", self.name())
    }
}


fn announce_dyn(g: &dyn Greeter) {
    println!("{}", g.greet());
}

fn main(){
    let person: Person = Person {name: String::from("Manish")};
    let robot: Robot = Robot {id: 123};

    let mut greeters: Vec<Box<dyn Greeter>> = Vec::new();

    greeters.push(Box::new(person));
    greeters.push(Box::new(robot));

    for item in greeters.iter(){
        announce_dyn(item.as_ref());
    }
}