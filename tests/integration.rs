use kdl_xml::{CompactMode, XmlToKdlConverter};

fn convert(xml: &str) -> String {
    let converter = XmlToKdlConverter::new();
    let mut doc = converter.convert(xml).expect("Failed to convert XML");
    doc.autoformat();
    doc.to_string()
}

fn convert_with_comments(xml: &str) -> String {
    let converter = XmlToKdlConverter::new().comments_as_nodes(true);
    let mut doc = converter.convert(xml).expect("Failed to convert XML");
    doc.autoformat();
    doc.to_string()
}

fn convert_compact(xml: &str) -> String {
    let converter = XmlToKdlConverter::new().compact(true);
    let doc = converter.convert(xml).expect("Failed to convert XML");
    doc.to_string()
}

fn convert_compact_leaf(xml: &str) -> String {
    let converter = XmlToKdlConverter::new().compact_mode(CompactMode::Leaf);
    let doc = converter.convert(xml).expect("Failed to convert XML");
    doc.to_string()
}

fn try_convert(xml: &str) -> Result<String, kdl_xml::Xml2KdlError> {
    let converter = XmlToKdlConverter::new();
    let mut doc = converter.convert(xml)?;
    doc.autoformat();
    Ok(doc.to_string())
}

// ============================================================================
// Element Tests
// ============================================================================

#[test]
fn simple_element() {
    let kdl = convert("<root/>");
    assert_eq!(kdl.trim(), "root");
}

#[test]
fn element_with_text() {
    let kdl = convert("<root></root>");
    assert_eq!(kdl.trim(), "root");
}

