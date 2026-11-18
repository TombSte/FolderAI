# Task: remote-consent-gate

## Type
implementation

## Description
Resolve the DA-017 lifecycle, then mediate all remote roles by policy, request, provider, scope and data category, including background work and revocation.

Traceability covers DA-017 RF-047 RF-050 RT-025.

## Depends On
- `providers-and-consent/connection-setup-and-secrets`

## Likely Files
- Rust authorization and dispatch boundary, React consent/settings

## Progress
- [ ] Complete the work described above.

## Validation
- Pending. Trace denied, pending, role-mismatched and revoked requests, changed connectors and calls already in flight.

## Notes
- See the specifications under `requirements/` for detailed acceptance criteria.

## Handoff
- 2026-11-18 codex. Planned and ready once dependencies and implementation autonomy are resolved. Review the cited acceptance criteria, then work in the areas above. Validation is pending.
