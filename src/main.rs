use proc::Processor;
use utils::StcError;

mod proc;
mod utils;
mod walkdir;

fn main() -> Result<(), StcError> {
    let p = Processor::new()?;
    //println!("{:?}", p);
    p.build()?;
    
    Ok(())
}
