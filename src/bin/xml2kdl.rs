use clap::{Parser, ValueEnum};
use kdl_xml::{CompactMode, Result, XmlToKdlConverter};
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;

/// Compact output mode for CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum CliCompactMode {
    /// Compact all children blocks onto single lines
    All,
    /// Only compact leaf mixed content (where child elements have no nested children)
    Leaf,
}

#[derive(Parser, Debug)]
#[command(name = "xml2kdl")]
#[command(about = "Convert XML to KDL following the XiK specification")]
#[command(version)]
struct Args {
    /// Input XML file (reads from stdin if not provided)
    #[arg(short, long)]
    input: Option<PathBuf>,

    /// Output KDL file (writes to stdout if not provided)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Preserve comments as `!` nodes instead of discarding them
    #[arg(long, default_value = "false")]
    comments_as_nodes: bool,

    /// Keep whitespace-only text nodes between elements
    #[arg(long, default_value = "false")]
    preserve_whitespace: bool,

    /// Pretty-print the output with autoformat
    #[arg(long, short = 'p', default_value = "true")]
    pretty: bool,

    /// Output children blocks on a single line.
    /// Use 'all' to compact everything, or 'leaf' to only compact leaf mixed content.
    #[arg(long, short = 'c', value_enum)]
    compact: Option<CliCompactMode>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Read input
    let xml = match &args.input {
        Some(path) => fs::read_to_string(path)?,
        None => {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)?;
            buf
        }
    };

    // Convert CLI compact mode to library compact mode
    let compact_mode = match args.compact {
        None => CompactMode::None,
        Some(CliCompactMode::All) => CompactMode::All,
        Some(CliCompactMode::Leaf) => CompactMode::Leaf,
    };

    // Configure and run converter
    let converter = XmlToKdlConverter::new()
        .comments_as_nodes(args.comments_as_nodes)
        .preserve_whitespace(args.preserve_whitespace)
        .compact_mode(compact_mode);

    let mut kdl_doc = converter.convert(&xml)?;

    // Apply pretty formatting if requested
    // For leaf compact mode, we apply autoformat first, then compact formatting
    // overrides only the leaf nodes
    if args.pretty {
        match compact_mode {
            CompactMode::None => {
                kdl_doc.autoformat();
            }
            CompactMode::Leaf => {
                // First autoformat everything, then the converter already applied
                // leaf compact formatting, so we need to re-apply after autoformat
                // Actually, the compact formatting is already applied, but we need
                // to apply autoformat to non-leaf nodes only.
                // For now, we don't autoformat in leaf mode - the formatting is
                // handled by the converter.
            }
            CompactMode::All => {
                // Compact mode sets its own formatting
            }
        }
    }

    // Write output
    let output = kdl_doc.to_string();
    match &args.output {
        Some(path) => fs::write(path, &output)?,
        None => io::stdout().write_all(output.as_bytes())?,
    }

    Ok(())
}
