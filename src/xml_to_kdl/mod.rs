//! XML to KDL conversion module.
//!
//! This module provides functionality to convert XML documents to KDL format
//! following the XiK specification.

mod parser;
mod transformer;
mod types;

use crate::error::{Result, Xml2KdlError};
use kdl::{KdlDocument, KdlDocumentFormat, KdlNode, KdlNodeFormat};
use quick_xml::Reader;
use std::io::BufRead;

pub use types::XmlNode;

/// Compact output mode for the converter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompactMode {
    /// No compact formatting (default multi-line output).
    #[default]
    None,
    /// Compact all children blocks onto single lines.
    All,
    /// Only compact leaf mixed content (where child elements have no nested children).
    Leaf,
}

/// Configuration for the XML to KDL converter.
#[derive(Debug, Clone)]
pub struct XmlToKdlConverter {
    /// Whether to preserve comments as `!` nodes (true) or discard them (false).
    /// Note: KDL comments cannot be programmatically added via the kdl crate API.
    pub comments_as_nodes: bool,

    /// Whether to keep whitespace-only text nodes.
    pub preserve_whitespace: bool,

    /// Compact output mode.
    pub compact_mode: CompactMode,
}

impl Default for XmlToKdlConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl XmlToKdlConverter {
    /// Creates a new converter with default settings.
    pub fn new() -> Self {
        Self {
            comments_as_nodes: false,
            preserve_whitespace: false,
            compact_mode: CompactMode::None,
        }
    }

    /// Sets whether to preserve comments as `!` nodes.
    pub fn comments_as_nodes(mut self, value: bool) -> Self {
        self.comments_as_nodes = value;
        self
    }

    /// Sets whether to preserve whitespace-only text nodes.
    pub fn preserve_whitespace(mut self, value: bool) -> Self {
        self.preserve_whitespace = value;
        self
    }

    /// Sets whether to output children blocks on a single line.
    /// This is a convenience method that sets compact_mode to All when true.
    pub fn compact(mut self, value: bool) -> Self {
        self.compact_mode = if value {
            CompactMode::All
        } else {
            CompactMode::None
        };
        self
    }

    /// Sets the compact output mode.
    pub fn compact_mode(mut self, mode: CompactMode) -> Self {
        self.compact_mode = mode;
        self
    }

    /// Converts an XML string to a KDL document.
    pub fn convert(&self, xml: &str) -> Result<KdlDocument> {
        let reader = Reader::from_str(xml);
        self.convert_reader(reader)
    }

    /// Converts XML from a reader to a KDL document.
    pub fn convert_reader<R: BufRead>(&self, reader: Reader<R>) -> Result<KdlDocument> {
        let nodes = parser::parse_xml_to_tree(reader, self.comments_as_nodes)?;
        self.nodes_to_kdl_document(&nodes)
    }

    /// Converts parsed XML nodes to a KDL document.
    fn nodes_to_kdl_document(&self, nodes: &[types::XmlNode]) -> Result<KdlDocument> {
        // Validate document structure per XiK spec:
        // - Exactly one root element
        // - At most one DOCTYPE
        let mut element_count = 0;
        let mut doctype_count = 0;
        let mut seen_element = false;

        for node in nodes {
            match node {
                types::XmlNode::Element { .. } => {
                    element_count += 1;
                    seen_element = true;
                }
                types::XmlNode::DocType(_) => {
                    doctype_count += 1;
                    if doctype_count > 1 {
                        return Err(Xml2KdlError::InvalidStructure(
                            "Multiple DOCTYPE declarations not allowed".to_string(),
                        ));
                    }
                    if seen_element {
                        return Err(Xml2KdlError::InvalidStructure(
                            "DOCTYPE must appear before root element".to_string(),
                        ));
                    }
                }
                types::XmlNode::Text(s) if !s.trim().is_empty() => {
                    return Err(Xml2KdlError::InvalidStructure(
                        "Text content not allowed at document root".to_string(),
                    ));
                }
                _ => {} // Comments, PIs, whitespace-only text are allowed
            }
        }

        if element_count == 0 {
            return Err(Xml2KdlError::InvalidStructure(
                "Document must contain exactly one root element".to_string(),
            ));
        }
        if element_count > 1 {
            return Err(Xml2KdlError::InvalidStructure(format!(
                "Document must contain exactly one root element, found {}",
                element_count
            )));
        }

        let mut doc = KdlDocument::new();
        for node in nodes {
            if let Some(kdl_node) = transformer::xml_node_to_kdl(node, self.preserve_whitespace) {
                doc.nodes_mut().push(kdl_node);
            }
        }

        match self.compact_mode {
            CompactMode::None => {}
            CompactMode::All => apply_compact_format(&mut doc, false),
            CompactMode::Leaf => apply_compact_format(&mut doc, true),
        }

        Ok(doc)
    }
}

