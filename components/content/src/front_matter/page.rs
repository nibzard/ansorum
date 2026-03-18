use std::collections::HashMap;

use serde::Deserialize;
use tera::{Map, Value};
use time::format_description::well_known::Rfc3339;
use time::macros::{format_description, time};
use time::{Date, OffsetDateTime, PrimitiveDateTime};

use errors::{Result, bail};
use utils::de::{fix_toml_dates, from_unknown_datetime};

use crate::answer::AnswerFrontMatter;
use crate::front_matter::answer::RawAnswerFrontMatter;
use crate::front_matter::split::RawFrontMatter;

/// The front matter of every page
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageFrontMatter {
    /// <title> of the page
    pub title: Option<String>,
    /// Description in <meta> that appears when linked, e.g. on twitter
    pub description: Option<String>,
    /// Updated date
    pub updated: Option<String>,
    /// Datetime content was last updated
    pub updated_datetime: Option<OffsetDateTime>,
    /// The converted update datetime into a (year, month, day) tuple
    pub updated_datetime_tuple: Option<(i32, u8, u8)>,
    /// Date if we want to order pages (ie blog post)
    pub date: Option<String>,
    /// Datetime content was created
    pub datetime: Option<OffsetDateTime>,
    /// The converted date into a (year, month, day) tuple
    pub datetime_tuple: Option<(i32, u8, u8)>,
    /// Whether this page is a draft
    pub draft: bool,
    /// Prevent generation of a folder for current page
    /// Defaults to `true`
    pub render: bool,
    /// The page slug. Will be used instead of the filename if present
    /// Can't be an empty string if present
    pub slug: Option<String>,
    /// The path the page appears at, overrides the slug if set in the front-matter
    /// otherwise is set after parsing front matter and sections
    /// Can't be an empty string if present
    pub path: Option<String>,
    pub taxonomies: HashMap<String, Vec<String>>,
    /// Integer to use to order content. Highest is at the bottom, lowest first
    pub weight: Option<usize>,
    /// The authors of the page.
    pub authors: Vec<String>,
    /// All aliases for that page. Zola will create HTML templates that will
    /// redirect to this
    pub aliases: Vec<String>,
    /// Specify a template different from `page.html` to use for that page
    pub template: Option<String>,
    /// Whether the page is included in the search index
    /// Defaults to `true` but is only used if search if explicitly enabled in the config.
    pub in_search_index: bool,
    /// Any extra parameter present in the front matter
    pub extra: Map<String, Value>,
    /// Ansorum answer front matter promoted to first-class typed metadata
    pub answer: Option<AnswerFrontMatter>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
struct PageFrontMatterRaw {
    title: Option<String>,
    description: Option<String>,
    #[serde(default, deserialize_with = "from_unknown_datetime")]
    updated: Option<String>,
    #[serde(default, deserialize_with = "from_unknown_datetime")]
    date: Option<String>,
    draft: bool,
    render: bool,
    slug: Option<String>,
    path: Option<String>,
    taxonomies: HashMap<String, Vec<String>>,
    weight: Option<usize>,
    authors: Vec<String>,
    aliases: Vec<String>,
    template: Option<String>,
    in_search_index: bool,
    extra: Map<String, Value>,
    id: Option<String>,
    summary: Option<String>,
    canonical_questions: Vec<String>,
    intent: Option<crate::answer::AnswerIntent>,
    entity: Option<String>,
    audience: Option<crate::answer::AnswerAudience>,
    related: Vec<String>,
    external_refs: Vec<String>,
    schema_type: Option<String>,
    #[serde(default, deserialize_with = "from_unknown_datetime")]
    review_by: Option<String>,
    priority: Option<String>,
    visibility: Option<crate::answer::AnswerVisibility>,
    ai_visibility: Option<crate::answer::AiVisibility>,
    llms_priority: Option<crate::answer::LlmsPriority>,
    token_budget: Option<crate::answer::TokenBudget>,
    ai_extra: Option<String>,
    last_reviewed_by: Option<String>,
    owner: Option<String>,
    confidence_notes: Option<String>,
}

impl Default for PageFrontMatterRaw {
    fn default() -> Self {
        Self {
            title: None,
            description: None,
            updated: None,
            date: None,
            draft: false,
            render: true,
            slug: None,
            path: None,
            taxonomies: HashMap::new(),
            weight: None,
            authors: Vec::new(),
            aliases: Vec::new(),
            template: None,
            in_search_index: true,
            extra: Map::new(),
            id: None,
            summary: None,
            canonical_questions: Vec::new(),
            intent: None,
            entity: None,
            audience: None,
            related: Vec::new(),
            external_refs: Vec::new(),
            schema_type: None,
            review_by: None,
            priority: None,
            visibility: None,
            ai_visibility: None,
            llms_priority: None,
            token_budget: None,
            ai_extra: None,
            last_reviewed_by: None,
            owner: None,
            confidence_notes: None,
        }
    }
}

impl PageFrontMatterRaw {
    fn answer(&self) -> RawAnswerFrontMatter {
        RawAnswerFrontMatter {
            id: self.id.clone(),
            summary: self.summary.clone(),
            canonical_questions: self.canonical_questions.clone(),
            intent: self.intent.clone(),
            entity: self.entity.clone(),
            audience: self.audience.clone(),
            related: self.related.clone(),
            external_refs: self.external_refs.clone(),
            schema_type: self.schema_type.clone(),
            review_by: self.review_by.clone(),
            priority: self.priority.clone(),
            visibility: self.visibility.clone(),
            ai_visibility: self.ai_visibility.clone(),
            llms_priority: self.llms_priority.clone(),
            token_budget: self.token_budget.clone(),
            aliases: self.aliases.clone(),
            ai_extra: self.ai_extra.clone(),
            last_reviewed_by: self.last_reviewed_by.clone(),
            owner: self.owner.clone(),
            confidence_notes: self.confidence_notes.clone(),
        }
    }
}

/// Parse a string for a datetime coming from one of the supported TOML format
/// There are three alternatives:
/// 1. an offset datetime (plain RFC3339)
/// 2. a local datetime (RFC3339 with timezone omitted)
/// 3. a local date (YYYY-MM-DD).
///
/// This tries each in order.
fn parse_datetime(d: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(d, &Rfc3339)
        .or_else(|_| OffsetDateTime::parse(format!("{}Z", d).as_ref(), &Rfc3339))
        .or_else(|_| match Date::parse(d, &format_description!("[year]-[month]-[day]")) {
            Ok(date) => Ok(PrimitiveDateTime::new(date, time!(0:00)).assume_utc()),
            Err(e) => Err(e),
        })
        .ok()
}

impl PageFrontMatter {
    pub fn parse(raw: &RawFrontMatter) -> Result<PageFrontMatter> {
        let raw: PageFrontMatterRaw = raw.deserialize()?;
        let mut extra = match fix_toml_dates(raw.extra.clone()) {
            Value::Object(o) => o,
            _ => unreachable!("Got something other than a table in page extra"),
        };
        let answer = raw.answer().merge(RawAnswerFrontMatter::from_extra(&mut extra)?);
        let mut f = PageFrontMatter {
            title: raw.title.clone(),
            description: raw.description.clone(),
            updated: raw.updated.clone(),
            updated_datetime: None,
            updated_datetime_tuple: None,
            date: raw.date.clone(),
            datetime: None,
            datetime_tuple: None,
            draft: raw.draft,
            render: raw.render,
            slug: raw.slug.clone(),
            path: raw.path.clone(),
            taxonomies: raw.taxonomies.clone(),
            weight: raw.weight,
            authors: raw.authors.clone(),
            aliases: raw.aliases.clone(),
            template: raw.template.clone(),
            in_search_index: raw.in_search_index,
            extra,
            answer: answer.into_answer(&raw.title)?,
        };

        if let Some(ref slug) = f.slug
            && slug.is_empty()
        {
            bail!("`slug` can't be empty if present")
        }

        if let Some(ref path) = f.path
            && path.is_empty()
        {
            bail!("`path` can't be empty if present")
        }

        f.date_to_datetime();

        for terms in f.taxonomies.values() {
            for term in terms {
                if term.trim().is_empty() {
                    bail!("A taxonomy term cannot be an empty string");
                }
            }
        }

        if let Some(ref date) = f.date
            && f.datetime.is_none()
        {
            bail!("`date` could not be parsed: {}.", date);
        }

        Ok(f)
    }

    /// Converts the TOML datetime to a time::OffsetDateTime
    /// Also grabs the year/month/day tuple that will be used in serialization
    pub fn date_to_datetime(&mut self) {
        self.datetime = self.date.as_ref().map(|s| s.as_ref()).and_then(parse_datetime);
        self.datetime_tuple = self
            .datetime
            .map(|dt: OffsetDateTime| (dt.year(), dt.month().into(), dt.day()));

        self.updated_datetime = self.updated.as_ref().map(|s| s.as_ref()).and_then(parse_datetime);
        self.updated_datetime_tuple = self
            .updated_datetime
            .map(|dt: OffsetDateTime| (dt.year(), dt.month().into(), dt.day()));
    }

    pub fn weight(&self) -> usize {
        self.weight.unwrap()
    }
}

impl Default for PageFrontMatter {
    fn default() -> PageFrontMatter {
        PageFrontMatter {
            in_search_index: true,
            title: None,
            description: None,
            updated: None,
            updated_datetime: None,
            updated_datetime_tuple: None,
            date: None,
            datetime: None,
            datetime_tuple: None,
            draft: false,
            render: true,
            slug: None,
            path: None,
            taxonomies: HashMap::new(),
            weight: None,
            authors: Vec::new(),
            aliases: Vec::new(),
            template: None,
            extra: Map::new(),
            answer: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::front_matter::page::PageFrontMatter;
    use crate::front_matter::split::RawFrontMatter;
    use crate::answer::{AiVisibility, AnswerAudience, AnswerIntent, AnswerVisibility, LlmsPriority, TokenBudget};
    use tera::to_value;
    use test_case::test_case;
    use time::macros::datetime;

    #[test_case(&RawFrontMatter::Toml(r#"  "#); "toml")]
    #[test_case(&RawFrontMatter::Toml(r#"  "#); "yaml")]
    fn can_have_empty_front_matter(content: &RawFrontMatter) {
        let res = PageFrontMatter::parse(content);
        println!("{:?}", res);
        assert!(res.is_ok());
    }

    #[test_case(&RawFrontMatter::Toml(r#"
title = "Hello"
description = "hey there"
"#); "toml")]
    #[test_case(&RawFrontMatter::Yaml(r#"
title: Hello
description: hey there
"#); "yaml")]
    fn can_parse_valid_front_matter(content: &RawFrontMatter) {
        let res = PageFrontMatter::parse(content);
        assert!(res.is_ok());
        let res = res.unwrap();
        assert_eq!(res.title.unwrap(), "Hello".to_string());
        assert_eq!(res.description.unwrap(), "hey there".to_string())
    }

    #[test_case(&RawFrontMatter::Toml(r#"title = |\n"#); "toml")]
    #[test_case(&RawFrontMatter::Yaml(r#"title: |\n"#); "yaml")]
    fn errors_with_invalid_front_matter(content: &RawFrontMatter) {
        let res = PageFrontMatter::parse(content);
        assert!(res.is_err());
    }

    #[test_case(&RawFrontMatter::Toml(r#"
title = "Hello"
description = "hey there"
slug = ""
"#); "toml")]
    #[test_case(&RawFrontMatter::Yaml(r#"
title: Hello
description: hey there
slug: ""
"#); "yaml")]
    fn errors_on_present_but_empty_slug(content: &RawFrontMatter) {
        let res = PageFrontMatter::parse(content);
        assert!(res.is_err());
    }

    #[test_case(&RawFrontMatter::Toml(r#"
title = "Hello"
description = "hey there"
path = ""
"#); "toml")]
    #[test_case(&RawFrontMatter::Yaml(r#"
title: Hello
description: hey there
path: ""
"#); "yaml")]
    fn errors_on_present_but_empty_path(content: &RawFrontMatter) {
        let res = PageFrontMatter::parse(content);
        assert!(res.is_err());
    }

    #[test_case(&RawFrontMatter::Toml(r#"
title = "Hello"
description = "hey there"
date = 2016-10-10
"#); "toml")]
    #[test_case(&RawFrontMatter::Yaml(r#"
title: Hello
description: hey there
date: 2016-10-10
"#); "yaml")]
    fn can_parse_date_yyyy_mm_dd(content: &RawFrontMatter) {
        let res = PageFrontMatter::parse(content).unwrap();
        assert!(res.datetime.is_some());
        assert_eq!(res.datetime.unwrap(), datetime!(2016 - 10 - 10 0:00 UTC));
    }

    #[test_case(&RawFrontMatter::Toml(r#"
title = "Hello"
description = "hey there"
date = 2002-10-02T15:00:00Z
"#); "toml")]
    #[test_case(&RawFrontMatter::Yaml(r#"
title: Hello
description: hey there
date: 2002-10-02T15:00:00Z
"#); "yaml")]
    fn can_parse_date_rfc3339(content: &RawFrontMatter) {
        let res = PageFrontMatter::parse(content).unwrap();
        assert!(res.datetime.is_some());
        assert_eq!(res.datetime.unwrap(), datetime!(2002 - 10 - 02 15:00:00 UTC));
    }

    #[test_case(&RawFrontMatter::Toml(r#"
title = "Hello"
description = "hey there"
date = 2002-10-02T15:00:00
"#); "toml")]
    #[test_case(&RawFrontMatter::Yaml(r#"
title: Hello
description: hey there
date: 2002-10-02T15:00:00
"#); "yaml")]
    fn can_parse_date_rfc3339_without_timezone(content: &RawFrontMatter) {
        let res = PageFrontMatter::parse(content).unwrap();
        assert!(res.datetime.is_some());
        assert_eq!(res.datetime.unwrap(), datetime!(2002 - 10 - 02 15:00:00 UTC));
    }

    #[test_case(&RawFrontMatter::Toml(r#"
title = "Hello"
description = "hey there"
date = 2002-10-02 15:00:00+02:00
"#); "toml")]
    #[test_case(&RawFrontMatter::Yaml(r#"
title: Hello
description: hey there
date: 2002-10-02 15:00:00+02:00
"#); "yaml")]
    fn can_parse_date_rfc3339_with_space(content: &RawFrontMatter) {
        let res = PageFrontMatter::parse(content).unwrap();
        assert!(res.datetime.is_some());
        assert_eq!(res.datetime.unwrap(), datetime!(2002 - 10 - 02 15:00:00+02:00));
    }

    #[test_case(&RawFrontMatter::Toml(r#"
title = "Hello"
description = "hey there"
date = 2002-10-02 15:00:00
"#); "toml")]
    #[test_case(&RawFrontMatter::Yaml(r#"
title: Hello
description: hey there
date: 2002-10-02 15:00:00
"#); "yaml")]
    fn can_parse_date_rfc3339_with_space_without_timezone(content: &RawFrontMatter) {
        let res = PageFrontMatter::parse(content).unwrap();
        assert!(res.datetime.is_some());
        assert_eq!(res.datetime.unwrap(), datetime!(2002 - 10 - 02 15:00:00 UTC));
    }

    #[test_case(&RawFrontMatter::Toml(r#"
title = "Hello"
description = "hey there"
date = 2002-10-02T15:00:00.123456Z
"#); "toml")]
    #[test_case(&RawFrontMatter::Yaml(r#"
title: Hello
description: hey there
date: 2002-10-02T15:00:00.123456Z
"#); "yaml")]
    fn can_parse_date_rfc3339_with_microseconds(content: &RawFrontMatter) {
        let res = PageFrontMatter::parse(content).unwrap();
        assert!(res.datetime.is_some());
        assert_eq!(res.datetime.unwrap(), datetime!(2002 - 10 - 02 15:00:00.123456 UTC));
    }

    #[test_case(&RawFrontMatter::Yaml(r#"
title: Hello
description: hey there
date: 2001-12-15T02:59:43.1Z
"#); "canonical")]
    #[test_case(&RawFrontMatter::Yaml(r#"
title: Hello
description: hey there
date: 2001-12-14t21:59:43.10-05:00
"#); "iso8601")]
    #[test_case(&RawFrontMatter::Yaml(r#"
title: Hello
description: hey there
date: 2001-12-14 21:59:43.10 -5
"#); "space separated")]
    #[test_case(&RawFrontMatter::Yaml(r#"
title: Hello
description: hey there
date: 2001-12-15 2:59:43.10
"#); "no time zone")]
    #[test_case(&RawFrontMatter::Yaml(r#"
title: Hello
description: hey there
date: 2001-12-15
"#); "date only")]
    fn can_parse_yaml_dates(content: &RawFrontMatter) {
        let res = PageFrontMatter::parse(content).unwrap();
        assert!(res.datetime.is_some());
    }

    #[test_case(&RawFrontMatter::Toml(r#"
title = "Hello"
description = "hey there"
date = 2002/10/12
"#); "toml")]
    #[test_case(&RawFrontMatter::Yaml(r#"
title: Hello
description: hey there
date: 2002/10/12
"#); "yaml")]
    fn cannot_parse_random_date_format(content: &RawFrontMatter) {
        let res = PageFrontMatter::parse(content);
        assert!(res.is_err());
    }

    #[test_case(&RawFrontMatter::Toml(r#"
title = "Hello"
description = "hey there"
date = 2002-14-01
"#); "toml")]
    #[test_case(&RawFrontMatter::Yaml(r#"
title: Hello
description: hey there
date: 2002-14-01
"#); "yaml")]
    fn cannot_parse_invalid_date_format(content: &RawFrontMatter) {
        let res = PageFrontMatter::parse(content);
        assert!(res.is_err());
    }

    #[test_case(&RawFrontMatter::Toml(r#"
title = "Hello"
description = "hey there"
date = "2016-10-10"
"#); "toml")]
    #[test_case(&RawFrontMatter::Yaml(r#"
title: Hello
description: hey there
date: "2016-10-10"
"#); "yaml")]
    fn can_parse_valid_date_as_string(content: &RawFrontMatter) {
        let res = PageFrontMatter::parse(content).unwrap();
        assert!(res.date.is_some());
        assert!(res.datetime.is_some());
        assert_eq!(res.datetime.unwrap(), datetime!(2016 - 10 - 10 0:00 UTC));
    }

    #[test_case(&RawFrontMatter::Toml(r#"
title = "Hello"
description = "hey there"
date = "2002-14-01"
"#); "toml")]
    #[test_case(&RawFrontMatter::Yaml(r#"
title: Hello
description: hey there
date: "2002-14-01"
"#); "yaml")]
    fn cannot_parse_invalid_date_as_string(content: &RawFrontMatter) {
        let res = PageFrontMatter::parse(content);
        assert!(res.is_err());
    }

    #[test_case(&RawFrontMatter::Toml(r#"
title = "Hello"
description = "hey there"

[extra]
some-date = 2002-11-01
"#); "toml")]
    #[test_case(&RawFrontMatter::Yaml(r#"
title: Hello
description: hey there

extra:
    some-date: 2002-11-01
"#); "yaml")]
    fn can_parse_dates_in_extra(content: &RawFrontMatter) {
        let res = PageFrontMatter::parse(content);
        println!("{:?}", res);
        assert!(res.is_ok());
        assert_eq!(res.unwrap().extra["some-date"], to_value("2002-11-01").unwrap());
    }

    #[test_case(&RawFrontMatter::Toml(r#"
title = "Hello"
description = "hey there"

[extra.something]
some-date = 2002-11-01
"#); "toml")]
    #[test_case(&RawFrontMatter::Yaml(r#"
title: Hello
description: hey there

extra:
    something:
        some-date: 2002-11-01
"#); "yaml")]
    fn can_parse_nested_dates_in_extra(content: &RawFrontMatter) {
        let res = PageFrontMatter::parse(content);
        println!("{:?}", res);
        assert!(res.is_ok());
        assert_eq!(res.unwrap().extra["something"]["some-date"], to_value("2002-11-01").unwrap());
    }

    #[test_case(&RawFrontMatter::Toml(r#"
title = "Hello"
description = "hey there"

[extra]
date_example = 2020-05-04
[[extra.questions]]
date = 2020-05-03
name = "Who is the prime minister of Uganda?"
"#); "toml")]
    #[test_case(&RawFrontMatter::Yaml(r#"
title: Hello
description: hey there

extra:
    date_example: 2020-05-04
    questions:
        - date: 2020-05-03
          name: "Who is the prime minister of Uganda?"
"#); "yaml")]
    fn can_parse_fully_nested_dates_in_extra(content: &RawFrontMatter) {
        let res = PageFrontMatter::parse(content);
        println!("{:?}", res);
        assert!(res.is_ok());
        assert_eq!(res.unwrap().extra["questions"][0]["date"], to_value("2020-05-03").unwrap());
    }

    #[test_case(&RawFrontMatter::Toml(r#"
title = "Hello World"

[taxonomies]
tags = ["Rust", "JavaScript"]
categories = ["Dev"]
"#); "toml")]
    #[test_case(&RawFrontMatter::Yaml(r#"
title: Hello World

taxonomies:
    tags:
        - Rust
        - JavaScript
    categories:
        - Dev
"#); "yaml")]
    fn can_parse_taxonomies(content: &RawFrontMatter) {
        let res = PageFrontMatter::parse(content);
        println!("{:?}", res);
        assert!(res.is_ok());
        let res2 = res.unwrap();
        assert_eq!(res2.taxonomies["categories"], vec!["Dev"]);
        assert_eq!(res2.taxonomies["tags"], vec!["Rust", "JavaScript"]);
    }

    #[test_case(&RawFrontMatter::Toml(r#"
title = "Hello World"

[taxonomies]
tags = [""]
"#); "toml")]
    #[test_case(&RawFrontMatter::Yaml(r#"
title: Hello World

taxonomies:
    tags:
        -
"#); "yaml")]
    fn errors_on_empty_taxonomy_term(content: &RawFrontMatter) {
        // https://github.com/getzola/zola/issues/2085
        let res = PageFrontMatter::parse(content);
        println!("{:?}", res);
        assert!(res.is_err());
    }

    #[test_case(&RawFrontMatter::Toml(r#"
authors = ["person1@example.com (Person One)", "person2@example.com (Person Two)"]
"#); "toml")]
    #[test_case(&RawFrontMatter::Yaml(r#"
title: Hello World
authors:
    - person1@example.com (Person One)
    - person2@example.com (Person Two)
"#); "yaml")]
    fn can_parse_authors(content: &RawFrontMatter) {
        let res = PageFrontMatter::parse(content);
        assert!(res.is_ok());
        let res2 = res.unwrap();
        assert_eq!(res2.authors.len(), 2);
        assert_eq!(
            vec!(
                "person1@example.com (Person One)".to_owned(),
                "person2@example.com (Person Two)".to_owned()
            ),
            res2.authors
        );
    }

    #[test]
    fn keeps_page_defaults_when_answer_fields_are_absent() {
        let res = PageFrontMatter::parse(&RawFrontMatter::Toml("")).unwrap();
        assert!(res.render);
        assert!(res.in_search_index);
    }

    #[test_case(&RawFrontMatter::Toml(r#"
title = "Refund policy"
id = "refunds-policy"
summary = "How refunds work."
canonical_questions = ["how do refunds work"]
intent = "policy"
entity = "billing"
audience = "customer"
related = ["pricing"]
external_refs = ["https://example.com/refunds"]
schema_type = "WebPage"
review_by = 2026-06-01
priority = "high"
visibility = "public"
ai_visibility = "summary_only"
llms_priority = "core"
token_budget = "medium"
aliases = ["refund policy"]
ai_extra = "Keep it concise"
last_reviewed_by = "docs@example.com"
owner = "Billing"
confidence_notes = "Reviewed by legal"
"#); "toml")]
    #[test_case(&RawFrontMatter::Yaml(r#"
title: Refund policy
id: refunds-policy
summary: How refunds work.
canonical_questions:
  - how do refunds work
intent: policy
entity: billing
audience: customer
related:
  - pricing
external_refs:
  - https://example.com/refunds
schema_type: WebPage
review_by: 2026-06-01
priority: high
visibility: public
ai_visibility: summary_only
llms_priority: core
token_budget: medium
aliases:
  - refund policy
ai_extra: Keep it concise
last_reviewed_by: docs@example.com
owner: Billing
confidence_notes: Reviewed by legal
"#); "yaml")]
    fn can_parse_first_class_answer_front_matter(content: &RawFrontMatter) {
        let res = PageFrontMatter::parse(content).unwrap();
        let answer = res.answer.unwrap();
        assert_eq!(answer.id, "refunds-policy");
        assert_eq!(answer.summary, "How refunds work.");
        assert_eq!(answer.intent, AnswerIntent::Policy);
        assert_eq!(answer.audience, AnswerAudience::Customer);
        assert_eq!(answer.visibility, AnswerVisibility::Public);
        assert_eq!(answer.ai_visibility, AiVisibility::SummaryOnly);
        assert_eq!(answer.llms_priority, LlmsPriority::Core);
        assert_eq!(answer.token_budget, TokenBudget::Medium);
        assert_eq!(answer.review_by, Some("2026-06-01".to_string()));
        assert_eq!(answer.aliases, vec!["refund policy".to_string()]);
    }

    #[test_case(&RawFrontMatter::Toml(r#"
title = "Refund policy"

[extra.answer]
id = "refunds-policy"
summary = "How refunds work."
canonical_questions = ["how do refunds work"]
intent = "policy"
entity = "billing"
audience = "customer"
visibility = "public"
ai_visibility = "public"
llms_priority = "core"
token_budget = "medium"
aliases = ["refund policy"]
"#); "toml")]
    #[test_case(&RawFrontMatter::Yaml(r#"
title: Refund policy
extra:
  answer:
    id: refunds-policy
    summary: How refunds work.
    canonical_questions:
      - how do refunds work
    intent: policy
    entity: billing
    audience: customer
    visibility: public
    ai_visibility: public
    llms_priority: core
    token_budget: medium
    aliases:
      - refund policy
"#); "yaml")]
    fn can_promote_legacy_extra_answer_front_matter(content: &RawFrontMatter) {
        let res = PageFrontMatter::parse(content).unwrap();
        assert!(res.extra.get("answer").is_none());
        let answer = res.answer.unwrap();
        assert_eq!(answer.id, "refunds-policy");
        assert_eq!(answer.aliases, vec!["refund policy".to_string()]);
    }

    #[test]
    fn first_class_answer_fields_override_extra_answer_values() {
        let res = PageFrontMatter::parse(&RawFrontMatter::Toml(
            r#"
title = "Refund policy"
id = "top-level"
summary = "Top level summary"
canonical_questions = ["how do refunds work"]
intent = "policy"
entity = "billing"
audience = "customer"
visibility = "public"
ai_visibility = "public"
llms_priority = "core"
token_budget = "medium"
aliases = ["top alias"]

[extra.answer]
id = "legacy"
summary = "Legacy summary"
canonical_questions = ["legacy question"]
intent = "faq"
entity = "support"
audience = "prospect"
visibility = "internal"
ai_visibility = "hidden"
llms_priority = "optional"
token_budget = "small"
aliases = ["legacy alias"]
"#,
        ))
        .unwrap();
        let answer = res.answer.unwrap();
        assert_eq!(answer.id, "top-level");
        assert_eq!(answer.summary, "Top level summary");
        assert_eq!(answer.aliases, vec!["top alias".to_string()]);
    }

    #[test]
    fn errors_when_answer_missing_required_fields() {
        let res = PageFrontMatter::parse(&RawFrontMatter::Toml(
            r#"
title = "Refund policy"
intent = "policy"
"#,
        ));
        assert!(res.is_err());
    }

    #[test]
    fn errors_when_answer_enum_is_invalid() {
        let res = PageFrontMatter::parse(&RawFrontMatter::Toml(
            r#"
title = "Refund policy"
id = "refunds-policy"
summary = "How refunds work."
canonical_questions = ["how do refunds work"]
intent = "bad-value"
entity = "billing"
audience = "customer"
visibility = "public"
ai_visibility = "public"
llms_priority = "core"
token_budget = "medium"
"#,
        ));
        assert!(res.is_err());
    }
}
