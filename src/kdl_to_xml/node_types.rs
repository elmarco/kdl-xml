//! Node type detection and extraction helpers.
//!
//! This module provides functions to identify special XiK node types
//! and extract content from KDL nodes.

use crate::constants::{COMMENT_NODE_NAME, DOCTYPE_NODE_NAME, PI_PREFIX, TEXT_NODE_NAME};
use crate::error::Kdl2XmlError;
use kdl::{KdlNode, KdlValue};

/// Returns true if the node represents a DOCTYPE declaration.
pub fn is_doctype_node(name: &str) -> bool {
    name == DOCTYPE_NODE_NAME
}

/// Returns true if the node represents a processing instruction.
pub fn is_pi_node(name: &str) -> bool {
    name.starts_with(PI_PREFIX)
}

/// Returns true if the node represents an XML comment.
pub fn is_comment_node(name: &str) -> bool {
    name == COMMENT_NODE_NAME
}

/// Returns true if the node represents a text content node (mixed content).
pub fn is_text_node(name: &str) -> bool {
    name == TEXT_NODE_NAME
}

/// Extracts the first unnamed string argument from a node.
///
/// # Returns
/// * `Ok(String)` - The string value of the first argument
/// * `Err(Kdl2XmlError)` - If no argument exists or it's not a string
pub fn get_first_argument(node: &KdlNode) -> Result<String, Kdl2XmlError> {
    for entry in node.entries() {
        if entry.name().is_none() {
            return match entry.value() {
                KdlValue::String(s) => Ok(s.clone()),
                other => Err(Kdl2XmlError::InvalidStructure(format!(
                    "Node '{}' argument must be a string, got {:?}",
                    node.name(),
                    other
                ))),
            };
        }
    }
    Err(Kdl2XmlError::InvalidStructure(format!(
        "Node '{}' requires a string argument",
        node.name()
    )))
}

/// Extracts the text argument from a node, if present and it has no children.
///
/// # Returns
/// * `Ok(Some(String))` - The text content
/// * `Ok(None)` - If no text argument or has children
/// * `Err(Kdl2XmlError)` - If argument is not a string
pub fn get_text_argument(node: &KdlNode) -> Result<Option<String>, Kdl2XmlError> {
    // Only get text argument if there are no children
    if node.children().is_some_and(|c| !c.nodes().is_empty()) {
        return Ok(None);
    }

    for entry in node.entries() {
        if entry.name().is_none() {
            return match entry.value() {
                KdlValue::String(s) => Ok(Some(s.clone())),
                other => Err(Kdl2XmlError::InvalidStructure(format!(
                    "Element '{}' text content must be a string, got {:?}",
                    node.name(),
                    other
                ))),
            };
        }
    }
    Ok(None)
}

/// Returns true if the node has any named entries (properties).
pub fn has_properties(node: &KdlNode) -> bool {
    node.entries().iter().any(|e| e.name().is_some())
}
