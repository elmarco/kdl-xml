//! KDL to XML conversion module.
//!
//! This module provides functionality to convert KDL documents (in XiK format)
//! to XML format.

mod node_types;
mod validation;
mod writers;

use crate::error::Kdl2XmlError;
use kdl::{KdlDocument, KdlNode};
use node_types::{is_comment_node, is_doctype_node, is_pi_node, is_text_node};
use quick_xml::Writer;
use std::io::Cursor;

/// Result type for KDL to XML conversion operations.
pub type Kdl2XmlResult<T> = std::result::Result<T, Kdl2XmlError>;

/// Configuration for the KDL to XML converter.
#[derive(Debug, Clone)]
pub struct KdlToXmlConverter {
    /// Indent string for pretty printing.
    pub indent: String,

    /// Whether to pretty-print output.
    pub pretty: bool,
}

impl Default for KdlToXmlConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl KdlToXmlConverter {
    /// Creates a new converter with default settings.
    pub fn new() -> Self {
        Self {
            indent: "  ".to_string(),
            pretty: true,
        }
    }

    /// Sets the indentation string for pretty printing.
    pub fn indent(mut self, value: impl Into<String>) -> Self {
        self.indent = value.into();
        self
    }

    /// Sets whether to pretty-print the output.
    pub fn pretty(mut self, value: bool) -> Self {
        self.pretty = value;
        self
    }

    /// Converts a KDL string to XML.
    pub fn convert(&self, kdl: &str) -> Kdl2XmlResult<String> {
        let doc: KdlDocument = kdl.parse()?;
        self.convert_document(&doc)
    }

    /// Converts a KDL document to an XML string.
    pub fn convert_document(&self, doc: &KdlDocument) -> Kdl2XmlResult<String> {
        // Validate document structure per XiK spec:
        // - Exactly one root element
        // - At most one DOCTYPE
        // - DOCTYPE and PIs must come before the root element
        let mut element_count = 0;
        let mut doctype_count = 0;
        let mut seen_element = false;

        for node in doc.nodes() {
            let name = node.name().to_string();
            if is_doctype_node(&name) {
                doctype_count += 1;
                if doctype_count > 1 {
                    return Err(Kdl2XmlError::InvalidStructure(
                        "Multiple DOCTYPE declarations not allowed".to_string(),
                    ));
                }
                if seen_element {
                    return Err(Kdl2XmlError::InvalidStructure(
                        "DOCTYPE must appear before root element".to_string(),
                    ));
                }
            } else if is_pi_node(&name) || is_comment_node(&name) {
                // PIs and comments can appear anywhere
            } else if is_text_node(&name) {
                return Err(Kdl2XmlError::InvalidStructure(
                    "Text nodes (-) not allowed at document root".to_string(),
                ));
            } else {
                // Regular element
                element_count += 1;
                seen_element = true;
            }
        }

        if element_count == 0 {
            return Err(Kdl2XmlError::InvalidStructure(
                "Document must contain exactly one root element".to_string(),
            ));
        }
        if element_count > 1 {
            return Err(Kdl2XmlError::InvalidStructure(format!(
                "Document must contain exactly one root element, found {}",
                element_count
            )));
        }

        let mut writer = if self.pretty {
            Writer::new_with_indent(Cursor::new(Vec::new()), b' ', self.indent.len())
        } else {
            Writer::new(Cursor::new(Vec::new()))
        };

        for node in doc.nodes() {
            self.node_to_xml(node, &mut writer, 0)?;
        }

        let result = writer.into_inner().into_inner();
        Ok(String::from_utf8(result)?)
    }

    /// Converts a single KDL node to XML.
    fn node_to_xml(
        &self,
        node: &KdlNode,
        writer: &mut Writer<Cursor<Vec<u8>>>,
        depth: usize,
    ) -> Kdl2XmlResult<()> {
        let name = node.name().to_string();

        // Check for special node types
        if is_doctype_node(&name) {
            return writers::write_doctype(node, writer, self.pretty);
        }

        if is_pi_node(&name) {
            return writers::write_processing_instruction(node, writer, self.pretty);
        }

        if is_comment_node(&name) {
            return writers::write_comment(node, writer, self.pretty);
        }

        if is_text_node(&name) {
            return writers::write_text_content(node, writer);
        }

        // Regular element - use closure to enable recursive calls
        writers::write_element(node, writer, depth, |n, w, d| self.node_to_xml(n, w, d))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn convert(kdl: &str) -> String {
        let converter = KdlToXmlConverter::new().pretty(false);
        converter.convert(kdl).expect("Conversion failed")
    }

    #[test]
    fn test_simple_element() {
        let xml = convert("root");
        assert_eq!(xml, "<root/>");
    }

    #[test]
    fn test_element_with_attribute() {
        let xml = convert(r#"element foo="bar""#);
        assert_eq!(xml, r#"<element foo="bar"/>"#);
    }

    #[test]
    fn test_element_with_text() {
        let xml = convert(r#"greeting "hello""#);
        assert_eq!(xml, "<greeting>hello</greeting>");
    }

    #[test]
    fn test_nested_elements() {
        let xml = convert("parent { child }");
        assert_eq!(xml, "<parent><child/></parent>");
    }

    #[test]
    fn test_mixed_content() {
        let xml = convert(r#"span { - "before "; b "bold"; - " after" }"#);
        assert_eq!(xml, "<span>before <b>bold</b> after</span>");
    }

    #[test]
    fn test_doctype() {
        let xml = convert(
            r#"!doctype "html"
html"#,
        );
        assert!(xml.contains("<!DOCTYPE html>"));
        assert!(xml.contains("<html/>"));
    }

    #[test]
    fn test_xml_declaration() {
        let xml = convert(
            r#"?xml version="1.0" encoding="UTF-8"
root"#,
        );
        assert!(xml.contains("<?xml"));
        assert!(xml.contains("version=\"1.0\""));
        assert!(xml.contains("<root/>"));
    }

    #[test]
    fn test_comment() {
        let xml = convert(
            r#"! " comment "
root"#,
        );
        assert!(xml.contains("<!-- comment -->"));
        assert!(xml.contains("<root/>"));
    }

    #[test]
    fn test_multiple_attributes() {
        let xml = convert(r#"link href="style.css" rel="stylesheet""#);
        assert!(xml.contains("href=\"style.css\""));
        assert!(xml.contains("rel=\"stylesheet\""));
    }

    #[test]
    fn test_namespace_prefix() {
        let xml = convert(r#"ns:element xmlns:ns="http://example.com""#);
        assert!(xml.contains("ns:element"));
        assert!(xml.contains("xmlns:ns="));
    }

    #[test]
    fn test_deeply_nested() {
        let xml = convert("a { b { c { d text } } }");
        assert_eq!(xml, "<a><b><c><d>text</d></c></b></a>");
    }

    #[test]
    fn test_entity_encoding() {
        let xml = convert(r#"p "<hello> & \"world\"""#);
        // The text should be escaped in XML
        assert!(xml.contains("&lt;hello&gt;"));
        assert!(xml.contains("&amp;"));
    }
}
