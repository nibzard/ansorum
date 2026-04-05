mod common;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use ahash::AHashMap;
use chrono::NaiveDate;
use common::{
    INVALID_FIXTURES_ROOT, REFERENCE_PROJECT, SITE_FIXTURE, assert_file_matches_fixture,
    build_site, build_site_with_setup, repo_path,
};
use config::TaxonomyConfig;
use content::Page;
use site::Site;
use site::answers::audit_library;
use site::sitemap;
use tempfile::tempdir;
use utils::types::InsertAnchor;

#[test]
fn can_parse_site() {
    let path = repo_path(SITE_FIXTURE);
    let config_file = path.join("config.toml");
    let mut site = Site::new(&path, &config_file).unwrap();
    site.load().unwrap();
    let library = site.library.read().unwrap();

    // Correct number of pages (sections do not count as pages, draft are ignored)
    assert_eq!(library.pages.len(), 36);
    let posts_path = path.join("content").join("posts");

    // Make sure the page with a url doesn't have any sections
    let url_post = library.pages.get(&posts_path.join("fixed-url.md")).unwrap();
    assert_eq!(url_post.path, "/a-fixed-url/");

    // Make sure the article in a folder with only asset doesn't get counted as a section
    let asset_folder_post =
        library.pages.get(&posts_path.join("with-assets").join("index.md")).unwrap();
    assert_eq!(asset_folder_post.file.components, vec!["posts".to_string()]);

    // That we have the right number of sections
    assert_eq!(library.sections.len(), 13);

    // And that the sections are correct
    let index_section = library.sections.get(&path.join("content").join("_index.md")).unwrap();
    assert_eq!(index_section.subsections.len(), 5);
    assert_eq!(index_section.pages.len(), 5);
    assert!(index_section.ancestors.is_empty());

    let posts_section = library.sections.get(&posts_path.join("_index.md")).unwrap();
    assert_eq!(posts_section.subsections.len(), 2);
    assert_eq!(posts_section.pages.len(), 10); // 11 with 1 draft == 10
    assert_eq!(posts_section.ancestors, vec![index_section.file.relative.clone()]);

    // Make sure we remove all the pwd + content from the sections
    let basic = library.pages.get(&posts_path.join("simple.md")).unwrap();
    assert_eq!(basic.file.components, vec!["posts".to_string()]);
    assert_eq!(
        basic.ancestors,
        vec![index_section.file.relative.clone(), posts_section.file.relative.clone(),]
    );

    let tutorials_section =
        library.sections.get(&posts_path.join("tutorials").join("_index.md")).unwrap();
    assert_eq!(tutorials_section.subsections.len(), 2);
    let sub1 = &library.sections[&tutorials_section.subsections[0]];
    let sub2 = &library.sections[&tutorials_section.subsections[1]];
    assert_eq!(sub1.clone().meta.title.unwrap(), "Programming");
    assert_eq!(sub2.clone().meta.title.unwrap(), "DevOps");
    assert_eq!(tutorials_section.pages.len(), 0);

    let devops_section = library
        .sections
        .get(&posts_path.join("tutorials").join("devops").join("_index.md"))
        .unwrap();
    assert_eq!(devops_section.subsections.len(), 0);
    assert_eq!(devops_section.pages.len(), 2);
    assert_eq!(
        devops_section.ancestors,
        vec![
            index_section.file.relative.clone(),
            posts_section.file.relative.clone(),
            tutorials_section.file.relative.clone(),
        ]
    );

    let prog_section = library
        .sections
        .get(&posts_path.join("tutorials").join("programming").join("_index.md"))
        .unwrap();
    assert_eq!(prog_section.subsections.len(), 0);
    assert_eq!(prog_section.pages.len(), 2);

    // Testing extra variables in sections & sitemaps
    // Regression test for #https://github.com/getzola/zola/issues/842
    assert_eq!(
        prog_section.meta.extra.get("we_have_extra").and_then(|s| s.as_str()),
        Some("variables")
    );
    let sitemap_entries = sitemap::find_entries(&library, &site.taxonomies[..], &site.config);
    let sitemap_entry = sitemap_entries
        .iter()
        .find(|e| e.permalink.ends_with("tutorials/programming/"))
        .expect("expected to find programming section in sitemap");
    assert_eq!(Some(&prog_section.meta.extra), sitemap_entry.extra);
}

