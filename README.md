# ansorum

[![GitHub all releases](https://img.shields.io/github/downloads/nibzard/ansorum/total)](https://github.com/nibzard/ansorum/releases)

An answer-first compiler for agent-readable and human-readable knowledge.

Ansorum turns Markdown content into an authoritative answer system with:

- canonical HTML for humans
- canonical Markdown for agents
- `llms.txt` and `llms-full.txt`
- machine indexes like `answers.json`
- structured data outputs
- audit and evaluation loops for answer quality

The product is not "a lighter website framework." The product is:

- Git-native answer storage
- typed Markdown answer units
- a compiler that emits multiple machine and human views of the same corpus
- a lightweight gateway for Markdown negotiation and redirects
- audit and eval tooling so answer quality is testable before publish

## Core Workflow

Ansorum is for teams that need one canonical answer layer behind docs, support,
help centers, product knowledge, and AI systems.

The operator workflow is:

1. Author one Markdown file per answerable unit in `content/`.
2. Add first-class answer frontmatter such as `id`, `summary`,
   `canonical_questions`, `intent`, `entity`, `audience`, `visibility`,
   `ai_visibility`, `llms_priority`, and `token_budget`.
3. Add optional `<answer-stem>.schema.json` sidecars for JSON-LD.
4. Configure packs, redirects, and eval defaults in `config.toml`.
5. Run `ansorum build`, `ansorum serve`, `ansorum audit`, and `ansorum eval`.

Ansorum is designed to help teams:

- make one answer authoritative and reusable everywhere
- control what agents can see and at what fidelity
- detect stale, duplicate, conflicting, or weak answers before publishing
- evaluate whether the corpus actually answers real questions well

## Observability Contract

Ansorum v0 emits observability data through:

- structured JSON log lines
- an optional JSON event hook via `ANSORUM_EVENT_HOOK_URL`

The same envelope is used for both sinks:

```json
{
  "schema_version": 1,
  "emitted_at": "2026-03-18T21:00:00.000Z",
  "source": {
    "product": "ansorum",
    "surface": "serve",
    "command": "serve"
  },
  "event": "ansorum.markdown.fetch",
  "payload": {}
}
```

Current stable v0 events are:

- `ansorum.markdown.fetch`
- `ansorum.llms.fetch`
- `ansorum.redirect.hit`
- `ansorum.audit.completed`
- `ansorum.eval.completed`

Set `ANSORUM_EVENT_HOOK_TIMEOUT_MS` to change the hook POST timeout from the
default `2000` milliseconds.

## Authoring Model

The source of truth is a Git repository with one Markdown file per answer. A
typical authored answer looks like this:

```toml
+++
title = "Refund policy"
id = "refunds-policy"
summary = "How refunds work, who qualifies, and when payment returns land."
canonical_questions = ["how do refunds work", "can i get a refund"]
intent = "policy"
entity = "billing"
audience = "customer"
visibility = "public"
ai_visibility = "public"
llms_priority = "core"
token_budget = "medium"
+++
```

Ansorum then compiles that answer into canonical HTML, canonical machine
Markdown at `/page.md`, answer indexes, `llms.txt` outputs, and structured data
sidecars when present. Sidecars follow a single v0 convention: a sibling file
named from the Markdown stem, such as `content/refunds.md` alongside
`content/refunds.schema.json`.

## Current Direction

This repository started as a fork of Zola and still reuses substantial parts of
its Rust implementation. The direction of the fork is now explicitly
answer-first, not upstream-compatible by default.

The current product specification lives in
[ANSWER_COMPILER_SPEC.md](ANSWER_COMPILER_SPEC.md).

Repository provenance and release reset details live in
[PROVENANCE.md](PROVENANCE.md).

Future MIT-only restart rules live in
[CLEAN_ROOM_READINESS.md](CLEAN_ROOM_READINESS.md).

## v0 Shape

The near-term Ansorum build should provide:

- first-class answer frontmatter
- `/page.md` outputs
- `Accept: text/markdown` negotiation
- `llms.txt` and scoped packs
- JSON-LD sidecar support
- `audit` and `eval` commands

When LLM scoring is enabled for `ansorum eval`, Ansorum defaults to
`gpt-5.4-mini` unless `ansorum.eval.model` or `--model` selects another
GPT-5.4 tier.

## Reference Project

The repository includes a canonical answer-first example project in
`test_site_answers/`.

Use it to exercise the full workflow end to end:

```bash
cd test_site_answers
ansorum build
ansorum serve
ansorum audit
ansorum eval
```

`ansorum init` now creates an answer-first starter project with:

- starter answers in `content/`
- a JSON-LD sidecar example in `content/refunds.schema.json`
- redirects and pack configuration in `config.toml`
- a curated pack definition in `collections/packs/billing.toml`
- deterministic eval fixtures in `eval/fixtures.yaml`

That project demonstrates:

- first-class answer frontmatter in TOML and YAML
- `summary_only` and `hidden` AI visibility controls
- `/page.md`, `answers.json`, `llms.txt`, `llms-full.txt`, and scoped packs
- `<answer-stem>.schema.json` sidecars
- `/r/:code` redirects with allowlist enforcement
- deterministic eval fixtures

## Why Ansorum

The wedge is not just `llms.txt`.

The wedge is:

- answer-first authoring rather than page-first authoring
- explicit AI visibility controls
- multiple compiled machine outputs from one canonical source
- built-in governance, audit, and eval loops
- static, Git-native operation without a heavy app platform

## License

This project contains code under multiple licenses.

Code introduced after version 0.22 is licensed under the EUPL-1.2.
Code that existed prior to commit 3c9131db0d203640b6d5619ca1f75ce1e0d49d8f remains licensed under the MIT License, including in later versions of the project.

See LICENSE and LICENSE-MIT for details.
