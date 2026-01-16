use kdl_xml::KdlToXmlConverter;
use quick_xml::Reader;
use quick_xml::events::Event;

fn validate_xml(xml: &str) {
    let mut reader = Reader::from_str(xml);
    loop {
        match reader.read_event() {
            Ok(Event::Eof) => break,
            Err(e) => panic!("Produced XML is not valid: {e:?}"),
            _ => {}
        }
    }
}

fn convert(kdl: &str) -> String {
    let converter = KdlToXmlConverter::new().pretty(false);
    let xml = converter.convert(kdl).expect("Conversion failed");
    // Validate output is valid XML
    validate_xml(&xml);
    xml
}

fn convert_pretty(kdl: &str) -> String {
    let converter = KdlToXmlConverter::new().pretty(true);
    let xml = converter.convert(kdl).expect("Conversion failed");
    // Validate output is valid XML
    validate_xml(&xml);
    xml
}

fn try_convert(kdl: &str) -> Result<String, kdl_xml::Kdl2XmlError> {
    let converter = KdlToXmlConverter::new().pretty(false);
    converter.convert(kdl)
}

// ============================================================================
// Element Tests
// ============================================================================

#[test]
fn simple_element() {
    let xml = convert("root");
    // Validation happens in convert() helper
    insta::assert_snapshot!(xml, @"<root/>");
}

