use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use config::Config;
use content::{AnswerAudience, AnswerIntent, LlmsPriority};
use errors::{Result, anyhow, bail};
use serde::Deserialize;
use utils::slugs::slugify_paths;

use crate::answers::{AnswerCorpus, AnswerRecord};

pub struct LlmsOutputs {
    pub llms_txt: String,
    pub llms_full_txt: String,
    pub packs: Vec<PackOutput>,
}

pub struct PackOutput {
    pub path: String,
    pub llms_txt: String,
    pub answers_json: String,
}

pub fn build_outputs(config: &Config, base_path: &Path, answers: &AnswerCorpus) -> Result<LlmsOutputs> {
    let generated_packs = build_packs(config, base_path, answers)?;
    let pack_refs = generated_packs
        .iter()
        .map(|pack| PackReference {
            name: pack.name.clone(),
            title: pack.title.clone(),
            llms_url: config.make_permalink(&format!("{}/llms.txt", pack.path)),
        })
        .collect::<Vec<_>>();

    let llms_txt = render_root_llms(config, answers, &pack_refs);
    let llms_full_txt = render_full_llms(config, answers);
    let packs = generated_packs
        .into_iter()
        .map(|pack| {
            let records = pack.ai_visible_records();
            let llms_txt = render_pack_llms(config, &pack, &records);
            let answers_json = answers.to_json_subset(&records)?;
            Ok(PackOutput { path: pack.path, llms_txt, answers_json })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(LlmsOutputs { llms_txt, llms_full_txt, packs })
}

#[derive(Clone)]
struct PackDefinition<'a> {
    name: String,
    path: String,
    title: String,
    description: String,
    records: Vec<&'a AnswerRecord>,
}

impl<'a> PackDefinition<'a> {
    fn ai_visible_records(&self) -> Vec<&'a AnswerRecord> {
        self.records
            .iter()
            .copied()
            .filter(|record| is_ai_visible(record))
            .collect()
    }
}

struct PackReference {
    name: String,
    title: String,
    llms_url: String,
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct CuratedPackFile {
    title: Option<String>,
    description: Option<String>,
    answers: Vec<String>,
    entities: Vec<String>,
    audiences: Vec<AnswerAudience>,
    intents: Vec<AnswerIntent>,
}

fn build_packs<'a>(config: &Config, base_path: &Path, answers: &'a AnswerCorpus) -> Result<Vec<PackDefinition<'a>>> {
    let mut packs = BTreeMap::<String, PackDefinition<'a>>::new();

    if config.ansorum.packs.auto_entity_packs {
        let mut entities = BTreeSet::new();
        for record in answers.iter() {
            if is_llms_visible(record) {
                entities.insert(record.entity.clone());
            }
        }

        for entity in entities {
            let records = answers
                .same_entity(&entity)
                .into_iter()
                .filter(|record| is_llms_visible(record))
                .collect::<Vec<_>>();
            if records.is_empty() {
                continue;
            }

            let name = slugify_paths(&entity, config.slugify.paths);
            packs.insert(
                name.clone(),
                PackDefinition {
                    name: name.clone(),
                    path: name,
                    title: format!("{} answers", entity),
                    description: format!("Scoped AI-visible answers for the `{entity}` entity."),
                    records,
                },
            );
        }
    }

    if config.ansorum.packs.auto_audience_packs {
        let audiences = [
            AnswerAudience::Customer,
            AnswerAudience::Prospect,
            AnswerAudience::Developer,
            AnswerAudience::Admin,
            AnswerAudience::Internal,
        ];

        for audience in audiences {
            let records = answers
                .same_audience(&audience)
                .into_iter()
                .filter(|record| is_llms_visible(record))
                .collect::<Vec<_>>();
            if records.is_empty() {
                continue;
            }

            let audience_name = audience_name(&audience);
            let name = slugify_paths(audience_name, config.slugify.paths);
            packs.insert(
                name.clone(),
                PackDefinition {
                    name: name.clone(),
                    path: name,
                    title: format!("{} answers", title_case(audience_name)),
                    description: format!(
                        "Scoped AI-visible answers for the `{}` audience.",
                        audience_name
                    ),
                    records,
                },
            );
        }
    }

    for curated in &config.ansorum.packs.curated {
        let path = base_path.join(&curated.source);
        let raw = fs::read_to_string(&path).map_err(|error| {
            anyhow!(
                "Failed to read curated pack `{}` from {}: {error}",
                curated.name,
                path.display()
            )
        })?;
        let definition: CuratedPackFile = toml::from_str(&raw).map_err(|error| {
            anyhow!(
                "Failed to parse curated pack `{}` from {}: {error}",
                curated.name,
                path.display()
            )
        })?;

        let mut selected = BTreeSet::new();
        let mut records = Vec::new();

        for id in &definition.answers {
            let record = answers.get(id).ok_or_else(|| {
                anyhow!(
                    "Curated pack `{}` references unknown answer id `{id}` in {}",
                    curated.name,
                    path.display()
                )
            })?;
            if selected.insert(record.id.clone()) && is_llms_visible(record) {
                records.push(record);
            }
        }

        for entity in &definition.entities {
            for record in answers.same_entity(entity) {
                if selected.insert(record.id.clone()) && is_llms_visible(record) {
                    records.push(record);
                }
            }
        }

        for audience in &definition.audiences {
            for record in answers.same_audience(audience) {
                if selected.insert(record.id.clone()) && is_llms_visible(record) {
                    records.push(record);
                }
            }
        }

        for intent in &definition.intents {
            for record in answers.same_intent(intent) {
                if selected.insert(record.id.clone()) && is_llms_visible(record) {
                    records.push(record);
                }
            }
        }

        if definition.answers.is_empty()
            && definition.entities.is_empty()
            && definition.audiences.is_empty()
            && definition.intents.is_empty()
        {
            bail!(
                "Curated pack `{}` in {} must select answers by `answers`, `entities`, `audiences`, or `intents`",
                curated.name,
                path.display()
            );
        }

        records.sort_by(|left, right| left.id.cmp(&right.id));
        let name = curated.name.clone();
        packs.insert(
            name.clone(),
            PackDefinition {
                name: name.clone(),
                path: name,
                title: definition
                    .title
                    .unwrap_or_else(|| format!("{} pack", title_case(&curated.name))),
                description: definition.description.unwrap_or_else(|| {
                    format!("Curated AI-visible answer pack `{}`.", curated.name)
                }),
                records,
            },
        );
    }

    Ok(packs.into_values().filter(|pack| !pack.ai_visible_records().is_empty()).collect())
}

