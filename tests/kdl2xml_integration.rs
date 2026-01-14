use kdl_xml::KdlToXmlConverter;
use quick_xml::Reader;
use quick_xml::escape::unescape;
use quick_xml::events::Event;

fn convert(kdl: &str) -> String {
    let converter = KdlToXmlConverter::new().pretty(false);
    converter.convert(kdl).expect("Conversion failed")
}

fn convert_pretty(kdl: &str) -> String {
    let converter = KdlToXmlConverter::new().pretty(true);
    converter.convert(kdl).expect("Conversion failed")
}

fn try_convert(kdl: &str) -> Result<String, kdl_xml::Kdl2XmlError> {
    let converter = KdlToXmlConverter::new().pretty(false);
    converter.convert(kdl)
}

// ============================================================================
// XML Parsing Helpers
// ============================================================================

/// Parsed XML element with attributes and children
#[derive(Debug, Clone)]
struct XmlElement {
    name: String,
    attributes: Vec<(String, String)>,
    children: Vec<XmlContent>,
}

#[derive(Debug, Clone)]
enum XmlContent {
    Element(XmlElement),
    Text(String),
    Comment(String),
    PI { target: String, content: String },
    DocType(String),
}

impl XmlElement {
    fn get_attr(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    fn has_attr(&self, name: &str, value: &str) -> bool {
        self.get_attr(name) == Some(value)
    }

    fn child_elements(&self) -> Vec<&XmlElement> {
        self.children
            .iter()
            .filter_map(|c| match c {
                XmlContent::Element(e) => Some(e),
                _ => None,
            })
            .collect()
    }

    fn find_element(&self, name: &str) -> Option<&XmlElement> {
        self.child_elements().into_iter().find(|e| e.name == name)
    }

    fn text_content(&self) -> String {
        self.children
            .iter()
            .filter_map(|c| match c {
                XmlContent::Text(t) => Some(t.as_str()),
                _ => None,
            })
            .collect()
    }

    fn full_text_content(&self) -> String {
        self.children
            .iter()
            .map(|c| match c {
                XmlContent::Text(t) => t.clone(),
                XmlContent::Element(e) => e.full_text_content(),
                _ => String::new(),
            })
            .collect()
    }
}

/// Parsed XML document
struct XmlDoc {
    prolog: Vec<XmlContent>,
    root: Option<XmlElement>,
}

impl XmlDoc {
    fn has_doctype(&self, contains: &str) -> bool {
        self.prolog.iter().any(|c| match c {
            XmlContent::DocType(s) => s.contains(contains),
            _ => false,
        })
    }

    fn has_xml_decl_attr(&self, name: &str, value: &str) -> bool {
        self.prolog.iter().any(|c| match c {
            XmlContent::PI { target, content } if target == "xml" => {
                content.contains(&format!("{}=\"{}\"", name, value))
            }
            _ => false,
        })
    }

    fn has_pi(&self, target: &str) -> bool {
        self.prolog.iter().any(|c| match c {
            XmlContent::PI { target: t, .. } => t == target,
            _ => false,
        })
    }

    fn get_pi_content(&self, target: &str) -> Option<&str> {
        self.prolog.iter().find_map(|c| match c {
            XmlContent::PI { target: t, content } if t == target => Some(content.as_str()),
            _ => None,
        })
    }

    fn has_comment(&self, contains: &str) -> bool {
        self.prolog.iter().any(|c| match c {
            XmlContent::Comment(s) => s.contains(contains),
            _ => false,
        })
    }

    fn root(&self) -> &XmlElement {
        self.root.as_ref().expect("No root element")
    }
}

fn parse_xml(xml: &str) -> XmlDoc {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);

