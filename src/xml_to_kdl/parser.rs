//! XML parsing functionality.
//!
//! This module handles parsing XML documents into the intermediate
//! `XmlNode` representation.

use crate::error::{Result, Xml2KdlError};
use quick_xml::Reader;
use quick_xml::escape::unescape;
use quick_xml::events::{BytesStart, Event};
use std::io::BufRead;

use super::types::XmlNode;

/// Stack frame for tracking element parsing state: (name, attributes, children).
type ElementStackFrame = (String, Vec<(String, String)>, Vec<XmlNode>);

/// Parses XML from a reader into a vector of XmlNode trees.
///
/// # Arguments
/// * `reader` - A quick_xml Reader instance
/// * `comments_as_nodes` - Whether to preserve comments in the output
///
/// # Returns
/// * `Ok(Vec<XmlNode>)` - The parsed XML tree(s)
/// * `Err(Xml2KdlError)` - If parsing fails
pub fn parse_xml_to_tree<R: BufRead>(
    mut reader: Reader<R>,
    comments_as_nodes: bool,
) -> Result<Vec<XmlNode>> {
    // Don't trim text - we need to preserve whitespace for mixed content detection
    reader.config_mut().trim_text(false);

    let mut stack: Vec<ElementStackFrame> = vec![];
    let mut root_nodes: Vec<XmlNode> = vec![];
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) => {
                let name = std::str::from_utf8(e.name().as_ref())?.to_string();
                let attrs = parse_attributes(&e)?;
                stack.push((name, attrs, vec![]));
            }
            Event::End(_) => {
                if let Some((name, attrs, children)) = stack.pop() {
                    let node = XmlNode::Element {
                        name,
                        attributes: attrs,
                        children,
                    };
                    if let Some(parent) = stack.last_mut() {
                        parent.2.push(node);
                    } else {
                        root_nodes.push(node);
                    }
                }
            }
            Event::Empty(e) => {
                let name = std::str::from_utf8(e.name().as_ref())?.to_string();
                let attrs = parse_attributes(&e)?;
                let node = XmlNode::Element {
                    name,
                    attributes: attrs,
                    children: vec![],
                };
                if let Some(parent) = stack.last_mut() {
                    parent.2.push(node);
                } else {
                    root_nodes.push(node);
                }
            }
            Event::Text(e) => {
                // Decode bytes to string, then unescape XML entities
                let decoded = e.decode().map_err(|e| {
                    Xml2KdlError::InvalidStructure(format!("Encoding error: {}", e))
                })?;
                let text = unescape(&decoded)
                    .map_err(|e| Xml2KdlError::InvalidStructure(format!("Unescape error: {}", e)))?
                    .to_string();
                if let Some(parent) = stack.last_mut() {
                    parent.2.push(XmlNode::Text(text));
                }
            }
            Event::CData(e) => {
                // CDATA is treated as regular text per XiK spec (no unescaping needed)
                let text = e
                    .decode()
                    .map_err(|e| Xml2KdlError::InvalidStructure(format!("Encoding error: {}", e)))?
                    .to_string();
                if let Some(parent) = stack.last_mut() {
                    parent.2.push(XmlNode::Text(text));
                }
            }
            Event::GeneralRef(e) => {
                // Handle general entity references like &amp;
                let decoded = e.decode().map_err(|e| {
                    Xml2KdlError::InvalidStructure(format!("Encoding error: {}", e))
                })?;
                let text = unescape(&format!("&{};", decoded))
                    .map_err(|e| Xml2KdlError::InvalidStructure(format!("Unescape error: {}", e)))?
                    .to_string();
                if let Some(parent) = stack.last_mut() {
                    parent.2.push(XmlNode::Text(text));
                }
            }
            Event::Comment(e) => {
                if comments_as_nodes {
                    let comment = std::str::from_utf8(&e)?.to_string();
                    let node = XmlNode::Comment(comment);
                    if let Some(parent) = stack.last_mut() {
                        parent.2.push(node);
                    } else {
                        root_nodes.push(node);
                    }
                }
                // If not comments_as_nodes, we simply discard comments
            }
            Event::Decl(e) => {
                // XML declaration: <?xml version="1.0"?>
                let version = e
                    .version()
                    .ok()
                    .and_then(|v| std::str::from_utf8(&v).ok().map(|s| s.to_string()));
                let encoding = e
                    .encoding()
                    .and_then(|r| r.ok())
                    .and_then(|v| std::str::from_utf8(&v).ok().map(|s| s.to_string()));
                let standalone = e
                    .standalone()
                    .and_then(|r| r.ok())
                    .and_then(|v| std::str::from_utf8(&v).ok().map(|s| s.to_string()));

                // Build content string for structured PI
                let mut content_parts = vec![];
                if let Some(v) = version {
                    content_parts.push(format!("version=\"{}\"", v));
                }
                if let Some(enc) = encoding {
                    content_parts.push(format!("encoding=\"{}\"", enc));
                }
                if let Some(sa) = standalone {
                    content_parts.push(format!("standalone=\"{}\"", sa));
                }

                let content = if content_parts.is_empty() {
                    None
                } else {
                    Some(content_parts.join(" "))
                };

                let node = XmlNode::ProcessingInstruction {
                    target: "xml".to_string(),
                    content,
                };
                root_nodes.push(node);
            }
            Event::PI(e) => {
                let content_bytes = &*e;
                let content_str = std::str::from_utf8(content_bytes)?;

                // Split target and content
                let (target, content) =
                    if let Some(space_pos) = content_str.find(char::is_whitespace) {
                        let target = content_str[..space_pos].to_string();
                        let rest = content_str[space_pos..].trim().to_string();
                        (target, if rest.is_empty() { None } else { Some(rest) })
                    } else {
                        (content_str.to_string(), None)
                    };

                let node = XmlNode::ProcessingInstruction { target, content };
                if let Some(parent) = stack.last_mut() {
                    parent.2.push(node);
                } else {
                    root_nodes.push(node);
                }
            }
            Event::DocType(e) => {
                let content = std::str::from_utf8(&e)?.trim().to_string();
                root_nodes.push(XmlNode::DocType(content));
            }
            Event::Eof => break,
        }
        buf.clear();
    }

    if !stack.is_empty() {
        return Err(Xml2KdlError::InvalidStructure(
            "Unclosed XML elements".to_string(),
        ));
    }

    Ok(root_nodes)
}

/// Parses attributes from an XML start tag.
fn parse_attributes(e: &BytesStart) -> Result<Vec<(String, String)>> {
    let mut attrs = vec![];
    for attr_result in e.attributes() {
        let attr = attr_result.map_err(quick_xml::Error::from)?;
        let key = std::str::from_utf8(attr.key.as_ref())?.to_string();
        let value = attr.unescape_value()?.to_string();
        attrs.push((key, value));
    }
    Ok(attrs)
}
