# Feature: assistant-and-auto

## Goal
Answer text requests using approved read-only retrieval, synthesis and agent workflows.

## Problem
Agent and Auto executions need bounded tools, reliable citations and session lifecycle coordination.

## Scope
Tool contracts, assistant UI, orchestration, Auto, citations and closure across windows.

## Out of Scope
File writes, web, terminal, image input and persistent conversation history are outside release scope.

## Acceptance Criteria
Show sourced answers, explicit saved-query execution, safe Auto fallbacks, cancellation and absence of conversation data on disk.

## Constraints and Dependencies
DA-018 sets tool/routing limits. DA-002 sets coordinated closure. All tools recheck DA-013 access and transfers obey DA-017.

## Planned Tasks
- [assistant-tools-and-routing-contract](tasks/assistant-tools-and-routing-contract/context.md) (`discovery`). Define read-only tools, routing and clarification rules, execution budgets, citations and control boundaries.
- [read-only-assistant-runtime](tasks/read-only-assistant-runtime/context.md) (`implementation`). Implement typed read-only tools and bounded request execution for questions, summaries, comparisons and extraction with partials and source revisions.
- [assistant-session-and-citations](tasks/assistant-session-and-citations/context.md) (`implementation`). Add multiline input and scope selection, tab-local conversation, progress and citation navigation. Coordinate tab/window/app close with running transfers and indexing.
- [auto-mode-routing](tasks/auto-mode-routing/context.md) (`implementation`). Route to the least complex adequate authorized approach with validation, clarifications, explicit effective mode and capability alternatives.

## Autonomy
- Mode pending user selection before implementation.
- Checkpoints pending autonomy selection.
- Checkpoint decisions pending.

## Definition of Done
- [ ] Behavior implemented
- [ ] Tests updated or reason documented
- [ ] Relevant validation passes
- [ ] Docs updated or reason documented
- [ ] Feature ready for validation
- [ ] User confirms feature completion after successful validation.