#[test]
fn extracts_normalized_answer_records() {
    let path = repo_path(REFERENCE_PROJECT);
    let config_file = path.join("config.toml");
    let mut site = Site::new(&path, &config_file).unwrap();
    site.load().unwrap();

    assert_eq!(site.answers.len(), 4);

    let refunds = site.answers.get("refunds-policy").expect("missing refunds-policy answer");
    assert_eq!(refunds.title, "Refund policy");
    assert_eq!(refunds.canonical_url, "https://answers.example.com/refunds/");
    assert_eq!(refunds.markdown_url, "https://answers.example.com/refunds.md");
    assert_eq!(refunds.entity, "billing");
    assert_eq!(
        refunds.retrieval_aliases,
        vec!["refund policy".to_string(), "refund rules".to_string()]
    );

    let cancel =
        site.answers.get("cancel-subscription").expect("missing cancel-subscription answer");
    assert_eq!(cancel.title, "Cancel a subscription");
    assert_eq!(cancel.markdown_url, "https://answers.example.com/cancel.md");

    let credits = site.answers.get("billing-credits").expect("missing billing-credits answer");
    assert_eq!(credits.title, "Billing credits");
    assert_eq!(credits.summary, "How billing credits are issued and when they are applied.");
    assert_eq!(credits.canonical_url, "https://answers.example.com/billing-credits/");
    assert_eq!(credits.markdown_url, "https://answers.example.com/billing-credits.md");
    assert_eq!(credits.entity, "billing");
    assert_eq!(credits.related, vec!["refunds-policy".to_string()]);

    let billing_ids = site
        .answers
        .same_entity("billing")
        .into_iter()
        .map(|record| record.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(billing_ids, vec!["billing-credits", "cancel-subscription", "refunds-policy"]);

    let related_ids = site
        .answers
        .related_to("refunds-policy")
        .into_iter()
        .map(|record| record.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(related_ids, vec!["cancel-subscription"]);
}

#[test]
fn reference_project_configures_redirects_and_passes_audit() {
    let path = repo_path(REFERENCE_PROJECT);
    let config_file = path.join("config.toml");
    let mut site = Site::new(&path, &config_file).unwrap();
    site.load().unwrap();

    assert_eq!(site.config.title.as_deref(), Some("Acme Billing Answers"));
    assert_eq!(site.config.ansorum.redirects.routes.len(), 2);
    assert_eq!(site.config.ansorum.redirects.routes[0].code, "sales-demo");
    assert_eq!(site.config.ansorum.redirects.routes[1].target, "/cancel/");

    let library = site.library.read().unwrap();
    let report = audit_library(
        &library,
        &site.answers,
        NaiveDate::from_ymd_opt(2026, 3, 18).expect("valid date"),
    );

    assert!(!report.has_errors());
    assert!(report.findings.is_empty());
}

#[test]
fn emits_machine_markdown_without_leaking_hidden_content() {
    let (_, _tmp_dir, public) = build_site(REFERENCE_PROJECT);

    assert!(file_exists!(public, "refunds.md"));
    assert!(file_contains!(public, "refunds.md", "# Refund policy"));
    assert!(file_contains!(
        public,
        "refunds.md",
        "canonical_url: https://answers.example.com/refunds/"
    ));
    assert!(file_contains!(public, "refunds.md", "retrieval_aliases:"));

    assert!(file_exists!(public, "cancel.md"));
    assert!(file_contains!(
        public,
        "cancel.md",
        "How to cancel a subscription and what happens after."
    ));
    assert!(file_contains!(
        public,
        "cancel.md",
        "Canonical page: <https://answers.example.com/cancel/>"
    ));
    assert!(!file_contains!(public, "cancel.md", "Cancellation details for customers."));

    assert!(!file_exists!(public, "internal-playbook.md"));
}

#[test]
fn machine_markdown_front_matter_stays_parseable_for_special_characters() {
    let source_root = repo_path(REFERENCE_PROJECT);
    let tmp_dir = tempdir().expect("create temp dir");
    let site_root = tmp_dir.path().join("site");
    copy_dir(&source_root, &site_root);

    fs::write(
        site_root.join("content").join("refunds.md"),
        r#"+++
title = "Billing: \"Refunds\""

id = "refunds-policy"
summary = "First line:\n\"Quoted\" details."
canonical_questions = ["how do refunds: work?", "can i get a \"refund\"?"]
intent = "policy"
entity = "billing:core"
audience = "customer"
related = ["cancel-subscription"]
external_refs = ["https://example.com/refunds"]
schema_type = "FAQPage"
review_by = 2026-06-01
visibility = "public"
ai_visibility = "public"
llms_priority = "core"
token_budget = "medium"
retrieval_aliases = ["refund:policy", "line one\nline two"]
aliases = ["legacy/refund-policy"]
+++

Refund details for customers."#,
    )
    .expect("write refunds fixture");

    let config_file = site_root.join("config.toml");
    let mut site = Site::new(&site_root, &config_file).unwrap();
    site.load().unwrap();
    let public = tmp_dir.path().join("public");
    site.set_output_path(&public);
    site.build().expect("Couldn't build the site");

    let markdown = fs::read_to_string(public.join("refunds.md")).expect("read machine markdown");
    let front_matter = machine_front_matter(&markdown);

    let yaml: serde_yaml::Value =
        serde_yaml::from_str(front_matter).expect("parse yaml front matter");
    assert_eq!(yaml["entity"].as_str(), Some("billing:core"));
    assert_eq!(yaml["canonical_questions"][0].as_str(), Some("how do refunds: work?"));
    assert_eq!(yaml["canonical_questions"][1].as_str(), Some("can i get a \"refund\"?"));
    assert_eq!(yaml["retrieval_aliases"][1].as_str(), Some("line one\nline two"));
}

#[test]
fn keeps_retrieval_aliases_out_of_redirect_outputs() {
    let (_, _tmp_dir, public) = build_site(REFERENCE_PROJECT);

    assert!(file_exists!(public, "legacy/refund-policy/index.html"));
    assert!(file_contains!(
        public,
        "legacy/refund-policy/index.html",
        "https://answers.example.com/refunds/"
    ));

    assert!(!file_exists!(public, "refund policy/index.html"));
    assert!(!file_exists!(public, "refund rules/index.html"));
}

fn copy_dir(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("create destination directory");
    for entry in fs::read_dir(source).expect("read source directory") {
        let entry = entry.expect("read source entry");
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir(&source_path, &destination_path);
        } else {
            fs::copy(&source_path, &destination_path).unwrap_or_else(|error| {
                panic!(
                    "failed to copy `{}` to `{}`: {error}",
                    source_path.display(),
                    destination_path.display()
                )
            });
        }
    }
}

fn machine_front_matter(markdown: &str) -> &str {
    markdown
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---\n").map(|(front_matter, _)| front_matter))
        .expect("expected yaml front matter")
}

#[test]
fn keeps_machine_markdown_when_only_markdown_negotiation_is_disabled() {
    let (_, _tmp_dir, public) = build_site_with_setup(REFERENCE_PROJECT, |mut site| {
        site.config.ansorum.delivery.markdown_negotiation = false;
        (site, true)
    });

    assert!(file_exists!(public, "refunds.md"));
    assert!(file_exists!(public, "cancel.md"));
}

#[test]
fn skips_machine_markdown_outputs_when_markdown_routes_are_disabled() {
    let (_, _tmp_dir, public) = build_site_with_setup(REFERENCE_PROJECT, |mut site| {
        site.config.ansorum.delivery.markdown_routes = false;
        site.config.ansorum.delivery.markdown_negotiation = false;
        (site, true)
    });

    assert!(!file_exists!(public, "refunds.md"));
    assert!(!file_exists!(public, "cancel.md"));
    assert!(!file_exists!(public, "internal-playbook.md"));
}

#[test]
fn machine_indexes_fall_back_to_canonical_urls_when_markdown_routes_are_disabled() {
    let (_, _tmp_dir, public) = build_site_with_setup(REFERENCE_PROJECT, |mut site| {
        site.config.ansorum.delivery.markdown_routes = false;
        site.config.ansorum.delivery.markdown_negotiation = false;
        (site, true)
    });

    let index = fs::read_to_string(public.join("answers.json")).expect("read answers.json");
    let json: serde_json::Value = serde_json::from_str(&index).expect("parse answers.json");
    let answers = json["answers"].as_array().expect("answers should be an array");

    assert_eq!(answers[0]["markdown_url"], "https://answers.example.com/cancel/");
    assert_eq!(answers[1]["markdown_url"], "https://answers.example.com/refunds/");
    assert!(!index.contains(".md"));

    assert!(file_contains!(
        public,
        "llms.txt",
        "- [Refund policy](https://answers.example.com/refunds/): How refunds work, who qualifies, and when payment returns land."
    ));
    assert!(file_contains!(
        public,
        "llms-full.txt",
        "### [Refund policy](https://answers.example.com/refunds/)"
    ));
    assert!(!file_contains!(public, "llms.txt", "https://answers.example.com/refunds.md"));
}

#[test]
fn matches_answer_first_golden_outputs() {
    let (_, _tmp_dir, public) = build_site(REFERENCE_PROJECT);

    assert_file_matches_fixture(
        &public,
        "refunds.md",
        "examples/reference-project/expected/public/refunds.md",
    );
    assert_file_matches_fixture(
        &public,
        "answers.json",
        "examples/reference-project/expected/public/answers.json",
    );
    assert_file_matches_fixture(
        &public,
        "llms.txt",
        "examples/reference-project/expected/public/llms.txt",
    );
    assert_file_matches_fixture(
        &public,
        "llms-full.txt",
        "examples/reference-project/expected/public/llms-full.txt",
    );
    assert_file_matches_fixture(
        &public,
        "refunds/schema.json",
        "examples/reference-project/expected/public/refunds/schema.json",
    );
}

#[test]
fn embeds_json_ld_and_writes_schema_sidecars() {
    let (_, _tmp_dir, public) = build_site(REFERENCE_PROJECT);

    assert!(file_contains!(public, "index.html", "\"@type\": \"WebSite\""));
    assert!(file_contains!(
        public,
        "index.html",
        "\"@id\": \"https://answers.example.com/#website\""
    ));
    assert!(file_contains!(public, "index.html", "\"@type\": \"Organization\""));
    assert!(!file_contains!(public, "index.html", "zentinelproxy.io"));

    assert!(file_exists!(public, "refunds/schema.json"));
    assert!(file_contains!(public, "refunds/schema.json", "\"@type\": \"FAQPage\""));
    assert!(file_contains!(public, "refunds/schema.json", "\"name\": \"Refund policy\""));
    assert!(file_contains!(public, "refunds/schema.json", "\"publisher\""));
    assert!(file_contains!(public, "refunds/index.html", "<script type=\"application/ld+json\">"));
    assert!(file_contains!(public, "refunds/index.html", "\"@type\": \"WebPage\""));
    assert!(file_contains!(public, "refunds/index.html", "\"@type\": \"BreadcrumbList\""));
    assert!(file_contains!(public, "refunds/index.html", "\"@type\": \"FAQPage\""));
    assert!(file_contains!(
        public,
        "refunds/index.html",
        "\"@id\": \"https://answers.example.com/refunds/#breadcrumb\""
    ));
    assert!(file_contains!(
        public,
        "refunds/index.html",
        "\"item\": \"https://answers.example.com/refunds/\""
    ));
    assert!(!file_contains!(public, "refunds/index.html", "zentinelproxy.io"));

    assert!(file_exists!(public, "cancel/schema.json"));
    assert!(file_contains!(public, "cancel/schema.json", "\"@type\": \"HowTo\""));
    assert!(file_contains!(public, "cancel/schema.json", "\"name\": \"Cancel a subscription\""));
}

#[test]
fn emits_answers_json_with_deterministic_order_and_visibility_metadata() {
    let (_, _tmp_dir, public) = build_site(REFERENCE_PROJECT);

    assert!(file_exists!(public, "answers.json"));

    let index = fs::read_to_string(public.join("answers.json")).expect("read answers.json");
    let json: serde_json::Value = serde_json::from_str(&index).expect("parse answers.json");

    assert_eq!(json["version"], 1);

    let answers = json["answers"].as_array().expect("answers should be an array");
    assert_eq!(answers.len(), 2);
    assert_eq!(answers[0]["id"], "cancel-subscription");
    assert_eq!(answers[1]["id"], "refunds-policy");

    assert_eq!(answers[0]["markdown_url"], "https://answers.example.com/cancel.md");
    assert_eq!(answers[0]["ai_visibility"], "summary_only");
    assert_eq!(answers[0]["summary"], "How to cancel a subscription and what happens after.");

    assert_eq!(answers[1]["review_by"], "2026-06-01");
    assert!(answers[1].get("last_modified").is_none());
    assert_eq!(answers[1]["related"][0], "cancel-subscription");
    assert_eq!(answers[1]["retrieval_aliases"][0], "refund policy");
    assert!(!index.contains("internal-support-escalation"));
}

#[test]
fn emits_llms_exports_and_scoped_packs_from_answer_corpus() {
    let (_, _tmp_dir, public) = build_site(REFERENCE_PROJECT);

    assert!(file_exists!(public, "llms.txt"));
    assert!(file_exists!(public, "llms-full.txt"));
    assert!(file_contains!(public, "llms.txt", "# Acme Billing Answers"));
    assert!(file_contains!(
        public,
        "llms.txt",
        "> Reference Ansorum project for a billing and support answer corpus."
    ));
    assert!(file_contains!(public, "llms.txt", "## Core Answers"));
    assert!(file_contains!(
        public,
        "llms.txt",
        "- [Refund policy](https://answers.example.com/refunds.md): How refunds work, who qualifies, and when payment returns land."
    ));
    assert!(file_contains!(public, "llms.txt", "## Optional"));
    assert!(file_contains!(
        public,
        "llms.txt",
        "- [Cancel a subscription](https://answers.example.com/cancel.md): How to cancel a subscription and what happens after."
    ));
    assert!(file_contains!(public, "llms.txt", "## Scoped Packs"));
    assert!(file_contains!(
        public,
        "llms.txt",
        "- [Billing answers](https://answers.example.com/billing/llms.txt): Scoped pack `billing`"
    ));
    assert!(file_contains!(
        public,
        "llms.txt",
        "- [Customer answers](https://answers.example.com/customer/llms.txt): Scoped pack `customer`"
    ));

    assert!(file_contains!(public, "llms-full.txt", "# Acme Billing Answers Full Export"));
    assert!(file_contains!(
        public,
        "llms-full.txt",
        "> Reference Ansorum project for a billing and support answer corpus."
    ));
    assert!(file_contains!(public, "llms-full.txt", "## Core Answers"));
    assert!(file_contains!(
        public,
        "llms-full.txt",
        "### [Refund policy](https://answers.example.com/refunds.md)"
    ));
    assert!(file_contains!(public, "llms-full.txt", "## Eligibility"));
    assert!(file_contains!(
        public,
        "llms-full.txt",
        "Contact billing support with the invoice number and the reason for the request."
    ));
    assert!(file_contains!(public, "llms-full.txt", "## Optional"));
    assert!(file_contains!(
        public,
        "llms-full.txt",
        "### [Cancel a subscription](https://answers.example.com/cancel.md)"
    ));
    assert!(file_contains!(
        public,
        "llms-full.txt",
        "Canonical page: <https://answers.example.com/cancel/>"
    ));
    assert!(!file_contains!(public, "llms-full.txt", "Open the billing portal."));
    assert!(!file_contains!(public, "llms-full.txt", "internal-support-escalation"));

    assert!(file_exists!(public, "billing/llms.txt"));
    assert!(file_exists!(public, "billing/answers.json"));
    assert!(file_contains!(
        public,
        "billing/llms.txt",
        "Curated billing pack for customer-visible billing answers."
    ));
    assert!(file_contains!(
        public,
        "billing/llms.txt",
        "- [Refund policy](https://answers.example.com/refunds.md): How refunds work, who qualifies, and when payment returns land."
    ));

    assert!(file_exists!(public, "customer/llms.txt"));
    assert!(file_exists!(public, "customer/answers.json"));
    assert!(file_contains!(
        public,
        "customer/llms.txt",
        "Scoped AI-visible answers for the `customer` audience."
    ));
    assert!(file_contains!(public, "customer/llms.txt", "## Optional"));
    assert!(!file_exists!(public, "support/llms.txt"));

    let billing_index = fs::read_to_string(public.join("billing").join("answers.json"))
        .expect("read billing answers.json");
    let billing_json: serde_json::Value =
        serde_json::from_str(&billing_index).expect("parse billing answers.json");
    let billing_answers = billing_json["answers"].as_array().expect("billing answers array");
    assert_eq!(billing_answers.len(), 2);
    assert_eq!(billing_answers[0]["id"], "cancel-subscription");
    assert_eq!(billing_answers[1]["id"], "refunds-policy");
}

#[test]
fn errors_on_conflicting_pack_output_names() {
    let path = repo_path(REFERENCE_PROJECT);
    let config_file = path.join("config.toml");
    let mut site = Site::new(&path, &config_file).unwrap();
    site.config.ansorum.packs.curated[0].name = "customer".to_string();
    let err = site.load().expect_err("pack collision should fail during load");
    assert_eq!(
        err.to_string(),
        "Duplicate ansorum pack output path `customer` from curated pack `customer` from /home/agent/ansorum/examples/reference-project/collections/packs/billing.toml conflicts with auto audience pack for `customer`"
    );
}

#[test]
fn ignores_empty_curated_pack_collisions() {
    let path = repo_path(REFERENCE_PROJECT);
    let config_file = path.join("config.toml");
    let mut site = Site::new(&path, &config_file).unwrap();

    let pack_dir = tempdir().expect("create temp dir");
    let pack_path = pack_dir.path().join("hidden-customer.toml");
    fs::write(&pack_path, "answers = [\"internal-support-escalation\"]\n")
        .expect("write pack file");

    site.config.ansorum.packs.curated[0].name = "customer".to_string();
    site.config.ansorum.packs.curated[0].source = pack_path.display().to_string();
    site.load().expect("empty curated collision should be ignored");

    let output_dir = tempdir().expect("create output dir");
    let public = output_dir.path().join("public");
    site.set_output_path(&public);
    site.build()
        .expect("site should build when colliding curated pack resolves to no visible answers");

    assert!(file_exists!(public, "customer/llms.txt"));
    assert!(file_contains!(
        public,
        "customer/llms.txt",
        "Scoped AI-visible answers for the `customer` audience."
    ));
}

#[test]
fn audits_freshness_visibility_and_machine_output_quality() {
    let path = repo_path(&format!("{INVALID_FIXTURES_ROOT}/answers_audit"));
    let config_file = path.join("config.toml");
    let mut site = Site::new(&path, &config_file).unwrap();
    site.load().unwrap();

    let library = site.library.read().unwrap();
    let report = audit_library(
        &library,
        &site.answers,
        NaiveDate::from_ymd_opt(2026, 3, 18).expect("valid date"),
    );

    let codes = report.findings.iter().map(|finding| finding.code.as_str()).collect::<Vec<_>>();

    assert!(codes.contains(&"missing_related_links"));
    assert!(codes.contains(&"stale_review_by"));
    assert!(codes.contains(&"token_budget_overflow"));
    assert!(codes.contains(&"related_visibility_leak"));
    assert!(codes.contains(&"missing_owner"));
    assert!(codes.contains(&"missing_confidence_notes"));
    assert_eq!(report.summary.errors, 4);
    assert_eq!(report.summary.warnings, 5);
    assert!(report.has_errors());
}

#[test]
fn errors_on_unknown_taxonomies() {
    let (mut site, _, _) = build_site(SITE_FIXTURE);
    let mut page = Page::default();
    page.file.path = PathBuf::from("unknown/taxo.md");
    page.meta.taxonomies.insert("wrong".to_string(), vec![]);
    let res = site.add_page(page, false);
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert_eq!(
        err.to_string(),
        "Page `unknown/taxo.md` has taxonomy `wrong` which is not defined in config.toml"
    );
}

#[test]
fn can_build_site_without_live_reload() {
    let (_, _tmp_dir, public) = build_site(SITE_FIXTURE);

    assert!(&public.exists());
    assert!(file_exists!(public, "index.html"));
    assert!(file_exists!(public, "sitemap.xml"));
    assert!(file_exists!(public, "robots.txt"));
    assert!(file_exists!(public, "a-fixed-url/index.html"));
    // the config.title is there
    assert!(file_contains!(public, "index.html", "My Integration Testing site"));

    assert!(file_exists!(public, "posts/python/index.html"));
    // Shortcodes work
    assert!(file_contains!(public, "posts/python/index.html", "Basic shortcode"));
    assert!(file_contains!(public, "posts/python/index.html", "Arrrh Bob"));
    assert!(file_contains!(public, "posts/python/index.html", "Arrrh Bob_Sponge"));
    assert!(file_exists!(public, "posts/tutorials/devops/nix/index.html"));
    assert!(file_exists!(public, "posts/with-assets/index.html"));
    assert!(file_exists!(public, "posts/no-section/simple/index.html"));

    // "render = false" should not generate an index.html
    assert!(!file_exists!(public, "posts/render/index.html"));

    // Sections
    assert!(file_exists!(public, "posts/index.html"));
    assert!(file_exists!(public, "posts/tutorials/index.html"));
    assert!(file_exists!(public, "posts/tutorials/devops/index.html"));
    assert!(file_exists!(public, "posts/tutorials/programming/index.html"));
    // Ensure subsection pages are correctly filled
    assert!(file_contains!(public, "posts/tutorials/index.html", "Sub-pages: 2"));

    // Pages and section get their relative path
    assert!(file_contains!(public, "posts/tutorials/index.html", "posts/tutorials/_index.md"));
    assert!(file_contains!(
        public,
        "posts/tutorials/devops/nix/index.html",
        "posts/tutorials/devops/nix.md"
    ));

    // aliases work
    assert!(file_exists!(public, "an-old-url/old-page/index.html"));
    assert!(file_contains!(public, "an-old-url/old-page/index.html", "something-else"));
    assert!(file_contains!(public, "another-old-url/index.html", "posts/"));

    // html aliases work
    assert!(file_exists!(public, "an-old-url/an-old-alias.html"));
    assert!(file_contains!(public, "an-old-url/an-old-alias.html", "something-else"));

    // redirect_to works
    assert!(file_exists!(public, "posts/tutorials/devops/index.html"));
    assert!(file_contains!(public, "posts/tutorials/devops/index.html", "docker"));

    // We do have categories
    assert!(file_exists!(public, "categories/index.html"));
    assert!(file_exists!(public, "categories/a-category/index.html"));
    assert!(file_exists!(public, "categories/a-category/atom.xml"));
    // and podcast_authors (https://github.com/getzola/zola/issues/1177)
    assert!(file_exists!(public, "podcast-authors/index.html"));
    assert!(file_exists!(public, "podcast-authors/some-person/index.html"));
    assert!(file_exists!(public, "podcast-authors/some-person/atom.xml"));
    // But no tags
    assert!(!file_exists!(public, "tags/index.html"));

    // Theme files are there
    assert!(file_exists!(public, "sample.css"));
    assert!(file_exists!(public, "some.js"));

    // SASS and SCSS files compile correctly
    assert!(file_exists!(public, "blog.css"));
    assert!(file_contains!(public, "blog.css", "red"));
    assert!(file_contains!(public, "blog.css", "blue"));
    assert!(!file_contains!(public, "blog.css", "@import \"included\""));
    assert!(file_contains!(public, "blog.css", "2rem")); // check include
    assert!(!file_exists!(public, "_included.css"));
    assert!(file_exists!(public, "scss.css"));
    assert!(file_exists!(public, "sass.css"));
    assert!(file_exists!(public, "nested_sass/sass.css"));
    assert!(file_exists!(public, "nested_sass/scss.css"));

    assert!(!file_exists!(public, "secret_section/index.html"));
    assert!(!file_exists!(public, "secret_section/page.html"));
    assert!(!file_exists!(public, "secret_section/secret_sub_section/hello.html"));
    // no live reload code
    assert!(!file_contains!(public, "index.html", "/livereload.js?port=1112&amp;mindelay=10"),);

    // Both pages and sections are in the sitemap
    assert!(file_contains!(
        public,
        "sitemap.xml",
        "<loc>https://replace-this-with-your-url.com/posts/simple/</loc>"
    ));
    assert!(file_contains!(
        public,
        "sitemap.xml",
        "<loc>https://replace-this-with-your-url.com/posts/</loc>"
    ));
    // Drafts are not in the sitemap
    assert!(!file_contains!(public, "sitemap.xml", "draft"));
    // render: false sections are not in the sitemap either
    assert!(!file_contains!(public, "sitemap.xml", "posts/2018/</loc>"));

    // robots.txt has been rendered from the template
    assert!(file_contains!(public, "robots.txt", "User-agent: zola"));
    assert!(file_contains!(
        public,
        "robots.txt",
        "Sitemap: https://replace-this-with-your-url.com/sitemap.xml"
    ));

    // And
    assert!(file_contains!(
        public,
        "colocated-assets/index.html",
        "Assets in root content directory"
    ));
}

#[test]
fn can_build_site_with_live_reload_and_drafts() {
    let (site, _tmp_dir, public) = build_site_with_setup(SITE_FIXTURE, |mut site| {
        use std::net::IpAddr;
        use std::str::FromStr;
        site.enable_live_reload(IpAddr::from_str("127.0.0.1").unwrap(), 1000);
        site.include_drafts();
        (site, true)
    });

    assert!(&public.exists());

    assert!(file_exists!(public, "index.html"));
    assert!(file_exists!(public, "sitemap.xml"));
    assert!(file_exists!(public, "robots.txt"));
    assert!(file_exists!(public, "a-fixed-url/index.html"));

    assert!(file_exists!(public, "posts/python/index.html"));
    assert!(file_exists!(public, "posts/tutorials/devops/nix/index.html"));
    assert!(file_exists!(public, "posts/with-assets/index.html"));

    // Sections
    assert!(file_exists!(public, "posts/index.html"));
    assert!(file_exists!(public, "posts/tutorials/index.html"));
    assert!(file_exists!(public, "posts/tutorials/devops/index.html"));
    assert!(file_exists!(public, "posts/tutorials/programming/index.html"));
    // TODO: add assertion for syntax highlighting

    // We do have categories
    assert!(file_exists!(public, "categories/index.html"));
    assert!(file_exists!(public, "categories/a-category/index.html"));
    assert!(file_exists!(public, "categories/a-category/atom.xml"));
    // But no tags
    assert!(!file_exists!(public, "tags/index.html"));

    // no live reload code
    assert!(file_contains!(public, "index.html", "/livereload.js"));

    // the summary target has been created
    assert!(file_contains!(
        public,
        "posts/python/index.html",
        r#"<span id="continue-reading"></span>"#
    ));

    // Drafts are included
    assert!(file_exists!(public, "posts/draft/index.html"));
    assert!(file_contains!(public, "sitemap.xml", "draft"));

    // drafted sections are included
    let library = site.library.read().unwrap();
    assert_eq!(library.sections.len(), 15);

    assert!(file_exists!(public, "secret_section/index.html"));
    assert!(file_exists!(public, "secret_section/draft-page/index.html"));
    assert!(file_exists!(public, "secret_section/page/index.html"));
    assert!(file_exists!(public, "secret_section/secret_sub_section/hello/index.html"));
}

#[test]
fn can_build_site_with_live_reload_under_mounted_base_path() {
    let (_site, _tmp_dir, public) = build_site_with_setup(SITE_FIXTURE, |mut site| {
        use std::net::IpAddr;
        use std::str::FromStr;

        site.set_base_url("https://replace-this-with-your-url.com/docs".to_string());
        site.enable_live_reload(IpAddr::from_str("127.0.0.1").unwrap(), 1000);
        (site, true)
    });

    assert!(file_contains!(public, "index.html", "/docs/livereload.js"));
    assert!(!file_contains!(public, "index.html", "src=\"/livereload.js?"));
}

#[test]
fn can_build_site_with_taxonomies() {
    let (site, _tmp_dir, public) = build_site_with_setup(SITE_FIXTURE, |mut site| {
        site.load().unwrap();
        {
            let library = &mut *site.library.write().unwrap();
            let mut pages = vec![];

            let pages_data = std::mem::replace(&mut library.pages, AHashMap::new());
            for (i, (_, mut page)) in pages_data.into_iter().enumerate() {
                page.meta.taxonomies = {
                    let mut taxonomies = HashMap::new();
                    taxonomies.insert(
                        "categories".to_string(),
                        vec![if i % 2 == 0 { "A" } else { "B" }.to_string()],
                    );
                    taxonomies
                };
                pages.push(page);
            }
            for p in pages {
                library.insert_page(p);
            }
        }
        site.populate_taxonomies().unwrap();
        (site, false)
    });

    assert!(&public.exists());
    assert_eq!(site.taxonomies.len(), 2);

    assert!(file_exists!(public, "index.html"));
    assert!(file_exists!(public, "sitemap.xml"));
    assert!(file_exists!(public, "robots.txt"));
    assert!(file_exists!(public, "a-fixed-url/index.html"));

    assert!(file_exists!(public, "posts/python/index.html"));
    assert!(file_exists!(public, "posts/tutorials/devops/nix/index.html"));
    assert!(file_exists!(public, "posts/with-assets/index.html"));

    // Sections
    assert!(file_exists!(public, "posts/index.html"));
    assert!(file_exists!(public, "posts/tutorials/index.html"));
    assert!(file_exists!(public, "posts/tutorials/devops/index.html"));
    assert!(file_exists!(public, "posts/tutorials/programming/index.html"));

    // Categories are there
    assert!(file_exists!(public, "categories/index.html"));
    assert!(file_exists!(public, "categories/a/index.html"));
    assert!(file_exists!(public, "categories/b/index.html"));
    assert!(file_exists!(public, "categories/a/atom.xml"));
    assert!(file_contains!(
        public,
        "categories/a/atom.xml",
        "https://replace-this-with-your-url.com/categories/a/atom.xml"
    ));
    // Extending from a theme works
    assert!(file_contains!(public, "categories/a/index.html", "EXTENDED"));
    // Tags aren't
    assert!(!file_exists!(public, "tags/index.html"));

    // Categories are in the sitemap
    assert!(file_contains!(
        public,
        "sitemap.xml",
        "<loc>https://replace-this-with-your-url.com/categories/</loc>"
    ));
    assert!(file_contains!(
        public,
        "sitemap.xml",
        "<loc>https://replace-this-with-your-url.com/categories/a/</loc>"
    ));
}

#[test]
fn can_build_site_and_insert_anchor_links() {
    let (_, _tmp_dir, public) = build_site(SITE_FIXTURE);

    assert!(Path::new(&public).exists());
    // anchor link inserted
    assert!(file_contains!(
        public,
        "posts/something-else/index.html",
        "<h1 id=\"title\"><a class=\"anchor-link\" href=\"#title\""
    ));
}

#[test]
fn can_build_site_insert_anchor_links_none_by_default() {
    let (_, _tmp_dir, public) = build_site(SITE_FIXTURE);

    assert!(Path::new(&public).exists());
    // anchor link not inserted
    assert!(file_contains!(public, "index.html", r#"<h1 id="heading-1">Heading 1</h1>"#));
}

#[test]
fn can_build_site_and_insert_anchor_links_global_config() {
    let (_, _tmp_dir, public) = build_site_with_setup(SITE_FIXTURE, |mut site| {
        site.config.markdown.insert_anchor_links = InsertAnchor::Right;
        (site, true)
    });

    assert!(Path::new(&public).exists());
    // anchor link inserted
    assert!(file_contains!(
        public,
        "index.html",
        r##"<h1 id="heading-1">Heading 1<a class="anchor-link" href="#heading-1" aria-label="Anchor link for: heading-1">🔗</a>"##
    ));
}

#[test]
fn can_build_site_with_pagination_for_section() {
    let (_, _tmp_dir, public) = build_site_with_setup(SITE_FIXTURE, |mut site| {
        site.load().unwrap();
        {
            let mut library = site.library.write().unwrap();
            for (_, section) in library.sections.iter_mut() {
                if section.is_index() {
                    continue;
                }
                section.meta.paginate_by = Some(2);
                section.meta.template = Some("section_paginated.html".to_string());
            }
        }
        (site, false)
    });

    assert!(&public.exists());

    assert!(file_exists!(public, "index.html"));
    assert!(file_exists!(public, "sitemap.xml"));
    assert!(file_exists!(public, "robots.txt"));
    assert!(file_exists!(public, "a-fixed-url/index.html"));
    assert!(file_exists!(public, "posts/python/index.html"));
    assert!(file_exists!(public, "posts/tutorials/devops/nix/index.html"));
    assert!(file_exists!(public, "posts/with-assets/index.html"));

    // Sections
    assert!(file_exists!(public, "posts/index.html"));
    // And pagination!
    assert!(file_exists!(public, "posts/page/1/index.html"));
    // even if there is no pages, only the section!
    assert!(file_exists!(public, "paginated/page/1/index.html"));
    assert!(file_exists!(public, "paginated/index.html"));
    // should redirect to posts/
    assert!(file_contains!(
        public,
        "posts/page/1/index.html",
        "http-equiv=\"refresh\" content=\"0; url=https://replace-this-with-your-url.com/posts/\""
    ));
    assert!(file_contains!(public, "posts/index.html", "Num pagers: 5"));
    assert!(file_contains!(public, "posts/index.html", "Page size: 2"));
    assert!(file_contains!(public, "posts/index.html", "Current index: 1"));
    assert!(!file_contains!(public, "posts/index.html", "has_prev"));
    assert!(file_contains!(public, "posts/index.html", "has_next"));
    assert!(file_contains!(
        public,
        "posts/index.html",
        "First: https://replace-this-with-your-url.com/posts/"
    ));
    assert!(file_contains!(
        public,
        "posts/index.html",
        "Last: https://replace-this-with-your-url.com/posts/page/5/"
    ));
    assert!(!file_contains!(public, "posts/index.html", "has_prev"));

    assert!(file_exists!(public, "posts/page/2/index.html"));
    assert!(file_contains!(public, "posts/page/2/index.html", "Num pagers: 5"));
    assert!(file_contains!(public, "posts/page/2/index.html", "Page size: 2"));
    assert!(file_contains!(public, "posts/page/2/index.html", "Current index: 2"));
    assert!(file_contains!(public, "posts/page/2/index.html", "has_prev"));
    assert!(file_contains!(public, "posts/page/2/index.html", "has_next"));
    assert!(file_contains!(
        public,
        "posts/page/2/index.html",
        "First: https://replace-this-with-your-url.com/posts/"
    ));
    assert!(file_contains!(
        public,
        "posts/page/2/index.html",
        "Last: https://replace-this-with-your-url.com/posts/page/5/"
    ));

    assert!(file_exists!(public, "posts/page/3/index.html"));
    assert!(file_contains!(public, "posts/page/3/index.html", "Num pagers: 5"));
    assert!(file_contains!(public, "posts/page/3/index.html", "Page size: 2"));
    assert!(file_contains!(public, "posts/page/3/index.html", "Current index: 3"));
    assert!(file_contains!(public, "posts/page/3/index.html", "has_prev"));
    assert!(file_contains!(public, "posts/page/3/index.html", "has_next"));
    assert!(file_contains!(
        public,
        "posts/page/3/index.html",
        "First: https://replace-this-with-your-url.com/posts/"
    ));
    assert!(file_contains!(
        public,
        "posts/page/3/index.html",
        "Last: https://replace-this-with-your-url.com/posts/page/5/"
    ));

    assert!(file_exists!(public, "posts/page/4/index.html"));
    assert!(file_contains!(public, "posts/page/4/index.html", "Num pagers: 5"));
    assert!(file_contains!(public, "posts/page/4/index.html", "Page size: 2"));
    assert!(file_contains!(public, "posts/page/4/index.html", "Current index: 4"));
    assert!(file_contains!(public, "posts/page/4/index.html", "has_prev"));
    assert!(file_contains!(public, "posts/page/4/index.html", "has_next"));
    assert!(file_contains!(
        public,
        "posts/page/4/index.html",
        "First: https://replace-this-with-your-url.com/posts/"
    ));
    assert!(file_contains!(
        public,
        "posts/page/4/index.html",
        "Last: https://replace-this-with-your-url.com/posts/page/5/"
    ));

    // sitemap contains the pager pages
    assert!(file_contains!(
        public,
        "sitemap.xml",
        "<loc>https://replace-this-with-your-url.com/posts/page/4/</loc>"
    ));

    // current_path
    assert!(file_contains!(public, "posts/index.html", &current_path("/posts/")));
    assert!(file_contains!(public, "posts/page/2/index.html", &current_path("/posts/page/2/")));
    assert!(file_contains!(public, "posts/python/index.html", &current_path("/posts/python/")));
    assert!(file_contains!(
        public,
        "posts/tutorials/index.html",
        &current_path("/posts/tutorials/")
    ));
}

#[test]
fn can_build_site_with_pagination_for_index() {
    let (_, _tmp_dir, public) = build_site_with_setup(SITE_FIXTURE, |mut site| {
        site.load().unwrap();
        {
            let mut library = site.library.write().unwrap();
            {
                let index = library
                    .sections
                    .get_mut(&site.base_path.join("content").join("_index.md"))
                    .unwrap();
                index.meta.paginate_by = Some(2);
                index.meta.template = Some("index_paginated.html".to_string());
            }
        }
        (site, false)
    });

    assert!(&public.exists());

    assert!(file_exists!(public, "index.html"));
    assert!(file_exists!(public, "sitemap.xml"));
    assert!(file_exists!(public, "robots.txt"));
    assert!(file_exists!(public, "a-fixed-url/index.html"));
    assert!(file_exists!(public, "posts/python/index.html"));
    assert!(file_exists!(public, "posts/tutorials/devops/nix/index.html"));
    assert!(file_exists!(public, "posts/with-assets/index.html"));

    // And pagination!
    assert!(file_exists!(public, "page/1/index.html"));
    // even if there is no pages, only the section!
    assert!(file_exists!(public, "paginated/page/1/index.html"));
    assert!(file_exists!(public, "paginated/index.html"));
    // should redirect to index
    assert!(file_contains!(
        public,
        "page/1/index.html",
        "http-equiv=\"refresh\" content=\"0; url=https://replace-this-with-your-url.com/\""
    ));
    assert!(file_contains!(public, "page/1/index.html", "<title>Redirect</title>"));
    assert!(file_contains!(
        public,
        "page/1/index.html",
        "<a href=\"https://replace-this-with-your-url.com/\">Click here</a>"
    ));
    assert!(file_contains!(public, "index.html", "Num pages: 3"));
    assert!(file_contains!(public, "index.html", "Current index: 1"));
    assert!(file_contains!(public, "index.html", "First: https://replace-this-with-your-url.com/"));
    assert!(file_contains!(
        public,
        "index.html",
        "Last: https://replace-this-with-your-url.com/page/3/"
    ));
    assert!(!file_contains!(public, "index.html", "has_prev"));
    assert!(file_contains!(public, "index.html", "has_next"));

    // sitemap contains the pager pages
    assert!(file_contains!(
        public,
        "sitemap.xml",
        "<loc>https://replace-this-with-your-url.com/page/1/</loc>"
    ));

    // current_path
    assert!(file_contains!(public, "index.html", &current_path("/")));
    assert!(file_contains!(public, "page/2/index.html", &current_path("/page/2/")));
    assert!(file_contains!(public, "paginated/index.html", &current_path("/paginated/")));
}

#[test]
fn can_build_site_with_pagination_for_taxonomy() {
    let mut nb_a_pages = 0;
    let (_, _tmp_dir, public) = build_site_with_setup(SITE_FIXTURE, |mut site| {
        site.config.languages.get_mut("en").unwrap().taxonomies.push(TaxonomyConfig {
            name: "tags".to_string(),
            slug: "tags".to_string(),
            paginate_by: Some(2),
            paginate_path: None,
            render: true,
            feed: true,
        });
        site.load().unwrap();
        {
            let library = &mut *site.library.write().unwrap();
            let mut pages = vec![];

            let pages_data = std::mem::replace(&mut library.pages, AHashMap::new());
            for (i, (_, mut page)) in pages_data.into_iter().enumerate() {
                // Discard not rendered pages
                if i % 2 == 0 && page.meta.render {
                    nb_a_pages += 1;
                }
                page.meta.taxonomies = {
                    let mut taxonomies = HashMap::new();
                    taxonomies.insert(
                        "tags".to_string(),
                        vec![if i % 2 == 0 { "A" } else { "B" }.to_string()],
                    );
                    taxonomies
                };
                pages.push(page);
            }
            for p in pages {
                library.insert_page(p);
            }
        }
        site.populate_taxonomies().unwrap();
        (site, false)
    });
    let nb_a_pagers: usize =
        if nb_a_pages % 2 == 0 { nb_a_pages / 2 } else { (nb_a_pages / 2) + 1 };
    assert!(&public.exists());

    assert!(file_exists!(public, "index.html"));
    assert!(file_exists!(public, "sitemap.xml"));
    assert!(file_exists!(public, "robots.txt"));
    assert!(file_exists!(public, "a-fixed-url/index.html"));
    assert!(file_exists!(public, "posts/python/index.html"));
    assert!(file_exists!(public, "posts/tutorials/devops/nix/index.html"));
    assert!(file_exists!(public, "posts/with-assets/index.html"));

    // Tags
    assert!(file_exists!(public, "tags/index.html"));
    // With Atom
    assert!(file_exists!(public, "tags/a/atom.xml"));
    assert!(file_exists!(public, "tags/b/atom.xml"));
    // And pagination!
    assert!(file_exists!(public, "tags/a/page/1/index.html"));
    assert!(file_exists!(public, "tags/b/page/1/index.html"));
    assert!(file_exists!(public, "tags/a/page/2/index.html"));
    assert!(file_exists!(public, "tags/b/page/2/index.html"));

    // should redirect to posts/
    assert!(file_contains!(
        public,
        "tags/a/page/1/index.html",
        "http-equiv=\"refresh\" content=\"0; url=https://replace-this-with-your-url.com/tags/a/\""
    ));
    assert!(file_contains!(public, "tags/a/index.html", &format!("Num pagers: {nb_a_pagers}")));
    assert!(file_contains!(public, "tags/a/index.html", "Page size: 2"));
    assert!(file_contains!(public, "tags/a/index.html", "Current index: 1"));
    assert!(!file_contains!(public, "tags/a/index.html", "has_prev"));
    assert!(file_contains!(public, "tags/a/index.html", "has_next"));
    assert!(file_contains!(
        public,
        "tags/a/index.html",
        "First: https://replace-this-with-your-url.com/tags/a/"
    ));

    assert!(file_contains!(
        public,
        "tags/a/index.html",
        &format!("Last: https://replace-this-with-your-url.com/tags/a/page/{nb_a_pagers}/")
    ));

    assert!(!file_contains!(public, "tags/a/index.html", "has_prev"));

    // sitemap contains the pager pages
    assert!(file_contains!(
        public,
        "sitemap.xml",
        "<loc>https://replace-this-with-your-url.com/tags/a/page/8/</loc>"
    ));

    // current_path
    assert!(file_contains!(public, "tags/index.html", &current_path("/tags/")));
    assert!(file_contains!(public, "tags/a/index.html", &current_path("/tags/a/")));
    assert!(file_contains!(public, "tags/a/page/2/index.html", &current_path("/tags/a/page/2/")));
}

#[test]
fn can_build_feeds() {
    let (_, _tmp_dir, public) = build_site(SITE_FIXTURE);

    assert!(&public.exists());
    assert!(file_exists!(public, "atom.xml"));
    // latest article is posts/extra-syntax.md
    assert!(file_contains!(public, "atom.xml", "Extra Syntax"));
    // Next is posts/simple.md
    assert!(file_contains!(public, "atom.xml", "Simple article with shortcodes"));

    // Test section feeds
    assert!(file_exists!(public, "posts/tutorials/programming/atom.xml"));
    // It contains both sections articles
    assert!(file_contains!(public, "posts/tutorials/programming/atom.xml", "Python tutorial"));
    assert!(file_contains!(public, "posts/tutorials/programming/atom.xml", "Rust"));
    // It doesn't contain articles from other sections
    assert!(!file_contains!(public, "posts/tutorials/programming/atom.xml", "Extra Syntax"));

    // Test Atom feed entry with 3 authors
    assert!(file_contains!(public, "posts/tutorials/programming/atom.xml", "Foo Doe"));
    assert!(file_contains!(public, "posts/tutorials/programming/atom.xml", "Bar Doe"));
    assert!(file_contains!(public, "posts/tutorials/programming/atom.xml", "Baz Doe"));
}

#[test]
fn can_build_search_index() {
    let (_, _tmp_dir, public) = build_site_with_setup(SITE_FIXTURE, |mut site| {
        site.config.build_search_index = true;
        (site, true)
    });

    assert!(Path::new(&public).exists());
    assert!(file_exists!(public, "elasticlunr.min.js"));
    assert!(file_exists!(public, "search_index.en.js"));
}

#[test]
fn can_build_with_extra_syntaxes() {
    let (_, _tmp_dir, public) = build_site(SITE_FIXTURE);

    assert!(&public.exists());
    assert!(file_exists!(public, "posts/extra-syntax/index.html"));
    assert!(file_contains!(public, "posts/extra-syntax/index.html", r#"<span style="color:"#));
}

#[test]
fn can_apply_page_templates() {
    let path = repo_path(SITE_FIXTURE);
    let config_file = path.join("config.toml");
    let mut site = Site::new(&path, &config_file).unwrap();
    site.load().unwrap();

    let template_path = path.join("content").join("applying_page_template");
    let library = site.library.read().unwrap();

    let template_section = library.sections.get(&template_path.join("_index.md")).unwrap();
    assert_eq!(template_section.subsections.len(), 2);
    assert_eq!(template_section.pages.len(), 2);

    let from_section_config = &library.pages[&template_section.pages[0]];
    assert_eq!(from_section_config.meta.template, Some("page_template.html".into()));
    assert_eq!(from_section_config.meta.title, Some("From section config".into()));

    let override_page_template = &library.pages[&template_section.pages[1]];
    assert_eq!(override_page_template.meta.template, Some("page_template_override.html".into()));
    assert_eq!(override_page_template.meta.title, Some("Override".into()));

    // It should have applied recursively as well
    let another_section =
        library.sections.get(&template_path.join("another_section").join("_index.md")).unwrap();
    assert_eq!(another_section.subsections.len(), 0);
    assert_eq!(another_section.pages.len(), 1);

    let changed_recursively = &library.pages[&another_section.pages[0]];
    assert_eq!(changed_recursively.meta.template, Some("page_template.html".into()));
    assert_eq!(changed_recursively.meta.title, Some("Changed recursively".into()));

    // But it should not have override a children page_template
    let yet_another_section =
        library.sections.get(&template_path.join("yet_another_section").join("_index.md")).unwrap();
    assert_eq!(yet_another_section.subsections.len(), 0);
    assert_eq!(yet_another_section.pages.len(), 1);

    let child = &library.pages[&yet_another_section.pages[0]];
    assert_eq!(child.meta.template, Some("page_template_child.html".into()));
    assert_eq!(child.meta.title, Some("Local section override".into()));
}

// https://github.com/getzola/zola/issues/571
#[test]
fn can_build_site_custom_builtins_from_theme() {
    let (_, _tmp_dir, public) = build_site(SITE_FIXTURE);

    assert!(&public.exists());
    // 404.html is a theme template.
    assert!(file_exists!(public, "404.html"));
    assert!(file_contains!(public, "404.html", "Oops"));
}

#[test]
fn can_build_site_with_html_minified() {
    let (_, _tmp_dir, public) = build_site_with_setup(SITE_FIXTURE, |mut site| {
        site.config.minify_html = true;
        (site, true)
    });

    assert!(&public.exists());
    assert!(file_exists!(public, "index.html"));
    assert!(file_contains!(
        public,
        "index.html",
        "<!doctype html><html lang=en><head><meta charset=UTF-8>"
    ));
}

#[test]
fn can_ignore_markdown_content() {
    let (_, _tmp_dir, public) = build_site(SITE_FIXTURE);
    assert!(!file_exists!(public, "posts/ignored/index.html"));
}

#[test]
fn can_cachebust_static_files() {
    let (_, _tmp_dir, public) = build_site(SITE_FIXTURE);
    assert!(file_contains!(
        public,
        "index.html",
        "<link href=\"https://replace-this-with-your-url.com/site.css?h=83bd983e8899946ee33d\" rel=\"stylesheet\">"
    ));
}

#[test]
fn can_get_hash_for_static_files() {
    let (_, _tmp_dir, public) = build_site(SITE_FIXTURE);
    assert!(file_contains!(
        public,
        "index.html",
        "src=\"https://replace-this-with-your-url.com/scripts/hello.js\""
    ));
    assert!(file_contains!(
        public,
        "index.html",
        "integrity=\"sha384-AUIvMeqnIabErIxvoJon3ZJZ4N/PPHWT14ENkSqd5covWC35eFN7zRD3aJbbYfu5\""
    ));
}

#[test]
fn can_check_site() {
    let (mut site, _tmp_dir, _public) = build_site(SITE_FIXTURE);

    assert_eq!(
        site.config.link_checker.skip_anchor_prefixes,
        vec!["https://github.com/rust-lang/rust/blob/"]
    );
    assert_eq!(
        site.config.link_checker.skip_prefixes,
        vec!["http://[2001:db8::]/", "http://invaliddomain"]
    );

    site.config.enable_check_mode();
    site.load().expect("link check site fixture");
}

#[test]
#[should_panic]
fn panics_on_invalid_external_domain() {
    let (mut site, _tmp_dir, _public) = build_site(SITE_FIXTURE);

    // remove the invalid domain skip prefix
    let i = site
        .config
        .link_checker
        .skip_prefixes
        .iter()
        .position(|prefix| prefix == "http://invaliddomain")
        .unwrap();
    site.config.link_checker.skip_prefixes.remove(i);

    // confirm the invalid domain skip prefix was removed
    assert_eq!(site.config.link_checker.skip_prefixes, vec!["http://[2001:db8::]/"]);

    // check the test site, this time without the invalid domain skip prefix, which should cause a
    // panic
    site.config.enable_check_mode();
    site.load().expect("link check site fixture");
}

#[test]
fn external_links_ignored_on_check() {
    let (mut site, _tmp_dir, _public) = build_site(SITE_FIXTURE);

    // remove the invalid domain skip prefix
    let i = site
        .config
        .link_checker
        .skip_prefixes
        .iter()
        .position(|prefix| prefix == "http://invaliddomain")
        .unwrap();
    site.config.link_checker.skip_prefixes.remove(i);

    // confirm the invalid domain skip prefix was removed
    assert_eq!(site.config.link_checker.skip_prefixes, vec!["http://[2001:db8::]/"]);

    // set a flag to skip external links check
    site.skip_external_links_check();

    // check the test site with all external links (including invalid domain) skipped, which should
    // not cause a panic
    site.config.enable_check_mode();
    site.load().expect("link check site fixture");
}

#[test]
fn can_find_site_and_page_authors() {
    let path = repo_path(SITE_FIXTURE);
    let config_file = path.join("config.toml");
    let mut site = Site::new(&path, config_file).unwrap();
    site.load().unwrap();
    let library = site.library.read().unwrap();

    // The config has a global default author set.
    let author = site.config.author;
    assert_eq!(Some("config@example.com (Config Author)".to_string()), author);

    let posts_path = path.join("content").join("posts");
    let posts_section = library.sections.get(&posts_path.join("_index.md")).unwrap();

    let p1 = &library.pages[&posts_section.pages[0]];
    let p2 = &library.pages[&posts_section.pages[1]];

    // Only the first page has had an author added.
    assert_eq!(1, p1.meta.authors.len());
    assert_eq!("page@example.com (Page Author)", p1.meta.authors.first().unwrap());
    assert_eq!(0, p2.meta.authors.len());
}

#[test]
fn filters_non_public_answers_from_machine_outputs() {
    let (site, _tmp_dir, public) = build_site("tests/fixtures/invalid/answers_visibility_outputs");

    assert!(file_exists!(public, "public/index.html"));
    assert!(file_exists!(public, "internal/index.html"));
    assert!(file_exists!(public, "private/index.html"));

    assert!(file_exists!(public, "public.md"));
    assert!(!file_exists!(public, "internal.md"));
    assert!(!file_exists!(public, "private.md"));

    assert!(file_exists!(public, "answers.json"));
    assert!(file_contains!(public, "answers.json", "\"billing-overview\""));
    assert!(!file_contains!(public, "answers.json", "\"internal-only\""));
    assert!(!file_contains!(public, "answers.json", "\"private-playbook\""));
    assert!(!file_contains!(public, "answers.json", "\"related\": [\n        \"internal-only\""));
    assert!(!file_contains!(
        public,
        "answers.json",
        "\"related\": [\n        \"private-playbook\""
    ));

    assert!(file_exists!(public, "billing/answers.json"));
    assert!(!file_contains!(public, "billing/answers.json", "\"internal-only\""));
    assert!(!file_contains!(public, "billing/answers.json", "\"private-playbook\""));

    assert!(file_exists!(public, "llms.txt"));
    assert!(!file_contains!(public, "llms.txt", "Internal only"));
    assert!(!file_contains!(public, "llms.txt", "Private playbook"));

    assert!(file_exists!(public, "billing/llms.txt"));
    assert!(!file_contains!(public, "billing/llms.txt", "Internal only"));
    assert!(!file_contains!(public, "billing/llms.txt", "Private playbook"));

    assert!(!file_exists!(public, "internal/answers.json"));
    assert!(!file_exists!(public, "internal/llms.txt"));

    let library = site.library.read().unwrap();
    let report = audit_library(
        &library,
        &site.answers,
        NaiveDate::from_ymd_opt(2026, 3, 18).expect("valid date"),
    );

    assert!(report.has_errors());
    assert!(report.findings.iter().any(|finding| finding.code == "visibility_leak"));
    assert!(report.findings.iter().any(|finding| finding.code == "related_visibility_leak"));
}

// Follows tests/fixtures/site/themes/sample/templates/current_path.html
fn current_path(path: &str) -> String {
    format!("[current_path]({})", path)
}
