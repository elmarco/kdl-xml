//! kdl-xml: Bidirectional XML to KDL conversion following the XiK specification.
//!
//! This crate provides converters for transforming XML documents to KDL format
//! and vice versa, implementing the XiK (XML in KDL) specification.
//!
//! # Example
//!
//! ```
//! use kdl_xml::{XmlToKdlConverter, KdlToXmlConverter};
//!
//! // Convert XML to KDL
//! let xml = "<greeting>Hello, World!</greeting>";
//! let kdl = XmlToKdlConverter::new().convert(xml).unwrap();
//!
//! // Convert KDL back to XML
//! let kdl_str = r#"greeting "Hello, World!""#;
//! let xml_out = KdlToXmlConverter::new().convert(kdl_str).unwrap();
//! ```

pub mod constants;
pub mod error;
pub mod kdl_to_xml;
pub mod validation;
pub mod xml_to_kdl;

pub use error::{Kdl2XmlError, Kdl2XmlResult, Result, Xml2KdlError};
pub use kdl_to_xml::KdlToXmlConverter;
pub use xml_to_kdl::{CompactMode, XmlToKdlConverter};
