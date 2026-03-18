# Clean-Room Readiness

Status: Draft

Date: 2026-03-18

Scope: This document defines how Ansorum could later start a new MIT-only
implementation line without relying on post-0.22 EUPL-covered source code from
this repository.

This is an engineering process document, not legal advice.

## Current Position

Today, `ansorum` is an explicit derivative fork that still reuses substantial
parts of Zola's implementation.

That is the right near-term posture for v0 because the current product strategy
is to reuse the existing implementation selectively while reshaping the product
around an answer-first model.

The current repository should therefore be treated as a mixed-license codebase:

- code introduced before commit `3c9131db0d203640b6d5619ca1f75ce1e0d49d8f`
  remains under the MIT license
- code introduced from `0.22.0` onward is under the EUPL-1.2

The `0.22.0` release was published on 2026-01-09.

## Goal

The goal of a future clean-room effort would be:

- to start a new repository that contains only MIT-licensed inherited code plus
  newly written code
- to avoid carrying forward post-0.22 EUPL-covered source code or derivative
  expression from this repository
- to preserve Ansorum's product direction and behavior without relying on the
  current implementation as a template

## Non-Goals

This document does not:

- claim that the current repository can be relicensed as MIT-only
- claim that reproducing the current diff by hand is a clean-room process
- authorize copying post-0.22 source files, patches, tests, or generated output
- replace legal review

## Core Rule

If Ansorum later starts an MIT-only line, the implementation team must rebuild
functionality, not port code.

The future MIT-only line should not be developed by translating, copying,
cherry-picking, or line-by-line recreating post-0.22 source from this
repository.

## Allowed Starting Points

The acceptable technical bases for a future MIT-only restart are:

- the last MIT-only upstream base before commit
  `3c9131db0d203640b6d5619ca1f75ce1e0d49d8f`
- a new empty repository

If an MIT-only line starts from a pre-`3c9131db` codebase, every imported file
must be verified to come from that MIT-only history.

## Forbidden Inputs For A Future MIT-Only Implementation Team

The future MIT-only implementation team should not use:

- post-0.22 source files from this repository
- post-0.22 commits, patches, or cherry-picks from this repository
- side-by-side comparison against the current Ansorum code while implementing
- copied code snippets from issues, PRs, reviews, discussions, or commit
  messages that describe post-0.22 implementation details
- generated outputs copied from the current implementation when those outputs
  encode implementation-specific expression rather than neutral behavior
- tests, fixtures, examples, or docs copied verbatim from the current EUPL-era
  repository unless their authorship and reuse rights are separately cleared

In practical terms, "same behavior rebuilt from a neutral spec" is acceptable.
"Same code rewritten after reading it" is not a clean-room process.

## Safe Carry-Forward Artifacts

The safest bridge into a future MIT-only line is a neutral requirements pack.

That pack should describe product behavior without embedding current source
expression.

Examples:

- product requirements
- CLI behavior definitions
- content model definitions
- output format contracts
- HTTP behavior contracts
- benchmark questions
- evaluation criteria
- performance targets
- compatibility notes written at the behavior level

Important constraint:

Artifacts created inside this mixed-license repository should not automatically
be assumed reusable in a future MIT-only repository. Before carrying them
forward, confirm authorship and licensing or rewrite them from scratch in a
separate repository with an explicit permissive license.

## Recommended Future Process

If an MIT-only restart becomes strategically important, use this process.

### 1. Freeze The Split Point

Record:

- the exact MIT-only base commit or tag
- the exact date of the split decision
- the current Ansorum repository URL and branch
- the contributors who are authorized to participate in the clean-room effort

### 2. Create A Neutral Requirements Pack

Before implementation starts, prepare a separate artifact set containing:

- feature definitions
- acceptance criteria
- API and CLI contracts
- sample content inputs
- expected high-level outputs
- non-functional requirements

This pack should avoid code excerpts and should be licensed explicitly for reuse
in the new repository.

### 3. Separate Roles

Use at least two roles if possible:

- a spec team that may inspect the current repository and produce neutral
  behavior requirements
- an implementation team that builds the MIT-only line from the requirements
  pack and approved MIT-only base only

The implementation team should not use the current post-0.22 code as a coding
reference.

### 4. Keep An Input Ledger

Maintain a simple log of all approved inputs used by the MIT-only line:

- base commit or repository
- requirements documents
- test plans
- benchmarks
- external dependencies

If an input is not in the ledger, it should be treated as unapproved.

### 5. Rebuild Behavior, Then Verify Behavior

Use black-box verification:

- CLI acceptance tests
- HTTP contract tests
- output schema tests
- benchmark question evaluation

The implementation should be judged by whether it satisfies the published
requirements, not by whether it matches the current code structure.

### 6. Review Before Release

Before publishing the MIT-only line:

- review provenance of imported files
- review the approved-input ledger
- review contributor authorship and rights
- run legal review if the repository will be commercialized or broadly
  redistributed

## What We Should Do Now

While continuing to build Ansorum in this repository, we can prepare for a
future clean-room option by doing the following:

- keep the current repository explicitly positioned as a derivative fork
- maintain clear provenance around the `0.22.0` license split
- move product intent into behavior-level specifications rather than
  implementation-coupled notes
- define acceptance tests and benchmarks in implementation-neutral language
- avoid describing future portability as "replaying the diff"

## Readiness Checklist

An MIT-only restart is not ready unless all of the following are true:

- an approved MIT-only base has been selected
- the future implementation team has an approved-input list
- a neutral requirements pack exists under clear reuse terms
- post-0.22 source code is excluded from implementation inputs
- provenance logging is in place
- legal review is scheduled if needed

## Bottom Line

Ansorum should continue in this repository as an explicit derivative fork for
now.

If an MIT-only line is later required, the right move is a fresh implementation
from an MIT-only base and a neutral requirements pack, not a hand-reproduced
port of the current EUPL-era diff.
