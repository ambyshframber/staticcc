use std::collections::HashMap;
use std::fs::read_to_string;
use std::io;
use std::path::Path;
use std::ffi::OsStr;

use thiserror::Error;
use fancy_regex::Regex;
use crate::rss::RssError;

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

pub fn replace_all_unescaped(s: &str, pat: &str, rep: &str) -> String {
    let mut ret = String::from(s);
    loop {
        let idxs = find_all_unescaped(&ret, pat);
        if idxs.len() == 0 {
            break
        }
        else {
            ret.replace_range(idxs[0]..idxs[0] + pat.len(), rep)
        }
    }
    ret
}

pub fn replace_unused_tags(s: &str) -> String {
    let re = Regex::new(r"(^|[^\\])##([^#\n]+)##").unwrap();
    re.replace_all(s, "").to_string()
    // TODO: remove escapes from escaped tags
}

pub fn split_doc(mut doc: &str) -> Result<(&str, HashMap<String, String>), StcError> {
    let mut ret = HashMap::new();

    let front_matter = if doc.starts_with("---\n") { // extract front matter
        let fm_end = doc.find("\n---\n").ok_or(StcError::BadFrontMatter)?;
        let fm = &doc[4..fm_end];
        doc = &doc[fm_end + 5..];
        fm
    }
    else { // if no front matter found, empty string
        ""
    };
    //println!("{}", doc);

    let re = Regex::new(r"(^|[^\\])##([^#\n]+)##").unwrap(); // ok this is where it gets funky
    let mut caps = re.captures_iter(doc).peekable(); // we need peekable. you will see why later
    loop {
        let c = match caps.next() { // get the next section label
            Some(v) => v?,
            None => break
        }; // catch errors

        let name = c.get(2).unwrap().as_str(); // extract section name

        let full = c.get(0).unwrap(); // get full match, for start/end positions
        let sec_start = full.end(); // section starts where start label ends
        let content = match caps.peek() { // get NEXT label to find sec end
            Some(m2_r) => {
                let m2 = m2_r.as_ref().unwrap(); // unwrap has to be used here, because borrow checker fuckery
                let sec_end = m2.get(0).unwrap().start();
                &doc[sec_start..sec_end]
            }
            None => { // if no next label, current sec goes to document end
                &doc[sec_start..]
            }
        };

        //println!("containing {}", content);

        ret.insert(String::from(name), String::from(content));
    }

    Ok((front_matter, ret))
}

pub fn read_or_none(p: impl AsRef<Path>) -> Result<Option<String>, StcError> {
    //println!("reading {} or none", p.as_ref().to_string_lossy());
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
        None => return Err(StcError::CfgErr(String::from(s)))
    }
}
pub fn parse_shit_markup(s: &str) -> Result<Vec<(String, String)>, StcError> {
    let blocks = s.split("\n----\n"); // get blocks
    let mut ret = Vec::new();
    for b in blocks {
        match b.split_once('\n') {
            Some((name, body)) => { // if its multi line, multi line it
                ret.push((name.trim().into(), body.into()))
            }
            None => { // if it's not, single line it
                ret.push(parse_rep(b)?)
            }
        }
    }
    Ok(ret)
}
pub fn parse_singleline_scf(s: &str) -> Result<Vec<(String, String)>, StcError> {
    let mut ret = Vec::new();
    for line in s.split('\n') {
        ret.push(parse_rep(line)?)
    }
    Ok(ret)
}
pub fn scf_to_hashmap(cfg: Vec<(String, String)>) -> HashMap<String, String> {
    let mut ret = HashMap::new();
    for (k, v) in cfg {
        ret.insert(k, v);
    }
    ret
}

pub fn os_str_to_str_or_err(s: &OsStr) -> Result<&str, StcError> { // helper fn
    match s.to_str() {
        Some(v) => Ok(v),
        None => Err(StcError::PathErr(s.to_string_lossy().to_string()))
    }
}

pub fn is_markdown(p: impl AsRef<Path>) -> Result<bool, StcError> {
    let ext = match p.as_ref().extension() { // if no extension, it's not markdown
        Some(v) => v,
        None => return Ok(false)
    };
    let ext_uni = os_str_to_str_or_err(ext)?; // error out on non-unicode
    Ok(if ext_uni == "md" {
        true
    }
    else {
        false
    })
}

/*pub trait OptionHelpers<T> {
    fn convert_inner<U, F>(self, f: F) -> Option<U>
    where F: FnOnce(T) -> U;
}
impl<T> OptionHelpers<T> for Option<T> {
    fn convert_inner<U, F>(self, f: F) -> Option<U>
    where F: FnOnce(T) -> U {
        match self {
            None => None,
            Some(v) => {
                Some(f(v))
            }
        }
    }
}*/

#[derive(Error, Debug)]
pub enum StcError {
    #[error("bad front matter formatting")]
    BadFrontMatter,
    #[error("regex error")]
    RegexErr(#[from]fancy_regex::Error),
    #[error("internal fs error")]
    FsError(#[from]io::Error),
    #[error("malformed config")]
    CfgErr(String),
    #[error("non-unicode path")]
    PathErr(String),
    #[error("missing template error")]
    TemplateError(String),
    #[error("blog data error")]
    RssError(#[from] RssError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_all_unescaped_t() {
        let s = "test string ##HEAD## \\##HEAD## test";
        let idxs = find_all_unescaped(s, "##HEAD##");
        assert_eq!(idxs, vec![12])
    }
    #[test]
    fn replace_all_unescaped_t() {
        let s = "test string ##HEAD## \\##HEAD## test ##HEAD## padding";
        let output = replace_all_unescaped(s, "##HEAD##", "aaa");
        assert_eq!(output, String::from("test string aaa \\##HEAD## test aaa padding"))
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