#[test]
fn element_with_single_attribute() {
    let xml = convert(r#"element foo="bar""#);
    insta::assert_snapshot!(xml, @r#"<element foo="bar"/>"#);
}

#[test]
fn element_with_multiple_attributes() {
    let xml = convert(r#"element foo="bar" baz="qux""#);
    insta::assert_snapshot!(xml, @r#"<element foo="bar" baz="qux"/>"#);
}

#[test]
fn nested_elements() {
    let xml = convert("parent { child }");
    insta::assert_snapshot!(xml, @"<parent><child/></parent>");
}

#[test]
fn deeply_nested_elements() {
    let xml = convert("a { b { c { d { e } } } }");
    insta::assert_snapshot!(xml, @"<a><b><c><d><e/></d></c></b></a>");
}

#[test]
fn multiple_children() {
    let xml = convert("parent { a; b; c }");
    insta::assert_snapshot!(xml, @"<parent><a/><b/><c/></parent>");
}

// ============================================================================
// Text Content Tests
// ============================================================================

#[test]
fn pure_text_content() {
    let xml = convert(r#"greeting "hello world""#);
    insta::assert_snapshot!(xml, @"<greeting>hello world</greeting>");
}

#[test]
fn text_with_attribute() {
    let xml = convert(r#"a href="http://example.com" "link text""#);
    insta::assert_snapshot!(xml, @r#"<a href="http://example.com">link text</a>"#);
}

#[test]
fn mixed_content() {
    let xml = convert(r#"span { - "some "; b "bold"; - " text" }"#);
    insta::assert_snapshot!(xml, @"<span>some <b>bold</b> text</span>");
}

#[test]
fn mixed_content_text_only() {
    let xml = convert(r#"p { - "hello "; - "world" }"#);
    insta::assert_snapshot!(xml, @"<p>hello world</p>");
}

// ============================================================================
// Entity Encoding Tests
// ============================================================================

#[test]
fn entity_encoding_lt() {
    let xml = convert(r#"p "<""#);
    insta::assert_snapshot!(xml, @"<p>&lt;</p>");
}

#[test]
fn entity_encoding_gt() {
    let xml = convert(r#"p ">""#);
    insta::assert_snapshot!(xml, @"<p>&gt;</p>");
}

#[test]
fn entity_encoding_amp() {
    let xml = convert(r#"p "&""#);
    insta::assert_snapshot!(xml, @"<p>&amp;</p>");
}

#[test]
fn entity_encoding_multiple() {
    let xml = convert(r#"p "<hello> & world""#);
    insta::assert_snapshot!(xml, @"<p>&lt;hello&gt; &amp; world</p>");
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
    insta::assert_snapshot!(xml, @"<!DOCTYPE html><html/>");
}

#[test]
fn doctype_xhtml() {
    let xml = convert(
        r#"!doctype "html PUBLIC \"-//W3C//DTD XHTML 1.0 Strict//EN\""
html"#,
    );
    insta::assert_snapshot!(xml, @r#"
    <!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Strict//EN"><html/>
    "#);
}

#[test]
fn doctype_with_internal_subset() {
    let xml = convert(
        r#"!doctype "html [<!ENTITY test \"value\">]"
html"#,
    );
    insta::assert_snapshot!(xml, @r#"<!DOCTYPE html [<!ENTITY test "value">]><html/>"#);
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
    insta::assert_snapshot!(xml, @r#"<?xml version="1.0"?><root/>"#);
}

#[test]
fn xml_declaration_with_encoding() {
    let xml = convert(
        r#"?xml version="1.0" encoding="UTF-8"
root"#,
    );
    insta::assert_snapshot!(xml, @r#"<?xml version="1.0" encoding="UTF-8"?><root/>"#);
}

#[test]
fn processing_instruction() {
    let xml = convert(
        r#"?xml-stylesheet type="text/css" href="style.css"
root"#,
    );
    insta::assert_snapshot!(xml, @r#"<?xml-stylesheet type="text/css" href="style.css"?><root/>"#);
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
    insta::assert_snapshot!(xml, @"<!-- This is a comment --><root/>");
}

// ============================================================================
// Namespace Tests
// ============================================================================

#[test]
fn default_namespace() {
    let xml = convert(r#"root xmlns="http://example.com""#);
    insta::assert_snapshot!(xml, @r#"<root xmlns="http://example.com"/>"#);
}

#[test]
fn prefixed_namespace() {
    let xml = convert(r#"root xmlns:ns="http://example.com""#);
    insta::assert_snapshot!(xml, @r#"<root xmlns:ns="http://example.com"/>"#);
}

#[test]
fn prefixed_element() {
    let xml = convert(r#"ns:root xmlns:ns="http://example.com""#);
    insta::assert_snapshot!(xml, @r#"<ns:root xmlns:ns="http://example.com"/>"#);
}

#[test]
fn prefixed_attribute() {
    let xml = convert(r#"a xlink:href="url""#);
    insta::assert_snapshot!(xml, @r#"<a xlink:href="url"/>"#);
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn empty_attribute_value() {
    let xml = convert(r#"input value="""#);
    insta::assert_snapshot!(xml, @r#"<input value=""/>"#);
}

#[test]
fn unicode_content() {
    let xml = convert(r#"greeting "Hello, 世界!""#);
    insta::assert_snapshot!(xml, @"<greeting>Hello, 世界!</greeting>");
}

#[test]
fn unicode_emoji() {
    let xml = convert(r#"emoji "🎉🚀""#);
    insta::assert_snapshot!(xml, @"<emoji>🎉🚀</emoji>");
}

#[test]
fn very_long_text() {
    let long_text = "x".repeat(10000);
    let kdl = format!(r#"content "{}""#, long_text);
    let xml = convert(&kdl);
    // Just verify length - too long for inline snapshot
    assert!(xml.len() > 10000);
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
    // Just verify structure - too deep for inline snapshot
    assert!(xml.contains("<n0>") && xml.contains("<leaf/>"));
}

#[test]
fn element_name_with_hyphen() {
    let xml = convert("my-element");
    insta::assert_snapshot!(xml, @"<my-element/>");
}

#[test]
fn element_name_with_underscore() {
    let xml = convert("my_element");
    insta::assert_snapshot!(xml, @"<my_element/>");
}

#[test]
fn element_name_with_numbers() {
    let xml = convert("element123");
    insta::assert_snapshot!(xml, @"<element123/>");
}

// ============================================================================
// Pretty Printing Tests
// ============================================================================

#[test]
fn pretty_nested_elements() {
    let xml = convert_pretty("parent { child { grandchild } }");
    insta::assert_snapshot!(xml, @r"
<parent>
  <child>
    <grandchild/>
  </child>
</parent>
");
}

#[test]
fn pretty_multiple_children() {
    let xml = convert_pretty("parent { a; b; c }");
    insta::assert_snapshot!(xml, @r"
<parent>
  <a/>
  <b/>
  <c/>
</parent>
");
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
    // Double hyphen should be escaped
    insta::assert_snapshot!(xml, @"<!--test- -comment--><root/>");
}

#[test]
fn comment_trailing_hyphen_escaped() {
    let xml = convert(
        r#"! "comment-"
root"#,
    );
    // Trailing hyphen should have space added
    insta::assert_snapshot!(xml, @"<!--comment- --><root/>");
}

#[test]
fn pi_content_escaped() {
    let xml = convert(
        r#"?custom "content?>"
root"#,
    );
    // ?> should be escaped
    insta::assert_snapshot!(xml, @"<?custom content? >?><root/>");
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
    insta::assert_snapshot!(xml, @"<element>text</element>");
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
    validate_xml(&result_xml);

    insta::assert_snapshot!(result_xml, @"<root><child>text</child></root>");
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
    validate_xml(&result_xml);

    insta::assert_snapshot!(
        result_xml,
        @r#"<a href="http://example.com" target="_blank">link</a>"#
    );
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
    validate_xml(&result_xml);

    insta::assert_snapshot!(result_xml, @"<span>some <b>bold</b> text</span>");
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
    insta::assert_snapshot!(xml);
}
