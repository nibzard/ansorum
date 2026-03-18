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

## Jobs To Be Done

Ansorum is for teams that need one canonical answer layer behind docs, support,
help centers, product knowledge, and AI systems.

It should help teams:

- make one answer authoritative and reusable everywhere
- control what agents can see and at what fidelity
- detect stale, duplicate, conflicting, or weak answers before publishing
- evaluate whether the corpus actually answers real questions well

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
