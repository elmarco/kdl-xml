//! Type definitions for XML-to-KDL conversion.
//!
//! This module defines the intermediate representation used during
//! XML parsing before transformation to KDL.

/// Intermediate representation for XML nodes.
///
/// This enum captures the different types of XML content that can be
/// parsed from an XML document before being transformed into KDL.
#[derive(Debug, Clone)]
pub enum XmlNode {
    /// An XML element with optional attributes and children.
    Element {
        name: String,
        attributes: Vec<(String, String)>,
        children: Vec<XmlNode>,
    },
    /// Text content within an element.
    Text(String),
    /// An XML comment.
    Comment(String),
    /// A processing instruction (e.g., `<?xml-stylesheet ...?>`).
    ProcessingInstruction {
        target: String,
        content: Option<String>,
    },
    /// A DOCTYPE declaration.
    DocType(String),
}

/// Classification of element content type.
///
/// Used to determine how element children should be represented in KDL:
/// - Pure text becomes a single string argument
/// - Elements only use a children block
/// - Mixed content uses `-` nodes for text interleaved with elements
#[derive(Debug)]
pub enum ContentType {
    /// Element has no content.
    Empty,
    /// Element contains only text (single or combined text nodes).
    PureText(String),
    /// Element contains only child elements (no significant text).
    ElementsOnly(Vec<XmlNode>),
    /// Element contains mixed content (text interleaved with elements).
    Mixed(Vec<XmlNode>),
}