/// Applies compact formatting to a KDL document, putting children blocks on single lines.
///
/// If `leaf_only` is true, only compact nodes where all children are "leaves"
/// (have no nested children with their own children).
fn apply_compact_format(doc: &mut KdlDocument, leaf_only: bool) {
    for node in doc.nodes_mut() {
        apply_compact_format_to_node(node, leaf_only, 0);
    }
}

/// Checks if all children of a node are "leaves" (have no grandchildren).
/// A leaf child either has no children block, or its children block has no nodes with children.
fn is_leaf_children(node: &KdlNode) -> bool {
    match node.children() {
        None => true, // No children block at all
        Some(children) => {
            // Check if any child has its own children with nested children
            children.nodes().iter().all(|child| {
                match child.children() {
                    None => true, // Child has no children block - it's a leaf
                    Some(grandchildren) => grandchildren.nodes().is_empty(), // Empty children block
                }
            })
        }
    }
}

/// Recursively applies compact formatting to a node and its children.
///
/// If `leaf_only` is true, only compact this node if all its children are leaves.
/// Non-leaf nodes get multi-line formatting in leaf mode.
fn apply_compact_format_to_node(node: &mut KdlNode, leaf_only: bool, depth: usize) {
    // Check if we should compact this node
    let should_compact = if leaf_only {
        is_leaf_children(node)
    } else {
        true
    };

    // First, check if this node has children and set the parent's before_children
    if node.children().is_some() {
        // Set the parent node's before_children to add space before `{`
        let current_format = node.format().cloned();
        node.set_format(KdlNodeFormat {
            leading: current_format
                .as_ref()
                .map(|f| f.leading.clone())
                .unwrap_or_default(),
            before_ty_name: String::new(),
            after_ty_name: String::new(),
            after_ty: String::new(),
            before_children: " ".to_string(),
            before_terminator: String::new(),
            terminator: current_format
                .as_ref()
                .map(|f| f.terminator.clone())
                .unwrap_or_default(),
            trailing: current_format.map(|f| f.trailing).unwrap_or_default(),
        });
    }

    // Now process the children (need a separate if let to avoid borrow issues)
    if let Some(children) = node.children_mut() {
        if should_compact {
            // Set the children document format to avoid auto-newlines
            children.set_format(KdlDocumentFormat {
                leading: String::new(),
                trailing: " ".to_string(),
            });

            // Process each child node
            let child_count = children.nodes().len();
            for (i, child) in children.nodes_mut().iter_mut().enumerate() {
                // Set node format: use semicolon separator between children
                let terminator = if i < child_count - 1 {
                    "; ".to_string()
                } else {
                    String::new()
                };

                child.set_format(KdlNodeFormat {
                    leading: if i == 0 {
                        " ".to_string()
                    } else {
                        String::new()
                    },
                    before_ty_name: String::new(),
                    after_ty_name: String::new(),
                    after_ty: String::new(),
                    before_children: " ".to_string(),
                    before_terminator: String::new(),
                    terminator,
                    trailing: String::new(),
                });

                // Recurse into children (for leaf_only mode, non-leaf children need processing too)
                apply_compact_format_to_node(child, leaf_only, depth + 1);
            }
        } else {
            // Not compacting this level - apply multi-line formatting
            // Set children document format for multi-line output
            children.set_format(KdlDocumentFormat {
                leading: "\n".to_string(),
                trailing: "    ".repeat(depth),
            });

            // Compute indentation for child depth
            let child_indent = "    ".repeat(depth + 1);

            // Process each child node with multi-line formatting
            for child in children.nodes_mut() {
                child.set_format(KdlNodeFormat {
                    leading: child_indent.clone(),
                    before_ty_name: String::new(),
                    after_ty_name: String::new(),
                    after_ty: String::new(),
                    before_children: " ".to_string(),
                    before_terminator: String::new(),
                    terminator: "\n".to_string(),
                    trailing: String::new(),
                });

                // Recurse into children
                apply_compact_format_to_node(child, leaf_only, depth + 1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kdl::KdlValue;

    #[test]
    fn test_simple_element() {
        let converter = XmlToKdlConverter::new();
        let kdl = converter.convert("<root/>").unwrap();
        assert_eq!(kdl.nodes().len(), 1);
        assert_eq!(kdl.nodes()[0].name().to_string(), "root");
    }

    #[test]
    fn test_element_with_text() {
        let converter = XmlToKdlConverter::new();
        let kdl = converter.convert("<greeting>hello</greeting>").unwrap();
        let node = &kdl.nodes()[0];
        assert_eq!(node.entries().len(), 1);
        assert_eq!(
            node.entries()[0].value(),
            &KdlValue::String("hello".to_string())
        );
    }

    #[test]
    fn test_element_with_attributes() {
        let converter = XmlToKdlConverter::new();
        let kdl = converter
            .convert(r#"<link href="http://example.com" rel="stylesheet"/>"#)
            .unwrap();
        let node = &kdl.nodes()[0];
        assert!(node.get("href").is_some());
        assert!(node.get("rel").is_some());
    }

    #[test]
    fn test_mixed_content() {
        let converter = XmlToKdlConverter::new();
        let kdl = converter
            .convert("<span>some <b>bold</b> text</span>")
            .unwrap();
        let node = &kdl.nodes()[0];
        let children = node.children().unwrap();

        // Should have: - "some ", b "bold", - " text"
        assert_eq!(children.nodes().len(), 3);
        assert_eq!(children.nodes()[0].name().to_string(), "-");
        assert_eq!(children.nodes()[1].name().to_string(), "b");
        assert_eq!(children.nodes()[2].name().to_string(), "-");
    }

    #[test]
    fn test_doctype() {
        let converter = XmlToKdlConverter::new();
        let kdl = converter.convert("<!DOCTYPE html><html/>").unwrap();
        assert_eq!(kdl.nodes()[0].name().to_string(), "!doctype");
    }

    #[test]
    fn test_processing_instruction() {
        let converter = XmlToKdlConverter::new();
        let kdl = converter
            .convert(r#"<?xml version="1.0" encoding="UTF-8"?><root/>"#)
            .unwrap();
        let pi_node = &kdl.nodes()[0];
        assert_eq!(pi_node.name().to_string(), "?xml");
        assert!(pi_node.get("version").is_some());
    }

    #[test]
    fn test_comment_as_node() {
        let converter = XmlToKdlConverter::new().comments_as_nodes(true);
        let kdl = converter.convert("<!-- test comment --><root/>").unwrap();
        let comment_node = &kdl.nodes()[0];
        assert_eq!(comment_node.name().to_string(), "!");
    }

    #[test]
    fn test_nested_elements() {
        let converter = XmlToKdlConverter::new();
        let kdl = converter
            .convert("<parent><child><grandchild/></child></parent>")
            .unwrap();
        let parent = &kdl.nodes()[0];
        let child = &parent.children().unwrap().nodes()[0];
        let grandchild = &child.children().unwrap().nodes()[0];
        assert_eq!(grandchild.name().to_string(), "grandchild");
    }

    #[test]
    fn test_cdata() {
        let converter = XmlToKdlConverter::new();
        let kdl = converter
            .convert("<script><![CDATA[var x = 1 < 2;]]></script>")
            .unwrap();
        let node = &kdl.nodes()[0];
        // CDATA should be treated as text
        assert_eq!(
            node.entries()[0].value(),
            &KdlValue::String("var x = 1 < 2;".to_string())
        );
    }

    #[test]
    fn test_entity_decoding() {
        let converter = XmlToKdlConverter::new();
        let kdl = converter
            .convert("<p>&lt;hello&gt; &amp; &quot;world&quot;</p>")
            .unwrap();
        let node = &kdl.nodes()[0];
        assert_eq!(
            node.entries()[0].value(),
            &KdlValue::String("<hello> & \"world\"".to_string())
        );
    }

    #[test]
    fn test_namespace_prefix() {
        let converter = XmlToKdlConverter::new();
        let kdl = converter
            .convert(
                r#"<svg xmlns:xlink="http://www.w3.org/1999/xlink"><a xlink:href="url"/></svg>"#,
            )
            .unwrap();
        let svg = &kdl.nodes()[0];
        let a = &svg.children().unwrap().nodes()[0];
        assert!(a.get("xlink:href").is_some());
    }
}
