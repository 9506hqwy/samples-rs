use ego_tree::NodeRef;
use html5ever::{Namespace, QualName, local_name};
use scraper::node::Node;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ParserState {
    None,
    Formated,
    NoBreak,
    TableData,
    TableHeader,
}

pub struct ScraperParser {
    pub parsed: Vec<String>,
    state: ParserState,
}

impl Default for ScraperParser {
    fn default() -> Self {
        ScraperParser {
            parsed: vec![String::new()],
            state: ParserState::None,
        }
    }
}

impl ScraperParser {
    fn with_state(state: ParserState) -> ScraperParser {
        ScraperParser {
            state,
            ..Default::default()
        }
    }

    pub fn parse(&mut self, html: &NodeRef<Node>) {
        match html.value() {
            Node::Text(text) => {
                self.insert_text(text);
            }
            Node::Element(element) => {
                if self.state == ParserState::Formated {
                    // TODO: attribute
                    self.insert_text(&format!("<{}>", element.name()));
                    self.parse_children(html);
                    self.insert_text(&format!("</{}>", element.name()));
                } else {
                    match element.name() {
                        "a" => {
                            self.convert_link(html);
                        }
                        "backquote" => {
                            self.convert_backquote(html);
                        }
                        "br" => {
                            self.newline();
                        }
                        "code" => {
                            self.decorate_text(html, "`");
                        }
                        "div" | "p" => {
                            // Insert line break to before and after block element.
                            self.newline_if_need();
                            self.newline();
                            self.parse_children(html);
                            self.newline_if_need();
                            self.newline();
                        }
                        "b" | "em" | "strong" => {
                            self.decorate_text(html, "**");
                        }
                        "del" | "s" => {
                            self.decorate_text(html, "~~");
                        }
                        "h1" => {
                            self.convert_header(1, html);
                        }
                        "h2" => {
                            self.convert_header(2, html);
                        }
                        "h3" => {
                            self.convert_header(3, html);
                        }
                        "head" | "script" | "style" => {
                            // Skip head, script, and style elements.
                        }
                        "hr" => {
                            self.convert_hr();
                        }
                        "i" => {
                            self.decorate_text(html, "*");
                        }
                        "li" => {
                            self.parse_children(html);
                        }
                        "ol" => {
                            self.convert_list(html, "1. ");
                        }
                        "pre" => {
                            self.convert_formated(html);
                        }
                        "tbody" | "thead" => {
                            self.convert_table(html);
                        }
                        "tr" => {
                            self.convert_table_row(html);
                        }
                        "ul" => {
                            self.convert_list(html, "* ");
                        }
                        _ => {
                            self.parse_children(html);
                        }
                    }
                }
            }
            _ => {
                self.parse_children(html);
            }
        }
    }

    fn is_backquote(&self, html: &NodeRef<Node>) -> bool {
        match html.value() {
            Node::Element(element) => element.name() == "backquote",
            _ => false,
        }
    }

    fn is_table_data(&self, html: &NodeRef<Node>) -> bool {
        match html.value() {
            Node::Element(element) => element.name() == "td",
            _ => false,
        }
    }

    fn is_table_header(&self, html: &NodeRef<Node>) -> bool {
        match html.value() {
            Node::Element(element) => element.name() == "th",
            _ => false,
        }
    }

    fn is_table_row(&self, html: &NodeRef<Node>) -> bool {
        match html.value() {
            Node::Element(element) => element.name() == "tr",
            _ => false,
        }
    }

    fn is_list(&self, html: &NodeRef<Node>) -> bool {
        match html.value() {
            Node::Element(element) => element.name() == "ol" || element.name() == "ul",
            _ => false,
        }
    }

    fn is_list_item(&self, html: &NodeRef<Node>) -> bool {
        match html.value() {
            Node::Element(element) => element.name() == "li",
            _ => false,
        }
    }

    fn get_link_href(&self, html: &NodeRef<Node>) -> Option<String> {
        match html.value() {
            Node::Element(element) => {
                let name = QualName::new(None, Namespace::from(""), local_name!("href"));
                element
                    .attrs
                    .iter()
                    .filter(|(n, _)| n == &name)
                    .map(|(_, href)| format!("{}", href))
                    .next()
            }
            _ => None,
        }
    }

    fn convert_backquote(&mut self, html: &NodeRef<Node>) {
        let prefix = "> ";

        let depth = html.ancestors().filter(|e| self.is_backquote(e)).count() + 1;

        for child in html.children() {
            let mut subparser = ScraperParser::default();
            subparser.parse(&child);

            for parsed in subparser.parsed {
                if !parsed.is_empty() {
                    self.insert_text(&format!("{}{}", prefix.repeat(depth), parsed));
                    self.newline();
                }
            }
        }

        self.insert_text(&prefix.repeat(depth));
        self.newline();
    }

