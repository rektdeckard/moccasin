use anyhow::Result;
use html_escape::decode_html_entities as decode;
use html_parser::{Dom, DomVariant, Node};

pub enum HTMLParseError {
    NotParseable,
    NotStringifiable,
}

fn flatten_nodes(nodes: &Vec<Node>, trim: bool) -> String {
    let flat = nodes
        .iter()
        .filter_map(|node| match flatten_html(node) {
            Ok(Some(s)) => Some(s),
            Ok(None) => None,
            Err(_) => None,
        })
        .collect::<String>();

    if trim {
        flat.trim_start().to_owned()
    } else {
        flat
    }
}

fn flatten_html(node: &Node) -> Result<Option<String>, HTMLParseError> {
    match node {
        Node::Text(s) => Ok(Some(decode(s).into_owned())),
        Node::Comment(_) => Ok(None),
        Node::Element(el) => match el.name.as_str() {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                let hashes = el.name.chars().nth(1).unwrap().to_digit(10).unwrap();
                let mut heading = "#".repeat(hashes as usize);
                let parts = flatten_nodes(&el.children, true);
                heading.push(' ');
                heading.push_str(&parts);
                heading.push_str("\n\n");
                Ok(Some(heading))
            }
            "p" | "div" => {
                let mut parts = flatten_nodes(&el.children, true);
                parts.push_str("\n\n");
                Ok(Some(parts))
            }
            "b" | "i" | "strong" | "em" | "small" | "span" | "pre" | "code" => {
                let parts = flatten_nodes(&el.children, true);
                Ok(Some(parts))
            }
            "ul" | "ol" => {
                let mut text = String::from("\n");
                let parts = flatten_nodes(&el.children, true);
                text.push_str(&parts);
                text.push_str("\n");
                Ok(Some(parts))
            }
            "li" => {
                let mut text = String::from("- ");
                let parts = flatten_nodes(&el.children, true);
                text.push_str(&parts);
                text.push_str("\n");
                Ok(Some(text))
            }
            "a" => {
                let parts = flatten_nodes(&el.children, true);
                if let Some(href) = el.attributes.get("href") {
                    Ok(Some(format!(
                        "{} ({})",
                        parts,
                        href.as_deref().unwrap_or_default()
                    )))
                } else {
                    Ok(Some(parts))
                }
            }
            // "img" => Ok(Some("[image] ".into())),
            _ => Ok(None),
        },
    }
}

pub fn parse_html(content: &str) -> Result<String, HTMLParseError> {
    match Dom::parse(content) {
        Ok(dom) => match dom.tree_type {
            DomVariant::DocumentFragment => {
                let text = dom
                    .children
                    .iter()
                    .filter_map(|node| match flatten_html(node) {
                        Ok(Some(s)) => Some(s),
                        Ok(None) => None,
                        Err(_) => None,
                    })
                    .collect::<String>();
                Ok(text)
            }
            _ => Err(HTMLParseError::NotStringifiable),
        },
        Err(_) => Err(HTMLParseError::NotParseable),
    }
}
