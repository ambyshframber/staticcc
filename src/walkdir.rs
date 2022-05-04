use std::path::{Path, PathBuf};
use std::fs::{read_dir, ReadDir, DirEntry};
use std::iter::Iterator;
use std::io;

use crate::utils::StcError;

pub struct WalkDir {
    root: PathBuf,
    internal_iters: Vec<FatReadDir> // iter over the last until it runs out
}
impl WalkDir {
    pub fn new(path: impl AsRef<Path>) -> Result<WalkDir, StcError> {
        let root_iter = FatReadDir::new(&path, "")?;
        Ok(WalkDir {
            root: PathBuf::from(path.as_ref()),
            internal_iters: vec![root_iter]
        })
    }
}
impl Iterator for WalkDir {
    type Item = Result<PathBuf, StcError>;

    fn next(&mut self) -> Option<Result<PathBuf, StcError>> {
        loop {
            let it_len = self.internal_iters.len();
            if it_len == 0 { // out of iterators, ie. finished root rd
                break
            }
            let cur_iter = &mut self.internal_iters[it_len - 1]; // get top iter
            match cur_iter.next() {
                Some(v) => { // got something
                    match v {
                        Ok(v2) => { 
                            let path_from_wd_root = cur_iter.path.join(v2.file_name()); // get path from wd root
                            let full_path = self.root.join(&path_from_wd_root);

                            if full_path.is_dir() { // if it's a directory, iter it
                                let new_rd = FatReadDir::new(&full_path, &path_from_wd_root);
                                match new_rd {
                                    Ok(v) => {
                                        self.internal_iters.push(v);
                                        return Some(Ok(path_from_wd_root))
                                    }
                                    Err(e) => return Some(Err(e))
                                }
                            }
                            else {
                                return Some(Ok(path_from_wd_root))
                            }
                        }
                        Err(e) => return Some(Err(e.into()))
                    }
                }
                None => { // iter is done, go down a layer
                    let _ = self.internal_iters.pop();
                    continue
                }
            }
        }

        None
    }
}
struct FatReadDir {
    pub rd: ReadDir,
    pub path: PathBuf
}
impl FatReadDir {
    pub fn new(iter_path: impl AsRef<Path>, rel_path: impl AsRef<Path>) -> Result<FatReadDir, StcError> {
        let rd = read_dir(&iter_path)?;
        //println!("new frd at {}", rel_path.as_ref().to_string_lossy());
        Ok(FatReadDir{
            rd, path: PathBuf::from(&rel_path.as_ref())
        })
    }
}
impl Iterator for FatReadDir {
    type Item = io::Result<DirEntry>;
    fn next(&mut self) -> Option<Self::Item> {
        self.rd.next()
    }
}
