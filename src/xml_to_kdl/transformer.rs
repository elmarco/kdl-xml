//! XML to KDL transformation logic.
//!
//! This module transforms the intermediate `XmlNode` representation
//! into KDL nodes and documents.

use crate::constants::{COMMENT_NODE_NAME, DOCTYPE_NODE_NAME, PI_PREFIX, TEXT_NODE_NAME};
use kdl::{KdlEntry, KdlIdentifier, KdlNode, KdlValue};

use super::types::{ContentType, XmlNode};

/// Transforms an XmlNode into a KdlNode.
///
/// # Arguments
/// * `node` - The XML node to transform
/// * `inherit_preserve` - Whether whitespace preservation is inherited from parent
///
/// # Returns
/// * `Some(KdlNode)` - The transformed KDL node
/// * `None` - If the node should not produce output (e.g., standalone text nodes)
pub fn xml_node_to_kdl(node: &XmlNode, inherit_preserve: bool) -> Option<KdlNode> {
    match node {
        XmlNode::Element {
            name,
            attributes,
            children,
        } => {
            let mut kdl_node = KdlNode::new(KdlIdentifier::from(name.clone()));

            // Check for xml:space attribute to determine whitespace handling
            let preserve_whitespace = attributes
                .iter()
                .find(|(k, _)| k == "xml:space")
                .map(|(_, v)| v == "preserve")
                .unwrap_or(inherit_preserve);

            // Add attributes as properties
            for (key, value) in attributes {
                let entry = KdlEntry::new_prop(
                    KdlIdentifier::from(key.clone()),
                    KdlValue::String(value.clone()),
                );
                kdl_node.push(entry);
            }

            // Analyze content type
            let significant_children = filter_children_with_preserve(children, preserve_whitespace);

            match analyze_content(&significant_children) {
                ContentType::Empty => {
                    // No children, no text
                }
                ContentType::PureText(text) => {
                    // Single text argument
                    kdl_node.push(KdlEntry::new(KdlValue::String(text)));
                }
                ContentType::ElementsOnly(children) | ContentType::Mixed(children) => {
                    // Use children block
                    let children_doc = kdl_node.ensure_children();
                    for child in &children {
                        match child {
                            XmlNode::Text(text) => {
                                // Mixed content: use `-` node
                                let mut dash_node =
                                    KdlNode::new(KdlIdentifier::from(TEXT_NODE_NAME.to_string()));
                                dash_node.push(KdlEntry::new(KdlValue::String(text.clone())));
                                children_doc.nodes_mut().push(dash_node);
                            }
                            _ => {
                                if let Some(child_node) =
                                    xml_node_to_kdl(child, preserve_whitespace)
                                {
                                    children_doc.nodes_mut().push(child_node);
                                }
                            }
                        }
                    }
                }
            }

            Some(kdl_node)
        }
        XmlNode::Comment(text) => {
            // Create `!` node with comment text
            let mut node = KdlNode::new(KdlIdentifier::from(COMMENT_NODE_NAME.to_string()));
            node.push(KdlEntry::new(KdlValue::String(text.clone())));
            Some(node)
        }
        XmlNode::ProcessingInstruction { target, content } => {
            let node_name = format!("{}{}", PI_PREFIX, target);
            let mut node = KdlNode::new(KdlIdentifier::from(node_name));

            if let Some(content) = content {
                if looks_like_attributes(content) {
                    // Structured: parse as properties
                    for (key, value) in parse_pi_attributes(content) {
                        node.push(KdlEntry::new_prop(
                            KdlIdentifier::from(key),
                            KdlValue::String(value),
                        ));
                    }
                } else {
                    // Unstructured: single string argument
                    node.push(KdlEntry::new(KdlValue::String(content.clone())));
                }
            }

            Some(node)
        }
        XmlNode::DocType(content) => {
            let mut node = KdlNode::new(KdlIdentifier::from(DOCTYPE_NODE_NAME.to_string()));
            node.push(KdlEntry::new(KdlValue::String(content.clone())));
            Some(node)
        }
        XmlNode::Text(_) => {
            // Text nodes are handled by parent element
            None
        }
    }
}

/// Filters children based on whitespace preservation settings.
pub fn filter_children_with_preserve(
    children: &[XmlNode],
    preserve_whitespace: bool,
) -> Vec<XmlNode> {
    if preserve_whitespace {
        return children.to_vec();
    }

    // Check if there are any element children
    let has_elements = children
        .iter()
        .any(|c| matches!(c, XmlNode::Element { .. }));

    children
        .iter()
        .filter(|child| {
            match child {
                XmlNode::Text(text) => {
                    if has_elements {
                        // In mixed/element content, keep non-whitespace text
                        !text.trim().is_empty()
                    } else {
                        // In pure text content, keep all text
                        true
                    }
                }
                _ => true,
            }
        })
        .cloned()
        .collect()
}

