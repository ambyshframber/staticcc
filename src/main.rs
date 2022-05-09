use proc::Processor;
use utils::StcError;

mod proc;
mod utils;
mod walkdir;
mod rss;

fn main() -> Result<(), StcError> {
    let mut p = Processor::new()?;
    //println!("{:?}", p);
    p.build()?;
    
    Ok(())
}
