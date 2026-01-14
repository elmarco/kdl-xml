//! DOCTYPE-specific validation.
//!
//! This module provides validation for DOCTYPE content to prevent
//! injection attacks and ensure valid XML output.

use crate::error::Kdl2XmlError;
use crate::validation::validate_xml_name;

/// Validates DOCTYPE content to prevent injection attacks.
///
/// DOCTYPE format: `NAME (PUBLIC|SYSTEM)? quoted-strings? ("[" internal-subset "]")?`
///
/// # Arguments
/// * `content` - The DOCTYPE content string
///
/// # Returns
/// * `Ok(())` - If the content is valid
/// * `Err(Kdl2XmlError)` - If the content is invalid or potentially malicious
pub fn validate_doctype_content(content: &str) -> Result<(), Kdl2XmlError> {
    let content = content.trim();
    if content.is_empty() {
        return Err(Kdl2XmlError::InvalidStructure(
            "DOCTYPE content cannot be empty".to_string(),
        ));
    }

    // Extract the root element name (first word)
    let root_name: String = content.chars().take_while(|c| !c.is_whitespace()).collect();

    // Validate root element name
    validate_xml_name(&root_name, "DOCTYPE root element")?;

    // Check for unbalanced `>` outside of quotes and internal subset brackets
    // This prevents injection like: `html><script>...`
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut bracket_depth = 0;

    for c in content.chars() {
        match c {
            '"' if !in_single_quote => in_double_quote = !in_double_quote,
            '\'' if !in_double_quote => in_single_quote = !in_single_quote,
            '[' if !in_single_quote && !in_double_quote => bracket_depth += 1,
            ']' if !in_single_quote && !in_double_quote => {
                if bracket_depth > 0 {
                    bracket_depth -= 1;
                }
            }
            '>' if !in_single_quote && !in_double_quote && bracket_depth == 0 => {
                return Err(Kdl2XmlError::InvalidStructure(
                    "DOCTYPE content contains unquoted '>' which would produce invalid XML"
                        .to_string(),
                ));
            }
            _ => {}
        }
    }

    // Check for unclosed quotes or brackets
    if in_single_quote || in_double_quote {
        return Err(Kdl2XmlError::InvalidStructure(
            "DOCTYPE content has unclosed quote".to_string(),
        ));
    }
    if bracket_depth != 0 {
        return Err(Kdl2XmlError::InvalidStructure(
            "DOCTYPE content has unclosed internal subset bracket".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_doctype() {
        assert!(validate_doctype_content("html").is_ok());
        assert!(
            validate_doctype_content(
                "html PUBLIC \"-//W3C//DTD XHTML 1.0//EN\" \"http://example.com\""
            )
            .is_ok()
        );
        assert!(validate_doctype_content("root [<!ELEMENT root ANY>]").is_ok());
    }

    #[test]
    fn test_invalid_doctype_injection() {
        assert!(validate_doctype_content("html><script>evil</script").is_err());
    }

    #[test]
    fn test_invalid_doctype_unclosed_quote() {
        assert!(validate_doctype_content("html PUBLIC \"unclosed").is_err());
    }

    #[test]
    fn test_empty_doctype() {
        assert!(validate_doctype_content("").is_err());
    }
}
