use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use config::{Config, DEFAULT_EVAL_MODEL};
use content::AiVisibility;
use errors::{Error, Result, anyhow, bail};
use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};
use site::Site;

use crate::cli::EvalFormat;
use crate::observability::{self, DispatchMode};

const DEFAULT_FIXTURES_PATH: &str = "eval/fixtures.yaml";
const OPENAI_RESPONSES_URL: &str = "https://api.openai.com/v1/responses";
const OPENAI_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const OPENAI_REQUEST_TIMEOUT: Duration = Duration::from_secs(45);

pub fn eval(
    root_dir: &Path,
    config_file: &Path,
    include_drafts: bool,
    fixtures: &Path,
    format: EvalFormat,
    llm_override: bool,
    model_override: Option<&str>,
    api_base_override: Option<&str>,
    min_pass_rate: Option<f64>,
    min_llm_average: Option<f64>,
    min_llm_score: Option<f64>,
    require_llm: bool,
) -> Result<()> {
    let report = match run_eval(
        root_dir,
        config_file,
        include_drafts,
        fixtures,
        llm_override,
        model_override,
        api_base_override,
        min_pass_rate,
        min_llm_average,
        min_llm_score,
        require_llm,
    ) {
        Ok(report) => report,
        Err(error) => {
            emit_eval_failure(
                root_dir,
                config_file,
                include_drafts,
                fixtures,
                format,
                llm_override,
                model_override,
                api_base_override,
                min_pass_rate,
                min_llm_average,
                min_llm_score,
                require_llm,
                &error.to_string(),
            );
            return Err(error);
        }
    };

    emit_eval_report(root_dir, config_file, include_drafts, format, llm_override, &report);

    match format {
        EvalFormat::Human => print_human_report(&report),
        EvalFormat::Json => print_json_report(&report)?,
    }

    if report_failed(&report) {
        bail!("Eval failed");
    }

    Ok(())
}

