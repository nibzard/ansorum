use std::fs::{canonicalize, create_dir};
use std::path::{Path, PathBuf};

use errors::{Result, bail};
use utils::fs::{create_directory, create_file};

#[cfg(test)]
use utils::fs::read_file;

use crate::prompt::ask_url;

const CONFIG: &str = r#"
base_url = "%BASE_URL%"
title = "%PROJECT_TITLE%"
description = "Starter Ansorum project for an answer-first knowledge corpus."
generate_feeds = false
generate_sitemap = false
generate_robots_txt = false
build_search_index = false

[ansorum.redirects]
external_host_allowlist = ["docs.example.com"]

[[ansorum.redirects.routes]]
code = "sales-demo"
target = "https://docs.example.com/demo"

[[ansorum.redirects.routes]]
code = "billing-portal"
target = "/cancel/"

[ansorum.packs]
auto_entity_packs = false
auto_audience_packs = true

[[ansorum.packs.curated]]
name = "billing"
source = "collections/packs/billing.toml"

[ansorum.eval]
enabled = false
model = "gpt-5.4-mini"
prompt_version = "starter-v1"
"#;

const README: &str = r#"# %PROJECT_TITLE%

This project was scaffolded by `ansorum init`.

It includes:

- answer-first starter content in `content/`
- a JSON-LD sidecar example at `content/refunds.schema.json`
- a curated pack definition in `collections/packs/billing.toml`
- deterministic eval fixtures in `eval/fixtures.yaml`

Run the full workflow:

```bash
ansorum build
ansorum serve
ansorum audit
ansorum eval
```

Use `ansorum eval --llm` only when `OPENAI_API_KEY` is set and you want OpenAI
Responses API rubric scoring.
"#;

const REFUNDS: &str = r#"+++
title = "Refund policy"

id = "refunds-policy"
summary = "How refunds work, who qualifies, and when payment returns land."
canonical_questions = ["how do refunds work", "can i get a refund"]
intent = "policy"
entity = "billing"
audience = "customer"
related = ["cancel-subscription"]
external_refs = ["https://example.com/refunds"]
schema_type = "FAQPage"
review_by = 2026-06-01
visibility = "public"
ai_visibility = "public"
llms_priority = "core"
token_budget = "medium"
retrieval_aliases = ["refund policy", "refund rules"]
+++

Refund details for customers.

## Eligibility

