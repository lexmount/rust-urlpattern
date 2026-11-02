use urlpattern::Error;
use urlpattern::RegexSyntax;
use urlpattern::UrlPatternInit;
use urlpattern::component::Component;
use urlpattern::quirks;
use urlpattern::regexp::RegExp;

#[derive(Debug)]
struct ScopedRegExp(String);

impl RegExp for ScopedRegExp {
  fn syntax() -> RegexSyntax {
    RegexSyntax::EcmaScript
  }

  fn parse(pattern: &str, _: &str, _: bool) -> Result<Self, ()> {
    Ok(Self(pattern.to_owned()))
  }

  fn matches<'a>(&self, _: &'a str) -> Option<Vec<Option<&'a str>>> {
    panic!("protocol matching must use the supplied scoped callback")
  }

  fn pattern_string(&self) -> &str {
    &self.0
  }
}

#[test]
fn constructor_uses_scoped_matcher_and_backend_syntax_before_splitting() {
  let mut protocols = Vec::new();
  let init = UrlPatternInit::parse_constructor_string_with_protocol_matcher::<
    ScopedRegExp,
  >(
    r"(http(?=s)s)\:example.test/users/:id",
    None,
    &mut |component: &Component<ScopedRegExp>| {
      protocols.push(component.pattern_string.clone());
      assert!(
        component
          .regexp
          .as_ref()
          .unwrap()
          .pattern_string()
          .contains("(?=s)")
      );
      Ok(true)
    },
  )
  .unwrap();

  assert_eq!(protocols, ["(http(?=s)s)"]);
  assert_eq!(init.hostname.as_deref(), Some("example.test"));
  assert_eq!(init.pathname.as_deref(), Some("/users/:id"));
}

#[test]
fn scoped_matcher_controls_path_separator_semantics() {
  for special in [true, false] {
    let mut protocols = Vec::new();
    let pattern = quirks::parse_pattern_with_protocol_matcher::<regex::Regex>(
      UrlPatternInit {
        protocol: Some("(https?)".to_owned()),
        pathname: Some(":part".to_owned()),
        ..Default::default()
      },
      Default::default(),
      &mut |component| {
        protocols.push(component.pattern_string.clone());
        Ok(special)
      },
    )
    .unwrap();

    assert_eq!(protocols, ["(https?)"]);
    let pathname = regex::Regex::new(&pattern.pathname.regexp_string).unwrap();
    assert!(pathname.is_match("one"));
    assert_eq!(pathname.is_match("one/two"), !special);
  }
}

#[test]
fn scoped_matcher_errors_propagate_without_default_matching() {
  let mut calls = 0;
  let mut matcher = |_: &Component<ScopedRegExp>| {
    calls += 1;
    Err(Error::RegExp(()))
  };
  let constructor =
    UrlPatternInit::parse_constructor_string_with_protocol_matcher::<
      ScopedRegExp,
    >("(https?)://example.test/path", None, &mut matcher);
  assert!(matches!(constructor, Err(Error::RegExp(()))));

  let pattern = quirks::parse_pattern_with_protocol_matcher::<ScopedRegExp>(
    UrlPatternInit {
      protocol: Some("(https?)".to_owned()),
      pathname: Some(":part".to_owned()),
      ..Default::default()
    },
    urlpattern::UrlPatternOptions {
      regex_syntax: RegexSyntax::EcmaScript,
      ..Default::default()
    },
    &mut matcher,
  );
  assert!(matches!(pattern, Err(Error::RegExp(()))));
  assert_eq!(calls, 2);
}

#[test]
fn relative_constructor_preserves_base_url_rules_without_protocol_matching() {
  let mut calls = 0;
  let mut matcher = |_: &Component<ScopedRegExp>| {
    calls += 1;
    Ok(true)
  };
  let missing_base =
    UrlPatternInit::parse_constructor_string_with_protocol_matcher::<
      ScopedRegExp,
    >("/users/:id", None, &mut matcher);
  assert!(matches!(missing_base, Err(Error::BaseUrlRequired)));

  let base = url::Url::parse("https://example.test/base/").unwrap();
  let init = UrlPatternInit::parse_constructor_string_with_protocol_matcher::<
    ScopedRegExp,
  >("/users/:id", Some(base.clone()), &mut matcher)
  .unwrap();
  assert_eq!(init.pathname.as_deref(), Some("/users/:id"));
  assert_eq!(init.base_url, Some(base));
  assert_eq!(calls, 0);
}
