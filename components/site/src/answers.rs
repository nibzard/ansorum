use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;

use content::{
    AiVisibility, AnswerAudience, AnswerIntent, AnswerVisibility, Library, LlmsPriority, Page,
    TokenBudget,
};
use errors::{Result, anyhow};

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
    pub aliases: Vec<String>,
    pub related: Vec<String>,
    pub visibility: AnswerVisibility,
    pub ai_visibility: AiVisibility,
    pub llms_priority: LlmsPriority,
    pub token_budget: TokenBudget,
    pub review_by: Option<String>,
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
            for duplicate in duplicate_entries(&record.aliases) {
                errors.push(format!(
                    "{}: `aliases` contains duplicate entry `{duplicate}`",
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
            for alias in &record.aliases {
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
                    "Duplicate alias `{alias}` found in {}",
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
            aliases: answer.aliases.clone(),
            related: answer.related.clone(),
            visibility: answer.visibility.clone(),
            ai_visibility: answer.ai_visibility.clone(),
            llms_priority: answer.llms_priority.clone(),
            token_budget: answer.token_budget.clone(),
            review_by: answer.review_by.clone(),
            source_path: page.file.path.clone(),
        })
    }
}

fn markdown_url_from_page(page: &Page) -> String {
    let trimmed = page.permalink.trim_end_matches('/');
    format!("{trimmed}/page.md")
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
