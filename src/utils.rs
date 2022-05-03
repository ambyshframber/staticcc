use std::collections::HashMap;
use std::fs::read_to_string;
use std::io;
use std::path::Path;
use std::ffi::OsStr;

use thiserror::Error;
use fancy_regex::Regex;

pub fn find_all_unescaped(s: &str, pat: &str) -> Vec<usize> {
    let mat = s.match_indices(pat);
    let mut ret1 = Vec::new();
    for m in mat {
        ret1.push(m.0) // get all indices and discard strings
    }

    let b = s.as_bytes(); // allow indexing
    let mut ret2 = Vec::new();
    for mat in ret1 { // check every match
        if mat > 0 {
            if b[mat - 1] != b'\\' { // if it's unescaped, add idx to ret
                ret2.push(mat)
            }
        }
    }

    ret2
}

pub fn split_doc(mut doc: &str) -> Result<(&str, HashMap<String, String>), StcError> { // 0 idx is front matter
    let mut ret = HashMap::new();

    let front_matter = if doc.starts_with("---\n") {
        let fm_end = doc.find("\n---\n").ok_or(StcError::BadFrontMatter)?;
        let fm = &doc[4..fm_end];
        doc = &doc[fm_end + 5..];
        fm
    }
    else {
        ""
    };
    println!("{}", doc);

    let re = Regex::new(r"(^|[^\\])##(.*)##").unwrap();
    let mut caps = re.captures_iter(doc).peekable();
    loop {
        let c = match caps.next() {
            Some(v) => v?,
            None => break
        }; // catch errors

        let name = c.get(2).unwrap().as_str();
        //println!("found {}", name.as_str());
        //println!("{} to {}", name.start(), name.end());

        let full = c.get(0).unwrap();
        let sec_start = full.end();
        //println!("{}", sec_start);
        let content = match caps.peek() {
            Some(m2_r) => {
                let m2 = m2_r.as_ref().unwrap();
                let sec_end = m2.get(0).unwrap().start();
                println!("{}", sec_end);
                &doc[sec_start..sec_end]
            }
            None => {
                //dbg!("none");
                &doc[sec_start..]
            }
        };

        println!("containing {}", content);

        ret.insert(String::from(name), String::from(content));
    }

    Ok((front_matter, ret))
}

pub fn read_or_none(p: impl AsRef<Path>) -> Result<Option<String>, StcError> {
    match read_to_string(p) {
        Ok(v) => {
            Ok(if v != "" {
                Some(v)
            }
            else {
                None // none if empty file
            })
        }
        Err(e) => {
            if e.kind() == io::ErrorKind::NotFound { // none if file doesnt exist
                Ok(None)
            }
            else {
                Err(StcError::from(e)) // error if read failed
            }
        }
    }
}

pub fn parse_rep(s: &str) -> Result<(String, String), StcError> {
    match s.split_once('=') {
        Some((name, body)) => Ok((name.trim().into(), body.trim().into())),
        None => return Err(StcError::RepErr(String::from(s)))
    }
}

pub fn parse_shit_markup(s: &str) -> Result<Vec<(String, String)>, StcError> {
    let blocks = s.split("\n----\n");
    let mut ret = Vec::new();
    for b in blocks {
        match b.split_once('\n') {
            Some((name, body)) => {
                ret.push((name.trim().into(), body.into()))
            }
            None => {
                ret.push(parse_rep(b)?)
            }
        }
    }
    Ok(ret)
}

pub fn os_str_to_str_or_err(s: &OsStr) -> Result<&str, StcError> {
    match s.to_str() {
        Some(v) => Ok(v),
        None => Err(StcError::PathErr(s.to_string_lossy().to_string()))
    }
}

pub fn is_markdown(p: impl AsRef<Path>) -> Result<bool, StcError> {
    let ext = match p.as_ref().extension() {
        Some(v) => v,
        None => return Ok(false)
    };
    let ext_uni = os_str_to_str_or_err(ext)?;
    Ok(if ext_uni == "md" {
        true  
    }
    else {
        false
    })
}

#[derive(Error, Debug)]
pub enum StcError {
    #[error("bad front matter formatting")]
    BadFrontMatter,
    #[error("regex error")]
    RegexErr(#[from]fancy_regex::Error),
    #[error("error opening file")]
    FsError(#[from]io::Error),
    #[error("malformed replacement")]
    RepErr(String),
    #[error("non-unicode path")]
    PathErr(String)
}

mod tests {
    use super::*;

    #[test]
    fn find_all_unescaped_t() {
        let s = "test string ##HEAD## \\##HEAD## test";
        let idxs = find_all_unescaped(s, "##HEAD##");
        assert_eq!(idxs, vec![12])
    }

    #[test]
    fn split_doc_t() {
        let s = r"---
test
---
##MAIN##
aaaa
\##TEST##
        ";
        let (fm, sections) = split_doc(s).unwrap();
        assert_eq!(fm, "test");
        assert!(sections.get("MAIN").is_some());
        assert!(sections.get("TEST").is_none());
    }

    #[test]
    fn shit_markup_test() {
        let m = r"name
value value
value
----
name2
val2
----
name3=val3";
        let p = parse_shit_markup(m);
        assert!(p.is_ok());
        let p = p.unwrap();
        assert_eq!(p[0], ("name".into(), "value value\nvalue".into()));
        assert_eq!(p[1], ("name2".into(), "val2".into()));
        assert_eq!(p[2], ("name3".into(), "val3".into()));
    }
}
