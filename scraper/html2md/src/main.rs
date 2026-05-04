use html2md::ScraperParser;
use scraper::Html;
use std::env;
use std::fs;

fn main() {
    let args = env::args().collect::<Vec<String>>();
    let content = fs::read_to_string(args.get(1).unwrap()).unwrap();

    let fragment = Html::parse_document(&content);

    let mut parser = ScraperParser::default();
    parser.parse(&fragment.tree.root());

    for parsed in parser.parsed {
        println!("{}", parsed);
    }
}
