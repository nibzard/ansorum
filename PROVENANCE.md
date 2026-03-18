# Provenance

Status: Draft

Date: 2026-03-18

## Summary

`ansorum` is a renamed and actively diverging fork of
`https://github.com/getzola/zola`.

This repository was renamed from `nibzard/zola` to `nibzard/ansorum` on
2026-03-18.

The product direction is now answer-first rather than a general-purpose static
site generator.

## Upstream Base

The current repository history still contains substantial inherited Zola code
and upstream release history through the imported `0.22.x` line.

The upstream `0.22.0` release changed the project licence posture to EUPL-1.2
on 2026-01-09.

Commit `3c9131db0d203640b6d5619ca1f75ce1e0d49d8f` is the relevant split point
called out in this repository's licensing notes.

## Current Licence Posture

This repository should be treated as a mixed-license codebase.

- code introduced before `3c9131db0d203640b6d5619ca1f75ce1e0d49d8f` remains under
  the MIT License
- code introduced from `0.22.0` onward is under the EUPL-1.2

See [README.md](README.md), [LICENSE](LICENSE), and [LICENSE-MIT](LICENSE-MIT)
for the current repository-level explanation.

## Release Identity Reset

Ansorum does not continue the inherited Zola release numbering as a product
signal.

The Ansorum-specific release line resets at:

- `0.1.0-alpha` on 2026-03-18

This reset is a product identity decision, not a claim that the inherited Zola
history disappeared.

## Related Documents

- [ANSWER_COMPILER_SPEC.md](ANSWER_COMPILER_SPEC.md)
- [CLEAN_ROOM_READINESS.md](CLEAN_ROOM_READINESS.md)
