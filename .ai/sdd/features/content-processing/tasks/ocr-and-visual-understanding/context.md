# Task: ocr-and-visual-understanding

## Type
implementation

## Description
Add bounded OCR for scans/images and model-backed visual understanding for images, pages and embedded charts, preserving provenance and partial coverage.

Traceability covers RF-039 RF-053 RT-015–017 RT-025.

## Depends On
- `content-processing/extract-text-and-structure`
- `providers-and-consent/local-model-adapters`
- `providers-and-consent/remote-consent-gate`

## Likely Files
- Rust OCR, vision and provenance services

## Progress
- [ ] Complete the work described above.

## Validation
- Pending. Measure approved corpus outputs including visual concepts, offline execution, cancellation and blocked unapproved remote sends.

## Notes
- See the specifications under `requirements/` for detailed acceptance criteria.

## Handoff
- 2026-11-18 codex. Planned and ready once dependencies and implementation autonomy are resolved. Review the cited acceptance criteria, then work in the areas above. Validation is pending.
