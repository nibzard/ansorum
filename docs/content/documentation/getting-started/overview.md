+++
title = "Overview"
weight = 5
+++

## Ansorum at a Glance

Ansorum is an answer-first compiler. The unit of authorship is not a generic
page or blog post. The unit of authorship is an answerable Markdown document
with typed metadata describing:

- what question it answers
- who it is for
- what kind of answer it is
- whether it is public, internal, or hidden from AI systems
- how prominently it should appear in machine outputs

Ansorum compiles that answer corpus into:

- HTML for humans
- canonical Markdown at `/page.md`
- root and scoped machine indexes such as `answers.json`
- `llms.txt` and `llms-full.txt`
- structured data outputs from `<answer-stem>.schema.json`
- audit and evaluation reports

## Recommended Onboarding Path

The fastest way to understand the product is:

1. Install `ansorum`.
2. Read the [Reference Project](@/documentation/getting-started/reference-project.md).
3. Run the full workflow against `test_site_answers/`.
4. Copy that answer-first shape into your own repository.

The repository's canonical answer-first example project lives in
[`test_site_answers/`](https://github.com/nibzard/ansorum/tree/main/test_site_answers).

## First Run

`ansorum init` creates an answer-first starter project:

```bash
ansorum init my-answers
cd my-answers
```

You will be asked for:

```text
> What is the URL of your site? (https://example.com):
```

The generated scaffold includes starter answers, a sidecar, a curated pack, and
deterministic eval fixtures:

```text
my-answers/
├── README.md
├── collections/
│   └── packs/
│       └── billing.toml
├── config.toml
├── content/
│   ├── cancel.md
│   ├── internal-playbook.md
│   ├── refunds.md
│   └── refunds.schema.json
├── eval/
│   └── fixtures.yaml
└── static/
```

That scaffold is ready to run through `build`, `serve`, `audit`, and `eval`
without hand edits. `test_site_answers/` remains the fuller reference corpus
for additional patterns.

## Author Your First Answer

Create `content/refunds.md`:

```md
+++
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
+++

Refund details for customers.
```

Optional JSON-LD for that answer lives beside it as
`content/refunds.schema.json`. The v0 naming rule is exact: use one sibling
sidecar per answer, named from the Markdown file stem as
`<answer-stem>.schema.json`.

## Run The Operator Loop

Build all compiled outputs:

```bash
ansorum build
```

Serve the answer corpus locally with HTML, Markdown negotiation, and redirects:

```bash
ansorum serve
```

Audit metadata quality, freshness, and visibility issues before publish:

```bash
ansorum audit
```

Evaluate retrieval and answer selection against fixtures:

```bash
ansorum eval
```

Use `ansorum eval --llm` only when `OPENAI_API_KEY` is configured and you want
OpenAI Responses API grading. If you do not set `ansorum.eval.model` or
`--model`, Ansorum uses `gpt-5.4-mini` by default.

For contributor parity with CI, run the deterministic compiler-contract gate
from the repository root:

```bash
cargo test --locked --all
./target/debug/ansorum --root test_site_answers build
./target/debug/ansorum --root test_site_answers audit --format json
./target/debug/ansorum --root test_site_answers eval --format json --min-pass-rate 1.0
```

That is the default automation path because it verifies the answer-first
reference corpus without requiring network access or OpenAI credentials.

## What "Done" Looks Like

For a healthy answer-first project, one authored answer should compile into:

- a human page such as `/refunds/`
- canonical machine Markdown at `/refunds/page.md`
- inclusion or exclusion from `answers.json` and `llms.txt` according to
  visibility
- optional structured data at `/refunds/schema.json`

See the [CLI usage](@/documentation/getting-started/cli-usage.md),
[Reference Project](@/documentation/getting-started/reference-project.md), and
[Page](@/documentation/content/page.md) docs for the concrete authoring and
command contract.