    let mut prolog = Vec::new();
    let mut root = None;
    let mut stack: Vec<XmlElement> = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let attributes: Vec<(String, String)> = e
                    .attributes()
                    .filter_map(|a| a.ok())
                    .map(|a| {
                        (
                            String::from_utf8_lossy(a.key.as_ref()).to_string(),
                            a.unescape_value().unwrap_or_default().to_string(),
                        )
                    })
                    .collect();
                stack.push(XmlElement {
                    name,
                    attributes,
                    children: Vec::new(),
                });
            }
            Ok(Event::End(_)) => {
                if let Some(elem) = stack.pop() {
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(XmlContent::Element(elem));
                    } else {
                        root = Some(elem);
                    }
                }
            }
            Ok(Event::Empty(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let attributes: Vec<(String, String)> = e
                    .attributes()
                    .filter_map(|a| a.ok())
                    .map(|a| {
                        (
                            String::from_utf8_lossy(a.key.as_ref()).to_string(),
                            a.unescape_value().unwrap_or_default().to_string(),
                        )
                    })
                    .collect();
                let elem = XmlElement {
                    name,
                    attributes,
                    children: Vec::new(),
                };
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(XmlContent::Element(elem));
                } else {
                    root = Some(elem);
                }
            }
            Ok(Event::Text(e)) => {
                let decoded = e.decode().unwrap_or_default();
                let text = unescape(&decoded).unwrap_or_default().to_string();
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(XmlContent::Text(text));
                }
            }
            Ok(Event::GeneralRef(e)) => {
                // Handle entity references like &lt; &gt; &amp;
                let decoded = e.decode().unwrap_or_default();
                let text = unescape(&format!("&{};", decoded))
                    .unwrap_or_default()
                    .to_string();
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(XmlContent::Text(text));
                }
            }
            Ok(Event::Comment(e)) => {
                let text = String::from_utf8_lossy(&e).to_string();
                if stack.is_empty() {
                    prolog.push(XmlContent::Comment(text));
                } else if let Some(parent) = stack.last_mut() {
                    parent.children.push(XmlContent::Comment(text));
                }
            }
            Ok(Event::Decl(e)) => {
                let version = e
                    .version()
                    .ok()
                    .map(|v| String::from_utf8_lossy(&v).to_string())
                    .unwrap_or_default();
                let encoding = e
                    .encoding()
                    .and_then(|r| r.ok())
                    .map(|v| String::from_utf8_lossy(&v).to_string());
                let mut content = format!("version=\"{}\"", version);
                if let Some(enc) = encoding {
                    content.push_str(&format!(" encoding=\"{}\"", enc));
                }
                prolog.push(XmlContent::PI {
                    target: "xml".to_string(),
                    content,
                });
            }
            Ok(Event::PI(e)) => {
                let raw = String::from_utf8_lossy(&e).to_string();
                let (target, content) = if let Some(pos) = raw.find(char::is_whitespace) {
                    (raw[..pos].to_string(), raw[pos..].trim().to_string())
                } else {
                    (raw, String::new())
                };
                prolog.push(XmlContent::PI { target, content });
            }
            Ok(Event::DocType(e)) => {
                let text = String::from_utf8_lossy(&e).to_string();
                prolog.push(XmlContent::DocType(text));
            }
            Ok(Event::Eof) => break,
            Err(e) => panic!("Error parsing XML: {:?}", e),
            _ => {}
        }
        buf.clear();
    }

    XmlDoc { prolog, root }
}

// ============================================================================
// Element Tests
// ============================================================================

#[test]
fn simple_element() {
    let xml = convert("root");
    let doc = parse_xml(&xml);
    assert_eq!(doc.root().name, "root");
    assert!(doc.root().children.is_empty());
}

