//! XML writing functions for different node types.
//!
//! This module contains the individual writer functions for DOCTYPE,
//! processing instructions, comments, text, and elements.

use crate::error::Kdl2XmlError;
use crate::validation::validate_xml_name;
use kdl::{KdlNode, KdlValue};
use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use std::io::{Cursor, Write};

use super::node_types::{get_first_argument, get_text_argument, has_properties};
use super::validation::validate_doctype_content;

/// Writes a DOCTYPE declaration to the output.
pub fn write_doctype(
    node: &KdlNode,
    writer: &mut Writer<Cursor<Vec<u8>>>,
    pretty: bool,
) -> Result<(), Kdl2XmlError> {
    let content = get_first_argument(node)?;

    // Validate DOCTYPE content to prevent injection attacks
    validate_doctype_content(&content)?;

    let doctype = format!("<!DOCTYPE {}>", content);

    writer
        .get_mut()
        .write_all(doctype.as_bytes())
        .map_err(Kdl2XmlError::Io)?;
    if pretty {
        writer
            .get_mut()
            .write_all(b"\n")
            .map_err(Kdl2XmlError::Io)?;
    }

    Ok(())
}

/// Writes a processing instruction to the output.
pub fn write_processing_instruction(
    node: &KdlNode,
    writer: &mut Writer<Cursor<Vec<u8>>>,
    pretty: bool,
) -> Result<(), Kdl2XmlError> {
    let name = node.name().to_string();

    // Safely remove the '?' prefix
    let target: String = name.chars().skip(1).collect();
    if target.is_empty() {
        return Err(Kdl2XmlError::InvalidStructure(
            "Processing instruction target name cannot be empty".to_string(),
        ));
    }

    // Validate PI target name
    validate_xml_name(&target, "processing instruction target")?;

    // Check if it's an XML declaration (special handling)
    if target == "xml" {
        let mut decl_parts = Vec::new();

        // Get version (required for XML decl)
        if let Some(version) = node.get("version")
            && let Some(v) = version.as_string()
        {
            decl_parts.push(format!("version=\"{}\"", v));
        }

        // Get encoding (optional)
        if let Some(encoding) = node.get("encoding")
            && let Some(e) = encoding.as_string()
        {
            decl_parts.push(format!("encoding=\"{}\"", e));
        }

        // Get standalone (optional)
        if let Some(standalone) = node.get("standalone")
            && let Some(s) = standalone.as_string()
        {
            decl_parts.push(format!("standalone=\"{}\"", s));
        }

        if !decl_parts.is_empty() {
            let decl = BytesDecl::new(
                node.get("version")
                    .and_then(|v| v.as_string())
                    .unwrap_or("1.0"),
                node.get("encoding").and_then(|v| v.as_string()),
                node.get("standalone").and_then(|v| v.as_string()),
            );
            writer
                .write_event(Event::Decl(decl))
                .map_err(|e| Kdl2XmlError::InvalidStructure(e.to_string()))?;
            if pretty {
                writer
                    .get_mut()
                    .write_all(b"\n")
                    .map_err(Kdl2XmlError::Io)?;
            }
            return Ok(());
        }
    }

    // Build PI content from properties or single argument
    let content = if has_properties(node) {
        // Structured PI: properties become attribute-like content
        let mut parts = Vec::new();
        for entry in node.entries() {
            if let Some(name) = entry.name()
                && let Some(value) = entry.value().as_string()
            {
                // Escape quotes and ?> in attribute values
                let escaped_value = value.replace('"', "&quot;").replace("?>", "? >");
                parts.push(format!("{}=\"{}\"", name, escaped_value));
            }
        }
        parts.join(" ")
    } else if let Ok(arg) = get_first_argument(node) {
        // Unstructured PI: single string argument
        // Escape ?> to prevent premature PI termination
        arg.replace("?>", "? >")
    } else {
        String::new()
    };

    let pi = if content.is_empty() {
        format!("<?{}?>", target)
    } else {
        format!("<?{} {}?>", target, content)
    };

    writer
        .get_mut()
        .write_all(pi.as_bytes())
        .map_err(Kdl2XmlError::Io)?;
    if pretty {
        writer
            .get_mut()
            .write_all(b"\n")
            .map_err(Kdl2XmlError::Io)?;
    }

    Ok(())
}

