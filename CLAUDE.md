# Agent Instructions

This project uses **bd** (beads) for issue tracking. Run `bd onboard` to get started.

## Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --status in_progress  # Claim work
bd close <id>         # Complete work
bd sync               # Sync with git
```

## Mandatory Tools and Workflow

### PAL Code Review & Pre-commit

**Before committing ANY code changes**, you MUST use PAL's pre-commit validation:

```bash
# Use PAL precommit to validate changes before commit
mcp__pal__precommit ...
```

This ensures:
- Code quality checks are performed
- Security vulnerabilities are caught
- Changes are properly validated
- Review covers quality, security, performance, and architecture

**For code reviews**, use PAL's code review tool:

```bash
mcp__pal__codereview ...
```

### Serena MCP Integration

This project uses **Serena MCP** for codebase navigation and semantic operations. Serena provides:

- **Symbol-based code navigation** - Find classes, methods, functions semantically
- **Precise code editing** - Edit at symbol level (not just text-based)
- **Intelligent codebase understanding** - Understand relationships between symbols

**Key Serena tools:**
- `find_symbol` - Locate symbols (classes, methods, functions) by name path
- `find_referencing_symbols` - Find where symbols are used
- `replace_symbol_body` - Replace entire symbol definitions
- `insert_before_symbol` / `insert_after_symbol` - Insert code relative to symbols
- `search_for_pattern` - Flexible pattern-based search
- `get_symbols_overview` - Get high-level understanding of file structure

**IMPORTANT:** Always prefer Serena's semantic tools over raw text editing for code changes.

## Planning Workflow: Rolling Wave

This project uses **rolling wave planning** for implementation:

- **Epic N**: Detailed plan with tasks, dependencies, acceptance criteria
- **Epic N+1**: Theme only (title, goal, rough scope)
- **No pre-declaring total epics** — let work emerge naturally
- **Re-evaluate after each epic** — adjust based on what you learned

**Process:**
1. Plan Epic 1 (detailed) + Epic 2 (theme only)
2. Implement Epic 1
3. Close Epic 1 → Detail Epic 2 → Add Epic 3 (theme only)
4. Repeat until phase complete

**Documentation:**
- Epic plans live in `docs/plans/YYYY-MM-DD-epicN-<name>.md`
- Each plan includes: goal, scope, tasks, acceptance criteria, next epic theme

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd sync
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds

