use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::env::{current_dir, set_current_dir};
use std::fs::{read_to_string, write, copy, create_dir_all, remove_dir_all, File};

use argparse::{ArgumentParser, StoreOption, Collect, StoreTrue};
use comrak::{ComrakOptions, markdown_to_html};

use crate::walkdir::WalkDir;
use crate::utils::*;
use crate::rss::{FatChannel, RssItem, RssError, get_channels};

#[derive(Default, Debug)]
pub struct Processor {
    inp_dir: PathBuf,
    out_dir: PathBuf,
    cfg_dir: PathBuf,

    md_ignore: Vec<PathBuf>,
    md_replace: HashMap<String, String>,
    md_templates: HashMap<String, String>,
    md_options: ComrakOptions,

    rss_channels: HashMap<String, FatChannel>
}
#[derive(Default)]
struct ProcOpts {
    pub dir: Option<String>,
    pub inp_dir: Option<String>,
    pub out_dir: Option<String>,
    pub cfg_dir: Option<String>,

    pub md_ignore: Vec<PathBuf>,
    pub md_replace: Vec<String>,
    //pub md_templates: Vec<String>,
    pub md_options: ComrakOptions,
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
            inp_dir: dir.join(po.inp_dir.unwrap_or_else(|| String::from("site"))), // FIX THIS
            out_dir: dir.join(po.out_dir.unwrap_or_else(|| String::from("build"))), // at present relative i/b/c dirs wont work
            cfg_dir: dir.join(po.cfg_dir.unwrap_or_else(|| String::from("cfg"))), // because join will go relative to the main dir
            // on second thoughts im not sure i have enough spoons for a fix

            md_ignore: po.md_ignore,
            md_replace: HashMap::new(),
            md_templates: HashMap::new(),
            md_options: po.md_options,

            rss_channels: HashMap::new()
        }; // init struct set up

        if p.out_dir.exists() { // make sure build dir exists
            remove_dir_all(&p.out_dir)?;
        }
        create_dir_all(&p.out_dir)?;

        // now plug everything in

        println!("getting configs");

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
        p.md_replace = scf_to_hashmap(parsed);

        println!("finding templates");
        let templates_dir = p.cfg_dir.join("templates");
        for i in templates_dir.read_dir()? {
            let path = i?.path();
            if !path.is_dir() {
                let name = String::from(os_str_to_str_or_err(path.file_name().unwrap())?); // should never be None
                let content = read_to_string(path)?;
                println!("found {}", name);
                p.md_templates.insert(name, content);
            }
        }
        match read_or_none(p.cfg_dir.join("channels"))? {
            None => {},
            Some(v) => {
                let cfg = scf_to_hashmap(parse_shit_markup(&v)?);
                p.rss_channels = get_channels(&cfg)?;
            }
        }

        Ok(p)
    }

    /// path MUST be relative to input dir or This Will Not Work
    fn process_file(&mut self, path: impl AsRef<Path>) -> Result<(), StcError> {
        if is_markdown(&path)? && !self.md_ignore.iter().any(|ign| path.as_ref() == ign) { // markdown AND NOT ignored
            self.process_markdown(path)?
        }
        else { // regular file
            copy(self.inp_dir.join(&path), self.out_dir.join(&path))?;
        }

        Ok(())
    }

    fn process_markdown(&mut self, path: impl AsRef<Path>) -> Result<(), StcError> {
        println!("processing {}", path.as_ref().to_string_lossy());
        let md = read_to_string(self.inp_dir.join(&path))?;
        
        let (fm, document) = split_doc(&md)?;

        let mut out_path = self.out_dir.join(&path);
        out_path.set_extension("html");
        
        let mut cfg = HashMap::new(); // get cfg from front matter
        for c in fm.split('\n') {
            if c.trim() != "" {
                let (k, v) = parse_rep(c)?;
                cfg.insert(k, v);
            }
        }

        if let Some(_) = cfg.get("rss_chan_id") {
            let (channels, item) = RssItem::new(&cfg, path.as_ref())?;
            for id in channels {
                let chan = self.rss_channels.get_mut(&id).ok_or(RssError::ChannelNotFound(id.into()))?;
                chan.items.push(item.clone())
            }
        }

        let main = &String::from("main");
        let temp_name = cfg.get("template").unwrap_or(main);
        let mut template = self.md_templates.get(temp_name).ok_or(StcError::TemplateError(temp_name.to_owned()))?.to_owned();
        
        for (block_name, block) in document { // document blocks
            let rep_trigger = format!("##{}##", block_name);
            template = replace_all_unescaped(&template, &rep_trigger, &block);
        }
        for (k, v) in cfg { // fm configs
            let rep_trigger = format!("##{}##", k);
            template = replace_all_unescaped(&template, &rep_trigger, &v);
        }
        for (trig, rep) in &self.md_replace { // global reps
            template = replace_all_unescaped(&template, &format!("REP={}", trig), rep)
        }
        template = replace_unused_tags(&template);

        //println!("{}", template);

        let html = markdown_to_html(&template, &self.md_options);

        write(out_path, html)?;

        Ok(())
    }

    pub fn build(&mut self) -> Result<(), StcError> {
        println!("building site");

        let wd = WalkDir::new(&self.inp_dir)?;
        for entry in wd {
            let entry = entry?;
            println!("found {}", entry.to_string_lossy());
            if self.inp_dir.join(&entry).is_dir() {
                create_dir_all(self.out_dir.join(&entry))?;
            }
            else {
                self.process_file(entry)?;
            }
        }

        for (id, c) in &mut self.rss_channels {
            println!("finalising rss channel {}", id);
            let mut items = Vec::new();
            for i in &c.items {
                items.push(i.finalise(&c.prepend, id));
            }
            c.c.items.append(&mut items);
            let f = File::create(self.out_dir.join(&c.out_file))?;
            let _ = c.c.write_to(f);
        }

        Ok(())
    }
}

