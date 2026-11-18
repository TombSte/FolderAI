# Task: read-only-assistant-runtime

## Type
implementation

## Description
Implement typed read-only tools and bounded request execution for questions, summaries, comparisons and extraction with partials and source revisions.

Traceability covers RF-041–044 RT-019 RT-027 RT-029–030.

## Depends On
- `assistant-and-auto/assistant-tools-and-routing-contract`
- `search-and-indexing/semantic-search-and-mode-recovery`
- `providers-and-consent/local-model-adapters`
- `providers-and-consent/remote-api-adapters`

## Likely Files
- Rust orchestrator and tools, simulated provider fixtures

## Progress
- [ ] Complete the work described above.

## Validation
- Pending. Exercise unauthorized paths, document instructions, malformed tool calls, limits, cancellation and unavailable/mutated sources.

## Notes
- See the specifications under `requirements/` for detailed acceptance criteria.

## Handoff
- 2026-11-18 codex. Planned and ready once dependencies and implementation autonomy are resolved. Review the cited acceptance criteria, then work in the areas above. Validation is pending.