fn render_root_llms(config: &Config, answers: &AnswerCorpus, pack_refs: &[PackReference]) -> String {
    let mut output = String::new();
    push_heading(&mut output, config.title.as_deref().unwrap_or("Ansorum"));
    push_paragraph(&mut output, corpus_description(config));
    push_section(
        &mut output,
        "Core Answers",
        &answers.iter().filter(|record| is_core(record)).collect::<Vec<_>>(),
    );

    let optional = answers.iter().filter(|record| is_optional(record)).collect::<Vec<_>>();
    if !optional.is_empty() {
        push_section(&mut output, "Additional Context", &optional);
    }

    if !pack_refs.is_empty() {
        output.push_str("## Scoped Packs\n\n");
        for pack in pack_refs {
            output.push_str(&format!(
                "- {} (`{}`): {}\n",
                pack.title, pack.name, pack.llms_url
            ));
        }
        output.push('\n');
    }

    output
}

fn render_full_llms(config: &Config, answers: &AnswerCorpus) -> String {
    let mut output = String::new();
    push_heading(&mut output, &format!("{} full export", config.title.as_deref().unwrap_or("Ansorum")));
    push_paragraph(&mut output, corpus_description(config));
    push_section(
        &mut output,
        "AI-visible Answers",
        &answers.iter().filter(|record| is_llms_visible(record)).collect::<Vec<_>>(),
    );
    output
}

fn render_pack_llms(config: &Config, pack: &PackDefinition<'_>, records: &[&AnswerRecord]) -> String {
    let mut output = String::new();
    push_heading(
        &mut output,
        &format!("{} | {}", config.title.as_deref().unwrap_or("Ansorum"), pack.title),
    );
    push_paragraph(&mut output, &pack.description);

    let core = records.iter().copied().filter(|record| is_core(record)).collect::<Vec<_>>();
    let optional = records
        .iter()
        .copied()
        .filter(|record| is_optional(record))
        .collect::<Vec<_>>();

    push_section(&mut output, "Core Answers", &core);
    if !optional.is_empty() {
        push_section(&mut output, "Additional Context", &optional);
    }

    output
}

fn push_heading(output: &mut String, heading: &str) {
    output.push_str(&format!("# {heading}\n\n"));
}

fn push_paragraph(output: &mut String, text: &str) {
    output.push_str(text);
    output.push_str("\n\n");
}

fn push_section(output: &mut String, title: &str, records: &[&AnswerRecord]) {
    if records.is_empty() {
        return;
    }

    output.push_str(&format!("## {title}\n\n"));
    for record in records {
        output.push_str(&format!(
            "- {}: {} ({})\n",
            record.title, record.summary, record.markdown_url
        ));
    }
    output.push('\n');
}

fn corpus_description(config: &Config) -> &str {
    config
        .description
        .as_deref()
        .unwrap_or("Authoritative answer corpus compiled for human and agent consumption.")
}

fn is_ai_visible(record: &AnswerRecord) -> bool {
    record.is_machine_ai_visible()
}

fn is_llms_visible(record: &AnswerRecord) -> bool {
    is_ai_visible(record) && record.llms_priority != LlmsPriority::Hidden
}

fn is_core(record: &AnswerRecord) -> bool {
    is_llms_visible(record) && record.llms_priority == LlmsPriority::Core
}

fn is_optional(record: &AnswerRecord) -> bool {
    is_llms_visible(record) && record.llms_priority == LlmsPriority::Optional
}

fn audience_name(audience: &AnswerAudience) -> &'static str {
    match audience {
        AnswerAudience::Customer => "customer",
        AnswerAudience::Prospect => "prospect",
        AnswerAudience::Developer => "developer",
        AnswerAudience::Admin => "admin",
        AnswerAudience::Internal => "internal",
    }
}

fn title_case(value: &str) -> String {
    value.split(['-', '_', ' '])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
