use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::PathBuf;

use chrono::NaiveDate;
use content::{
    AiVisibility, AnswerAudience, AnswerIntent, AnswerVisibility, Library, LlmsPriority, Page,
    TokenBudget, is_machine_ai_visible,
};
use errors::{Result, anyhow};
use serde::Serialize;
use serde_json::{Map as JsonMap, Value as JsonValue, json};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnswerRecord {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub canonical_url: String,
    pub markdown_url: String,
    pub intent: AnswerIntent,
    pub entity: String,
    pub audience: AnswerAudience,
    pub canonical_questions: Vec<String>,
    pub retrieval_aliases: Vec<String>,
    pub related: Vec<String>,
    pub visibility: AnswerVisibility,
    pub ai_visibility: AiVisibility,
    pub llms_priority: LlmsPriority,
    pub token_budget: TokenBudget,
    pub review_by: Option<String>,
    pub last_modified: Option<String>,
    pub source_path: PathBuf,
}

#[derive(Clone, Debug, Default)]
pub struct AnswerCorpus {
    records: Vec<AnswerRecord>,
    by_id: HashMap<String, usize>,
    by_entity: BTreeMap<String, Vec<usize>>,
    by_intent: BTreeMap<String, Vec<usize>>,
    by_audience: BTreeMap<String, Vec<usize>>,
}

