# Feature: content-processing

## Goal
Expose precise preview, extraction, OCR and visual coverage for the specified families.

## Problem
Preview availability and extracted or inferred content have different coverage and resource constraints.

## Scope
Format contracts, representative corpus, Details, thumbnails, folder sizes and content pipelines.

## Out of Scope
Office previews and audio/video analysis remain outside scope.

## Acceptance Criteria
All RF-039 rows have explicit full/partial/unavailable behavior, provenance and tested bounds. Previews work locally where supported.

## Constraints and Dependencies
DA-011 and DA-012 determine variants and limits. Visual inference depends on verified provider capabilities.

## Planned Tasks
- [format-coverage-and-limits](tasks/format-coverage-and-limits/context.md) (`discovery`). Version the matrix and representative corpus for encodings, encrypted/malformed inputs, embedded objects, resource limits and coverage; define size and thumbnail cache policies.
- [details-previews-and-thumbnails](tasks/details-previews-and-thumbnails/context.md) (`implementation`). Build the shared Details/Assistant panel, read-only previews, lazy thumbnails and on-demand cancellable folder sizing.
- [extract-text-and-structure](tasks/extract-text-and-structure/context.md) (`implementation`). Extract text and structured references for text/code, CSV/TSV, PDF and Office/OpenDocument families, including embedded visual inputs for downstream processing.
- [ocr-and-visual-understanding](tasks/ocr-and-visual-understanding/context.md) (`implementation`). Add bounded OCR for scans/images and model-backed visual understanding for images, pages and embedded charts, preserving provenance and partial coverage.

## Autonomy
- Mode: `continuous`, selected project-wide by the user on 2026-11-19.
- Checkpoints: feature validation and explicit final user confirmation.

## Definition of Done
- [ ] Behavior implemented
- [ ] Tests updated or reason documented
- [ ] Relevant validation passes
- [ ] Docs updated or reason documented
- [ ] Feature ready for validation
- [ ] User confirms feature completion after successful validation.
