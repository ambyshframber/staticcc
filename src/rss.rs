use std::collections::HashMap;
use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};
use crate::utils::*;
use thiserror::Error;
use rss::{Channel, Item, ChannelBuilder, ItemBuilder, ImageBuilder, Guid};

#[derive(Debug)]
pub struct RssItem {
    page: String, // path relative to site root
    // guid is "{url}@{channel_id}"
    title: String,
    description: String,
    pubdate: Option<DateTime<Utc>>,
}

impl RssItem {
    pub fn new(front_matter: &HashMap<String, String>, path: &Path) -> Result<(String, RssItem), StcError> {
        let mut path = PathBuf::from(path);
        path.set_extension("html");
        let page = os_str_to_str_or_err(path.as_os_str())?.into();
        println!("creating rss item at {}", page);
        let channel_id = front_matter.get("rss_chan_id").unwrap().to_owned(); // will always exist, as this method will only be called when that key exists
        let title = match front_matter.get("rss_title") {
            Some(v) => v,
            None => {
                match front_matter.get("title") { // try page title as well, helps reduce magic numbers
                    Some(v) => v,
                    None => return Err(RssError::MissingTitle.into())
                }
            }
        }.to_owned();
        let pubdate = match front_matter.get("rss_pubdate") {
            None => None, // optional
            Some(v) => {
                Some(DateTime::parse_from_rfc2822(v).map_err(|e| RssError::BadPubdate(e))?.with_timezone(&Utc))
            }
        };
        let description = front_matter.get("rss_description").convert_inner(|s| s.to_owned()).unwrap_or("".into()); // required for rss, but not for staticcc

        let r = RssItem {
            pubdate, page, title, description
        };
        Ok((channel_id, r))
    }

    pub fn finalise(&self, prepend: &str) -> Item {
        let mut ib = ItemBuilder::default();
        ib.title(self.title.clone());
        ib.link(format!("{}{}", prepend, &self.page));
        let guid = Guid {
            value: format!("{}{}", prepend, &self.page),
            permalink: true
        };
        ib.guid(guid);
        ib.description(self.description.clone());
        ib.pub_date(self.pubdate.convert_inner(|d| d.to_rfc2822())); // chefs kiss

        ib.build()
    }
}

pub fn get_channels(cfg: &HashMap<String, String>) -> Result<HashMap<String, FatChannel>, StcError> {
    let mut ret = HashMap::new();

    for (id, data) in cfg {
        let cfg_inner = scf_to_hashmap(parse_singleline_scf(&data)?);
        let mut b = ChannelBuilder::default();
        let prepend = cfg_inner.get("prepend").ok_or(RssError::MissingPrepend)?;

        let title = cfg_inner.get("title").ok_or(RssError::MissingTitle)?;
        b.title(title);
        b.description(cfg_inner.get("description").unwrap_or(&"".into()));
        let link = format!("{}{}", prepend, cfg_inner.get("path").ok_or(RssError::MissingLink)?);
        b.link(&link);
        b.docs(String::from("https://www.rssboard.org/rss-specification"));
        match cfg_inner.get("image") {
            None => {},
            Some(v) => {
                let img_path = format!("{}{}", prepend, v);
                let mut ib = ImageBuilder::default();
                ib.url(img_path);
                ib.link(&link);
                ib.title(title);
                b.image(ib.build());
            }
        }

        let path = cfg_inner.get("outfile").ok_or(RssError::MissingPath)?;
        let fc = FatChannel::new(b.build(), path, prepend);
        ret.insert(id.into(), fc);
    }

    Ok(ret)
}

#[derive(Debug)]
pub struct FatChannel {
    pub c: Channel,
    pub out_file: PathBuf,
    pub items: Vec<RssItem>,
    pub prepend: String
}
impl FatChannel {
    pub fn new(c: Channel, p: impl AsRef<Path>, prepend: &str) -> FatChannel {
        FatChannel {
            c, items: Vec::new(),
            out_file: PathBuf::from(p.as_ref()),
            prepend: prepend.into()
        }
    }
}

#[derive(Error, Debug)]
pub enum RssError {
    #[error("rss item/channel with missing title")]
    MissingTitle,
    #[error("rss item with malformed pubdate")]
    BadPubdate(#[from] chrono::ParseError),
    #[error("rss item with non-existent channel id")]
    ChannelNotFound(String),
    #[error("rss channel with missing prepend")]
    MissingPrepend,
    #[error("rss channel with missing link")]
    MissingLink,
    #[error("rss channel with missing outfile")]
    MissingPath,
}
