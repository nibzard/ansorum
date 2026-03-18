+++
title = "CLI usage"
weight = 15
+++

Ansorum's core command surface is:

- `ansorum init`
- `ansorum build`
- `ansorum serve`
- `ansorum audit`
- `ansorum eval`
- `ansorum check`

Use `ansorum --help` for the full program help and `ansorum <command> --help`
for command-specific flags.

## init

Creates an answer-first project scaffold at the given directory after asking for
the site URL. Choices made during `init` can be changed later in the generated
files.

```bash
ansorum init my_answers
ansorum init
```

`init` writes a working starter corpus, including:

- answer-first Markdown under `content/`
- a sidecar JSON-LD example
- redirect and pack configuration in `config.toml`
- a curated pack in `collections/packs/`
- deterministic eval fixtures in `eval/fixtures.yaml`

The generated project is meant to pass `ansorum build`, `ansorum serve`,
`ansorum audit`, and `ansorum eval` on first run.

If the target directory already exists, Ansorum only populates it when it
contains only hidden files. If no directory argument is passed, Ansorum tries
to populate the current directory.

To attempt population of a non-empty directory, use `--force`. Ansorum still
will not overwrite existing files or folders.

Typical flow:

```bash
git init
ansorum init
ansorum build
ansorum audit
ansorum eval
```

## build

Builds the answer corpus into the output directory, `public` by default.
Existing output is deleted first unless dotfile preservation is enabled.

```bash
ansorum build
```

In an Ansorum project, `build` is expected to emit:

- HTML pages for humans
- canonical machine Markdown at `/page.md`
- `answers.json`
- `llms.txt` and `llms-full.txt`
- scoped pack outputs where configured
- structured data sidecars where present

You can override `base_url` with `--base-url`:

```bash
ansorum build --base-url "$DEPLOY_URL"
```

You can override the output directory with `--output-dir`:

```bash
ansorum build --output-dir "$DOCUMENT_ROOT"
```

To use a different config file, place `--config` before the command:

```bash
ansorum --config config.staging.toml build
```

You can also operate on a project from another directory:

```bash
ansorum --root /path/to/project build
```

By default drafts are not loaded. Use `--drafts` to include them.

## serve

Builds and serves the corpus locally, then rebuilds on change. The default bind
address is `127.0.0.1:1111`.

`serve` is the easiest way to test the full delivery behavior:

- canonical HTML routes
- negotiated Markdown via `Accept: text/markdown`
- explicit `/page.md` routes
- configured redirect routes under `/r/:code`
- live reload during editing

Common examples:

```bash
ansorum serve
ansorum serve --port 2000
ansorum serve --interface 0.0.0.0
ansorum serve --interface 0.0.0.0 --port 2000
ansorum serve --interface 0.0.0.0 --base-url /
ansorum serve --open
```

If you need the served site reachable on your local network, bind to
`0.0.0.0`.

By default, `serve` keeps HTML in memory. Use `--store-html` to also write HTML
to disk while serving.

Use `--debounce <ms>` to tune the file-watch debounce time.

## audit

Audits answer metadata, freshness, and machine-output quality.

```bash
ansorum audit
ansorum audit --format json
```

Use `audit` before publish to catch issues such as:

- missing or invalid answer metadata
- stale `review_by` dates
- duplicate or conflicting answer coverage
- visibility or machine-output policy violations

`--format json` is useful for CI or downstream tooling.

## eval

Evaluates retrieval and answer selection against fixture cases, with optional
LLM rubric grading through the OpenAI Responses API.

```bash
ansorum eval
ansorum eval --fixtures eval/fixtures.yaml
ansorum eval --llm
ansorum eval --llm --model gpt-5.4-mini
```

`eval` uses `eval/fixtures.yaml` by default. Each fixture case defines a user
question, expected answers, forbidden answers, and required terms. When `--llm`
is enabled, Ansorum also asks a GPT-5.4 model to score the selected answer
against a rubric. If no model is configured or passed on the command line,
Ansorum defaults to `gpt-5.4-mini`.

Use the threshold flags to make eval enforce a quality bar:

- `--min-pass-rate`
- `--min-llm-average`
- `--min-llm-score`
- `--require-llm`

## check

`check` tries to build the project without writing output, and it validates
links in Markdown content.

```bash
ansorum check
ansorum check --skip-external-links
```

This remains useful for fast authoring feedback, but for answer quality and AI
delivery governance you should rely on `audit` and `eval`.

## Observability

Ansorum v0 exposes machine-delivery and governance telemetry through structured
logs and an optional event hook. Both sinks use the same JSON envelope:

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

Enable the hook by exporting:

```bash
export ANSORUM_EVENT_HOOK_URL="https://hooks.example.com/ansorum"
```

You can tune delivery timeout with:

```bash
export ANSORUM_EVENT_HOOK_TIMEOUT_MS=5000
```

Stable v0 event names:

- `ansorum.markdown.fetch`: canonical `/page.md` fetches and negotiated
  `Accept: text/markdown` fetches. Payload includes `request_path`,
  `served_path`, `content_source`, and `delivery_mode`.
- `ansorum.llms.fetch`: root `llms.txt`, `llms-full.txt`, and scoped pack
  `llms.txt` fetches. Payload includes `variant` and optional `pack_path`.
- `ansorum.redirect.hit`: redirect hits under `/r/:code`. Payload includes
  `code`, `target`, `external`, and `status`.
- `ansorum.audit.completed`: audit runs. Payload includes `outcome`, command
  settings, and the audit `report` or failure details.
- `ansorum.eval.completed`: eval runs. Payload includes `outcome`, command
  settings, and the eval `report` or failure details.

## Colored output

Colored output is used if your terminal supports it.

*Note*: coloring is automatically disabled when the output is redirected to a pipe or a file (i.e., when the standard output is not a TTY).

You can disable this behavior by exporting one of the following two environment variables:

- `NO_COLOR` (the value does not matter)
- `CLICOLOR=0`

To force the use of colors, you can set the following environment variable:

- `CLICOLOR_FORCE=1`

## Extra information

Ansorum can provide detailed logging about its behavior via the `RUST_LOG`
variable:

- To see timing information, set `RUST_LOG=zola=info,site=debug`.
- To see debug information, set `RUST_LOG=debug`. *Note*: The output will be **very noisy**, use with caution.
- To disable all log output entirely, set `RUST_LOG=off`.

See the [env_logger documentation](https://docs.rs/env_logger/0.11.8/env_logger/#enabling-logging) for a full reference on `RUST_LOG`.
