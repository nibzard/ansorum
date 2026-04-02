use std::path::PathBuf;

use config::Config;
use content::{Library, Page, Section};
use errors::Result;
use serde_json::{Map as JsonMap, Value as JsonValue, json};
use toml::Value as TomlValue;
use url::Url;
use utils::net::is_external_link;

pub fn site_structured_data_for_page(
    page: &Page,
    config: &Config,
    library: &Library,
) -> Result<String> {
    let home_url = home_url(config);
    let mut graph = site_graph(config, &page.lang, &home_url);
    let breadcrumb = breadcrumb_for_page(page, config, library, &home_url);
    graph.push(webpage_node(
        &page.permalink,
        page_title(page, config),
        page_description(page, config),
        &page.lang,
        page.meta.date.as_deref(),
        page.meta.updated.as_deref().or(page.meta.date.as_deref()),
        breadcrumb.as_ref(),
        &home_url,
    ));
    if let Some(breadcrumb) = breadcrumb {
        graph.push(breadcrumb);
    }

    serialize_graph(graph)
}

pub fn site_structured_data_for_section(
    section: &Section,
    config: &Config,
    library: &Library,
) -> Result<String> {
    let home_url = home_url(config);
    let mut graph = site_graph(config, &section.lang, &home_url);
    let breadcrumb = breadcrumb_for_section(section, config, library, &home_url);
    graph.push(webpage_node(
        &section.permalink,
        section_title(section, config),
        section_description(section, config),
        &section.lang,
        None,
        None,
        breadcrumb.as_ref(),
        &home_url,
    ));
    if let Some(breadcrumb) = breadcrumb {
        graph.push(breadcrumb);
    }

    serialize_graph(graph)
}

fn serialize_graph(graph: Vec<JsonValue>) -> Result<String> {
    let document = json!({
        "@context": "https://schema.org",
        "@graph": graph,
    });
    Ok(serde_json::to_string_pretty(&document)?)
}

fn site_graph(config: &Config, lang: &str, home_url: &str) -> Vec<JsonValue> {
    let website_id = format!("{home_url}#website");
    let organization_id = format!("{home_url}#organization");
    let mut website = JsonMap::new();
    website.insert("@type".to_string(), JsonValue::String("WebSite".to_string()));
    website.insert("@id".to_string(), JsonValue::String(website_id));
    website.insert("url".to_string(), JsonValue::String(home_url.to_string()));
    website.insert("name".to_string(), JsonValue::String(site_name(config, home_url).to_string()));
    if let Some(description) =
        config.description.as_deref().filter(|value| !value.trim().is_empty())
    {
        website.insert("description".to_string(), JsonValue::String(description.to_string()));
    }
    website.insert("inLanguage".to_string(), JsonValue::String(lang.to_string()));
    website.insert("publisher".to_string(), json!({ "@id": organization_id }));
    if config.build_search_index {
        website.insert(
            "potentialAction".to_string(),
            json!({
                "@type": "SearchAction",
                "target": {
                    "@type": "EntryPoint",
                    "urlTemplate": format!("{home_url}?q={{search_term_string}}"),
                },
                "query-input": "required name=search_term_string",
            }),
        );
    }

    let mut organization = JsonMap::new();
    organization.insert("@type".to_string(), JsonValue::String("Organization".to_string()));
    organization.insert("@id".to_string(), JsonValue::String(format!("{home_url}#organization")));
    organization.insert("name".to_string(), JsonValue::String(organization_name(config, home_url)));
    organization.insert("url".to_string(), JsonValue::String(organization_url(config, home_url)));
    if let Some(logo) = organization_logo(config) {
        organization.insert(
            "logo".to_string(),
            json!({
                "@type": "ImageObject",
                "url": logo,
            }),
        );
    }
    let same_as = config_extra_string_array(config, &["schema", "organization_same_as"]);
    if !same_as.is_empty() {
        organization.insert(
            "sameAs".to_string(),
            JsonValue::Array(same_as.into_iter().map(JsonValue::String).collect()),
        );
    }

    vec![JsonValue::Object(website), JsonValue::Object(organization)]
}

fn webpage_node(
    url: &str,
    title: &str,
    description: Option<&str>,
    lang: &str,
    date_published: Option<&str>,
    date_modified: Option<&str>,
    breadcrumb: Option<&JsonValue>,
    home_url: &str,
) -> JsonValue {
    let mut object = JsonMap::new();
    object.insert("@type".to_string(), JsonValue::String("WebPage".to_string()));
    object.insert("@id".to_string(), JsonValue::String(format!("{url}#webpage")));
    object.insert("url".to_string(), JsonValue::String(url.to_string()));
    object.insert("isPartOf".to_string(), json!({ "@id": format!("{home_url}#website") }));
    object.insert("publisher".to_string(), json!({ "@id": format!("{home_url}#organization") }));
    object.insert("inLanguage".to_string(), JsonValue::String(lang.to_string()));
    if !title.trim().is_empty() {
        object.insert("name".to_string(), JsonValue::String(title.to_string()));
    }
    if let Some(description) = description.filter(|value| !value.trim().is_empty()) {
        object.insert("description".to_string(), JsonValue::String(description.to_string()));
    }
    if let Some(date_published) = date_published.filter(|value| !value.trim().is_empty()) {
        object.insert("datePublished".to_string(), JsonValue::String(date_published.to_string()));
    }
    if let Some(date_modified) = date_modified.filter(|value| !value.trim().is_empty()) {
        object.insert("dateModified".to_string(), JsonValue::String(date_modified.to_string()));
    }
    if let Some(breadcrumb) = breadcrumb
        && let Some(id) = breadcrumb.get("@id").and_then(JsonValue::as_str)
    {
        object.insert("breadcrumb".to_string(), json!({ "@id": id }));
    }

    JsonValue::Object(object)
}

