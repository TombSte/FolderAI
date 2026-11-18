# Task: local-model-adapters

## Type
implementation

## Description
Support validated local embedding, assistant and vision roles with explicit capability and availability reporting.

Traceability covers RF-046 RF-053 RT-024.

## Depends On
- `providers-and-consent/connection-setup-and-secrets`
- `search-and-indexing/embedding-index-isolation`

## Likely Files
- Rust local runtime adapters and provider fixtures

## Progress
- [ ] Complete the work described above.

## Validation
- Pending. Exercise real model roles offline on the support matrix and simulated unavailable runtime and partial capabilities.

## Notes
- See the specifications under `requirements/` for detailed acceptance criteria.

## Handoff
- 2026-11-18 codex. Planned and ready once dependencies and implementation autonomy are resolved. Review the cited acceptance criteria, then work in the areas above. Validation is pending.