/// Writes an XML comment to the output.
pub fn write_comment(
    node: &KdlNode,
    writer: &mut Writer<Cursor<Vec<u8>>>,
    pretty: bool,
) -> Result<(), Kdl2XmlError> {
    // XiK spec: Comment nodes must contain a single unnamed string argument and nothing else
    // Validate: no properties (named entries)
    if has_properties(node) {
        return Err(Kdl2XmlError::InvalidStructure(
            "Comment node must not have properties".to_string(),
        ));
    }

    // Validate: no children
    if node.children().is_some_and(|c| !c.nodes().is_empty()) {
        return Err(Kdl2XmlError::InvalidStructure(
            "Comment node must not have children".to_string(),
        ));
    }

    // Validate: exactly one unnamed argument
    let unnamed_count = node.entries().iter().filter(|e| e.name().is_none()).count();
    if unnamed_count != 1 {
        return Err(Kdl2XmlError::InvalidStructure(format!(
            "Comment node must have exactly one string argument, found {}",
            unnamed_count
        )));
    }

    let content = get_first_argument(node)?;

    // Escape invalid comment content:
    // 1. Replace "--" with "- -" (XML forbids -- inside comments)
    // 2. If content ends with "-", add a space (to prevent "--->")
    let mut escaped = content.replace("--", "- -");
    if escaped.ends_with('-') {
        escaped.push(' ');
    }

    let comment = format!("<!--{}-->", escaped);

    writer
        .get_mut()
        .write_all(comment.as_bytes())
        .map_err(Kdl2XmlError::Io)?;
    if pretty {
        writer
            .get_mut()
            .write_all(b"\n")
            .map_err(Kdl2XmlError::Io)?;
    }

    Ok(())
}

/// Writes text content to the output (for mixed content `-` nodes).
pub fn write_text_content(
    node: &KdlNode,
    writer: &mut Writer<Cursor<Vec<u8>>>,
) -> Result<(), Kdl2XmlError> {
    let content = get_first_argument(node)?;
    let text = BytesText::new(&content);
    writer
        .write_event(Event::Text(text))
        .map_err(|e| Kdl2XmlError::InvalidStructure(e.to_string()))?;
    Ok(())
}

/// Writes an XML element and its children to the output.
pub fn write_element<F>(
    node: &KdlNode,
    writer: &mut Writer<Cursor<Vec<u8>>>,
    depth: usize,
    node_to_xml: F,
) -> Result<(), Kdl2XmlError>
where
    F: Fn(&KdlNode, &mut Writer<Cursor<Vec<u8>>>, usize) -> Result<(), Kdl2XmlError>,
{
    let name = node.name().to_string();

    // Validate element name
    validate_xml_name(&name, "element")?;

    // XiK spec: Element cannot have both text argument AND children
    let unnamed_count = node.entries().iter().filter(|e| e.name().is_none()).count();
    let has_text_argument = unnamed_count > 0;
    let has_children = node.children().is_some_and(|c| !c.nodes().is_empty());

    // XiK spec: Elements can have at most one unnamed argument (text content)
    if unnamed_count > 1 {
        return Err(Kdl2XmlError::InvalidStructure(format!(
            "Element '{}' cannot have multiple text arguments, found {}",
            name, unnamed_count
        )));
    }

    if has_text_argument && has_children {
        return Err(Kdl2XmlError::InvalidStructure(format!(
            "Element '{}' cannot have both text argument and child nodes",
            name
        )));
    }

    // Build start tag with attributes
    let mut elem = BytesStart::new(&name);

    // Add properties as attributes
    for entry in node.entries() {
        if let Some(attr_name) = entry.name() {
            // Validate attribute name
            let attr_name_str = attr_name.to_string();
            validate_xml_name(&attr_name_str, "attribute")?;

            // XiK spec: Property values must be strings
            let value = match entry.value() {
                KdlValue::String(s) => s.clone(),
                other => {
                    return Err(Kdl2XmlError::InvalidStructure(format!(
                        "Attribute '{}' on element '{}' must have a string value, got {:?}",
                        attr_name_str, name, other
                    )));
                }
            };
            elem.push_attribute((attr_name_str.as_str(), value.as_str()));
        }
    }

    // Check for content
    let text_argument = get_text_argument(node)?;

    if !has_children && text_argument.is_none() {
        // Empty element
        writer
            .write_event(Event::Empty(elem))
            .map_err(|e| Kdl2XmlError::InvalidStructure(e.to_string()))?;
    } else {
        // Start tag
        writer
            .write_event(Event::Start(elem))
            .map_err(|e| Kdl2XmlError::InvalidStructure(e.to_string()))?;

        // Write text content if present (single argument)
        if let Some(text) = text_argument {
            let text_event = BytesText::new(&text);
            writer
                .write_event(Event::Text(text_event))
                .map_err(|e| Kdl2XmlError::InvalidStructure(e.to_string()))?;
        }

        // Write children
        if let Some(children) = node.children() {
            for child in children.nodes() {
                node_to_xml(child, writer, depth + 1)?;
            }
        }

        // End tag
        writer
            .write_event(Event::End(BytesEnd::new(&name)))
            .map_err(|e| Kdl2XmlError::InvalidStructure(e.to_string()))?;
    }

    Ok(())
}
