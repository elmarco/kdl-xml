use clap::Parser;
use kdl_xml::{Kdl2XmlResult, KdlToXmlConverter};
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "kdl2xml")]
#[command(about = "Convert KDL (XiK format) to XML")]
#[command(version)]
struct Args {
    /// Input KDL file (reads from stdin if not provided)
    #[arg(short, long)]
    input: Option<PathBuf>,

    /// Output XML file (writes to stdout if not provided)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Disable pretty-printing (output on single line)
    #[arg(long)]
    no_pretty: bool,

    /// Indentation string for pretty printing
    #[arg(long, default_value = "  ")]
    indent: String,
}

fn main() -> Kdl2XmlResult<()> {
    let args = Args::parse();

    // Read input
    let kdl = match &args.input {
        Some(path) => fs::read_to_string(path)?,
        None => {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)?;
            buf
        }
    };

    // Configure and run converter
    let converter = KdlToXmlConverter::new()
        .pretty(!args.no_pretty)
        .indent(&args.indent);

    let xml = converter.convert(&kdl)?;

    // Write output
    match &args.output {
        Some(path) => fs::write(path, &xml)?,
        None => io::stdout().write_all(xml.as_bytes())?,
    }

    Ok(())
}
