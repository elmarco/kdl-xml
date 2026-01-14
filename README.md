# kdl-xml

[![codecov](https://codecov.io/gh/elmarco/kdl-xml/graph/badge.svg)](https://codecov.io/gh/elmarco/kdl-xml)

Bidirectional XML to KDL conversion following the [XiK (XML in KDL)](https://kdl.dev/xik/) specification.

## Features

- Convert XML to KDL format
- Convert KDL back to XML
- Library API and CLI tools
- Configurable formatting options

## Installation

```sh
cargo install --path .
```

## CLI Usage

### XML to KDL

```sh
xml2kdl -i input.xml -o output.kdl
```

### KDL to XML

```sh
kdl2xml -i input.kdl -o output.xml
```

Both tools support stdin/stdout when input/output files are not specified.

## Library Usage

```rust
use kdl_xml::{XmlToKdlConverter, KdlToXmlConverter};

// Convert XML to KDL
let xml = "<greeting>Hello, World!</greeting>";
let kdl = XmlToKdlConverter::new().convert(xml).unwrap();

// Convert KDL back to XML
let kdl_str = r#"greeting "Hello, World!""#;
let xml_out = KdlToXmlConverter::new().convert(kdl_str).unwrap();
```

## License

MIT OR Apache-2.0