Refunds follow the [billing policy](https://example.com/policy).
"#;

const CANCEL: &str = r#"+++
title = "Cancel a subscription"

id = "cancel-subscription"
summary = "How to cancel a subscription and what happens after."
canonical_questions = ["how do i cancel my subscription"]
intent = "task"
entity = "billing"
audience = "customer"
related = ["refunds-policy"]
external_refs = []
schema_type = "HowTo"
visibility = "public"
ai_visibility = "summary_only"
llms_priority = "optional"
token_budget = "small"
retrieval_aliases = ["cancel subscription"]
+++

Cancellation details for customers.

## Keep access

Use the [billing portal](https://example.com/billing) to manage changes.
"#;

const INTERNAL_PLAYBOOK: &str = r#"+++
title = "Internal support escalation"

id = "internal-support-escalation"
summary = "Internal escalation process for complex billing cases."
canonical_questions = ["how do support agents escalate billing issues"]
intent = "reference"
entity = "support"
audience = "internal"
related = []
external_refs = []
schema_type = "Article"
visibility = "internal"
ai_visibility = "hidden"
llms_priority = "hidden"
token_budget = "small"
retrieval_aliases = ["billing escalation playbook"]
+++

Escalation details for internal teams only.
"#;

const REFUNDS_SCHEMA: &str = r#"{
  "publisher": {
    "@type": "Organization",
    "name": "Ansorum Billing"
  },
  "mainEntity": [
    {
      "@type": "Question",
      "name": "Who qualifies for a refund?",
      "acceptedAnswer": {
        "@type": "Answer",
        "text": "Most unused subscriptions are refundable within 30 days."
      }
    }
  ]
}
"#;

const BILLING_PACK: &str = r#"title = "Billing answers"
description = "Curated billing pack for customer-visible billing answers."
answers = ["refunds-policy", "cancel-subscription"]
"#;

const EVAL_FIXTURES: &str = r#"- question: can i get a refund after 30 days
  expected_ids: [refunds-policy]
  forbidden_ids: [internal-support-escalation]
  required_terms: [eligibility, billing policy]
  rubric_focus: reflect refund eligibility policy without using internal-only content

- question: how do i cancel my subscription
  expected_ids: [cancel-subscription]
  forbidden_ids: [internal-support-escalation]
  required_terms: [canonical page, cancel]
  rubric_focus: prefer the public cancellation answer and keep canonical links visible
"#;

// canonicalize(path) function on windows system returns a path with UNC.
// Example: \\?\C:\Users\VssAdministrator\AppData\Local\Temp\new_project
// More details on Universal Naming Convention (UNC):
// https://en.wikipedia.org/wiki/Path_(computing)#Uniform_Naming_Convention
// So the following const will be used to remove the network part of the UNC to display users a more common
// path on windows systems.
// This is a workaround until this issue https://github.com/rust-lang/rust/issues/42869 was fixed.
const LOCAL_UNC: &str = "\\\\?\\";

// Given a path, return true if it is a directory and it doesn't have any
// non-hidden files, otherwise return false (path is assumed to exist)
pub fn is_directory_quasi_empty(path: &Path) -> Result<bool> {
    if path.is_dir() {
        let mut entries = match path.read_dir() {
            Ok(entries) => entries,
            Err(e) => {
                bail!(
                    "Could not read `{}` because of error: {}",
                    path.to_string_lossy().to_string(),
                    e
                );
            }
        };
        // If any entry raises an error or isn't hidden (i.e. starts with `.`), we raise an error
        if entries.any(|x| match x {
            Ok(file) => !file
                .file_name()
                .to_str()
                .expect("Could not convert filename to &str")
                .starts_with('.'),
            Err(_) => true,
        }) {
            return Ok(false);
        }
        return Ok(true);
    }

    Ok(false)
}

// Remove the unc part of a windows path
fn strip_unc(path: &Path) -> String {
    let path_to_refine = path.to_str().unwrap();
    path_to_refine.trim_start_matches(LOCAL_UNC).to_string()
}

pub fn create_new_project(name: &str, force: bool) -> Result<()> {
    let path = Path::new(name);

    // Better error message than the rust default
    if path.exists() && !is_directory_quasi_empty(path)? && !force {
        if name == "." {
            bail!("The current directory is not an empty folder (hidden files are ignored).");
        } else {
            bail!(
                "`{}` is not an empty folder (hidden files are ignored).",
                path.to_string_lossy().to_string()
            )
        }
    }

    console::info("Welcome to Ansorum!");
    console::info(
        "This scaffold creates an answer-first project with starter content, packs, redirects, and eval fixtures.",
    );
    console::info("Any choices made can be changed by modifying the generated files later.");

    let base_url = ask_url("> What is the URL of your site?", "https://example.com")?;
    let project_title = project_title(path);

    let config = CONFIG
        .trim_start()
        .replace("%BASE_URL%", &base_url)
        .replace("%PROJECT_TITLE%", &project_title);

    populate(path, &project_title, &config)?;

    println!();
    console::success(&format!(
        "Done! Your answer-first project was created in {}",
        strip_unc(&canonicalize(path).unwrap())
    ));
    println!();
    console::info(
        "Next steps: `ansorum build`, `ansorum serve`, `ansorum audit`, and `ansorum eval`.",
    );
    println!("Visit https://ansorum.com/documentation/ for the full documentation.");
    Ok(())
}

fn populate(path: &Path, project_title: &str, config: &str) -> Result<()> {
    if !path.exists() {
        create_dir(path)?;
    }

    create_file(&path.join("config.toml"), config)?;
    create_file(&path.join("README.md"), README.replace("%PROJECT_TITLE%", project_title))?;

    create_directory(&path.join("collections/packs"))?;
    create_directory(&path.join("content"))?;
    create_directory(&path.join("eval"))?;
    create_directory(&path.join("static"))?;

    create_file(&path.join("collections/packs/billing.toml"), BILLING_PACK)?;
    create_file(&path.join("content/refunds.md"), REFUNDS)?;
    create_file(&path.join("content/cancel.md"), CANCEL)?;
    create_file(&path.join("content/internal-playbook.md"), INTERNAL_PLAYBOOK)?;
    create_file(&path.join("content/refunds.schema.json"), REFUNDS_SCHEMA)?;
    create_file(&path.join("eval/fixtures.yaml"), EVAL_FIXTURES)?;

    Ok(())
}

fn project_title(path: &Path) -> String {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty() && *name != ".")
        .map(ToOwned::to_owned)
        .or_else(|| {
            canonicalize(path)
                .ok()
                .as_deref()
                .and_then(Path::file_name)
                .and_then(|name| name.to_str())
                .map(ToOwned::to_owned)
        })
        .or_else(|| current_dir_name())
        .unwrap_or_else(|| "ansorum-answers".to_string());

    title_case_slug(&file_name)
}

