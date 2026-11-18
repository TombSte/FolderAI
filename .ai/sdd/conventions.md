# SDD conventions

SDD persists working memory in `.ai/sdd/` so another session can resume without chat history. Features represent user outcomes; tasks are typed activities; context preserves progress and evidence; the index is a derived view.

Use kebab-case names and extensionless `current`/`state`. `current` contains the active feature name or is empty. State records contain `<state> <YYYY-MM-DD> <agent-name>`; use the runtime identity, or `codex` if unknown.

| State | Meaning |
| --- | --- |
| `todo` | Planned, not started |
| `in-progress` | Active, with resumable progress |
| `blocked` | Waiting on a dependency, decision or fix |
| `to-be-validated` | Implemented, awaiting validation/review |
| `done` | Completed and validated |

Before work, inspect current feature/task state. During meaningful progress and before stopping or switching, maintain state and handoff: progress, files, commands/results, blockers/risks/decisions and next step. A pause alone does not prove blockage or completion.

On task changes, synchronize feature state unless the user explicitly selected another valid feature state: any `in-progress`, else any `blocked`, else any `to-be-validated`, else all `done` -> `to-be-validated`, else all `todo` -> `todo`; retain current state for other mixtures. Feature `done` always requires successful validation and explicit final user confirmation.

New work requires repo-grounded intake and an approved task plan. Ask for and store autonomy per approved plan: `continuous`, `per-task`, or `human-checkpoints`; reuse it on resume. All modes honor blockers, necessary human input and final feature confirmation. Task types are `discovery`, `implementation`, `test`, `documentation`, `validation`, `cleanup`.

Natural language and `/sdd` commands operate the same workflow; neither interface is mandatory. Preserve existing project conventions; ask about actual conflicting project restrictions rather than silently rewriting them. Use the feature/task tree as truth, rebuild stale indexes during workflow updates, and report health issues without repairing them on a status read. Require exact-name confirmation before feature deletion.