fn breadcrumb_for_page(
    page: &Page,
    config: &Config,
    library: &Library,
    home_url: &str,
) -> Option<JsonValue> {
    let title = page_title(page, config);
    if title.trim().is_empty() {
        return None;
    }

    let mut items = vec![breadcrumb_item(1, site_name(config, home_url), home_url)];
    let mut position = 2;
    for ancestor in &page.ancestors {
        if let Some(section) = library.sections.get(&PathBuf::from(ancestor)) {
            items.push(breadcrumb_item(
                position,
                section_title(section, config),
                &section.permalink,
            ));
            position += 1;
        }
    }
    items.push(breadcrumb_item(position, title, &page.permalink));

    Some(json!({
        "@type": "BreadcrumbList",
        "@id": format!("{}#breadcrumb", page.permalink),
        "itemListElement": items,
    }))
}

fn breadcrumb_for_section(
    section: &Section,
    config: &Config,
    library: &Library,
    home_url: &str,
) -> Option<JsonValue> {
    if section.path == "/" {
        return None;
    }

    let mut items = vec![breadcrumb_item(1, site_name(config, home_url), home_url)];
    let mut position = 2;
    for ancestor in &section.ancestors {
        if let Some(ancestor_section) = library.sections.get(&PathBuf::from(ancestor)) {
            if ancestor_section.path == "/" {
                continue;
            }
            items.push(breadcrumb_item(
                position,
                section_title(ancestor_section, config),
                &ancestor_section.permalink,
            ));
            position += 1;
        }
    }
    items.push(breadcrumb_item(position, section_title(section, config), &section.permalink));

    Some(json!({
        "@type": "BreadcrumbList",
        "@id": format!("{}#breadcrumb", section.permalink),
        "itemListElement": items,
    }))
}

fn breadcrumb_item(position: usize, name: &str, item: &str) -> JsonValue {
    json!({
        "@type": "ListItem",
        "position": position,
        "name": name,
        "item": item,
    })
}

fn page_title<'a>(page: &'a Page, config: &'a Config) -> &'a str {
    let title = page.answer_title();
    if !title.trim().is_empty() { title } else { config.title.as_deref().unwrap_or("Page") }
}

fn page_description<'a>(page: &'a Page, config: &'a Config) -> Option<&'a str> {
    page.meta
        .description
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .or(page.summary.as_deref().filter(|value| !value.trim().is_empty()))
        .or(page.answer().map(|answer| answer.summary.as_str()))
        .or(config.description.as_deref().filter(|value| !value.trim().is_empty()))
}

fn section_description<'a>(section: &'a Section, config: &'a Config) -> Option<&'a str> {
    section
        .meta
        .description
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .or(config.description.as_deref().filter(|value| !value.trim().is_empty()))
}

fn section_title<'a>(section: &'a Section, config: &'a Config) -> &'a str {
    section
        .meta
        .title
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .or(config.title.as_deref().filter(|value| !value.trim().is_empty()))
        .unwrap_or("Home")
}

fn site_name<'a>(config: &'a Config, home_url: &'a str) -> &'a str {
    let _ = home_url;
    config.title.as_deref().filter(|value| !value.trim().is_empty()).unwrap_or("Site")
}

fn organization_name(config: &Config, home_url: &str) -> String {
    config_extra_string(config, &["schema", "organization_name"])
        .or_else(|| config.title.clone())
        .or_else(|| Url::parse(home_url).ok().and_then(|url| url.host_str().map(str::to_string)))
        .unwrap_or_else(|| "Organization".to_string())
}

fn organization_url(config: &Config, home_url: &str) -> String {
    config_extra_string(config, &["schema", "organization_url"])
        .map(|value| normalize_url(config, &value))
        .unwrap_or_else(|| home_url.to_string())
}

fn organization_logo(config: &Config) -> Option<String> {
    config_extra_string(config, &["schema", "organization_logo"])
        .or_else(|| config_extra_string(config, &["logo"]))
        .or_else(|| config_extra_string(config, &["og_image"]))
        .map(|value| normalize_url(config, &value))
}

fn home_url(config: &Config) -> String {
    config.make_permalink("/")
}

fn normalize_url(config: &Config, value: &str) -> String {
    if is_external_link(value) { value.to_string() } else { config.make_permalink(value) }
}

fn config_extra_string(config: &Config, path: &[&str]) -> Option<String> {
    config_extra_value(config, path)?.as_str().map(str::to_string)
}

fn config_extra_string_array(config: &Config, path: &[&str]) -> Vec<String> {
    config_extra_value(config, path)
        .and_then(TomlValue::as_array)
        .map(|values| {
            values.iter().filter_map(TomlValue::as_str).map(str::to_string).collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn config_extra_value<'a>(config: &'a Config, path: &[&str]) -> Option<&'a TomlValue> {
    let (first, rest) = path.split_first()?;
    let mut value = config.extra.get(*first)?;
    for segment in rest {
        value = value.get(*segment)?;
    }
    Some(value)
}
