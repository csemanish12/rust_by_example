#[async_trait::async_trait]
trait Worker {
    async fn run(&self) -> String;
}

struct Fetcher {}
struct Cruncher {}


#[async_trait::async_trait]
impl Worker for Fetcher {
    async fn run(&self) -> String {
        format!("hello from Fetcher")
    }
}

#[async_trait::async_trait]
impl Worker for Cruncher {
    async fn run(&self) -> String {
        format!("Hello from Cruncher")
    }
}

#[tokio::main]
async fn main() {
    let fetcher = Fetcher{};
    println!("{}", fetcher.run().await);

    let cruncher = Cruncher{};
    println!("{}", cruncher.run().await);

    let workers: Vec<Box<dyn Worker>> = vec![Box::new(Fetcher {}), Box::new(Cruncher {})];
    for worker in workers.iter(){
        println!("{}", worker.run().await);
    }
}