#[test]
fn element_with_single_attribute() {
    let xml = convert(r#"element foo="bar""#);
    let doc = parse_xml(&xml);
    assert_eq!(doc.root().name, "element");
    assert!(doc.root().has_attr("foo", "bar"));
}

#[test]
fn element_with_multiple_attributes() {
    let xml = convert(r#"element foo="bar" baz="qux""#);
    let doc = parse_xml(&xml);
    assert!(doc.root().has_attr("foo", "bar"));
    assert!(doc.root().has_attr("baz", "qux"));
}

#[test]
fn nested_elements() {
    let xml = convert("parent { child }");
    let doc = parse_xml(&xml);
    assert_eq!(doc.root().name, "parent");
    let child = doc.root().find_element("child").expect("child not found");
    assert_eq!(child.name, "child");
}

#[test]
fn deeply_nested_elements() {
    let xml = convert("a { b { c { d { e } } } }");
    let doc = parse_xml(&xml);
    assert_eq!(doc.root().name, "a");
    let b = doc.root().find_element("b").unwrap();
    let c = b.find_element("c").unwrap();
    let d = c.find_element("d").unwrap();
    let e = d.find_element("e").unwrap();
    assert_eq!(e.name, "e");
}

#[test]
fn multiple_children() {
    let xml = convert("parent { a; b; c }");
    let doc = parse_xml(&xml);
    let children = doc.root().child_elements();
    assert_eq!(children.len(), 3);
    assert_eq!(children[0].name, "a");
    assert_eq!(children[1].name, "b");
    assert_eq!(children[2].name, "c");
}

// ============================================================================
// Text Content Tests
// ============================================================================

#[test]
fn pure_text_content() {
    let xml = convert(r#"greeting "hello world""#);
    let doc = parse_xml(&xml);
    assert_eq!(doc.root().name, "greeting");
    assert_eq!(doc.root().text_content(), "hello world");
}

#[test]
fn text_with_attribute() {
    let xml = convert(r#"a href="http://example.com" "link text""#);
    let doc = parse_xml(&xml);
    assert!(doc.root().has_attr("href", "http://example.com"));
    assert_eq!(doc.root().text_content(), "link text");
}

#[test]
fn mixed_content() {
    let xml = convert(r#"span { - "some "; b "bold"; - " text" }"#);
    let doc = parse_xml(&xml);
    assert_eq!(doc.root().name, "span");
    let b = doc.root().find_element("b").unwrap();
    assert_eq!(b.text_content(), "bold");
    assert_eq!(doc.root().full_text_content(), "some bold text");
}

#[test]
fn mixed_content_text_only() {
    let xml = convert(r#"p { - "hello "; - "world" }"#);
    let doc = parse_xml(&xml);
    assert_eq!(doc.root().text_content(), "hello world");
}

// ============================================================================
// Entity Encoding Tests
// ============================================================================

#[test]
fn entity_encoding_lt() {
    let xml = convert(r#"p "<""#);
    let doc = parse_xml(&xml);
    assert_eq!(doc.root().text_content(), "<");
}

#[test]
fn entity_encoding_gt() {
    let xml = convert(r#"p ">""#);
    let doc = parse_xml(&xml);
    assert_eq!(doc.root().text_content(), ">");
}

#[test]
fn entity_encoding_amp() {
    let xml = convert(r#"p "&""#);
    let doc = parse_xml(&xml);
    assert_eq!(doc.root().text_content(), "&");
}

#[test]
fn entity_encoding_multiple() {
    let xml = convert(r#"p "<hello> & world""#);
    let doc = parse_xml(&xml);
    assert_eq!(doc.root().text_content(), "<hello> & world");
}

// ============================================================================
// Special Node Tests
// ============================================================================

#[test]
fn doctype_html() {
    let xml = convert(
        r#"!doctype "html"
html"#,
    );
    let doc = parse_xml(&xml);
    assert!(doc.has_doctype("html"));
    assert_eq!(doc.root().name, "html");
}

#[test]
fn doctype_xhtml() {
    let xml = convert(
        r#"!doctype "html PUBLIC \"-//W3C//DTD XHTML 1.0 Strict//EN\""
html"#,
    );
    let doc = parse_xml(&xml);
    assert!(doc.has_doctype("html PUBLIC"));
    assert_eq!(doc.root().name, "html");
}

#[test]
fn doctype_with_internal_subset() {
    let xml = convert(
        r#"!doctype "html [<!ENTITY test \"value\">]"
html"#,
    );
    let doc = parse_xml(&xml);
    assert!(doc.has_doctype("html ["));
    assert_eq!(doc.root().name, "html");
}

#[test]
fn doctype_injection_rejected() {
    let result = try_convert(
        r#"!doctype "html><script>alert('xss')</script><div"
root"#,
    );
    assert!(
        result.is_err(),
        "DOCTYPE with unquoted > should be rejected"
    );
}

#[test]
fn doctype_unclosed_quote_rejected() {
    let result = try_convert(
        r#"!doctype "html PUBLIC \"unclosed"
root"#,
    );
    assert!(
        result.is_err(),
        "DOCTYPE with unclosed quote should be rejected"
    );
}

#[test]
fn doctype_invalid_root_name_rejected() {
    let result = try_convert(
        r#"!doctype "123invalid"
root"#,
    );
    assert!(
        result.is_err(),
        "DOCTYPE with invalid root name should be rejected"
    );
}

#[test]
fn doctype_empty_rejected() {
    let result = try_convert(
        r#"!doctype ""
root"#,
    );
    assert!(result.is_err(), "Empty DOCTYPE should be rejected");
}

#[test]
fn xml_declaration() {
    let xml = convert(
        r#"?xml version="1.0"
root"#,
    );
    let doc = parse_xml(&xml);
    assert!(doc.has_xml_decl_attr("version", "1.0"));
    assert_eq!(doc.root().name, "root");
}

#[test]
fn xml_declaration_with_encoding() {
    let xml = convert(
        r#"?xml version="1.0" encoding="UTF-8"
root"#,
    );
    let doc = parse_xml(&xml);
    assert!(doc.has_xml_decl_attr("encoding", "UTF-8"));
    assert_eq!(doc.root().name, "root");
}

#[test]
fn processing_instruction() {
    let xml = convert(
        r#"?xml-stylesheet type="text/css" href="style.css"
root"#,
    );
    let doc = parse_xml(&xml);
    assert!(doc.has_pi("xml-stylesheet"));
    let pi_content = doc.get_pi_content("xml-stylesheet").unwrap();
    assert!(pi_content.contains("type=\"text/css\""));
    assert!(pi_content.contains("href=\"style.css\""));
    assert_eq!(doc.root().name, "root");
}

#[test]
fn empty_pi_target_rejected() {
    let result = try_convert(
        r#"?
root"#,
    );
    assert!(result.is_err(), "Empty PI target should be rejected");
}

#[test]
fn pi_with_invalid_target_name_rejected() {
    let result = try_convert(
        r#"?123target
root"#,
    );
    assert!(
        result.is_err(),
        "PI target starting with number should be rejected"
    );
}

#[test]
fn comment_node() {
    let xml = convert(
        r#"! " This is a comment "
root"#,
    );
    let doc = parse_xml(&xml);
    assert!(doc.has_comment("This is a comment"));
    assert_eq!(doc.root().name, "root");
}

// ============================================================================
// Namespace Tests
// ============================================================================

#[test]
fn default_namespace() {
    let xml = convert(r#"root xmlns="http://example.com""#);
    let doc = parse_xml(&xml);
    assert!(doc.root().has_attr("xmlns", "http://example.com"));
}

#[test]
fn prefixed_namespace() {
    let xml = convert(r#"root xmlns:ns="http://example.com""#);
    let doc = parse_xml(&xml);
    assert!(doc.root().has_attr("xmlns:ns", "http://example.com"));
}

#[test]
fn prefixed_element() {
    let xml = convert(r#"ns:root xmlns:ns="http://example.com""#);
    let doc = parse_xml(&xml);
    assert_eq!(doc.root().name, "ns:root");
    assert!(doc.root().has_attr("xmlns:ns", "http://example.com"));
}

#[test]
fn prefixed_attribute() {
    let xml = convert(r#"a xlink:href="url""#);
    let doc = parse_xml(&xml);
    assert!(doc.root().has_attr("xlink:href", "url"));
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn empty_attribute_value() {
    let xml = convert(r#"input value="""#);
    let doc = parse_xml(&xml);
    assert!(doc.root().has_attr("value", ""));
}

#[test]
fn unicode_content() {
    let xml = convert(r#"greeting "Hello, 世界!""#);
    let doc = parse_xml(&xml);
    assert_eq!(doc.root().text_content(), "Hello, 世界!");
}

#[test]
fn unicode_emoji() {
    let xml = convert(r#"emoji "🎉🚀""#);
    let doc = parse_xml(&xml);
    assert_eq!(doc.root().text_content(), "🎉🚀");
}

#[test]
fn very_long_text() {
    let long_text = "x".repeat(10000);
    let kdl = format!(r#"content "{}""#, long_text);
    let xml = convert(&kdl);
    let doc = parse_xml(&xml);
    assert_eq!(doc.root().text_content().len(), 10000);
}

#[test]
fn deeply_nested_50_levels() {
    let mut kdl = String::new();
    for i in 0..50 {
        kdl.push_str(&format!("n{} {{ ", i));
    }
    kdl.push_str("leaf");
    for _ in 0..50 {
        kdl.push_str(" }");
    }
    let xml = convert(&kdl);
    let doc = parse_xml(&xml);
    assert_eq!(doc.root().name, "n0");
    // Navigate to deepest element
    let mut current = doc.root();
    for i in 1..50 {
        current = current
            .find_element(&format!("n{}", i))
            .unwrap_or_else(|| panic!("n{} not found", i));
    }
    assert!(current.find_element("leaf").is_some());
}

#[test]
fn element_name_with_hyphen() {
    let xml = convert("my-element");
    let doc = parse_xml(&xml);
    assert_eq!(doc.root().name, "my-element");
}

#[test]
fn element_name_with_underscore() {
    let xml = convert("my_element");
    let doc = parse_xml(&xml);
    assert_eq!(doc.root().name, "my_element");
}

#[test]
fn element_name_with_numbers() {
    let xml = convert("element123");
    let doc = parse_xml(&xml);
    assert_eq!(doc.root().name, "element123");
}

// ============================================================================
// Pretty Printing Tests
// ============================================================================

#[test]
fn pretty_nested_elements() {
    let xml = convert_pretty("parent { child { grandchild } }");
    assert!(xml.contains('\n'));
    assert!(xml.contains("  ")); // Indentation
    let doc = parse_xml(&xml);
    assert_eq!(doc.root().name, "parent");
}

#[test]
fn pretty_multiple_children() {
    let xml = convert_pretty("parent { a; b; c }");
    let lines: Vec<&str> = xml.lines().collect();
    assert!(lines.len() > 1);
    let doc = parse_xml(&xml);
    assert_eq!(doc.root().child_elements().len(), 3);
}

// ============================================================================
// Error Cases
// ============================================================================

#[test]
fn invalid_kdl_syntax() {
    let result = try_convert("{ invalid }");
    assert!(result.is_err());
}

#[test]
fn empty_input() {
    let result = try_convert("");
    assert!(result.is_err());
}

#[test]
fn multiple_root_elements_rejected() {
    let result = try_convert("a; b");
    assert!(result.is_err(), "Multiple root elements should be rejected");
}

#[test]
fn text_with_children_rejected() {
    let result = try_convert(r#"span "text" { child }"#);
    assert!(
        result.is_err(),
        "Text argument with children should be rejected"
    );
}

#[test]
fn invalid_element_name_rejected() {
    let result = try_convert("123invalid");
    assert!(
        result.is_err(),
        "Element names starting with numbers should be rejected"
    );
}

#[test]
fn comment_double_hyphen_escaped() {
    let xml = convert(
        r#"! "test--comment"
root"#,
    );
    // Double hyphen should be escaped - verify the comment parses correctly
    let doc = parse_xml(&xml);
    assert!(doc.has_comment("test- -comment"));
    assert_eq!(doc.root().name, "root");
}

#[test]
fn comment_trailing_hyphen_escaped() {
    let xml = convert(
        r#"! "comment-"
root"#,
    );
    // Trailing hyphen should have space added - verify parsing succeeds
    let doc = parse_xml(&xml);
    assert!(doc.has_comment("comment-"));
    assert_eq!(doc.root().name, "root");
}

#[test]
fn pi_content_escaped() {
    let xml = convert(
        r#"?custom "content?>"
root"#,
    );
    // ?> should be escaped - just verify the XML parses without error
    let doc = parse_xml(&xml);
    assert!(doc.has_pi("custom"));
    assert_eq!(doc.root().name, "root");
}

// ============================================================================
// Comment Node Structure Validation Tests
// ============================================================================

#[test]
fn comment_with_properties_rejected() {
    let result = try_convert(
        r#"! "comment" prop="value"
root"#,
    );
    assert!(
        result.is_err(),
        "Comment node with properties should be rejected"
    );
}

#[test]
fn comment_with_children_rejected() {
    let result = try_convert(
        r#"! "comment" { child }
root"#,
    );
    assert!(
        result.is_err(),
        "Comment node with children should be rejected"
    );
}

#[test]
fn comment_with_no_argument_rejected() {
    let result = try_convert(
        r#"!
root"#,
    );
    assert!(
        result.is_err(),
        "Comment node without argument should be rejected"
    );
}

// ============================================================================
// Multiple Unnamed Arguments Tests
// ============================================================================

#[test]
fn multiple_unnamed_arguments_rejected() {
    let result = try_convert(r#"element "text1" "text2""#);
    assert!(
        result.is_err(),
        "Multiple unnamed arguments should be rejected"
    );
}

#[test]
fn multiple_unnamed_arguments_with_properties_rejected() {
    let result = try_convert(r#"element "text1" attr="value" "text2""#);
    assert!(
        result.is_err(),
        "Multiple unnamed arguments should be rejected even with properties"
    );
}

#[test]
fn single_unnamed_argument_accepted() {
    let xml = convert(r#"element "text""#);
    let doc = parse_xml(&xml);
    assert_eq!(doc.root().name, "element");
    assert_eq!(doc.root().text_content(), "text");
}

// ============================================================================
// Round-trip Tests
// ============================================================================

#[test]
fn round_trip_simple() {
    use kdl_xml::XmlToKdlConverter;

    let original_xml = "<root><child>text</child></root>";

    // XML -> KDL
    let xml2kdl = XmlToKdlConverter::new();
    let mut kdl_doc = xml2kdl.convert(original_xml).unwrap();
    kdl_doc.autoformat();
    let kdl_str = kdl_doc.to_string();

    // KDL -> XML
    let kdl2xml = KdlToXmlConverter::new().pretty(false);
    let result_xml = kdl2xml.convert(&kdl_str).unwrap();

    // Verify structure preserved
    let doc = parse_xml(&result_xml);
    assert_eq!(doc.root().name, "root");
    let child = doc.root().find_element("child").unwrap();
    assert_eq!(child.text_content(), "text");
}

#[test]
fn round_trip_with_attributes() {
    use kdl_xml::XmlToKdlConverter;

    let original_xml = r#"<a href="http://example.com" target="_blank">link</a>"#;

    let xml2kdl = XmlToKdlConverter::new();
    let mut kdl_doc = xml2kdl.convert(original_xml).unwrap();
    kdl_doc.autoformat();
    let kdl_str = kdl_doc.to_string();

    let kdl2xml = KdlToXmlConverter::new().pretty(false);
    let result_xml = kdl2xml.convert(&kdl_str).unwrap();

    let doc = parse_xml(&result_xml);
    assert!(doc.root().has_attr("href", "http://example.com"));
    assert!(doc.root().has_attr("target", "_blank"));
    assert_eq!(doc.root().text_content(), "link");
}

#[test]
fn round_trip_mixed_content() {
    use kdl_xml::XmlToKdlConverter;

    let original_xml = "<span>some <b>bold</b> text</span>";

    let xml2kdl = XmlToKdlConverter::new();
    let mut kdl_doc = xml2kdl.convert(original_xml).unwrap();
    kdl_doc.autoformat();
    let kdl_str = kdl_doc.to_string();

    let kdl2xml = KdlToXmlConverter::new().pretty(false);
    let result_xml = kdl2xml.convert(&kdl_str).unwrap();

    let doc = parse_xml(&result_xml);
    assert_eq!(doc.root().name, "span");
    assert_eq!(doc.root().full_text_content(), "some bold text");
    assert!(doc.root().find_element("b").is_some());
}

// ============================================================================
// Fixture Tests
// ============================================================================

#[test]
fn fixture_website_kdl() {
    let kdl = r#"
        !doctype "html"
        html lang="en" {
            head {
                meta charset="utf-8"
                title "Test Page"
            }
            body {
                h1 "Hello World"
                p {
                    - "This is a "
                    a href="http://example.com" "link"
                    - " in a paragraph."
                }
            }
        }
    "#;

    let xml = convert(kdl);
    let doc = parse_xml(&xml);

    assert!(doc.has_doctype("html"));
    assert_eq!(doc.root().name, "html");
    assert!(doc.root().has_attr("lang", "en"));

    let head = doc.root().find_element("head").unwrap();
    let meta = head.find_element("meta").unwrap();
    assert!(meta.has_attr("charset", "utf-8"));

    let title = head.find_element("title").unwrap();
    assert_eq!(title.text_content(), "Test Page");

    let body = doc.root().find_element("body").unwrap();
    let h1 = body.find_element("h1").unwrap();
    assert_eq!(h1.text_content(), "Hello World");

    let p = body.find_element("p").unwrap();
    let a = p.find_element("a").unwrap();
    assert!(a.has_attr("href", "http://example.com"));
    assert_eq!(a.text_content(), "link");
    assert_eq!(p.full_text_content(), "This is a link in a paragraph.");
}
