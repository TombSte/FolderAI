<!-- sdd:start -->
## SDD
When `.ai/sdd/` exists, use the installed `sdd` skill for relevant coding work and SDD requests. Follow project preferences; for tracked work, read feature/task state, follow saved autonomy and update handoff before stopping. Informational requests do not start implementation. If the skill is unavailable, report it; do not invent workflow rules.
<!-- sdd:end -->

## FolderAI agent team

The primary agent owns the task, integrates changes, runs final validation, and updates the handoff for SDD-tracked work. Follow the user's instructions on whether to delegate. Keep ordinary questions and changes with the primary agent.

For approved work, delegate to the named profiles when their criteria apply.

- Use `folderai_implementation` for a second, independent implementation task with its own files and acceptance criteria.
- Use `folderai_visual_qa` when a UI change needs visual or interaction evidence against the approved Workspace sobrio mockup.
- Use `folderai_evidence` when mapping requirements to tests, checking specifications, or collecting validation results is a separate piece of work.
- Use `folderai_architecture` for focused design or review of file replacement and recovery, storage migrations, embedding compatibility, or consent for remote data transfers.

Give each subagent the relevant requirement IDs, paths, and expected result. Keep concurrent write scopes disjoint and delegate from the primary agent only. Collect completed results and close those threads when the client supports it. Report missing model or platform evidence explicitly.
