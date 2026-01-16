use kdl_xml::{CompactMode, XmlToKdlConverter};

fn convert(xml: &str) -> String {
    let converter = XmlToKdlConverter::new();
    let mut doc = converter.convert(xml).expect("Failed to convert XML");
    doc.autoformat();
    let kdl_str = doc.to_string();
    // Validate output is valid KDL
    let _: kdl::KdlDocument = kdl_str.parse().expect("Produced KDL is not valid");
    kdl_str
}

fn convert_with_comments(xml: &str) -> String {
    let converter = XmlToKdlConverter::new().comments_as_nodes(true);
    let mut doc = converter.convert(xml).expect("Failed to convert XML");
    doc.autoformat();
    let kdl_str = doc.to_string();
    // Validate output is valid KDL
    let _: kdl::KdlDocument = kdl_str.parse().expect("Produced KDL is not valid");
    kdl_str
}

fn convert_compact(xml: &str) -> String {
    let converter = XmlToKdlConverter::new().compact(true);
    let doc = converter.convert(xml).expect("Failed to convert XML");
    let kdl_str = doc.to_string();
    // Validate output is valid KDL
    let _: kdl::KdlDocument = kdl_str.parse().expect("Produced KDL is not valid");
    kdl_str
}

fn convert_compact_leaf(xml: &str) -> String {
    let converter = XmlToKdlConverter::new().compact_mode(CompactMode::Leaf);
    let doc = converter.convert(xml).expect("Failed to convert XML");
    let kdl_str = doc.to_string();
    // Validate output is valid KDL
    let _: kdl::KdlDocument = kdl_str.parse().expect("Produced KDL is not valid");
    kdl_str
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
    insta::assert_snapshot!(kdl.trim(), @"root");
}

#[test]
fn element_with_text() {
    let kdl = convert("<root></root>");
    insta::assert_snapshot!(kdl.trim(), @"root");
}

