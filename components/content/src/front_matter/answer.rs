use serde::Deserialize;
use tera::{Map, Value, from_value};

use errors::{Result, bail};
use utils::de::{fix_toml_dates, from_unknown_datetime};

use crate::answer::{
    AiVisibility, AnswerAudience, AnswerFrontMatter, AnswerIntent, AnswerVisibility,
    LlmsPriority, TokenBudget,
};

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RawAnswerFrontMatter {
    pub id: Option<String>,
    pub summary: Option<String>,
    pub canonical_questions: Vec<String>,
    pub intent: Option<AnswerIntent>,
    pub entity: Option<String>,
    pub audience: Option<AnswerAudience>,
    pub related: Vec<String>,
    pub external_refs: Vec<String>,
    pub schema_type: Option<String>,
    #[serde(default, deserialize_with = "from_unknown_datetime")]
    pub review_by: Option<String>,
    pub priority: Option<String>,
    pub visibility: Option<AnswerVisibility>,
    pub ai_visibility: Option<AiVisibility>,
    pub llms_priority: Option<LlmsPriority>,
    pub token_budget: Option<TokenBudget>,
    pub aliases: Vec<String>,
    pub ai_extra: Option<String>,
    pub last_reviewed_by: Option<String>,
    pub owner: Option<String>,
    pub confidence_notes: Option<String>,
}

impl RawAnswerFrontMatter {
    pub fn from_extra(extra: &mut Map<String, Value>) -> Result<Self> {
        let Some(raw_answer) = extra.remove("answer") else {
            return Ok(Self::default());
        };

        let normalized = match raw_answer {
            Value::Object(table) => fix_toml_dates(table),
            other => other,
        };
        Ok(from_value(normalized)?)
    }

    pub fn merge(self, fallback: Self) -> Self {
        Self {
            id: self.id.or(fallback.id),
            summary: self.summary.or(fallback.summary),
            canonical_questions: if self.canonical_questions.is_empty() {
                fallback.canonical_questions
            } else {
                self.canonical_questions
            },
            intent: self.intent.or(fallback.intent),
            entity: self.entity.or(fallback.entity),
            audience: self.audience.or(fallback.audience),
            related: if self.related.is_empty() { fallback.related } else { self.related },
            external_refs: if self.external_refs.is_empty() {
                fallback.external_refs
            } else {
                self.external_refs
            },
            schema_type: self.schema_type.or(fallback.schema_type),
            review_by: self.review_by.or(fallback.review_by),
            priority: self.priority.or(fallback.priority),
            visibility: self.visibility.or(fallback.visibility),
            ai_visibility: self.ai_visibility.or(fallback.ai_visibility),
            llms_priority: self.llms_priority.or(fallback.llms_priority),
            token_budget: self.token_budget.or(fallback.token_budget),
            aliases: if self.aliases.is_empty() { fallback.aliases } else { self.aliases },
            ai_extra: self.ai_extra.or(fallback.ai_extra),
            last_reviewed_by: self.last_reviewed_by.or(fallback.last_reviewed_by),
            owner: self.owner.or(fallback.owner),
            confidence_notes: self.confidence_notes.or(fallback.confidence_notes),
        }
    }

    pub fn has_data(&self) -> bool {
        self.id.is_some()
            || self.summary.is_some()
            || !self.canonical_questions.is_empty()
            || self.intent.is_some()
            || self.entity.is_some()
            || self.audience.is_some()
            || !self.related.is_empty()
            || !self.external_refs.is_empty()
            || self.schema_type.is_some()
            || self.review_by.is_some()
            || self.priority.is_some()
            || self.visibility.is_some()
            || self.ai_visibility.is_some()
            || self.llms_priority.is_some()
            || self.token_budget.is_some()
            || !self.aliases.is_empty()
            || self.ai_extra.is_some()
            || self.last_reviewed_by.is_some()
            || self.owner.is_some()
            || self.confidence_notes.is_some()
    }

    pub fn into_answer(self, title: &Option<String>) -> Result<Option<AnswerFrontMatter>> {
        if !self.has_data() {
            return Ok(None);
        }

        let id = required_string(self.id, "id")?;
        let summary = required_string(self.summary, "summary")?;
        let entity = required_string(self.entity, "entity")?;
        let intent = required_field(self.intent, "intent")?;
        let audience = required_field(self.audience, "audience")?;
        let visibility = required_field(self.visibility, "visibility")?;
        let ai_visibility = required_field(self.ai_visibility, "ai_visibility")?;
        let llms_priority = required_field(self.llms_priority, "llms_priority")?;
        let token_budget = required_field(self.token_budget, "token_budget")?;

        if title.as_ref().is_none_or(|value| value.trim().is_empty())
            && self.canonical_questions.is_empty()
        {
            bail!("An answer page requires at least one of `title` or `canonical_questions`");
        }

        let canonical_questions =
            non_empty_vec(self.canonical_questions, "canonical_questions")?;
        let related = non_empty_vec(self.related, "related")?;
        let external_refs = non_empty_vec(self.external_refs, "external_refs")?;
        let aliases = non_empty_vec(self.aliases, "aliases")?;

        Ok(Some(AnswerFrontMatter {
            id,
            summary,
            canonical_questions,
            intent,
            entity,
            audience,
            related,
            external_refs,
            schema_type: optional_string(self.schema_type, "schema_type")?,
            review_by: self.review_by,
            priority: optional_string(self.priority, "priority")?,
            visibility,
            ai_visibility,
            llms_priority,
            token_budget,
            aliases,
            ai_extra: optional_string(self.ai_extra, "ai_extra")?,
            last_reviewed_by: optional_string(self.last_reviewed_by, "last_reviewed_by")?,
            owner: optional_string(self.owner, "owner")?,
            confidence_notes: optional_string(self.confidence_notes, "confidence_notes")?,
        }))
    }
}

fn required_field<T>(value: Option<T>, field: &str) -> Result<T> {
    value.ok_or_else(|| errors::Error::msg(format!("An answer page requires `{field}`")))
}

fn required_string(value: Option<String>, field: &str) -> Result<String> {
    let value = required_field(value, field)?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        bail!("`{field}` cannot be empty");
    }

    Ok(trimmed.to_string())
}

fn optional_string(value: Option<String>, field: &str) -> Result<Option<String>> {
    match value {
        Some(value) => Ok(Some(required_string(Some(value), field)?)),
        None => Ok(None),
    }
}

fn non_empty_vec(values: Vec<String>, field: &str) -> Result<Vec<String>> {
    values
        .into_iter()
        .map(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                bail!("`{field}` entries cannot be empty");
            }

            Ok(trimmed.to_string())
        })
        .collect()
}