impl ProcOpts {
    pub fn new() -> ProcOpts {
        let mut po = ProcOpts::default();

        po.md_options.render.unsafe_ = true;
        
        {
            let mut ap = ArgumentParser::new();

            ap.refer(&mut po.dir).add_option(&["-d"], StoreOption, "the working directory");
            ap.refer(&mut po.inp_dir).add_option(&["-i"], StoreOption, "the input directory, relative to wd");
            ap.refer(&mut po.out_dir).add_option(&["-o"], StoreOption, "the output directory, relative to wd");
            ap.refer(&mut po.cfg_dir).add_option(&["-c"], StoreOption, "the config directory, relative to wd");

            ap.refer(&mut po.md_ignore).add_option(&["-I"], Collect, "a file to ignore, relative to the input dir");
            ap.refer(&mut po.md_replace).add_option(&["-R"], Collect, "a replacement to make in markdown");
            //ap.refer(&mut po.md_templates).add_option(&["-T"], Collect, "a path to a template file");

            ap.refer(&mut po.md_options.extension.strikethrough).add_option(&["-s"], StoreTrue, "strikethrough");
            ap.refer(&mut po.md_options.extension.table).add_option(&["-t"], StoreTrue, "tables");
            ap.refer(&mut po.md_options.extension.autolink).add_option(&["-a"], StoreTrue, "autolink");
            ap.refer(&mut po.md_options.extension.tasklist).add_option(&["-l"], StoreTrue, "tasklist");
            ap.refer(&mut po.md_options.extension.superscript).add_option(&["-S"], StoreTrue, "superscript");
            ap.refer(&mut po.md_options.extension.footnotes).add_option(&["-f"], StoreTrue, "footnotes");
            ap.refer(&mut po.md_options.extension.description_lists).add_option(&["-D"], StoreTrue, "description lists");

            ap.parse_args_or_exit()
        }

        po
    }
}
