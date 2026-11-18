# Task: remote-api-adapters

## Type
implementation

## Description
Integrate OpenAI and Anthropic APIs through the consent boundary and incorporate additional providers only where the matrix confirms coverage.

Traceability covers RF-046–047 RF-053 RT-024–026.

## Depends On
- `providers-and-consent/remote-consent-gate`
- `search-and-indexing/embedding-index-isolation`

## Likely Files
- Rust remote adapters and simulated provider fixtures

## Progress
- [ ] Complete the work described above.

## Validation
- Pending. Verify provider contracts and approved live capability scenarios, authentication and credential scrubbing.

## Notes
- See the specifications under `requirements/` for detailed acceptance criteria.

## Handoff
- 2026-11-18 codex. Planned and ready once dependencies and implementation autonomy are resolved. Review the cited acceptance criteria, then work in the areas above. Validation is pending.
