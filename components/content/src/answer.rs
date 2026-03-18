use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnswerIntent {
    Concept,
    Task,
    Policy,
    Troubleshooting,
    Comparison,
    Pricing,
    Integration,
    Faq,
    Reference,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnswerAudience {
    Customer,
    Prospect,
    Developer,
    Admin,
    Internal,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnswerVisibility {
    Public,
    Private,
    Internal,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiVisibility {
    Public,
    Hidden,
    SummaryOnly,
}

pub fn is_machine_ai_visible(visibility: &AnswerVisibility, ai_visibility: &AiVisibility) -> bool {
    matches!(visibility, AnswerVisibility::Public) && !matches!(ai_visibility, AiVisibility::Hidden)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmsPriority {
    Core,
    Optional,
    Hidden,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenBudget {
    Small,
    Medium,
    Full,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnswerFrontMatter {
    pub id: String,
    pub summary: String,
    pub canonical_questions: Vec<String>,
    pub intent: AnswerIntent,
    pub entity: String,
    pub audience: AnswerAudience,
    pub related: Vec<String>,
    pub external_refs: Vec<String>,
    pub schema_type: Option<String>,
    pub review_by: Option<String>,
    pub priority: Option<String>,
    pub visibility: AnswerVisibility,
    pub ai_visibility: AiVisibility,
    pub llms_priority: LlmsPriority,
    pub token_budget: TokenBudget,
    pub aliases: Vec<String>,
    pub ai_extra: Option<String>,
    pub last_reviewed_by: Option<String>,
    pub owner: Option<String>,
    pub confidence_notes: Option<String>,
}

impl AnswerFrontMatter {
    pub fn is_machine_ai_visible(&self) -> bool {
        is_machine_ai_visible(&self.visibility, &self.ai_visibility)
    }
}