/// Analyzes the content type of element children.
pub fn analyze_content(children: &[XmlNode]) -> ContentType {
    if children.is_empty() {
        return ContentType::Empty;
    }

    // Only count actual Element nodes for mixed content detection
    // Comments and PIs should not trigger mixed content mode
    let has_elements = children
        .iter()
        .any(|c| matches!(c, XmlNode::Element { .. }));

    // Check for comments/PIs that need to be included in output
    let has_comments_or_pis = children.iter().any(|c| {
        matches!(
            c,
            XmlNode::Comment(_) | XmlNode::ProcessingInstruction { .. }
        )
    });

    let text_nodes: Vec<&String> = children
        .iter()
        .filter_map(|c| {
            if let XmlNode::Text(t) = c {
                Some(t)
            } else {
                None
            }
        })
        .collect();

    let has_text = !text_nodes.is_empty();

    if !has_elements && !has_comments_or_pis {
        // Only text nodes (no elements, comments, or PIs)
        if text_nodes.len() == 1 {
            return ContentType::PureText(text_nodes[0].clone());
        } else if !text_nodes.is_empty() {
            // Multiple text nodes - combine them
            let combined: String = text_nodes.iter().map(|s| s.as_str()).collect();
            return ContentType::PureText(combined);
        }
        return ContentType::Empty;
    }

    // If we have comments/PIs with text but no elements, we still need mixed mode
    // to preserve the comments/PIs in proper positions
    if has_comments_or_pis && !has_elements && has_text {
        return ContentType::Mixed(children.to_vec());
    }

    if has_elements && has_text {
        ContentType::Mixed(children.to_vec())
    } else {
        ContentType::ElementsOnly(children.to_vec())
    }
}

/// Checks if content looks like XML attribute syntax.
fn looks_like_attributes(content: &str) -> bool {
    let content = content.trim();

    // Must have at least one key="value" or key='value' pattern
    // Check that it starts with a valid attribute name character
    let first_char = match content.chars().next() {
        Some(c) => c,
        None => return false, // Empty content
    };
    if !first_char.is_ascii_alphabetic() && first_char != '_' && first_char != ':' {
        return false;
    }

    // Try to parse and see if we get any valid attributes
    let attrs = parse_pi_attributes(content);
    if attrs.is_empty() {
        return false;
    }

    // Reconstruct and compare length - if we parsed most of the content, it's structured
    let reconstructed_len: usize = attrs
        .iter()
        .map(|(k, v)| k.len() + 2 + v.len() + 1) // key="value" + space
        .sum();

    // Allow some tolerance for whitespace differences
    reconstructed_len >= content.len() / 2
}

/// Parses processing instruction content as key="value" pairs.
pub fn parse_pi_attributes(content: &str) -> Vec<(String, String)> {
    let mut attrs = vec![];
    let chars: Vec<char> = content.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Skip whitespace
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }

        // Parse attribute name: must start with letter, underscore, or colon
        let name_start = i;
        if !chars[i].is_ascii_alphabetic() && chars[i] != '_' && chars[i] != ':' {
            break; // Not a valid attribute name start
        }

        // Continue reading name characters
        while i < chars.len()
            && (chars[i].is_ascii_alphanumeric()
                || chars[i] == '-'
                || chars[i] == '_'
                || chars[i] == '.'
                || chars[i] == ':')
        {
            i += 1;
        }
        let name: String = chars[name_start..i].iter().collect();

        // Skip whitespace before '='
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }

        // Expect '='
        if i >= chars.len() || chars[i] != '=' {
            break;
        }
        i += 1;

        // Skip whitespace after '='
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }

        // Expect quote
        if i >= chars.len() || (chars[i] != '"' && chars[i] != '\'') {
            break;
        }
        let quote_char = chars[i];
        i += 1;

        // Parse value, handling escaped quotes
        let mut value = String::new();
        while i < chars.len() {
            if chars[i] == '\\' && i + 1 < chars.len() {
                // Escape sequence - include the escaped character
                i += 1;
                value.push(chars[i]);
                i += 1;
            } else if chars[i] == quote_char {
                // End of value
                i += 1;
                break;
            } else {
                value.push(chars[i]);
                i += 1;
            }
        }

        attrs.push((name, value));
    }

    attrs
}