#[test]
fn element_with_single_attribute() {
    let kdl = convert(r#"<element foo="bar"/>"#);
    insta::assert_snapshot!(kdl.trim(), @"element foo=bar");
}

#[test]
fn element_with_multiple_attributes() {
    let kdl = convert(r#"<element foo="bar" baz="qux"/>"#);
    insta::assert_snapshot!(kdl.trim(), @"element foo=bar baz=qux");
}

#[test]
fn nested_elements() {
    let kdl = convert("<parent><child/></parent>");
    insta::assert_snapshot!(kdl.trim(), @r"
parent {
    child
}
");
}

#[test]
fn deeply_nested_elements() {
    let kdl = convert("<a><b><c><d><e>deep</e></d></c></b></a>");
    insta::assert_snapshot!(kdl.trim(), @r"
    a {
        b {
            c {
                d {
                    e deep
                }
            }
        }
    }
    ");
}

#[test]
fn multiple_children() {
    let kdl = convert("<parent><a/><b/><c/></parent>");
    insta::assert_snapshot!(kdl.trim(), @r"
parent {
    a
    b
    c
}
");
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
    insta::assert_snapshot!(kdl.trim(), @r#"greeting "hello world""#);
}

#[test]
fn text_with_attribute() {
    let kdl = convert(r#"<a href="http://example.com">link text</a>"#);
    insta::assert_snapshot!(kdl.trim(), @r#"a href="http://example.com" "link text""#);
}

#[test]
fn mixed_content_text_before() {
    let kdl = convert("<span>before <b>bold</b></span>");
    insta::assert_snapshot!(kdl.trim(), @r#"
    span {
        - "before "
        b bold
    }
    "#);
}

#[test]
fn mixed_content_text_after() {
    let kdl = convert("<span><b>bold</b> after</span>");
    insta::assert_snapshot!(kdl.trim(), @r#"
    span {
        b bold
        - " after"
    }
    "#);
}

#[test]
fn mixed_content_both_sides() {
    let kdl = convert("<span>some <b>bold</b> text</span>");
    insta::assert_snapshot!(kdl.trim(), @r#"
    span {
        - "some "
        b bold
        - " text"
    }
    "#);
}

#[test]
fn empty_text_preserved() {
    let kdl = convert("<p></p>");
    insta::assert_snapshot!(kdl.trim(), @"p");
}

#[test]
fn whitespace_only_between_elements_trimmed() {
    let kdl = convert("<parent>\n  <child/>\n</parent>");
    insta::assert_snapshot!(kdl.trim(), @r"
parent {
    child
}
");
}

#[test]
fn significant_whitespace_in_text() {
    let kdl = convert("<pre>  indented  </pre>");
    insta::assert_snapshot!(kdl.trim(), @r#"pre "  indented  ""#);
}

// ============================================================================
// Entity Decoding Tests
// ============================================================================

#[test]
fn lt_entity() {
    let kdl = convert("<p>&lt;</p>");
    insta::assert_snapshot!(kdl.trim(), @"p <");
}

#[test]
fn gt_entity() {
    let kdl = convert("<p>&gt;</p>");
    insta::assert_snapshot!(kdl.trim(), @"p >");
}

#[test]
fn amp_entity() {
    let kdl = convert("<p>&amp;</p>");
    insta::assert_snapshot!(kdl.trim(), @"p &");
}

#[test]
fn quot_entity() {
    let kdl = convert("<p>&quot;</p>");
    insta::assert_snapshot!(kdl.trim(), @r#"p "\"""#);
}

#[test]
fn apos_entity() {
    let kdl = convert("<p>&apos;</p>");
    insta::assert_snapshot!(kdl.trim(), @"p '");
}

#[test]
fn multiple_entities() {
    let kdl = convert("<p>&lt;hello&gt; &amp; world</p>");
    insta::assert_snapshot!(kdl.trim(), @r#"p "<hello> & world""#);
}

#[test]
fn numeric_entity_decimal() {
    let kdl = convert("<p>&#65;</p>");
    insta::assert_snapshot!(kdl.trim(), @"p A");
}

#[test]
fn numeric_entity_hex() {
    let kdl = convert("<p>&#x41;</p>");
    insta::assert_snapshot!(kdl.trim(), @"p A");
}

// ============================================================================
// CDATA Tests
// ============================================================================

#[test]
fn cdata_as_text() {
    let kdl = convert("<script><![CDATA[var x = 1 < 2;]]></script>");
    insta::assert_snapshot!(kdl.trim(), @r#"script "var x = 1 < 2;""#);
}

#[test]
fn cdata_preserves_special_chars() {
    let kdl = convert("<data><![CDATA[<>&\"']]></data>");
    insta::assert_snapshot!(kdl.trim(), @r#"data "<>&\"'""#);
}

// ============================================================================
// Processing Instruction Tests
// ============================================================================

#[test]
fn xml_declaration() {
    let kdl = convert(r#"<?xml version="1.0"?><root/>"#);
    insta::assert_snapshot!(kdl.trim(), @r#"
?xml version="1.0"
root
"#);
}

#[test]
fn xml_declaration_with_encoding() {
    let kdl = convert(r#"<?xml version="1.0" encoding="UTF-8"?><root/>"#);
    insta::assert_snapshot!(kdl.trim(), @r#"
    ?xml version="1.0" encoding=UTF-8
    root
    "#);
}

#[test]
fn stylesheet_pi() {
    let kdl = convert(r#"<?xml-stylesheet type="text/css" href="style.css"?><root/>"#);
    insta::assert_snapshot!(kdl.trim(), @r#"
    ?xml-stylesheet type="text/css" href=style.css
    root
    "#);
}

// ============================================================================
// Comment Tests
// ============================================================================

#[test]
fn comment_discarded_by_default() {
    let kdl = convert("<root><!-- comment --></root>");
    insta::assert_snapshot!(kdl.trim(), @"root");
}

#[test]
fn comment_as_node_when_enabled() {
    let kdl = convert_with_comments("<root><!-- comment --></root>");
    insta::assert_snapshot!(kdl.trim(), @r#"
root {
    ! " comment "
}
"#);
}

#[test]
fn top_level_comment() {
    let kdl = convert_with_comments("<!-- top level --><root/>");
    insta::assert_snapshot!(kdl.trim(), @r#"
! " top level "
root
"#);
}

#[test]
fn comment_in_pure_text_element_discarded() {
    // When comments are discarded (default), pure text content should stay pure text
    let kdl = convert("<span>text<!-- comment --></span>");
    insta::assert_snapshot!(kdl.trim(), @"span text");
}

#[test]
fn comment_in_text_with_comments_enabled() {
    // When comments are enabled, text + comment should be mixed content
    let kdl = convert_with_comments("<span>text<!-- comment --></span>");
    insta::assert_snapshot!(kdl.trim(), @r#"
    span {
        - text
        ! " comment "
    }
    "#);
}

// ============================================================================
// Namespace Tests
// ============================================================================

#[test]
fn default_namespace() {
    let kdl = convert(r#"<root xmlns="http://example.com"/>"#);
    insta::assert_snapshot!(kdl.trim(), @r#"root xmlns="http://example.com""#);
}

#[test]
fn prefixed_namespace() {
    let kdl = convert(r#"<root xmlns:ns="http://example.com"/>"#);
    insta::assert_snapshot!(kdl.trim(), @r#"root xmlns:ns="http://example.com""#);
}

#[test]
fn prefixed_element() {
    let kdl = convert(r#"<ns:root xmlns:ns="http://example.com"/>"#);
    insta::assert_snapshot!(kdl.trim(), @r#"ns:root xmlns:ns="http://example.com""#);
}

#[test]
fn prefixed_attribute() {
    let xml = r#"<root xmlns:xlink="http://www.w3.org/1999/xlink">
                   <a xlink:href="url"/></root>"#;
    let kdl = convert(xml);
    insta::assert_snapshot!(kdl.trim(), @r#"
    root xmlns:xlink="http://www.w3.org/1999/xlink" {
        a xlink:href=url
    }
    "#);
}

#[test]
fn xml_space_preserved() {
    let kdl = convert(r#"<pre xml:space="preserve">text</pre>"#);
    insta::assert_snapshot!(kdl.trim(), @"pre xml:space=preserve text");
}

#[test]
fn xml_space_preserve_keeps_whitespace() {
    // Whitespace between elements should be preserved with xml:space="preserve"
    let kdl = convert(r#"<pre xml:space="preserve"><a/>   <b/></pre>"#);
    insta::assert_snapshot!(kdl.trim(), @r#"
    pre xml:space=preserve {
        a
        - "   "
        b
    }
    "#);
}

#[test]
fn xml_space_preserve_inheritance() {
    // xml:space should be inherited by child elements
    let kdl = convert(r#"<div xml:space="preserve"><span>  text  </span></div>"#);
    insta::assert_snapshot!(kdl.trim(), @r#"
    div xml:space=preserve {
        span "  text  "
    }
    "#);
}

#[test]
fn xml_space_default_normalizes() {
    // xml:space="default" should normalize whitespace
    let kdl = convert(r#"<div xml:space="default"><a/>   <b/></div>"#);
    insta::assert_snapshot!(kdl.trim(), @r"
    div xml:space=default {
        a
        b
    }
    ");
}

// ============================================================================
// Processing Instruction Parsing Edge Cases
// ============================================================================

#[test]
fn pi_with_escaped_quote_in_value() {
    // PI with attribute value containing escaped quote
    let kdl = convert(r#"<?custom attr="value with &quot; quote"?><root/>"#);
    insta::assert_snapshot!(kdl.trim(), @r#"
    ?custom attr="value with &quot; quote"
    root
    "#);
}

#[test]
fn pi_with_equals_in_value() {
    // PI with equals sign in attribute value (should not confuse parser)
    let kdl = convert(r#"<?custom expr="a=b"?><root/>"#);
    insta::assert_snapshot!(kdl.trim(), @r#"
?custom expr="a=b"
root
"#);
}

#[test]
fn pi_with_single_quotes() {
    // PI using single quotes for attribute values
    let kdl = convert(r#"<?custom attr='value'?><root/>"#);
    insta::assert_snapshot!(kdl.trim(), @r"
    ?custom attr=value
    root
    ");
}

#[test]
fn pi_with_mixed_quotes() {
    // PI using mix of single and double quotes
    let kdl = convert(r#"<?custom attr1="double" attr2='single'?><root/>"#);
    insta::assert_snapshot!(kdl.trim(), @r"
    ?custom attr1=double attr2=single
    root
    ");
}

#[test]
fn pi_unstructured_content() {
    // PI with unstructured content (no attribute syntax)
    let kdl = convert(r#"<?php echo "hello"; ?><root/>"#);
    insta::assert_snapshot!(kdl.trim(), @r#"
    ?php "echo \"hello\";"
    root
    "#);
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn empty_attribute_value() {
    let kdl = convert(r#"<input value=""/>"#);
    insta::assert_snapshot!(kdl.trim(), @r#"input value="""#);
}

#[test]
fn unicode_content() {
    let kdl = convert("<greeting>Hello, 世界!</greeting>");
    insta::assert_snapshot!(kdl.trim(), @r#"greeting "Hello, 世界!""#);
}

#[test]
fn unicode_emoji() {
    let kdl = convert("<emoji>🎉🚀</emoji>");
    insta::assert_snapshot!(kdl.trim(), @"emoji 🎉🚀");
}

#[test]
fn attribute_with_single_quotes_in_source() {
    let kdl = convert(r#"<div title='hello'/>"#);
    insta::assert_snapshot!(kdl.trim(), @"div title=hello");
}

#[test]
fn very_long_text() {
    let long_text = "x".repeat(10000);
    let xml = format!("<content>{}</content>", long_text);
    let kdl = convert(&xml);
    // Just verify it's long enough - don't snapshot 10k chars
    assert!(kdl.len() > 10000);
}

#[test]
fn deeply_nested_50_levels() {
    let open_tags: String = (0..50).map(|i| format!("<n{}>", i)).collect();
    let close_tags: String = (0..50).rev().map(|i| format!("</n{}>", i)).collect();
    let xml = format!("{}text{}", open_tags, close_tags);
    let kdl = convert(&xml);
    // Just verify structure - too deep to snapshot inline
    assert!(kdl.contains("n49"));
}

#[test]
fn element_name_with_hyphen() {
    let kdl = convert("<my-element/>");
    insta::assert_snapshot!(kdl.trim(), @"my-element");
}

#[test]
fn element_name_with_underscore() {
    let kdl = convert("<my_element/>");
    insta::assert_snapshot!(kdl.trim(), @"my_element");
}

#[test]
fn element_name_with_numbers() {
    let kdl = convert("<element123/>");
    insta::assert_snapshot!(kdl.trim(), @"element123");
}

#[test]
fn attribute_value_with_newline() {
    let kdl = convert("<div title=\"line1\nline2\"/>");
    insta::assert_snapshot!(kdl.trim(), @r#"div title="line1\nline2""#);
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
    let kdl = convert(xml);
    // Validation happens in convert() helper
    insta::assert_snapshot!(kdl.trim(), @r"
    root attr=value {
        child text
    }
    ");
}

#[test]
fn structure_preserved() {
    let xml = "<parent><a/><b/><c/></parent>";
    let kdl = convert(xml);
    insta::assert_snapshot!(kdl.trim(), @r"
parent {
    a
    b
    c
}
");
}

#[test]
fn attributes_preserved() {
    let xml = r#"<element foo="1" bar="2" baz="3"/>"#;
    let kdl = convert(xml);
    insta::assert_snapshot!(kdl.trim(), @r#"element foo="1" bar="2" baz="3""#);
}

#[test]
fn text_content_preserved() {
    let xml = "<greeting>Hello, World!</greeting>";
    let kdl = convert(xml);
    insta::assert_snapshot!(kdl.trim(), @r#"greeting "Hello, World!""#);
}

// ============================================================================
// Real Document Tests (Fixtures)
// ============================================================================

#[test]
fn fixture_html_document() {
    let xml = include_str!("fixtures/sample.html");
    let kdl = convert(xml);
    // Validation happens in convert() helper
    insta::assert_snapshot!(kdl);
}

#[test]
fn fixture_svg_document() {
    let xml = include_str!("fixtures/sample.svg");
    let kdl = convert(xml);
    // Validation happens in convert() helper
    insta::assert_snapshot!(kdl);
}

#[test]
fn fixture_atom_feed() {
    let xml = include_str!("fixtures/atom.xml");
    let kdl = convert(xml);
    // Validation happens in convert() helper
    insta::assert_snapshot!(kdl);
}

// ============================================================================
// Compact Output Tests
// ============================================================================

#[test]
fn compact_mixed_content() {
    let kdl = convert_compact("<p>This is a <strong>sample</strong> paragraph.</p>");
    // Validation happens in convert_compact() helper
    insta::assert_snapshot!(kdl.trim(), @r#"
    p { - "This is a "; strong sample; - " paragraph." }
    "#);
}

#[test]
fn compact_nested_elements() {
    let kdl = convert_compact("<div><span><b>bold</b></span></div>");
    // Validation happens in convert_compact() helper
    insta::assert_snapshot!(kdl.trim(), @"div { span { b bold } }");
}

#[test]
fn compact_multiple_children() {
    let kdl = convert_compact("<ul><li>a</li><li>b</li><li>c</li></ul>");
    // Validation happens in convert_compact() helper
    insta::assert_snapshot!(kdl.trim(), @"ul { li a; li b; li c }");
}

#[test]
fn compact_pure_text_no_children() {
    // Pure text content should still be compact (single argument, no children block)
    let kdl = convert_compact("<greeting>hello</greeting>");
    insta::assert_snapshot!(kdl.trim(), @"greeting hello");
}

#[test]
fn compact_empty_element() {
    let kdl = convert_compact("<empty/>");
    insta::assert_snapshot!(kdl.trim(), @"empty");
}

// ============================================================================
// Leaf Compact Output Tests
// ============================================================================

#[test]
fn compact_leaf_mixed_content() {
    // Leaf mixed content should be compact
    let kdl = convert_compact_leaf("<p>Hello <b>world</b>!</p>");
    // Validation happens in convert_compact_leaf() helper
    insta::assert_snapshot!(kdl.trim(), @r#"p { - "Hello "; b world; - ! }"#);
}

#[test]
fn compact_leaf_nested_not_compacted() {
    // Non-leaf nodes should not be compacted
    let kdl = convert_compact_leaf("<div><p>Hello <b>world</b></p></div>");
    // Validation happens in convert_compact_leaf() helper
    insta::assert_snapshot!(kdl.trim(), @r#"
    div {
        p { - "Hello "; b world }
    }
    "#);
}

#[test]
fn compact_leaf_deeply_nested() {
    // Only the deepest leaf level should be compact
    let kdl =
        convert_compact_leaf("<article><section><p>Text <em>here</em></p></section></article>");
    // Validation happens in convert_compact_leaf() helper
    insta::assert_snapshot!(kdl.trim(), @r#"
    article {
        section {
            p { - "Text "; em here }
        }
    }
    "#);
}

#[test]
fn compact_leaf_list_items() {
    // List items that are leaves should be compact
    let kdl = convert_compact_leaf("<ul><li>a</li><li>b</li></ul>");
    // Validation happens in convert_compact_leaf() helper
    insta::assert_snapshot!(kdl.trim(), @"ul { li a; li b }");
}

#[test]
fn compact_leaf_non_leaf_stays_multiline() {
    // Non-leaf content should stay multi-line
    let kdl = convert_compact_leaf("<nav><ul><li><a>link</a></li></ul></nav>");
    // Validation happens in convert_compact_leaf() helper
    insta::assert_snapshot!(kdl.trim(), @r"
    nav {
        ul {
            li { a link }
        }
    }
    ");
}
