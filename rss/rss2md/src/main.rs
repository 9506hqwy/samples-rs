use atom_syndication::Entry as AtomEntry;
use atom_syndication::Feed;
use chrono::{DateTime, Utc};
use html2md::ScraperParser;
use rss::{Channel, Item};
use scraper::html::Html;
use std::boxed::Box;
use std::collections::VecDeque;
use std::env;
use std::fs;
use std::io::{BufReader, Read};
use std::iter::Iterator;

//------------------------------------------------------------------------------------------------

enum Factory {
    Rss(Box<Channel>),
    Atom(Box<Feed>),
}

impl Factory {
    pub fn entries(self) -> (String, Entries) {
        let (title, entries) = match self {
            Factory::Rss(channel) => {
                let title = channel.title.clone();
                let mut items = VecDeque::new();
                for item in channel.into_items() {
                    items.push_back(Entry::Rss(item));
                }
                (title, items)
            }
            Factory::Atom(feed) => {
                let mut items = VecDeque::new();
                for item in feed.entries {
                    items.push_back(Entry::Atom(item));
                }

                (feed.title.to_string(), items)
            }
        };
        (title, Entries::new(entries))
    }
}

//------------------------------------------------------------------------------------------------

enum Entry {
    Rss(Item),
    Atom(AtomEntry),
}

impl Entry {
    fn section(&self) -> Section {
        match self {
            Entry::Rss(item) => convert_entry_rss(item).unwrap(),
            Entry::Atom(item) => convert_entry_atom(item).unwrap(),
        }
    }
}

//------------------------------------------------------------------------------------------------

struct Entries {
    entries: VecDeque<Entry>,
}

impl Iterator for Entries {
    type Item = Entry;

    fn next(&mut self) -> Option<Self::Item> {
        self.entries.pop_front()
    }
}

impl Entries {
    fn new(entries: VecDeque<Entry>) -> Entries {
        Entries { entries }
    }
}

//------------------------------------------------------------------------------------------------

struct Section {
    id: String,
    title: String,
    publish_date: Option<DateTime<Utc>>,
    update_date: Option<DateTime<Utc>>,
    author: Option<String>,
    description: Option<String>,
}

//------------------------------------------------------------------------------------------------

fn convert_description(description: &str) -> String {
    let fragment = Html::parse_fragment(description);

    let mut parser = ScraperParser::default();
    parser.parse(&fragment.tree.root());

    parser
        .parsed
        .iter()
        .fold(String::new(), |acc, s| format!("{}{}{}", acc, "\r\n", s))
}

fn convert_entry_atom(entry: &AtomEntry) -> Result<Section, Error> {
    let description = if let Some(content) = entry.content() {
        if let Some(value) = content.value() {
            let md = convert_description(value);
            Some(md)
        } else {
            None
        }
    } else {
        None
    };

    Ok(Section {
        id: entry.id().to_string(),
        title: entry.title().to_string(),
        publish_date: entry.published().map(|p| p.to_utc()),
        update_date: Some(entry.updated().to_utc()),
        author: entry.authors().first().map(|a| a.name().to_string()),
        description,
    })
}

fn convert_entry_rss(entry: &Item) -> Result<Section, Error> {
    let id = entry
        .guid()
        .map(|g| g.value().to_string())
        .unwrap_or_default();

    let update_date = if let Some(pub_date) = entry.pub_date() {
        if let Ok(date) = DateTime::parse_from_rfc2822(pub_date) {
            Some(date.with_timezone(&Utc))
        } else {
            let npub_date = pub_date.replace("Z", "GMT");
            match DateTime::parse_from_rfc2822(&npub_date) {
                Ok(date) => Some(date.with_timezone(&Utc)),
                _ => None,
            }
        }
    } else {
        None
    };

    let author = match entry.dublin_core_ext() {
        Some(dublin) if !dublin.creators().is_empty() => {
            let creator = dublin.creators()[0].to_owned();
            Some(creator)
        }
        _ => {
            if let Some(creator) = entry
                .extensions()
                .get("dc")
                .and_then(|dc| dc.get("creator"))
                .and_then(|creators| creators.first())
            {
                creator.value().map(|value| value.to_string())
            } else {
                None
            }
        }
    };

    let description = if let Some(description) = entry.description() {
        let md = convert_description(description);
        Some(md)
    } else {
        None
    };

    Ok(Section {
        id,
        title: entry.title().unwrap_or_default().to_string(),
        publish_date: None,
        update_date,
        author,
        description,
    })
}

fn get_entry<R: Read + Clone>(feed: R) -> Result<Factory, Error> {
    let reader = BufReader::new(feed.clone());
    match Channel::read_from(reader) {
        Ok(ch) => Ok(Factory::Rss(Box::new(ch))),
        _ => {
            let reader2 = BufReader::new(feed);
            if let Ok(feed) = Feed::read_from(reader2) {
                Ok(Factory::Atom(Box::new(feed)))
            } else {
                Err(Error::NotSupportedFormat)
            }
        }
    }
}

#[derive(Debug)]
enum Error {
    NotSupportedFormat,
}

fn main() {
    let args = env::args().collect::<Vec<String>>();
    let content = fs::read_to_string(args.get(1).unwrap()).unwrap();

    let factory = get_entry(content.as_bytes()).unwrap();
    let (title, entries) = factory.entries();
    println!("# {}", title);

    println!("---");

    for entry in entries {
        let section = entry.section();

        println!("## {}", section.title);
        println!();
        println!("- **ID**: {}", section.id);
        if let Some(author) = section.author {
            println!("- **Author**: {}", author);
        }
        if let Some(publish_date) = section.publish_date {
            println!("- **Published**: {}", publish_date);
        }
        if let Some(update_date) = section.update_date {
            println!("- **Updated**: {}", update_date);
        }
        if let Some(description) = section.description {
            println!("{}", description);
        }

        println!("---");
    }
}