impl AnswerCorpus {
    pub fn from_library(library: &Library) -> Result<Self> {
        let mut records = library
            .pages
            .values()
            .filter_map(AnswerRecord::from_page)
            .collect::<Vec<_>>();
        records.sort_by(|left, right| {
            left.id.cmp(&right.id).then(left.source_path.cmp(&right.source_path))
        });

        let mut errors = Vec::new();
        let mut id_sources = BTreeMap::<String, Vec<PathBuf>>::new();
        let mut question_sources = BTreeMap::<String, Vec<(String, PathBuf)>>::new();
        let mut alias_sources = BTreeMap::<String, Vec<(String, PathBuf)>>::new();

        for record in &records {
            id_sources.entry(record.id.clone()).or_default().push(record.source_path.clone());

            for duplicate in duplicate_entries(&record.canonical_questions) {
                errors.push(format!(
                    "{}: `canonical_questions` contains duplicate entry `{duplicate}`",
                    record.source_path.display()
                ));
            }
            for duplicate in duplicate_entries(&record.retrieval_aliases) {
                errors.push(format!(
                    "{}: `retrieval_aliases` contains duplicate entry `{duplicate}`",
                    record.source_path.display()
                ));
            }
            for duplicate in duplicate_entries(&record.related) {
                errors.push(format!(
                    "{}: `related` contains duplicate entry `{duplicate}`",
                    record.source_path.display()
                ));
            }

            for question in &record.canonical_questions {
                question_sources
                    .entry(normalize_key(question))
                    .or_default()
                    .push((question.clone(), record.source_path.clone()));
            }
            for alias in &record.retrieval_aliases {
                alias_sources
                    .entry(normalize_key(alias))
                    .or_default()
                    .push((alias.clone(), record.source_path.clone()));
            }
        }

        for (id, sources) in id_sources {
            if sources.len() > 1 {
                errors.push(format!(
                    "Duplicate answer id `{id}` found in {}",
                    format_paths(&sources)
                ));
            }
        }
        for sources in question_sources.into_values() {
            if sources.len() > 1 {
                let question = sources[0].0.clone();
                let paths = sources.into_iter().map(|(_, path)| path).collect::<Vec<_>>();
                errors.push(format!(
                    "Duplicate canonical question `{question}` found in {}",
                    format_paths(&paths)
                ));
            }
        }
        for sources in alias_sources.into_values() {
            if sources.len() > 1 {
                let alias = sources[0].0.clone();
                let paths = sources.into_iter().map(|(_, path)| path).collect::<Vec<_>>();
                errors.push(format!(
                    "Duplicate retrieval alias `{alias}` found in {}",
                    format_paths(&paths)
                ));
            }
        }

        let mut by_id = HashMap::new();
        let mut by_entity = BTreeMap::new();
        let mut by_intent = BTreeMap::new();
        let mut by_audience = BTreeMap::new();

        for (index, record) in records.iter().enumerate() {
            by_id.insert(record.id.clone(), index);
            by_entity.entry(record.entity.clone()).or_insert_with(Vec::new).push(index);
            by_intent
                .entry(intent_key(&record.intent))
                .or_insert_with(Vec::new)
                .push(index);
            by_audience
                .entry(audience_key(&record.audience))
                .or_insert_with(Vec::new)
                .push(index);
        }

        for record in &records {
            for related_id in &record.related {
                if related_id == &record.id {
                    errors.push(format!(
                        "{}: `related` cannot reference the answer itself (`{related_id}`)",
                        record.source_path.display()
                    ));
                    continue;
                }

                if !by_id.contains_key(related_id) {
                    errors.push(format!(
                        "{}: `related` references unknown answer id `{related_id}`",
                        record.source_path.display()
                    ));
                }
            }
        }

        if !errors.is_empty() {
            errors.sort();
            return Err(anyhow!("Answer validation failed:\n- {}", errors.join("\n- ")));
        }

        Ok(Self { records, by_id, by_entity, by_intent, by_audience })
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &AnswerRecord> {
        self.records.iter()
    }

    pub fn get(&self, id: &str) -> Option<&AnswerRecord> {
        self.by_id.get(id).map(|index| &self.records[*index])
    }

    pub fn same_entity(&self, entity: &str) -> Vec<&AnswerRecord> {
        self.by_entity
            .get(entity)
            .into_iter()
            .flat_map(|indexes| indexes.iter().map(|index| &self.records[*index]))
            .collect()
    }

    pub fn same_intent(&self, intent: &AnswerIntent) -> Vec<&AnswerRecord> {
        self.by_intent
            .get(&intent_key(intent))
            .into_iter()
            .flat_map(|indexes| indexes.iter().map(|index| &self.records[*index]))
            .collect()
    }

    pub fn same_audience(&self, audience: &AnswerAudience) -> Vec<&AnswerRecord> {
        self.by_audience
            .get(&audience_key(audience))
            .into_iter()
            .flat_map(|indexes| indexes.iter().map(|index| &self.records[*index]))
            .collect()
    }

    pub fn related_to(&self, id: &str) -> Vec<&AnswerRecord> {
        self.get(id)
            .into_iter()
            .flat_map(|record| record.related.iter())
            .filter_map(|related_id| self.get(related_id))
            .collect()
    }

    pub fn to_json(&self) -> Result<String> {
        let visible = self
            .records
            .iter()
            .filter(|record| record.is_machine_ai_visible())
            .collect::<Vec<_>>();
        self.to_json_subset(&visible)
    }

    pub fn to_json_subset(&self, records: &[&AnswerRecord]) -> Result<String> {
        let visible_ids = records.iter().map(|record| record.id.as_str()).collect::<HashSet<_>>();
        let document = AnswersIndex {
            version: 1,
            generated_at: None,
            answers: records
                .iter()
                .copied()
                .map(|record| SerializedAnswerRecord::from_record(record, &visible_ids))
                .collect(),
        };

        serde_json::to_string_pretty(&document)
            .map_err(|error| anyhow!("Failed to serialize answers.json: {error}"))
    }
}

impl AnswerRecord {
    fn from_page(page: &Page) -> Option<Self> {
        let answer = page.answer()?;
        Some(Self {
            id: answer.id.clone(),
            title: page.answer_title().to_string(),
            summary: answer.summary.clone(),
            canonical_url: page.permalink.clone(),
            markdown_url: markdown_url_from_page(page),
            intent: answer.intent.clone(),
            entity: answer.entity.clone(),
            audience: answer.audience.clone(),
            canonical_questions: answer.canonical_questions.clone(),
            retrieval_aliases: answer.retrieval_aliases.clone(),
            related: answer.related.clone(),
            visibility: answer.visibility.clone(),
            ai_visibility: answer.ai_visibility.clone(),
            llms_priority: answer.llms_priority.clone(),
            token_budget: answer.token_budget.clone(),
            review_by: answer.review_by.clone(),
            last_modified: page.meta.updated.clone().or_else(|| page.meta.date.clone()),
            source_path: page.file.path.clone(),
        })
    }