fn current_dir_name() -> Option<String> {
    canonicalize(PathBuf::from("."))
        .ok()
        .as_deref()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
}

fn title_case_slug(input: &str) -> String {
    let words: Vec<String> = input
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    format!("{}{}", first.to_ascii_uppercase(), chars.as_str().to_ascii_lowercase())
                }
                None => String::new(),
            }
        })
        .filter(|part| !part.is_empty())
        .collect();

    if words.is_empty() { "Ansorum Answers".to_string() } else { words.join(" ") }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;
    use std::fs::{create_dir, remove_dir, remove_dir_all};
    use std::path::Path;

    #[test]
    fn init_empty_directory() {
        let mut dir = temp_dir();
        dir.push("test_empty_dir");
        if dir.exists() {
            remove_dir_all(&dir).expect("Could not free test directory");
        }
        create_dir(&dir).expect("Could not create test directory");
        let allowed = is_directory_quasi_empty(&dir)
            .expect("An error happened reading the directory's contents");
        remove_dir(&dir).unwrap();
        assert!(allowed);
    }

    #[test]
    fn init_non_empty_directory() {
        let mut dir = temp_dir();
        dir.push("test_non_empty_dir");
        if dir.exists() {
            remove_dir_all(&dir).expect("Could not free test directory");
        }
        create_dir(&dir).expect("Could not create test directory");
        let mut content = dir.clone();
        content.push("content");
        create_dir(&content).unwrap();
        let allowed = is_directory_quasi_empty(&dir)
            .expect("An error happened reading the directory's contents");
        remove_dir(&content).unwrap();
        remove_dir(&dir).unwrap();
        assert!(!allowed);
    }

    #[test]
    fn init_quasi_empty_directory() {
        let mut dir = temp_dir();
        dir.push("test_quasi_empty_dir");
        if dir.exists() {
            remove_dir_all(&dir).expect("Could not free test directory");
        }
        create_dir(&dir).expect("Could not create test directory");
        let mut git = dir.clone();
        git.push(".git");
        create_dir(&git).unwrap();
        let allowed = is_directory_quasi_empty(&dir)
            .expect("An error happened reading the directory's contents");
        remove_dir(&git).unwrap();
        remove_dir(&dir).unwrap();
        assert!(allowed);
    }

    #[test]
    fn populate_existing_directory() {
        let mut dir = temp_dir();
        dir.push("test_existing_dir");
        if dir.exists() {
            remove_dir_all(&dir).expect("Could not free test directory");
        }
        create_dir(&dir).expect("Could not create test directory");
        let config = CONFIG
            .trim_start()
            .replace("%BASE_URL%", "https://example.com")
            .replace("%PROJECT_TITLE%", "Test Existing Dir");
        populate(&dir, "Test Existing Dir", &config)
            .expect("Could not populate ansorum directories");

        assert!(dir.join("config.toml").exists());
        assert!(dir.join("README.md").exists());
        assert!(dir.join("collections/packs/billing.toml").exists());
        assert!(dir.join("content/refunds.md").exists());
        assert!(dir.join("content/cancel.md").exists());
        assert!(dir.join("content/internal-playbook.md").exists());
        assert!(dir.join("content/refunds.schema.json").exists());
        assert!(dir.join("eval/fixtures.yaml").exists());
        assert!(dir.join("static").exists());
        assert!(dir.join("content").exists());
        assert!(read_file(&dir.join("config.toml")).unwrap().contains("[ansorum.redirects]"));
        assert!(read_file(&dir.join("eval/fixtures.yaml")).unwrap().contains("expected_ids"));

        remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn populate_non_existing_directory() {
        let mut dir = temp_dir();
        dir.push("test_non_existing_dir");
        if dir.exists() {
            remove_dir_all(&dir).expect("Could not free test directory");
        }
        let config = CONFIG
            .trim_start()
            .replace("%BASE_URL%", "https://example.com")
            .replace("%PROJECT_TITLE%", "Test Non Existing Dir");
        populate(&dir, "Test Non Existing Dir", &config)
            .expect("Could not populate ansorum directories");

        assert!(dir.exists());
        assert!(dir.join("config.toml").exists());
        assert!(dir.join("README.md").exists());
        assert!(dir.join("collections/packs/billing.toml").exists());
        assert!(dir.join("content").exists());
        assert!(dir.join("content/refunds.md").exists());
        assert!(dir.join("content/refunds.schema.json").exists());
        assert!(dir.join("eval/fixtures.yaml").exists());

        remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn project_title_uses_directory_name() {
        assert_eq!(project_title(Path::new("customer-answers")), "Customer Answers");
        assert_eq!(title_case_slug("internal_support"), "Internal Support");
    }

    #[test]
    fn strip_unc_test() {
        let mut dir = temp_dir();
        dir.push("new_project1");
        if dir.exists() {
            remove_dir_all(&dir).expect("Could not free test directory");
        }
        create_dir(&dir).expect("Could not create test directory");
        if cfg!(target_os = "windows") {
            let stripped_path = strip_unc(&canonicalize(Path::new(&dir)).unwrap());
            assert!(same_file::is_same_file(Path::new(&stripped_path), &dir).unwrap());
            assert!(!stripped_path.starts_with(LOCAL_UNC), "The path was not stripped.");
        } else {
            assert_eq!(
                strip_unc(&canonicalize(Path::new(&dir)).unwrap()),
                canonicalize(Path::new(&dir)).unwrap().to_str().unwrap().to_string()
            );
        }

        remove_dir_all(&dir).unwrap();
    }

    // If the following test fails it means that the canonicalize function is fixed and strip_unc
    // function/workaround is not anymore required.
    // See issue https://github.com/rust-lang/rust/issues/42869 as a reference.
    #[test]
    #[cfg(target_os = "windows")]
    fn strip_unc_required_test() {
        let mut dir = temp_dir();
        dir.push("new_project2");
        if dir.exists() {
            remove_dir_all(&dir).expect("Could not free test directory");
        }
        create_dir(&dir).expect("Could not create test directory");

        let canonicalized_path = canonicalize(Path::new(&dir)).unwrap();
        assert!(same_file::is_same_file(Path::new(&canonicalized_path), &dir).unwrap());
        assert!(canonicalized_path.to_str().unwrap().starts_with(LOCAL_UNC));

        remove_dir_all(&dir).unwrap();
    }
}