fn run_eval(
    root_dir: &Path,
    config_file: &Path,
    include_drafts: bool,
    fixtures: &Path,
    llm_override: bool,
    model_override: Option<&str>,
    api_base_override: Option<&str>,
    min_pass_rate: Option<f64>,
    min_llm_average: Option<f64>,
    min_llm_score: Option<f64>,
    require_llm: bool,
) -> Result<EvalReport> {
    validate_ratio("min-pass-rate", min_pass_rate)?;
    validate_ratio("min-llm-average", min_llm_average)?;
    validate_ratio("min-llm-score", min_llm_score)?;

    let fixture_path = resolve_fixture_path(root_dir, fixtures);
    let fixture_file = load_fixture_file(&fixture_path)?;

    let mut site = Site::new(root_dir, config_file)?;
    if include_drafts {
        site.include_drafts();
    }
    site.load()?;

    let machine_markdown = machine_markdown_by_id(&site);

    let llm_enabled = llm_override || site.config.ansorum.eval.enabled;
    let model = resolve_model(&site.config, llm_enabled, model_override)?;
    let api_base = resolve_api_base(&site.config, api_base_override);

    let llm_client = if llm_enabled {
        Some(OpenAiEvalClient::new(&api_base, model.as_deref())?)
    } else {
        None
    };

    let mut report = EvalReport {
        fixture_path: fixture_path.display().to_string(),
        backend: if llm_enabled {
            Some("openai_responses".to_string())
        } else {
            None
        },
        model,
        prompt_version: site.config.ansorum.eval.prompt_version.clone(),
        summary: EvalSummary::default(),
        thresholds: EvalThresholds {
            min_pass_rate,
            min_llm_average,
            min_llm_score,
            require_llm,
        },
        cases: Vec::with_capacity(fixture_file.cases.len()),
    };

    for case in fixture_file.cases {
        let ranked = rank_answers(&case.question, &site);
        let ranked_ids = ranked.iter().map(|candidate| candidate.id.clone()).collect::<Vec<_>>();
        let missing_expected_ids = case
            .expected_ids
            .iter()
            .filter(|id| !ranked_ids.iter().any(|candidate| candidate == *id))
            .cloned()
            .collect::<Vec<_>>();
        let present_forbidden_ids = case
            .forbidden_ids
            .iter()
            .filter(|id| ranked_ids.iter().any(|candidate| candidate == *id))
            .cloned()
            .collect::<Vec<_>>();
        let retrieval_passed =
            missing_expected_ids.is_empty() && present_forbidden_ids.is_empty() && !ranked_ids.is_empty();

        let selected = ranked
            .iter()
            .find(|candidate| !case.forbidden_ids.iter().any(|id| id == &candidate.id))
            .cloned();
        let selected_markdown =
            selected.as_ref().and_then(|candidate| machine_markdown.get(&candidate.id)).cloned();
        let missing_required_terms = case
            .required_terms
            .iter()
            .filter(|term| {
                !selected_markdown
                    .as_deref()
                    .map(|markdown| contains_term(markdown, term))
                    .unwrap_or(false)
            })
            .cloned()
            .collect::<Vec<_>>();
        let selection_passed = selected
            .as_ref()
            .map(|candidate| {
                (case.expected_ids.is_empty() || case.expected_ids.contains(&candidate.id))
                    && missing_required_terms.is_empty()
            })
            .unwrap_or(false);

        let llm = if let Some(client) = llm_client.as_ref() {
            let selected_answer = selected
                .as_ref()
                .and_then(|candidate| site.answers.get(&candidate.id))
                .cloned();
            let expected_answers = case
                .expected_ids
                .iter()
                .filter_map(|id| site.answers.get(id))
                .cloned()
                .collect::<Vec<_>>();

            Some(client.grade_case(
                &report.prompt_version,
                &case,
                ranked.iter().take(5).cloned().collect(),
                selected_answer,
                selected_markdown,
                expected_answers,
                &machine_markdown,
            )?)
        } else {
            None
        };

        let case_report = EvalCaseReport {
            question: case.question,
            retrieval: RetrievalCaseReport {
                passed: retrieval_passed,
                ranked_ids,
                missing_expected_ids,
                present_forbidden_ids,
            },
            selection: SelectionCaseReport {
                passed: selection_passed,
                selected_id: selected.as_ref().map(|candidate| candidate.id.clone()),
                missing_required_terms,
            },
            llm,
            passed: false,
        };
        report.cases.push(case_report);
    }

    finalize_report(&mut report);
    Ok(report)
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvalCase {
    question: String,
    #[serde(default)]
    expected_ids: Vec<String>,
    #[serde(default)]
    forbidden_ids: Vec<String>,
    #[serde(default)]
    required_terms: Vec<String>,
    #[serde(default)]
    rubric_focus: Option<String>,
}

#[derive(Clone, Debug, Default)]
struct EvalFixtureFile {
    cases: Vec<EvalCase>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawFixtureFile {
    Cases(Vec<EvalCase>),
    Versioned { cases: Vec<EvalCase> },
}

#[derive(Clone, Debug)]
struct RankedAnswer {
    id: String,
    score: i32,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct EvalSummary {
    total_cases: usize,
    passed_cases: usize,
    retrieval_passed: usize,
    selection_passed: usize,
    llm_scored_cases: usize,
    llm_passed_cases: usize,
    llm_average: Option<f64>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct EvalThresholds {
    #[serde(skip_serializing_if = "Option::is_none")]
    min_pass_rate: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_llm_average: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_llm_score: Option<f64>,
    require_llm: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct EvalReport {
    fixture_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    backend: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    prompt_version: String,
    summary: EvalSummary,
    thresholds: EvalThresholds,
    cases: Vec<EvalCaseReport>,
}

#[derive(Clone, Debug, Serialize)]
pub struct EvalCaseReport {
    question: String,
    retrieval: RetrievalCaseReport,
    selection: SelectionCaseReport,
    #[serde(skip_serializing_if = "Option::is_none")]
    llm: Option<LlmCaseReport>,
    passed: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct RetrievalCaseReport {
    passed: bool,
    ranked_ids: Vec<String>,
    missing_expected_ids: Vec<String>,
    present_forbidden_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SelectionCaseReport {
    passed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    selected_id: Option<String>,
    missing_required_terms: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LlmCaseReport {
    relevance: f64,
    completeness: f64,
    factual_consistency: f64,
    citation_quality: f64,
    preference_ordering: f64,
    overall: f64,
    rationale: String,
    passed: bool,
}

#[derive(Clone, Debug, Deserialize)]
struct LlmGrade {
    relevance: f64,
    completeness: f64,
    factual_consistency: f64,
    citation_quality: f64,
    preference_ordering: f64,
    rationale: String,
}

struct OpenAiEvalClient {
    client: Client,
    api_base: String,
    model: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct OpenAiFailureContext {
    status: Option<StatusCode>,
    is_timeout: bool,
    is_connect: bool,
}

impl OpenAiEvalClient {
    fn new(api_base: &str, model: Option<&str>) -> Result<Self> {
        let model = model.ok_or_else(|| {
            anyhow!(
                "LLM eval requires a GPT-5.4 model. Set `ansorum.eval.model`, or pass `--model`."
            )
        })?;
        if !model.starts_with("gpt-5.4") {
            bail!("Eval model `{model}` must be in the GPT-5.4 family");
        }

        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| anyhow!("OPENAI_API_KEY is required when LLM eval is enabled"))?;

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {api_key}"))
                .map_err(|error| anyhow!("Invalid OPENAI_API_KEY header value: {error}"))?,
        );

        let client = Client::builder()
            .connect_timeout(OPENAI_CONNECT_TIMEOUT)
            .timeout(OPENAI_REQUEST_TIMEOUT)
            .default_headers(headers)
            .build()?;
        Ok(Self { client, api_base: api_base.to_string(), model: model.to_string() })
    }

    fn grade_case(
        &self,
        prompt_version: &str,
        case: &EvalCase,
        ranked: Vec<RankedAnswer>,
        selected_answer: Option<site::answers::AnswerRecord>,
        selected_markdown: Option<String>,
        expected_answers: Vec<site::answers::AnswerRecord>,
        markdown_by_id: &BTreeMap<String, String>,
    ) -> Result<LlmCaseReport> {
        let system_prompt = format!(
            "You are grading an Ansorum answer eval case. Prompt version: {prompt_version}. Grade each rubric dimension from 0.0 to 1.0. Be strict. Return JSON only."
        );

        let expected_context = expected_answers
            .iter()
            .map(|answer| {
                json!({
                    "id": answer.id,
                    "title": answer.title,
                    "summary": answer.summary,
                    "canonical_url": answer.canonical_url,
                    "markdown_url": answer.markdown_url,
                    "markdown": markdown_by_id.get(&answer.id),
                })
            })
            .collect::<Vec<_>>();

        let selected_context = selected_answer.as_ref().map(|answer| {
            json!({
                "id": answer.id,
                "title": answer.title,
                "summary": answer.summary,
                "canonical_url": answer.canonical_url,
                "markdown_url": answer.markdown_url,
                "markdown": selected_markdown,
            })
        });

        let user_prompt = json!({
            "question": case.question,
            "expected_ids": case.expected_ids,
            "forbidden_ids": case.forbidden_ids,
            "required_terms": case.required_terms,
            "rubric_focus": case.rubric_focus,
            "ranked_ids": ranked.iter().map(|candidate| candidate.id.clone()).collect::<Vec<_>>(),
            "selected_answer": selected_context,
            "expected_answers": expected_context,
            "rubric": {
                "relevance": "Does the selected answer address the user question directly?",
                "completeness": "Does it cover the required concepts without obvious omissions?",
                "factual_consistency": "Is it consistent with the expected answer set and does it avoid forbidden answers?",
                "citation_quality": "Does it preserve useful canonical or markdown links for traceability?",
                "preference_ordering": "If multiple answers compete, is the selected answer the best available choice?"
            }
        });

        let request = self
            .client
            .post(&self.api_base)
            .json(&json!({
                "model": self.model,
                "input": [
                    {
                        "role": "system",
                        "content": [{ "type": "input_text", "text": system_prompt }]
                    },
                    {
                        "role": "user",
                        "content": [{ "type": "input_text", "text": user_prompt.to_string() }]
                    }
                ],
                "text": {
                    "format": {
                        "type": "json_schema",
                        "name": "ansorum_eval_grade",
                        "strict": true,
                        "schema": llm_grade_schema(),
                    }
                }
            }));
        let response = request.send().map_err(|error| self.classify_request_error(error))?;
        let response = response.error_for_status().map_err(|error| self.classify_request_error(error))?;
        let response = response
            .json::<JsonValue>()
            .map_err(|error| anyhow!("OpenAI Responses API returned invalid JSON: {error}"))?;

        let output = extract_response_text(&response)
            .ok_or_else(|| anyhow!("OpenAI Responses API did not return text output"))?;
        let grade: LlmGrade = serde_json::from_str(&output)
            .map_err(|error| anyhow!("Failed to parse LLM grade JSON: {error}"))?;
        let overall = (grade.relevance
            + grade.completeness
            + grade.factual_consistency
            + grade.citation_quality
            + grade.preference_ordering)
            / 5.0;

        Ok(LlmCaseReport {
            relevance: grade.relevance,
            completeness: grade.completeness,
            factual_consistency: grade.factual_consistency,
            citation_quality: grade.citation_quality,
            preference_ordering: grade.preference_ordering,
            overall,
            rationale: grade.rationale,
            passed: true,
        })
    }

    fn classify_request_error(&self, error: reqwest::Error) -> Error {
        anyhow!(classify_openai_failure(
            &self.api_base,
            OpenAiFailureContext {
                status: error.status(),
                is_timeout: error.is_timeout(),
                is_connect: error.is_connect(),
            },
            &error.to_string(),
        ))
    }
}

fn classify_openai_failure(api_base: &str, context: OpenAiFailureContext, detail: &str) -> String {
    if context.is_timeout {
        return format!(
            "OpenAI eval request timed out after {}s while calling {api_base}. Check network reachability, proxy settings, or try again later.",
            OPENAI_REQUEST_TIMEOUT.as_secs()
        );
    }

    if let Some(status) = context.status {
        return match status {
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => format!(
                "OpenAI eval request was rejected with {status} from {api_base}. Check OPENAI_API_KEY and API access for the selected GPT-5.4 model."
            ),
            StatusCode::TOO_MANY_REQUESTS => format!(
                "OpenAI eval request was rate limited with {status} from {api_base}. Wait for capacity to recover or reduce eval concurrency."
            ),
            status if status.is_server_error() => format!(
                "OpenAI eval request failed with upstream status {status} from {api_base}. The OpenAI API is unavailable or unstable; retry later."
            ),
            status if status.is_client_error() => format!(
                "OpenAI eval request failed with API status {status} from {api_base}. Check the eval request configuration and selected model."
            ),
            status => format!(
                "OpenAI eval request failed with unexpected status {status} from {api_base}."
            ),
        };
    }

    if context.is_connect {
        return format!(
            "OpenAI eval request could not connect to {api_base}. Check DNS, firewall, proxy, or outbound network access."
        );
    }

    format!("OpenAI eval transport failure while calling {api_base}: {detail}")
}

fn resolve_fixture_path(root_dir: &Path, fixtures: &Path) -> PathBuf {
    if fixtures == Path::new(DEFAULT_FIXTURES_PATH) {
        root_dir.join(fixtures)
    } else if fixtures.is_absolute() {
        fixtures.to_path_buf()
    } else {
        root_dir.join(fixtures)
    }
}

fn load_fixture_file(path: &Path) -> Result<EvalFixtureFile> {
    let content = fs::read_to_string(path)
        .map_err(|error| anyhow!("Failed to read eval fixtures from {}: {error}", path.display()))?;
    let raw: RawFixtureFile = serde_yaml::from_str(&content)
        .map_err(|error| anyhow!("Failed to parse eval fixtures from {}: {error}", path.display()))?;

    let mut cases = match raw {
        RawFixtureFile::Cases(cases) => cases,
        RawFixtureFile::Versioned { cases } => cases,
    };

    if cases.is_empty() {
        bail!("Eval fixture file {} does not contain any cases", path.display());
    }

    for case in &mut cases {
        case.question = case.question.trim().to_string();
        if case.question.is_empty() {
            bail!("Eval fixture file {} contains a case with an empty question", path.display());
        }
        ensure_unique("expected_ids", &case.expected_ids, path)?;
        ensure_unique("forbidden_ids", &case.forbidden_ids, path)?;
        ensure_unique("required_terms", &case.required_terms, path)?;
    }

    Ok(EvalFixtureFile { cases })
}

fn ensure_unique(field: &str, values: &[String], path: &Path) -> Result<()> {
    let unique = values.iter().collect::<BTreeSet<_>>();
    if unique.len() != values.len() {
        bail!("Eval fixture file {} contains duplicate values in `{field}`", path.display());
    }
    Ok(())
}

fn machine_markdown_by_id(site: &Site) -> BTreeMap<String, String> {
    let library = site.library.read().unwrap();
    library
        .pages
        .values()
        .filter_map(|page| {
            let answer = page.answer()?;
            let markdown = page.canonical_machine_markdown()?;
            Some((answer.id.clone(), markdown))
        })
        .collect()
}

fn rank_answers(question: &str, site: &Site) -> Vec<RankedAnswer> {
    let query = normalize_text(question);
    let query_tokens = tokenize(&query);

    let mut ranked = site
        .answers
        .iter()
        .filter(|answer| answer.ai_visibility != AiVisibility::Hidden)
        .map(|answer| {
            let mut score = 0;
            let title = normalize_text(&answer.title);
            let summary = normalize_text(&answer.summary);
            let entity = normalize_text(&answer.entity);

            if title == query {
                score += 60;
            }
            if entity == query {
                score += 15;
            }

            for candidate in &answer.canonical_questions {
                let normalized = normalize_text(candidate);
                if normalized == query {
                    score += 120;
                } else if normalized.contains(&query) || query.contains(&normalized) {
                    score += 40;
                }
                score += overlap_score(&query_tokens, &tokenize(&normalized), 12);
            }

            for alias in &answer.retrieval_aliases {
                let normalized = normalize_text(alias);
                if normalized == query {
                    score += 80;
                } else if normalized.contains(&query) || query.contains(&normalized) {
                    score += 25;
                }
                score += overlap_score(&query_tokens, &tokenize(&normalized), 8);
            }

            score += overlap_score(&query_tokens, &tokenize(&title), 10);
            score += overlap_score(&query_tokens, &tokenize(&summary), 4);
            score += overlap_score(&query_tokens, &tokenize(&entity), 3);

            RankedAnswer { id: answer.id.clone(), score }
        })
        .filter(|candidate| candidate.score > 0)
        .collect::<Vec<_>>();

    ranked.sort_by(|left, right| right.score.cmp(&left.score).then(left.id.cmp(&right.id)));
    ranked
}

fn overlap_score(query_tokens: &BTreeSet<String>, candidate_tokens: &BTreeSet<String>, weight: i32) -> i32 {
    query_tokens.iter().filter(|token| candidate_tokens.contains(*token)).count() as i32 * weight
}

fn normalize_text(input: &str) -> String {
    let mut normalized = String::with_capacity(input.len());
    let mut last_space = false;

    for ch in input.chars().flat_map(|ch| ch.to_lowercase()) {
        if ch.is_alphanumeric() {
            normalized.push(ch);
            last_space = false;
        } else if !last_space {
            normalized.push(' ');
            last_space = true;
        }
    }

    normalized.trim().to_string()
}

fn tokenize(input: &str) -> BTreeSet<String> {
    input
        .split_whitespace()
        .filter(|token| token.len() > 1)
        .map(ToOwned::to_owned)
        .collect()
}

fn contains_term(markdown: &str, term: &str) -> bool {
    normalize_text(markdown).contains(&normalize_text(term))
}

fn resolve_model(
    config: &Config,
    llm_enabled: bool,
    model_override: Option<&str>,
) -> Result<Option<String>> {
    let model = model_override
        .map(ToOwned::to_owned)
        .or_else(|| config.ansorum.eval.model.clone())
        .or_else(|| llm_enabled.then(|| DEFAULT_EVAL_MODEL.to_string()));
    if let Some(model) = &model && !model.starts_with("gpt-5.4") {
        bail!("Eval model `{model}` must be in the GPT-5.4 family");
    }

    Ok(model)
}

fn resolve_api_base(config: &Config, api_base_override: Option<&str>) -> String {
    api_base_override
        .map(ToOwned::to_owned)
        .or_else(|| config.ansorum.eval.api_base.clone())
        .unwrap_or_else(|| OPENAI_RESPONSES_URL.to_string())
}

fn finalize_report(report: &mut EvalReport) {
    report.summary.total_cases = report.cases.len();

    let mut llm_totals = Vec::new();
    for case in &mut report.cases {
        if case.retrieval.passed {
            report.summary.retrieval_passed += 1;
        }
        if case.selection.passed {
            report.summary.selection_passed += 1;
        }

        let llm_passed = case
            .llm
            .as_mut()
            .map(|llm| {
                report.summary.llm_scored_cases += 1;
                let min_score = report.thresholds.min_llm_score.unwrap_or(0.0);
                llm.passed = llm.overall >= min_score;
                llm_totals.push(llm.overall);
                if llm.passed {
                    report.summary.llm_passed_cases += 1;
                }
                llm.passed
            })
            .unwrap_or(!report.thresholds.require_llm);

        case.passed = case.retrieval.passed && case.selection.passed && llm_passed;
        if case.passed {
            report.summary.passed_cases += 1;
        }
    }

    if !llm_totals.is_empty() {
        let total = llm_totals.iter().sum::<f64>();
        report.summary.llm_average = Some(total / llm_totals.len() as f64);
    }
}

fn report_failed(report: &EvalReport) -> bool {
    if report.summary.passed_cases < report.summary.total_cases {
        return true;
    }

    if let Some(min_pass_rate) = report.thresholds.min_pass_rate {
        let pass_rate = report.summary.passed_cases as f64 / report.summary.total_cases as f64;
        if pass_rate < min_pass_rate {
            return true;
        }
    }

    if let Some(min_llm_average) = report.thresholds.min_llm_average {
        let llm_average = report.summary.llm_average.unwrap_or(0.0);
        if llm_average < min_llm_average {
            return true;
        }
    }

    if report.thresholds.require_llm && report.summary.llm_scored_cases != report.summary.total_cases {
        return true;
    }

    false
}

fn print_human_report(report: &EvalReport) {
    println!(
        "Eval summary: {}/{} case(s) passed, retrieval {}/{}, selection {}/{}",
        report.summary.passed_cases,
        report.summary.total_cases,
        report.summary.retrieval_passed,
        report.summary.total_cases,
        report.summary.selection_passed,
        report.summary.total_cases
    );
    if let Some(model) = report.model.as_deref() {
        println!(
            "LLM scoring: model={model} prompt_version={} average={:.3}",
            report.prompt_version,
            report.summary.llm_average.unwrap_or(0.0)
        );
    } else {
        println!("LLM scoring: skipped prompt_version={}", report.prompt_version);
    }

    for case in &report.cases {
        println!(
            "- [{}] {}",
            if case.passed { "pass" } else { "fail" },
            case.question
        );
        println!(
            "  retrieval={} ranked={}",
            case.retrieval.passed,
            case.retrieval.ranked_ids.join(", ")
        );
        if !case.retrieval.missing_expected_ids.is_empty() {
            println!(
                "  missing expected ids: {}",
                case.retrieval.missing_expected_ids.join(", ")
            );
        }
        if !case.retrieval.present_forbidden_ids.is_empty() {
            println!(
                "  present forbidden ids: {}",
                case.retrieval.present_forbidden_ids.join(", ")
            );
        }
        println!(
            "  selection={} selected={}",
            case.selection.passed,
            case.selection.selected_id.as_deref().unwrap_or("<none>")
        );
        if !case.selection.missing_required_terms.is_empty() {
            println!(
                "  missing required terms: {}",
                case.selection.missing_required_terms.join(", ")
            );
        }
        if let Some(llm) = case.llm.as_ref() {
            println!("  llm={} overall={:.3}", llm.passed, llm.overall);
            println!("  rationale: {}", llm.rationale);
        }
    }
}

fn print_json_report(report: &EvalReport) -> Result<()> {
    let json = serde_json::to_string_pretty(report)
        .map_err(|error| anyhow!("Failed to serialize eval report: {error}"))?;
    println!("{json}");
    Ok(())
}

fn validate_ratio(name: &str, value: Option<f64>) -> Result<()> {
    if let Some(value) = value && !(0.0..=1.0).contains(&value) {
        bail!("`{name}` must be between 0.0 and 1.0");
    }
    Ok(())
}

fn extract_response_text(response: &JsonValue) -> Option<String> {
    response
        .get("output_text")
        .and_then(JsonValue::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| {
            response
                .get("output")
                .and_then(JsonValue::as_array)
                .into_iter()
                .flatten()
                .flat_map(|item| item.get("content").and_then(JsonValue::as_array).into_iter().flatten())
                .find_map(|content| content.get("text").and_then(JsonValue::as_str).map(ToOwned::to_owned))
        })
}

fn llm_grade_schema() -> JsonValue {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": [
            "relevance",
            "completeness",
            "factual_consistency",
            "citation_quality",
            "preference_ordering",
            "rationale"
        ],
        "properties": {
            "relevance": { "type": "number", "minimum": 0.0, "maximum": 1.0 },
            "completeness": { "type": "number", "minimum": 0.0, "maximum": 1.0 },
            "factual_consistency": { "type": "number", "minimum": 0.0, "maximum": 1.0 },
            "citation_quality": { "type": "number", "minimum": 0.0, "maximum": 1.0 },
            "preference_ordering": { "type": "number", "minimum": 0.0, "maximum": 1.0 },
            "rationale": { "type": "string", "minLength": 1 }
        }
    })
}

fn emit_eval_report(
    root_dir: &Path,
    config_file: &Path,
    include_drafts: bool,
    format: EvalFormat,
    llm_override: bool,
    report: &EvalReport,
) {
    observability::emit_event(
        "governance",
        "eval",
        "ansorum.eval.completed",
        json!({
            "outcome": if report_failed(report) { "failed" } else { "passed" },
            "format": eval_format_name(format),
            "include_drafts": include_drafts,
            "llm_override": llm_override,
            "root_dir": root_dir.display().to_string(),
            "config_file": config_file.display().to_string(),
            "report": report,
        }),
        DispatchMode::Sync,
    );
}

#[allow(clippy::too_many_arguments)]
fn emit_eval_failure(
    root_dir: &Path,
    config_file: &Path,
    include_drafts: bool,
    fixtures: &Path,
    format: EvalFormat,
    llm_override: bool,
    model_override: Option<&str>,
    api_base_override: Option<&str>,
    min_pass_rate: Option<f64>,
    min_llm_average: Option<f64>,
    min_llm_score: Option<f64>,
    require_llm: bool,
    error: &str,
) {
    observability::emit_event(
        "governance",
        "eval",
        "ansorum.eval.completed",
        json!({
            "outcome": "failed",
            "format": eval_format_name(format),
            "include_drafts": include_drafts,
            "llm_override": llm_override,
            "root_dir": root_dir.display().to_string(),
            "config_file": config_file.display().to_string(),
            "fixtures": fixtures.display().to_string(),
            "model_override": model_override,
            "api_base_override": api_base_override,
            "thresholds": {
                "min_pass_rate": min_pass_rate,
                "min_llm_average": min_llm_average,
                "min_llm_score": min_llm_score,
                "require_llm": require_llm,
            },
            "error": error,
        }),
        DispatchMode::Sync,
    );
}

fn eval_format_name(format: EvalFormat) -> &'static str {
    match format {
        EvalFormat::Human => "human",
        EvalFormat::Json => "json",
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::path::Path;

    use reqwest::StatusCode;

    use super::{
        OpenAiFailureContext, classify_openai_failure, contains_term, eval, finalize_report,
        load_fixture_file, rank_answers, resolve_fixture_path, resolve_model,
    };
    use crate::cli::EvalFormat;
    use config::Config;
    use site::Site;

    #[test]
    fn parses_eval_fixture_lists() {
        let path = env::current_dir().unwrap().join("test_site_answers/eval/fixtures.yaml");
        let fixture = load_fixture_file(&path).expect("fixture should parse");
        assert_eq!(fixture.cases.len(), 2);
        assert_eq!(fixture.cases[0].question, "can i get a refund after 30 days");
    }

    #[test]
    fn ranks_expected_billing_answers_first() {
        let path = env::current_dir().unwrap().join("test_site_answers");
        let config_file = path.join("config.toml");
        let mut site = Site::new(&path, &config_file).unwrap();
        site.load().unwrap();

        let ranked = rank_answers("can i get a refund", &site);
        assert_eq!(ranked[0].id, "refunds-policy");
        assert!(ranked.iter().all(|candidate| candidate.id != "internal-support-escalation"));
    }

    #[test]
    fn reference_project_eval_fixtures_pass_deterministic_checks() {
        let root = env::current_dir().unwrap().join("test_site_answers");
        let config_file = root.join("config.toml");
        let fixture = load_fixture_file(&root.join("eval/fixtures.yaml")).expect("fixture should parse");
        let mut site = Site::new(&root, &config_file).unwrap();
        site.load().unwrap();
        let markdown_by_id = super::machine_markdown_by_id(&site);

        for case in fixture.cases {
            let ranked = rank_answers(&case.question, &site);
            assert!(!ranked.is_empty(), "expected at least one ranked answer for {}", case.question);

            let ranked_ids = ranked.iter().map(|candidate| candidate.id.as_str()).collect::<Vec<_>>();
            for expected_id in &case.expected_ids {
                assert!(
                    ranked_ids.iter().any(|candidate| candidate == expected_id),
                    "missing expected id {} for {}",
                    expected_id,
                    case.question
                );
            }
            for forbidden_id in &case.forbidden_ids {
                assert!(
                    ranked_ids.iter().all(|candidate| candidate != forbidden_id),
                    "forbidden id {} was ranked for {}",
                    forbidden_id,
                    case.question
                );
            }

            let selected = ranked
                .iter()
                .find(|candidate| !case.forbidden_ids.iter().any(|id| id == &candidate.id))
                .expect("expected selected answer");
            assert!(
                case.expected_ids.is_empty() || case.expected_ids.iter().any(|id| id == &selected.id),
                "selected unexpected answer {} for {}",
                selected.id,
                case.question
            );

            let markdown = markdown_by_id
                .get(&selected.id)
                .expect("expected machine markdown for selected answer");
            for term in &case.required_terms {
                assert!(contains_term(markdown, term), "missing required term {} for {}", term, case.question);
            }
        }
    }

    #[test]
    fn required_terms_match_machine_markdown() {
        assert!(contains_term("## Eligibility\nRefunds follow the billing policy.", "eligibility"));
        assert!(contains_term("Canonical page: <https://example.com/refunds/>", "canonical page"));
        assert!(!contains_term("Refund details only", "exceptions"));
    }

    #[test]
    fn final_report_enforces_case_success() {
        let mut report = super::EvalReport {
            fixture_path: "eval/fixtures.yaml".to_string(),
            backend: None,
            model: None,
            prompt_version: "v1".to_string(),
            summary: super::EvalSummary::default(),
            thresholds: super::EvalThresholds {
                min_pass_rate: Some(1.0),
                min_llm_average: None,
                min_llm_score: None,
                require_llm: false,
            },
            cases: vec![super::EvalCaseReport {
                question: "example".to_string(),
                retrieval: super::RetrievalCaseReport {
                    passed: true,
                    ranked_ids: vec!["refunds-policy".to_string()],
                    missing_expected_ids: Vec::new(),
                    present_forbidden_ids: Vec::new(),
                },
                selection: super::SelectionCaseReport {
                    passed: true,
                    selected_id: Some("refunds-policy".to_string()),
                    missing_required_terms: Vec::new(),
                },
                llm: None,
                passed: false,
            }],
        };
        finalize_report(&mut report);
        assert_eq!(report.summary.passed_cases, 1);
        assert_eq!(report.summary.total_cases, 1);
    }

    #[test]
    fn default_fixture_path_resolves_from_root() {
        let root = Path::new("/tmp/site");
        let resolved = resolve_fixture_path(root, Path::new(super::DEFAULT_FIXTURES_PATH));
        assert_eq!(resolved, Path::new("/tmp/site/eval/fixtures.yaml"));
        let _ = EvalFormat::Human;
    }

    #[test]
    fn llm_eval_defaults_to_mini_model() {
        let config = Config::default_for_test();
        let model = resolve_model(&config, true, None).expect("expected default model");
        assert_eq!(model.as_deref(), Some("gpt-5.4-mini"));
    }

    #[test]
    fn model_override_beats_default_model() {
        let config = Config::default_for_test();
        let model =
            resolve_model(&config, true, Some("gpt-5.4")).expect("expected overridden model");
        assert_eq!(model.as_deref(), Some("gpt-5.4"));
    }

    #[test]
    fn classifies_openai_timeout_failures() {
        let message = classify_openai_failure(
            "https://api.openai.com/v1/responses",
            OpenAiFailureContext { status: None, is_timeout: true, is_connect: false },
            "deadline elapsed",
        );
        assert!(message.contains("timed out after 45s"));
    }

    #[test]
    fn classifies_openai_auth_failures() {
        let message = classify_openai_failure(
            "https://api.openai.com/v1/responses",
            OpenAiFailureContext {
                status: Some(StatusCode::UNAUTHORIZED),
                is_timeout: false,
                is_connect: false,
            },
            "401 Unauthorized",
        );
        assert!(message.contains("Check OPENAI_API_KEY"));
    }

    #[test]
    fn classifies_openai_rate_limits() {
        let message = classify_openai_failure(
            "https://api.openai.com/v1/responses",
            OpenAiFailureContext {
                status: Some(StatusCode::TOO_MANY_REQUESTS),
                is_timeout: false,
                is_connect: false,
            },
            "429 Too Many Requests",
        );
        assert!(message.contains("rate limited"));
    }

    #[test]
    fn classifies_openai_server_failures() {
        let message = classify_openai_failure(
            "https://api.openai.com/v1/responses",
            OpenAiFailureContext {
                status: Some(StatusCode::BAD_GATEWAY),
                is_timeout: false,
                is_connect: false,
            },
            "502 Bad Gateway",
        );
        assert!(message.contains("upstream status 502 Bad Gateway"));
    }

    #[test]
    fn classifies_openai_transport_failures() {
        let message = classify_openai_failure(
            "https://api.openai.com/v1/responses",
            OpenAiFailureContext { status: None, is_timeout: false, is_connect: true },
            "connection refused",
        );
        assert!(message.contains("could not connect"));
    }

    #[test]
    fn reference_project_eval_command_passes_without_llm() {
        let root = env::current_dir().unwrap().join("test_site_answers");
        let config_file = root.join("config.toml");

        eval(
            &root,
            &config_file,
            false,
            Path::new(super::DEFAULT_FIXTURES_PATH),
            EvalFormat::Json,
            false,
            None,
            None,
            Some(1.0),
            None,
            None,
            false,
        )
        .expect("eval should pass");
    }

    #[test]
    fn eval_command_fails_when_thresholds_are_unmet() {
        let root = env::current_dir().unwrap().join("test_site_answers");
        let config_file = root.join("config.toml");

        let err = eval(
            &root,
            &config_file,
            false,
            Path::new(super::DEFAULT_FIXTURES_PATH),
            EvalFormat::Human,
            false,
            None,
            None,
            Some(1.0),
            None,
            Some(1.0),
            true,
        )
        .expect_err("eval should fail when llm is required");
        assert_eq!(err.to_string(), "Eval failed");
    }
}