    pub fn is_machine_ai_visible(&self) -> bool {
        is_machine_ai_visible(&self.visibility, &self.ai_visibility)
    }
}

#[derive(Serialize)]
struct AnswersIndex<'a> {
    version: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    generated_at: Option<&'a str>,
    answers: Vec<SerializedAnswerRecord<'a>>,
}

#[derive(Serialize)]
struct SerializedAnswerRecord<'a> {
    id: &'a str,
    title: &'a str,
    summary: &'a str,
    canonical_url: &'a str,
    markdown_url: &'a str,
    entity: &'a str,
    intent: &'a AnswerIntent,
    audience: &'a AnswerAudience,
    related: Vec<&'a str>,
    canonical_questions: &'a [String],
    retrieval_aliases: &'a [String],
    #[serde(skip_serializing_if = "Option::is_none")]
    review_by: Option<&'a str>,
    llms_priority: &'a LlmsPriority,
    token_budget: &'a TokenBudget,
    visibility: &'a AnswerVisibility,
    ai_visibility: &'a AiVisibility,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_modified: Option<&'a str>,
}

impl<'a> SerializedAnswerRecord<'a> {
    fn from_record(record: &'a AnswerRecord, visible_ids: &HashSet<&str>) -> Self {
        Self {
            id: &record.id,
            title: &record.title,
            summary: &record.summary,
            canonical_url: &record.canonical_url,
            markdown_url: &record.markdown_url,
            entity: &record.entity,
            intent: &record.intent,
            audience: &record.audience,
            related: record
                .related
                .iter()
                .filter(|related_id| visible_ids.contains(related_id.as_str()))
                .map(String::as_str)
                .collect(),
            canonical_questions: &record.canonical_questions,
            retrieval_aliases: &record.retrieval_aliases,
            review_by: record.review_by.as_deref(),
            llms_priority: &record.llms_priority,
            token_budget: &record.token_budget,
            visibility: &record.visibility,
            ai_visibility: &record.ai_visibility,
            last_modified: record.last_modified.as_deref(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructuredDataOutput {
    pub json: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditSeverity {
    Error,
    Warn,
    Info,
}

impl AuditSeverity {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AuditFinding {
    pub severity: AuditSeverity,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answer_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct AuditSummary {
    pub errors: usize,
    pub warnings: usize,
    pub infos: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct AuditReport {
    pub summary: AuditSummary,
    pub findings: Vec<AuditFinding>,
}

impl AuditReport {
    pub fn push(
        &mut self,
        severity: AuditSeverity,
        code: impl Into<String>,
        message: impl Into<String>,
        answer_id: Option<&str>,
        source_path: Option<&std::path::Path>,
    ) {
        match severity {
            AuditSeverity::Error => self.summary.errors += 1,
            AuditSeverity::Warn => self.summary.warnings += 1,
            AuditSeverity::Info => self.summary.infos += 1,
        }

        self.findings.push(AuditFinding {
            severity,
            code: code.into(),
            message: message.into(),
            answer_id: answer_id.map(ToOwned::to_owned),
            source_path: source_path.map(|path| path.display().to_string()),
        });
    }

    pub fn has_errors(&self) -> bool {
        self.summary.errors > 0
    }
}

pub fn audit_library(
    library: &Library,
    answers: &AnswerCorpus,
    today: NaiveDate,
) -> AuditReport {
    let mut report = AuditReport::default();
    let excluded_machine_ids = answers
        .iter()
        .filter(|record| !record.is_machine_ai_visible())
        .map(|record| record.id.as_str())
        .collect::<HashSet<_>>();

    for page in library.pages.values() {
        let Some(answer) = page.answer() else {
            continue;
        };

        let answer_id = Some(answer.id.as_str());
        let source_path = Some(page.file.path.as_path());

        if !answer.is_machine_ai_visible() && answer.ai_visibility != AiVisibility::Hidden
        {
            report.push(
                AuditSeverity::Error,
                "visibility_leak",
                format!(
                    "`visibility = {}` requires `ai_visibility = hidden` to avoid exposing non-public content",
                    visibility_key(&answer.visibility)
                ),
                answer_id,
                source_path,
            );
        }

        if answer.priority.as_deref() == Some("high") && answer.related.is_empty() {
            report.push(
                AuditSeverity::Error,
                "missing_related_links",
                "high-priority answers must define at least one related answer link",
                answer_id,
                source_path,
            );
        }

        if let Some(review_by) = answer.review_by.as_deref() {
            match NaiveDate::parse_from_str(review_by, "%Y-%m-%d") {
                Ok(date) if date < today => {
                    let severity = if answer.priority.as_deref() == Some("high") {
                        AuditSeverity::Error
                    } else {
                        AuditSeverity::Warn
                    };
                    report.push(
                        severity,
                        "stale_review_by",
                        format!("review date `{review_by}` is stale relative to {today}"),
                        answer_id,
                        source_path,
                    );
                }
                Ok(_) => {}
                Err(_) => {
                    report.push(
                        AuditSeverity::Error,
                        "invalid_review_by",
                        format!("review date `{review_by}` must use YYYY-MM-DD"),
                        answer_id,
                        source_path,
                    );
                }
            }
        } else if answer.priority.as_deref() == Some("high") {
            report.push(
                AuditSeverity::Warn,
                "missing_review_by",
                "high-priority answers should set `review_by` for freshness tracking",
                answer_id,
                source_path,
            );
        }

        let has_structured_data_authoring =
            answer.schema_type.is_some() || page.structured_data_sidecar.is_some();
        if !has_structured_data_authoring {
            report.push(
                AuditSeverity::Warn,
                "missing_json_ld_type",
                "answer pages should define `schema_type` or a structured-data sidecar",
                answer_id,
                source_path,
            );
        }

        match structured_data_for_page(page) {
            Ok(Some(_)) => {}
            Ok(None) => {}
            Err(error) => {
                report.push(
                    AuditSeverity::Error,
                    "structured_data_invalid",
                    error.to_string(),
                    answer_id,
                    source_path,
                );
            }
        }

        if let Some(markdown) = page.canonical_machine_markdown() {
            let estimated_tokens = estimate_tokens(&markdown);
            let token_limit = token_budget_limit(&answer.token_budget);
            if estimated_tokens > token_limit {
                let severity = if answer.token_budget == TokenBudget::Small {
                    AuditSeverity::Error
                } else {
                    AuditSeverity::Warn
                };
                report.push(
                    severity,
                    "token_budget_overflow",
                    format!(
                        "estimated machine markdown size {estimated_tokens} exceeds the `{}` budget threshold of {token_limit} tokens",
                        token_budget_key(&answer.token_budget)
                    ),
                    answer_id,
                    source_path,
                );
            }

            if answer.ai_visibility == AiVisibility::SummaryOnly
                && !markdown.contains(&format!("Canonical page: <{}>", page.permalink))
            {
                report.push(
                    AuditSeverity::Error,
                    "summary_only_leak",
                    "summary-only machine output is missing the canonical page pointer",
                    answer_id,
                    source_path,
                );
            }
        } else if answer.ai_visibility != AiVisibility::Hidden {
            report.push(
                AuditSeverity::Error,
                "missing_machine_output",
                "AI-visible answers must produce canonical machine markdown",
                answer_id,
                source_path,
            );
        }

        if answer.ai_visibility == AiVisibility::Hidden && page.canonical_machine_markdown().is_some() {
            report.push(
                AuditSeverity::Error,
                "hidden_content_leak",
                "hidden answers must not emit canonical machine markdown",
                answer_id,
                source_path,
            );
        }
    }

    for record in answers.iter() {
        let answer_id = Some(record.id.as_str());
        let source_path = Some(record.source_path.as_path());
        for related_id in &record.related {
            if excluded_machine_ids.contains(related_id.as_str()) {
                report.push(
                    AuditSeverity::Error,
                    "related_visibility_leak",
                    format!(
                        "`related` references answer `{related_id}` that is excluded from machine outputs, which would leak non-public graph metadata"
                    ),
                    answer_id,
                    source_path,
                );
            }
        }
    }

    report.findings.sort_by(|left, right| {
        severity_rank(&left.severity)
            .cmp(&severity_rank(&right.severity))
            .then(left.code.cmp(&right.code))
            .then(left.answer_id.cmp(&right.answer_id))
            .then(left.source_path.cmp(&right.source_path))
            .then(left.message.cmp(&right.message))
    });

    report
}

pub fn structured_data_for_page(page: &Page) -> Result<Option<StructuredDataOutput>> {
    if page.answer().is_none() {
        return Ok(None);
    }

    let mut document = match (structured_data_preset(page), page.structured_data_sidecar.as_ref()) {
        (Some(mut preset), Some(sidecar)) => {
            merge_json(&mut preset, sidecar.clone());
            preset
        }
        (Some(preset), None) => preset,
        (None, Some(sidecar)) => sidecar.clone(),
        (None, None) => return Ok(None),
    };

    let object = document.as_object_mut().expect("structured data sidecars are validated as objects");
    object
        .entry("@context".to_string())
        .or_insert_with(|| JsonValue::String("https://schema.org".to_string()));

    if !object.contains_key("@type") && !object.contains_key("@graph") {
        return Err(anyhow!(
            "{}: structured data must contain `@type` or `@graph`",
            page.file.path.display()
        ));
    }

    let json = serde_json::to_string_pretty(&document).map_err(|error| {
        anyhow!(
            "{}: failed to serialize structured data: {error}",
            page.file.path.display()
        )
    })?;

    Ok(Some(StructuredDataOutput { json }))
}

fn markdown_url_from_page(page: &Page) -> String {
    let trimmed = page.permalink.trim_end_matches('/');
    format!("{trimmed}/page.md")
}

fn structured_data_preset(page: &Page) -> Option<JsonValue> {
    let answer = page.answer()?;
    let schema_type = answer.schema_type.as_deref()?;

    let title = page.answer_title();
    let mut object = JsonMap::new();
    object.insert("@context".to_string(), JsonValue::String("https://schema.org".to_string()));
    object.insert("@type".to_string(), JsonValue::String(schema_type.to_string()));
    object.insert("url".to_string(), JsonValue::String(page.permalink.clone()));
    object.insert("description".to_string(), JsonValue::String(answer.summary.clone()));
    object.insert("identifier".to_string(), JsonValue::String(answer.id.clone()));
    object.insert(
        "inLanguage".to_string(),
        JsonValue::String(page.lang.clone()),
    );
    object.insert(
        "audience".to_string(),
        json!({
            "@type": "Audience",
            "audienceType": audience_key(&answer.audience),
        }),
    );
    object.insert(
        "about".to_string(),
        json!({
            "@type": "Thing",
            "name": answer.entity,
        }),
    );

    if !title.is_empty() {
        object.insert("name".to_string(), JsonValue::String(title.to_string()));
    }
    if let Some(description) = page.meta.description.as_ref().filter(|value| !value.trim().is_empty()) {
        object.insert("description".to_string(), JsonValue::String(description.clone()));
    }
    if let Some(updated) = page.meta.updated.as_ref().or(page.meta.date.as_ref()) {
        object.insert("dateModified".to_string(), JsonValue::String(updated.clone()));
    }
    if !answer.external_refs.is_empty() {
        object.insert(
            "sameAs".to_string(),
            JsonValue::Array(
                answer
                    .external_refs
                    .iter()
                    .cloned()
                    .map(JsonValue::String)
                    .collect(),
            ),
        );
    }
    if !answer.retrieval_aliases.is_empty() {
        object.insert(
            "alternateName".to_string(),
            JsonValue::Array(
                answer.retrieval_aliases.iter().cloned().map(JsonValue::String).collect(),
            ),
        );
    }

    match schema_type {
        "WebPage" | "Article" | "TechArticle" => {
            if !title.is_empty() {
                object.insert("headline".to_string(), JsonValue::String(title.to_string()));
            }
            object.insert("mainEntityOfPage".to_string(), JsonValue::String(page.permalink.clone()));
        }
        "FAQPage" => {
            object.insert(
                "mainEntity".to_string(),
                JsonValue::Array(
                    answer
                        .canonical_questions
                        .iter()
                        .map(|question| {
                            json!({
                                "@type": "Question",
                                "name": question,
                                "acceptedAnswer": {
                                    "@type": "Answer",
                                    "text": answer.summary,
                                }
                            })
                        })
                        .collect(),
                ),
            );
        }
        "DefinedTerm" => {
            if !title.is_empty() {
                object.insert("termCode".to_string(), JsonValue::String(answer.id.clone()));
                object.insert("name".to_string(), JsonValue::String(title.to_string()));
            }
            object.insert(
                "description".to_string(),
                JsonValue::String(answer.summary.clone()),
            );
        }
        "BreadcrumbList" => {
            let items = page
                .components
                .iter()
                .enumerate()
                .map(|(index, component)| {
                    let item_path = format!(
                        "{}/",
                        page.components[..=index].join("/")
                    );
                    json!({
                        "@type": "ListItem",
                        "position": index + 1,
                        "name": component,
                        "item": format!("{}{}", page.permalink.trim_end_matches(&page.path), item_path),
                    })
                })
                .collect::<Vec<_>>();
            object.insert("itemListElement".to_string(), JsonValue::Array(items));
        }
        _ => {}
    }

    Some(JsonValue::Object(object))
}

fn merge_json(base: &mut JsonValue, overlay: JsonValue) {
    match (base, overlay) {
        (JsonValue::Object(base), JsonValue::Object(overlay)) => {
            for (key, value) in overlay {
                match base.get_mut(&key) {
                    Some(existing) => merge_json(existing, value),
                    None => {
                        base.insert(key, value);
                    }
                }
            }
        }
        (base, overlay) => *base = overlay,
    }
}

fn normalize_key(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

fn duplicate_entries(values: &[String]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut duplicates = BTreeSet::new();
    for value in values {
        let normalized = normalize_key(value);
        if !seen.insert(normalized) {
            duplicates.insert(value.clone());
        }
    }

    duplicates.into_iter().collect()
}

fn format_paths(paths: &[PathBuf]) -> String {
    paths.iter().map(|path| path.display().to_string()).collect::<Vec<_>>().join(", ")
}

fn intent_key(intent: &AnswerIntent) -> String {
    match intent {
        AnswerIntent::Concept => "concept",
        AnswerIntent::Task => "task",
        AnswerIntent::Policy => "policy",
        AnswerIntent::Troubleshooting => "troubleshooting",
        AnswerIntent::Comparison => "comparison",
        AnswerIntent::Pricing => "pricing",
        AnswerIntent::Integration => "integration",
        AnswerIntent::Faq => "faq",
        AnswerIntent::Reference => "reference",
    }
    .to_string()
}

fn audience_key(audience: &AnswerAudience) -> String {
    match audience {
        AnswerAudience::Customer => "customer",
        AnswerAudience::Prospect => "prospect",
        AnswerAudience::Developer => "developer",
        AnswerAudience::Admin => "admin",
        AnswerAudience::Internal => "internal",
    }
    .to_string()
}

fn visibility_key(visibility: &AnswerVisibility) -> &'static str {
    match visibility {
        AnswerVisibility::Public => "public",
        AnswerVisibility::Private => "private",
        AnswerVisibility::Internal => "internal",
    }
}

fn token_budget_key(token_budget: &TokenBudget) -> &'static str {
    match token_budget {
        TokenBudget::Small => "small",
        TokenBudget::Medium => "medium",
        TokenBudget::Full => "full",
    }
}

fn token_budget_limit(token_budget: &TokenBudget) -> usize {
    match token_budget {
        TokenBudget::Small => 256,
        TokenBudget::Medium => 768,
        TokenBudget::Full => 2048,
    }
}

fn estimate_tokens(markdown: &str) -> usize {
    markdown.chars().count().div_ceil(4)
}

fn severity_rank(severity: &AuditSeverity) -> u8 {
    match severity {
        AuditSeverity::Error => 0,
        AuditSeverity::Warn => 1,
        AuditSeverity::Info => 2,
    }
}
