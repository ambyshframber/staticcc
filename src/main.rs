use proc::Processor;

mod proc;
mod utils;

fn main() {
    let p = Processor::new();
    println!("{:?}", p)
}