#[test]
fn element_with_single_attribute() {
    let kdl = convert(r#"<element foo="bar"/>"#);
    assert!(kdl.contains("foo="));
    assert!(kdl.contains("bar"));
}

#[test]
fn element_with_multiple_attributes() {
    let kdl = convert(r#"<element foo="bar" baz="qux"/>"#);
    assert!(kdl.contains("foo="));
    assert!(kdl.contains("baz="));
}

#[test]
fn nested_elements() {
    let kdl = convert("<parent><child/></parent>");
    assert!(kdl.contains("parent"));
    assert!(kdl.contains("child"));
}

#[test]
fn deeply_nested_elements() {
    let kdl = convert("<a><b><c><d><e>deep</e></d></c></b></a>");
    assert!(kdl.contains("deep"));
    let _doc: kdl::KdlDocument = kdl.parse().expect("Output should be valid KDL");
}

#[test]
fn multiple_children() {
    let kdl = convert("<parent><a/><b/><c/></parent>");
    let doc: kdl::KdlDocument = kdl.parse().unwrap();
    let parent = doc.get("parent").unwrap();
    let children = parent.children().unwrap();
    assert_eq!(children.nodes().len(), 3);
}

#[test]
fn sibling_elements_at_root() {
    // Multiple root elements should fail per XiK spec
    let result = try_convert("<a/><b/>");
    assert!(result.is_err(), "Multiple root elements should be rejected");
}

// ============================================================================
// Text Content Tests
// ============================================================================

#[test]
fn pure_text_content() {
    let kdl = convert("<greeting>hello world</greeting>");
    assert!(kdl.contains("\"hello world\""));
    assert!(!kdl.contains("- \""));
}

#[test]
fn text_with_attribute() {
    let kdl = convert(r#"<a href="http://example.com">link text</a>"#);
    assert!(kdl.contains("href="));
    assert!(kdl.contains("\"link text\""));
}

#[test]
fn mixed_content_text_before() {
    let kdl = convert("<span>before <b>bold</b></span>");
    assert!(kdl.contains("- \"before \""));
    assert!(kdl.contains("b bold") || kdl.contains("b \"bold\""));
}

#[test]
fn mixed_content_text_after() {
    let kdl = convert("<span><b>bold</b> after</span>");
    assert!(kdl.contains("- \" after\""));
}

#[test]
fn mixed_content_both_sides() {
    let kdl = convert("<span>some <b>bold</b> text</span>");
    assert!(kdl.contains("- \"some \""));
    assert!(kdl.contains("- \" text\""));
}

#[test]
fn empty_text_preserved() {
    let kdl = convert("<p></p>");
    assert_eq!(kdl.trim(), "p");
}

#[test]
fn whitespace_only_between_elements_trimmed() {
    let kdl = convert("<parent>\n  <child/>\n</parent>");
    assert!(!kdl.contains("- \"\\n"));
}

#[test]
fn significant_whitespace_in_text() {
    let kdl = convert("<pre>  indented  </pre>");
    assert!(kdl.contains("  indented  "));
}

// ============================================================================
// Entity Decoding Tests
// ============================================================================

#[test]
fn lt_entity() {
    let kdl = convert("<p>&lt;</p>");
    assert!(kdl.contains("<"));
}

#[test]
fn gt_entity() {
    let kdl = convert("<p>&gt;</p>");
    assert!(kdl.contains(">"));
}

#[test]
fn amp_entity() {
    let kdl = convert("<p>&amp;</p>");
    assert!(kdl.contains("&"));
}

#[test]
fn quot_entity() {
    let kdl = convert("<p>&quot;</p>");
    assert!(kdl.contains("\""));
}

#[test]
fn apos_entity() {
    let kdl = convert("<p>&apos;</p>");
    assert!(kdl.contains("'"));
}

#[test]
fn multiple_entities() {
    let kdl = convert("<p>&lt;hello&gt; &amp; world</p>");
    let doc: kdl::KdlDocument = kdl.parse().unwrap();
    let entry = &doc.get("p").unwrap().entries()[0];
    let value = entry.value().as_string().unwrap();
    assert_eq!(value, "<hello> & world");
}

#[test]
fn numeric_entity_decimal() {
    let kdl = convert("<p>&#65;</p>");
    assert!(kdl.contains("A"));
}

#[test]
fn numeric_entity_hex() {
    let kdl = convert("<p>&#x41;</p>");
    assert!(kdl.contains("A"));
}

// ============================================================================
// CDATA Tests
// ============================================================================

#[test]
fn cdata_as_text() {
    let kdl = convert("<script><![CDATA[var x = 1 < 2;]]></script>");
    let doc: kdl::KdlDocument = kdl.parse().unwrap();
    let entry = &doc.get("script").unwrap().entries()[0];
    let value = entry.value().as_string().unwrap();
    assert_eq!(value, "var x = 1 < 2;");
}

#[test]
fn cdata_preserves_special_chars() {
    let kdl = convert("<data><![CDATA[<>&\"']]></data>");
    let doc: kdl::KdlDocument = kdl.parse().unwrap();
    let entry = &doc.get("data").unwrap().entries()[0];
    let value = entry.value().as_string().unwrap();
    assert_eq!(value, "<>&\"'");
}

// ============================================================================
// Processing Instruction Tests
// ============================================================================

#[test]
fn xml_declaration() {
    let kdl = convert(r#"<?xml version="1.0"?><root/>"#);
    assert!(kdl.contains("?xml"));
    assert!(kdl.contains("version="));
}

#[test]
fn xml_declaration_with_encoding() {
    let kdl = convert(r#"<?xml version="1.0" encoding="UTF-8"?><root/>"#);
    assert!(kdl.contains("?xml"));
    assert!(kdl.contains("encoding="));
}

#[test]
fn stylesheet_pi() {
    let kdl = convert(r#"<?xml-stylesheet type="text/css" href="style.css"?><root/>"#);
    assert!(kdl.contains("?xml-stylesheet"));
    assert!(kdl.contains("type="));
    assert!(kdl.contains("href="));
}

// ============================================================================
// Comment Tests
// ============================================================================

#[test]
fn comment_discarded_by_default() {
    let kdl = convert("<root><!-- comment --></root>");
    assert!(!kdl.contains("comment"));
    assert!(!kdl.contains("!"));
}

#[test]
fn comment_as_node_when_enabled() {
    let kdl = convert_with_comments("<root><!-- comment --></root>");
    assert!(kdl.contains("!") || kdl.contains("comment"));
}

#[test]
fn top_level_comment() {
    let kdl = convert_with_comments("<!-- top level --><root/>");
    assert!(kdl.contains("root"));
}

#[test]
fn comment_in_pure_text_element_discarded() {
    // When comments are discarded (default), pure text content should stay pure text
    let kdl = convert("<span>text<!-- comment --></span>");
    let doc: kdl::KdlDocument = kdl.parse().unwrap();
    let span = doc.get("span").unwrap();
    // Should be pure text (single argument), not mixed content with `-` nodes
    let entries: Vec<_> = span
        .entries()
        .iter()
        .filter(|e| e.name().is_none())
        .collect();
    assert_eq!(entries.len(), 1, "Should be pure text, not mixed content");
    assert_eq!(entries[0].value().as_string().unwrap(), "text");
}

#[test]
fn comment_in_text_with_comments_enabled() {
    // When comments are enabled, text + comment should be mixed content
    let kdl = convert_with_comments("<span>text<!-- comment --></span>");
    let doc: kdl::KdlDocument = kdl.parse().unwrap();
    let span = doc.get("span").unwrap();
    // Should have children (mixed content)
    assert!(
        span.children().is_some(),
        "Should be mixed content when comments enabled"
    );
}

// ============================================================================
// Namespace Tests
// ============================================================================

#[test]
fn default_namespace() {
    let kdl = convert(r#"<root xmlns="http://example.com"/>"#);
    assert!(kdl.contains("xmlns="));
}

#[test]
fn prefixed_namespace() {
    let kdl = convert(r#"<root xmlns:ns="http://example.com"/>"#);
    assert!(kdl.contains("xmlns:ns="));
}

#[test]
fn prefixed_element() {
    let kdl = convert(r#"<ns:root xmlns:ns="http://example.com"/>"#);
    assert!(kdl.contains("ns:root"));
}

#[test]
fn prefixed_attribute() {
    let kdl =
        convert(r#"<root xmlns:xlink="http://www.w3.org/1999/xlink"><a xlink:href="url"/></root>"#);
    assert!(kdl.contains("xlink:href="));
}

#[test]
fn xml_space_preserved() {
    let kdl = convert(r#"<pre xml:space="preserve">text</pre>"#);
    assert!(kdl.contains("xml:space="));
}

#[test]
fn xml_space_preserve_keeps_whitespace() {
    // Whitespace between elements should be preserved with xml:space="preserve"
    let kdl = convert(r#"<pre xml:space="preserve"><a/>   <b/></pre>"#);
    let doc: kdl::KdlDocument = kdl.parse().unwrap();
    let pre = doc.get("pre").unwrap();
    let children = pre.children().unwrap();
    // Should have a, text node with spaces, and b
    assert!(
        children.nodes().len() >= 3,
        "Should preserve whitespace text nodes"
    );
}

#[test]
fn xml_space_preserve_inheritance() {
    // xml:space should be inherited by child elements
    let kdl = convert(r#"<div xml:space="preserve"><span>  text  </span></div>"#);
    assert!(kdl.contains("  text  "));
}

#[test]
fn xml_space_default_normalizes() {
    // xml:space="default" should normalize whitespace
    let kdl = convert(r#"<div xml:space="default"><a/>   <b/></div>"#);
    let doc: kdl::KdlDocument = kdl.parse().unwrap();
    let div = doc.get("div").unwrap();
    let children = div.children().unwrap();
    // Whitespace-only text nodes should be filtered out
    assert_eq!(children.nodes().len(), 2, "Should normalize whitespace");
}

// ============================================================================
// Processing Instruction Parsing Edge Cases
// ============================================================================

#[test]
fn pi_with_escaped_quote_in_value() {
    // PI with attribute value containing escaped quote
    let kdl = convert(r#"<?custom attr="value with &quot; quote"?><root/>"#);
    assert!(kdl.contains("?custom"));
    // The quote should be decoded
    let doc: kdl::KdlDocument = kdl.parse().expect("Should be valid KDL");
    assert!(!doc.nodes().is_empty());
}

#[test]
fn pi_with_equals_in_value() {
    // PI with equals sign in attribute value (should not confuse parser)
    let kdl = convert(r#"<?custom expr="a=b"?><root/>"#);
    assert!(kdl.contains("?custom"));
    let doc: kdl::KdlDocument = kdl.parse().expect("Should be valid KDL");
    let pi = doc.get("?custom").unwrap();
    let value = pi.get("expr").and_then(|v| v.as_string()).unwrap();
    assert_eq!(value, "a=b");
}

#[test]
fn pi_with_single_quotes() {
    // PI using single quotes for attribute values
    let kdl = convert(r#"<?custom attr='value'?><root/>"#);
    assert!(kdl.contains("?custom"));
    let doc: kdl::KdlDocument = kdl.parse().expect("Should be valid KDL");
    let pi = doc.get("?custom").unwrap();
    let value = pi.get("attr").and_then(|v| v.as_string()).unwrap();
    assert_eq!(value, "value");
}

#[test]
fn pi_with_mixed_quotes() {
    // PI using mix of single and double quotes
    let kdl = convert(r#"<?custom attr1="double" attr2='single'?><root/>"#);
    assert!(kdl.contains("?custom"));
    let doc: kdl::KdlDocument = kdl.parse().expect("Should be valid KDL");
    let pi = doc.get("?custom").unwrap();
    assert_eq!(
        pi.get("attr1").and_then(|v| v.as_string()).unwrap(),
        "double"
    );
    assert_eq!(
        pi.get("attr2").and_then(|v| v.as_string()).unwrap(),
        "single"
    );
}

#[test]
fn pi_unstructured_content() {
    // PI with unstructured content (no attribute syntax)
    let kdl = convert(r#"<?php echo "hello"; ?><root/>"#);
    assert!(kdl.contains("?php"));
    // Should be stored as a single string argument
    let doc: kdl::KdlDocument = kdl.parse().expect("Should be valid KDL");
    let pi = doc.get("?php").unwrap();
    // Should have an unnamed argument, not properties
    assert!(pi.entries().iter().any(|e| e.name().is_none()));
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn empty_attribute_value() {
    let kdl = convert(r#"<input value=""/>"#);
    assert!(kdl.contains("value=\"\""));
}

#[test]
fn unicode_content() {
    let kdl = convert("<greeting>Hello, 世界!</greeting>");
    assert!(kdl.contains("世界"));
}

#[test]
fn unicode_emoji() {
    let kdl = convert("<emoji>🎉🚀</emoji>");
    assert!(kdl.contains("🎉"));
    assert!(kdl.contains("🚀"));
}

#[test]
fn attribute_with_single_quotes_in_source() {
    let kdl = convert(r#"<div title='hello'/>"#);
    assert!(kdl.contains("title="));
}

#[test]
fn very_long_text() {
    let long_text = "x".repeat(10000);
    let xml = format!("<content>{}</content>", long_text);
    let kdl = convert(&xml);
    assert!(kdl.len() > 10000);
}

#[test]
fn deeply_nested_50_levels() {
    let open_tags: String = (0..50).map(|i| format!("<n{}>", i)).collect();
    let close_tags: String = (0..50).rev().map(|i| format!("</n{}>", i)).collect();
    let xml = format!("{}text{}", open_tags, close_tags);
    let kdl = convert(&xml);
    assert!(kdl.contains("n49"));
    let _doc: kdl::KdlDocument = kdl.parse().expect("Should be valid KDL");
}

#[test]
fn element_name_with_hyphen() {
    let kdl = convert("<my-element/>");
    assert!(kdl.contains("my-element"));
}

#[test]
fn element_name_with_underscore() {
    let kdl = convert("<my_element/>");
    assert!(kdl.contains("my_element"));
}

#[test]
fn element_name_with_numbers() {
    let kdl = convert("<element123/>");
    assert!(kdl.contains("element123"));
}

#[test]
fn attribute_value_with_newline() {
    let kdl = convert("<div title=\"line1\nline2\"/>");
    let _doc: kdl::KdlDocument = kdl.parse().expect("Should be valid KDL");
}

// ============================================================================
// Error Cases
// ============================================================================

#[test]
fn malformed_unclosed_tag() {
    let result = try_convert("<root><child></root>");
    assert!(result.is_err());
}

#[test]
fn malformed_mismatched_tags() {
    let result = try_convert("<a></b>");
    assert!(result.is_err());
}

#[test]
fn empty_input() {
    let result = try_convert("");
    if let Ok(kdl) = result {
        assert!(kdl.trim().is_empty());
    }
}

#[test]
fn invalid_xml_characters() {
    // Just verify it doesn't panic - either success or error is acceptable
    let _ = try_convert("<root>\x00</root>");
}

// ============================================================================
// Round-trip Validation (KDL output is valid)
// ============================================================================

#[test]
fn output_is_valid_kdl() {
    let xml = r#"<root attr="value"><child>text</child></root>"#;
    let kdl_str = convert(xml);
    let _doc: kdl::KdlDocument = kdl_str.parse().expect("Output should be valid KDL");
}

#[test]
fn structure_preserved() {
    let xml = "<parent><a/><b/><c/></parent>";
    let kdl_str = convert(xml);
    let doc: kdl::KdlDocument = kdl_str.parse().unwrap();

    let parent = doc.get("parent").unwrap();
    let children = parent.children().unwrap();
    assert_eq!(children.nodes().len(), 3);
}

#[test]
fn attributes_preserved() {
    let xml = r#"<element foo="1" bar="2" baz="3"/>"#;
    let kdl_str = convert(xml);
    let doc: kdl::KdlDocument = kdl_str.parse().unwrap();

    let element = doc.get("element").unwrap();
    assert!(element.get("foo").is_some());
    assert!(element.get("bar").is_some());
    assert!(element.get("baz").is_some());
}

#[test]
fn text_content_preserved() {
    let xml = "<greeting>Hello, World!</greeting>";
    let kdl_str = convert(xml);
    let doc: kdl::KdlDocument = kdl_str.parse().unwrap();

    let greeting = doc.get("greeting").unwrap();
    let text = greeting.entries()[0].value().as_string().unwrap();
    assert_eq!(text, "Hello, World!");
}

// ============================================================================
// Real Document Tests (Fixtures)
// ============================================================================

#[test]
fn fixture_html_document() {
    let xml = include_str!("fixtures/sample.html");
    let kdl = convert(xml);
    // Verify structure
    assert!(kdl.contains("html"));
    assert!(kdl.contains("head"));
    assert!(kdl.contains("body"));
    assert!(kdl.contains("title"));
    // Verify it's valid KDL
    let _doc: kdl::KdlDocument = kdl.parse().expect("Should be valid KDL");
}

#[test]
fn fixture_svg_document() {
    let xml = include_str!("fixtures/sample.svg");
    let kdl = convert(xml);
    // Verify SVG elements
    assert!(kdl.contains("svg"));
    assert!(kdl.contains("rect"));
    assert!(kdl.contains("circle"));
    // Verify namespace handling
    assert!(kdl.contains("xlink:href="));
    // Verify it's valid KDL
    let _doc: kdl::KdlDocument = kdl.parse().expect("Should be valid KDL");
}

#[test]
fn fixture_atom_feed() {
    let xml = include_str!("fixtures/atom.xml");
    let kdl = convert(xml);
    // Verify Atom structure
    assert!(kdl.contains("feed"));
    assert!(kdl.contains("entry"));
    assert!(kdl.contains("title"));
    assert!(kdl.contains("author"));
    // Verify it's valid KDL
    let _doc: kdl::KdlDocument = kdl.parse().expect("Should be valid KDL");
}

// ============================================================================
// Compact Output Tests
// ============================================================================

#[test]
fn compact_mixed_content() {
    let kdl = convert_compact("<p>This is a <strong>sample</strong> paragraph.</p>");
    // Should be on a single line with semicolon separators
    assert!(
        kdl.contains("{ "),
        "Compact output should have opening brace with space"
    );
    assert!(
        kdl.contains("; "),
        "Compact output should use semicolons between children"
    );
    // Should contain the expected content
    assert!(kdl.contains("- \"This is a \""));
    assert!(kdl.contains("strong"));
    assert!(kdl.contains("- \" paragraph.\""));
    // Verify it's valid KDL
    let _doc: kdl::KdlDocument = kdl.parse().expect("Compact output should be valid KDL");
}

#[test]
fn compact_nested_elements() {
    let kdl = convert_compact("<div><span><b>bold</b></span></div>");
    // Each level should be on a single line
    assert!(kdl.contains("div { "));
    assert!(kdl.contains("span { "));
    // Verify it's valid KDL
    let _doc: kdl::KdlDocument = kdl.parse().expect("Compact output should be valid KDL");
}

#[test]
fn compact_multiple_children() {
    let kdl = convert_compact("<ul><li>a</li><li>b</li><li>c</li></ul>");
    // All children on single line with semicolons
    assert!(
        kdl.contains("; "),
        "Children should be separated by semicolons"
    );
    // Count semicolons - should have 2 (between 3 children)
    let semicolon_count = kdl.matches("; ").count();
    assert!(
        semicolon_count >= 2,
        "Should have at least 2 semicolons for 3 children"
    );
    // Verify it's valid KDL
    let _doc: kdl::KdlDocument = kdl.parse().expect("Compact output should be valid KDL");
}

#[test]
fn compact_pure_text_no_children() {
    // Pure text content should still be compact (single argument, no children block)
    let kdl = convert_compact("<greeting>hello</greeting>");
    // "hello" may be bare or quoted depending on KDL serialization
    assert!(kdl.contains("greeting") && kdl.contains("hello"));
    assert!(
        !kdl.contains("{"),
        "Pure text should not have children block"
    );
}

#[test]
fn compact_empty_element() {
    let kdl = convert_compact("<empty/>");
    assert_eq!(kdl.trim(), "empty");
    assert!(
        !kdl.contains("{"),
        "Empty element should not have children block"
    );
}

// ============================================================================
// Leaf Compact Output Tests
// ============================================================================

#[test]
fn compact_leaf_mixed_content() {
    // Leaf mixed content should be compact
    let kdl = convert_compact_leaf("<p>Hello <b>world</b>!</p>");
    // Should be on a single line with semicolon separators
    assert!(kdl.contains("{ "), "Leaf should have compact children");
    assert!(
        kdl.contains("; "),
        "Leaf should use semicolons between children"
    );
    // Verify it's valid KDL
    let _doc: kdl::KdlDocument = kdl
        .parse()
        .expect("Leaf compact output should be valid KDL");
}

#[test]
fn compact_leaf_nested_not_compacted() {
    // Non-leaf nodes should not be compacted
    let kdl = convert_compact_leaf("<div><p>Hello <b>world</b></p></div>");
    // div should have multi-line children (contains newline after {)
    assert!(
        kdl.contains("div {"),
        "Parent should have space before brace"
    );
    // p should be compact (leaf)
    assert!(kdl.contains("p { - "), "Leaf child should be compact");
    // Verify proper structure
    let _doc: kdl::KdlDocument = kdl
        .parse()
        .expect("Leaf compact output should be valid KDL");
}

#[test]
fn compact_leaf_deeply_nested() {
    // Only the deepest leaf level should be compact
    let kdl =
        convert_compact_leaf("<article><section><p>Text <em>here</em></p></section></article>");
    // Verify nesting structure - article and section should be multi-line
    assert!(kdl.contains("article {"));
    assert!(kdl.contains("section {"));
    // p should be compact (it's the leaf)
    assert!(kdl.contains("p { - "));
    // Verify it's valid KDL
    let _doc: kdl::KdlDocument = kdl
        .parse()
        .expect("Leaf compact output should be valid KDL");
}

#[test]
fn compact_leaf_list_items() {
    // List items that are leaves should be compact
    let kdl = convert_compact_leaf("<ul><li>a</li><li>b</li></ul>");
    // ul has leaf children (li with text only), so ul should be compact
    assert!(
        kdl.contains("ul { "),
        "ul with leaf children should be compact"
    );
    assert!(
        kdl.contains("; "),
        "Leaf children should be separated by semicolons"
    );
    // Verify it's valid KDL
    let _doc: kdl::KdlDocument = kdl
        .parse()
        .expect("Leaf compact output should be valid KDL");
}

#[test]
fn compact_leaf_non_leaf_stays_multiline() {
    // Non-leaf content should stay multi-line
    let kdl = convert_compact_leaf("<nav><ul><li><a>link</a></li></ul></nav>");
    // nav and ul are not leaves because li has a child (a)
    // li is a leaf because a only has text
    assert!(
        kdl.contains("li { a link }"),
        "li with leaf child should be compact"
    );
    // Verify it's valid KDL
    let _doc: kdl::KdlDocument = kdl
        .parse()
        .expect("Leaf compact output should be valid KDL");
}
