//! XiK specification constants for special node names.
//!
//! These constants define the node naming conventions used in the XiK format
//! to represent special XML constructs (DOCTYPE, comments, text, processing instructions).

/// Node name for DOCTYPE declarations: `!doctype "html"`
pub const DOCTYPE_NODE_NAME: &str = "!doctype";

/// Node name for XML comments: `! "comment text"`
pub const COMMENT_NODE_NAME: &str = "!";

/// Node name for text content in mixed content: `- "text"`
pub const TEXT_NODE_NAME: &str = "-";

/// Prefix character for processing instructions: `?xml`, `?php`
pub const PI_PREFIX: char = '?';
