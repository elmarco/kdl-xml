use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Xml2KdlError {
    #[error("XML parsing error: {0}")]
    XmlParse(#[from] quick_xml::Error),

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("UTF-8 decoding error: {0}")]
    Utf8(#[from] std::str::Utf8Error),

    #[error("Invalid XML structure: {0}")]
    InvalidStructure(String),
}

pub type Result<T> = std::result::Result<T, Xml2KdlError>;

#[derive(Error, Debug)]
pub enum Kdl2XmlError {
    #[error("KDL parsing error: {0}")]
    KdlParse(#[from] kdl::KdlError),

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("UTF-8 encoding error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("Invalid XiK structure: {0}")]
    InvalidStructure(String),
}

pub type Kdl2XmlResult<T> = std::result::Result<T, Kdl2XmlError>;
