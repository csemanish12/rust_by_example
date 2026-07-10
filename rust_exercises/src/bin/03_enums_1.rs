enum TrafficLight {
    Red,
    Yellow,
    Green,
    FlashingYellow(u32),
}

fn instruction(light: &TrafficLight) -> String{
    match light {
        TrafficLight::Red => String::from("Stop"),
        TrafficLight::Yellow => String::from("Slow Down"),
        TrafficLight::Green => String::from("Go"),
        TrafficLight::FlashingYellow(second) => format!("Flashing yellow, {second}s left"),
    }
}

fn main() {
    let lights = [TrafficLight::Red, TrafficLight::Yellow, TrafficLight::Green, TrafficLight::FlashingYellow(5)];

    for light in &lights {
        println!("instruction: {}", instruction(light));
    }

    println!("lights has {} elements", lights.len());
}