    fn convert_header(&mut self, level: usize, html: &NodeRef<Node>) {
        let prefix = "#";

        let mut subparser = ScraperParser::default();
        subparser.parse_children(html);

        if let Some(title) = subparser.parsed.first() {
            self.newline();
            self.insert_text(&format!("{} {}", prefix.repeat(level), title));
            self.newline();
        }
    }

    fn convert_hr(&mut self) {
        self.newline_if_need();
        self.newline();
        self.insert_text("----");
        self.newline();
        self.newline();
    }

    fn convert_link(&mut self, html: &NodeRef<Node>) {
        let mut subparser = ScraperParser::default();
        subparser.parse_children(html);

        if let Some(text) = subparser.parsed.first() {
            if let Some(link) = self.get_link_href(html) {
                self.insert_text(&format!("[{}]({})", text, link));
            } else {
                self.insert_text(text);
            }
        }
    }

    fn convert_list(&mut self, html: &NodeRef<Node>, prefix: &str) {
        let mut mark = prefix;
        let ws = " ".repeat(mark.len());
        let depth = html.ancestors().filter(|e| self.is_list(e)).count();

        for child in html.children() {
            if self.is_list_item(&child) {
                let mut subparser = ScraperParser::default();
                subparser.parse(&child);

                for parsed in subparser.parsed {
                    if !parsed.is_empty() {
                        self.newline();
                        self.insert_text(&format!("{}{}{}", ws.repeat(depth), mark, parsed));
                        mark = &ws; // Marker is only first item.
                    }
                }
            }

            mark = prefix;
        }

        self.newline();
    }

    fn convert_formated(&mut self, html: &NodeRef<Node>) {
        let prefix = "    ";

        let mut subparser = ScraperParser::with_state(ParserState::Formated);
        subparser.parse_children(html);

        for parsed in subparser.parsed {
            self.insert_text(&format!("{}{}", prefix, parsed));
            self.newline();
        }
    }

    fn convert_table(&mut self, html: &NodeRef<Node>) {
        for child in html.children() {
            if self.is_table_row(&child) {
                let mut subparser = ScraperParser::default();

                if self.state == ParserState::None {
                    self.state = ParserState::TableHeader;
                    subparser.state = self.state;
                } else if self.state == ParserState::TableHeader {
                    self.state = ParserState::TableData;
                    subparser.state = self.state;
                }

                subparser.parse(&child);

                for parsed in subparser.parsed {
                    if !parsed.is_empty() {
                        self.newline();
                        self.insert_text(&parsed);
                    }
                }
            }
        }

        self.state = ParserState::None;
        self.newline();
    }

    fn convert_table_row(&mut self, html: &NodeRef<Node>) {
        let mut subparser = ScraperParser::default();

        for child in html.children() {
            if self.is_table_data(&child) || self.is_table_header(&child) {
                let pre_state = subparser.state;
                subparser.state = ParserState::NoBreak;
                subparser.parse(&child);
                subparser.state = pre_state;
                subparser.newline();
            }
        }

        let column_len = subparser.parsed.len() - 1;
        subparser.parsed.truncate(column_len);
        let parsed = subparser.parsed.join(" | ");
        if !parsed.is_empty() {
            self.insert_text(&format!("| {} |", parsed));
        }

        if self.state == ParserState::TableHeader {
            // Output separator after header row.
            self.newline();
            let sep = subparser
                .parsed
                .iter()
                .map(|p| p.len())
                .map(|s| if s < 3 { 3 } else { s })
                .map(|s| "-".repeat(s))
                .collect::<Vec<String>>()
                .join(" | ");
            self.insert_text(&format!("| {} |", sep));
        }
    }

    fn decorate_text(&mut self, html: &NodeRef<Node>, decorater: &str) {
        let mut subparser = ScraperParser::default();
        subparser.parse_children(html);

        if let Some(text) = subparser.parsed.first() {
            self.insert_text(&format!("{}{}{}", decorater, text, decorater));
        }
    }

    fn insert_text(&mut self, text: &str) {
        // TODO: escape text
        if let Some(mut s) = self.parsed.last_mut() {
            let mut cr = false;
            for c in text.chars() {
                if c != '\r' && c != '\n' {
                    s.push(c);
                } else if self.state == ParserState::Formated {
                    // To avoid duplicate line breaks, skip CRLF code.
                    if !cr || c == '\r' {
                        self.newline();
                        s = self.parsed.last_mut().unwrap();
                    }

                    cr = c == '\r';
                }
            }
        }
    }

    fn newline(&mut self) {
        if self.state == ParserState::NoBreak {
            self.insert_text(" ");
        } else {
            self.parsed.push(String::new());
        }
    }

    fn newline_if_need(&mut self) {
        if let Some(s) = self.parsed.last()
            && !s.is_empty()
        {
            self.newline();
        }
    }

    fn parse_children(&mut self, html: &NodeRef<Node>) {
        for child in html.children() {
            self.parse(&child);
        }
    }
}
