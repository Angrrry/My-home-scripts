use caps_layout_switcher::kde::{find_layout_index, parse_layouts};

#[test]
fn parses_busctl_layout_triplets() {
    let layouts = parse_layouts(
        r#"a(sss) 3 "by" "intl" "Belarusian (intl.)" "us" "" "English (US)" "ru" "" "Russian""#,
    );

    assert_eq!(layouts.len(), 3);
    assert_eq!(layouts[0].code, "by");
    assert_eq!(layouts[0].variant, "intl");
    assert_eq!(layouts[1].code, "us");
    assert_eq!(layouts[2].display_name, "Russian");
}

#[test]
fn finds_layout_by_code_and_optional_variant() {
    let layouts = parse_layouts(
        r#"a(sss) 3 "by" "intl" "Belarusian (intl.)" "us" "" "English (US)" "ru" "" "Russian""#,
    );

    assert_eq!(find_layout_index(&layouts, "by", Some("intl")), Some(0));
    assert_eq!(find_layout_index(&layouts, "us", None), Some(1));
    assert_eq!(find_layout_index(&layouts, "ru", None), Some(2));
    assert_eq!(find_layout_index(&layouts, "by", Some("")), None);
}
