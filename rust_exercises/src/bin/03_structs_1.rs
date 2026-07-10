struct Employee {
    name: String,
    age: u32,
}

impl Employee {
    fn describe(&self) -> String {
        format!("{} is {} years old", self.name, self.age)
    }
}

fn main() {
    let emp = Employee {
        name: String::from("Alex"),
        age: 30,
    };

    println!("{}", emp.describe());
}