//! Shared XML validation utilities.
//!
//! This module provides validation functions used by both the XML-to-KDL
//! and KDL-to-XML converters.

use crate::error::Kdl2XmlError;

/// Validates that a name follows XML naming rules.
///
/// XML names must:
/// - Start with a letter, underscore, or colon
/// - Contain only letters, digits, hyphens, underscores, periods, and colons
///
/// # Arguments
/// * `name` - The name to validate
/// * `context` - A description for error messages (e.g., "element", "attribute")
///
/// # Returns
/// * `Ok(())` if the name is valid
/// * `Err(Kdl2XmlError)` if the name is invalid
pub fn validate_xml_name(name: &str, context: &str) -> Result<(), Kdl2XmlError> {
    if name.is_empty() {
        return Err(Kdl2XmlError::InvalidStructure(format!(
            "{} name cannot be empty",
            context
        )));
    }

    let mut chars = name.chars();
    let first = chars.next().unwrap();

    // First character must be a letter, underscore, or colon
    if !first.is_ascii_alphabetic() && first != '_' && first != ':' {
        return Err(Kdl2XmlError::InvalidStructure(format!(
            "Invalid {} name '{}': must start with a letter, underscore, or colon",
            context, name
        )));
    }

    // Remaining characters can be letters, digits, hyphens, underscores, periods, or colons
    for c in chars {
        if !c.is_ascii_alphanumeric() && c != '-' && c != '_' && c != '.' && c != ':' {
            return Err(Kdl2XmlError::InvalidStructure(format!(
                "Invalid {} name '{}': contains invalid character '{}'",
                context, name, c
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_names() {
        assert!(validate_xml_name("root", "element").is_ok());
        assert!(validate_xml_name("_private", "element").is_ok());
        assert!(validate_xml_name(":colon", "element").is_ok());
        assert!(validate_xml_name("my-element", "element").is_ok());
        assert!(validate_xml_name("my.element", "element").is_ok());
        assert!(validate_xml_name("ns:element", "element").is_ok());
        assert!(validate_xml_name("element123", "element").is_ok());
    }

    #[test]
    fn test_invalid_names() {
        assert!(validate_xml_name("", "element").is_err());
        assert!(validate_xml_name("123start", "element").is_err());
        assert!(validate_xml_name("-hyphen", "element").is_err());
        assert!(validate_xml_name("has space", "element").is_err());
        assert!(validate_xml_name("has@symbol", "element").is_err());
    }
}
