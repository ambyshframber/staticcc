use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::env::{current_dir, set_current_dir};
use std::fs::{read_to_string, read, copy};

use argparse::{ArgumentParser, StoreOption, Collect};

use crate::utils::{read_or_none, split_doc, StcError, parse_shit_markup, parse_rep, os_str_to_str_or_err, is_markdown};

#[derive(Default, Debug)]
pub struct Processor {
    inp_dir: PathBuf,
    out_dir: PathBuf,
    cfg_dir: PathBuf,

    md_ignore: Vec<PathBuf>,
    md_replace: HashMap<String, String>,
    md_templates: HashMap<String, String>
}
#[derive(Default)]
struct ProcOpts {
    pub dir: Option<String>,
    pub inp_dir: Option<String>,
    pub out_dir: Option<String>,
    pub cfg_dir: Option<String>,

    pub md_ignore: Vec<PathBuf>,
    pub md_replace: Vec<String>,
    pub md_templates: Vec<String>,
}

impl Processor {
    pub fn new() -> Result<Processor, StcError> {
        let po = ProcOpts::new(); // get command line options

        let dir = match po.dir {
            Some(v) => {
                set_current_dir(&v)?;
                PathBuf::from(v).canonicalize()?
            }

            None => current_dir()?
        };

        let mut p = Processor { // pull out easy stuff
            inp_dir: dir.join(po.inp_dir.unwrap_or_else(|| String::from("site"))),
            out_dir: dir.join(po.out_dir.unwrap_or_else(|| String::from("build"))),
            cfg_dir: dir.join(po.cfg_dir.unwrap_or_else(|| String::from("cfg"))),

            md_ignore: po.md_ignore,
            md_replace: HashMap::new(),
            md_templates: HashMap::new()
        }; // init struct set up

        // now plug everything in

        match read_or_none(p.cfg_dir.join("md_ignore"))? { // ignores from cfg
            Some(v) => {
                for ig in v.split('\n') {
                    p.md_ignore.push(PathBuf::from(ig));
                }
            }
            None => {}
        }

        let mut parsed = match read_or_none(p.cfg_dir.join("md_replace"))? { // reps from cfg
            Some(v) => {
                parse_shit_markup(&v)?
            }
            None => Vec::new()
        };
        for r in po.md_replace { // reps from command line
            parsed.push(parse_rep(&r)?)
        }
        for (t, r) in parsed { // insert into hashmap
            p.md_replace.insert(format!("REP={}", t), r);
        }

        let templates_dir = p.cfg_dir.join("templates");
        for i in templates_dir.read_dir()? {
            let path = i?.path();
            if !path.is_dir() {
                let name = String::from(os_str_to_str_or_err(path.file_name().unwrap())?); // should never be None
                let content = read_to_string(path)?;
                p.md_templates.insert(name, content);
            }
        }

        Ok(p)
    }

    /// path MUST be relative to input dir or This Will Not Work
    fn process_file(&self, path: impl AsRef<Path>) -> Result<(), StcError> {
        if is_markdown(&path)? && !self.md_ignore.iter().any(|ign| path.as_ref() == ign) { // markdown AND NOT ignored

        }
        else { // regular file
            copy(self.inp_dir.join(&path), self.out_dir.join(&path))?;
        }

        Ok(())
    }
}

impl ProcOpts {
    pub fn new() -> ProcOpts {
        let mut po = ProcOpts::default();
        
        {
            let mut ap = ArgumentParser::new();

            ap.refer(&mut po.dir).add_option(&["-d"], StoreOption, "the working directory");
            ap.refer(&mut po.inp_dir).add_option(&["-i"], StoreOption, "the input directory");
            ap.refer(&mut po.out_dir).add_option(&["-o"], StoreOption, "the output directory");
            ap.refer(&mut po.cfg_dir).add_option(&["-c"], StoreOption, "the config directory");

            ap.refer(&mut po.md_ignore).add_option(&["-I"], Collect, "a file to ignore, relative to the input dir");
            ap.refer(&mut po.md_replace).add_option(&["-R"], Collect, "a replacement to make in markdown");
            //ap.refer(&mut po.md_templates).add_option(&["-T"], Collect, "a path to a template file");

            ap.parse_args_or_exit()
        }

        po
    }
}