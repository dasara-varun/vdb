# Agent Tooling and Development Workflow

This guide records the optional tools used to develop VDB and the rules for using them safely. It is a repository guide, not a runtime dependency: VDB must build, test, and operate without Claude Code, third-party skills, or network access.

## Verified tooling baseline

The following components were inspected and installed in the development environment on 2026-08-24. Versions and source revisions are a snapshot, not a promise that future releases are compatible.

| Component | Source | Installed form | Observed version or revision | Purpose | VDB requirement |
|---|---|---|---|---|---|
| Claude Code | [Anthropic documentation](https://code.claude.com/docs/en/quickstart) | User-level native CLI | `2.1.241` | Coding-agent host | Optional developer tool |
| Bun | [Bun documentation](https://bun.com/docs/installation) | User-level executable | `1.4.0` | gstack build/runtime support | Optional developer tool |
| gstack | [`garrytan/gstack`](https://github.com/garrytan/gstack) | User-level Claude skills plus optional VDB team guidance | `85fd9db` | Planning, review, QA, security, release, browsing, and documentation workflows | Optional; repository mode is non-blocking |
| Superpowers | [`obra/superpowers`](https://github.com/obra/superpowers) | Claude Code marketplace plugin | `6.3.0` | Structured design, planning, TDD, review, and finishing workflow | Optional |
| Claude-Mem | [`thedotmack/claude-mem`](https://github.com/thedotmack/claude-mem) | Claude Code marketplace plugin | `13.15.3` | Persistent local agent-session memory | Optional and privacy-sensitive |
| find-skills | [`vercel-labs/skills`](https://github.com/vercel-labs/skills) | User-level Claude skill | `find-skills` | Discover and inspect candidate skills | Optional |
| Loop Engineering | [`iamarvindh/claude-loop-engineering`](https://github.com/iamarvindh/claude-loop-engineering) | User-level symlinked Claude skill | `513e631` | Design bounded, verifiable agent loops | Optional; never an authorization to run unattended writes |
| Taste Skill | [`Leonxlnx/taste-skill`](https://github.com/Leonxlnx/taste-skill) | User-level Claude skill | `design-taste-frontend` | Anti-slop frontend design guidance | Optional; mostly irrelevant to the Rust CLI |
| Apple Design | [`dickwu/apple-design-skill`](https://github.com/dickwu/apple-design-skill) | User-level Claude skill | `apple-design` | Cross-platform UI and accessibility review | Optional; relevant only to a future VDB GUI |

The Claude Code marketplace plugins are installed for the current user, while VDB’s repository-level `CLAUDE.md` contains only optional gstack guidance. The repository does not vendor these projects or silently install them for other contributors.

## Installation and refresh

A new maintainer should first read [`AGENTS.md`](../AGENTS.md), [`CONTRIBUTING.md`](../CONTRIBUTING.md), and this file. Install only what is needed for the task, inspect the upstream license and setup files, and pin or record the source revision used for a security-sensitive review.

The official Claude Code native installer is documented at [code.claude.com/docs](https://code.claude.com/docs/en/quickstart). The verified plugin commands used for the current environment were:

```text
/plugin marketplace add obra/superpowers-marketplace
/plugin install superpowers@superpowers-marketplace
/plugin marketplace add DietrichGebert/ponytail
/plugin install ponytail@ponytail
/plugin marketplace add thedotmack/claude-mem
/plugin install claude-mem@thedotmack
```

For gstack, use a user-owned checkout and run its setup after reviewing the release:

```bash
git clone --single-branch --depth 1 https://github.com/garrytan/gstack.git ~/.claude/skills/gstack
cd ~/.claude/skills/gstack
./setup --host claude --no-prefix
```

The VDB repository uses gstack’s optional team mode, which is intentionally non-blocking. A contributor who wants the same skills can run `./setup --team` from the gstack checkout. A contributor who uses another coding agent should follow the repository’s `AGENTS.md` and `CONTRIBUTING.md` instead of installing Claude-specific tooling.

The Vercel skill installer can install the verified find-skills skill and the design skills:

```bash
npx --yes skills add vercel-labs/skills --skill find-skills --agent claude-code --global --yes
npx --yes skills add https://github.com/Leonxlnx/taste-skill --skill design-taste-frontend --agent claude-code --global --yes
npx --yes skills add https://github.com/dickwu/apple-design-skill --agent claude-code --global --yes
```

Loop Engineering is maintained as a user-level checkout rather than copied into VDB:

```bash
git clone --single-branch --depth 1 https://github.com/iamarvindh/claude-loop-engineering.git ~/.local/share/vdb-agent-tools/loop-engineering
mkdir -p ~/.claude/skills
ln -sfn ~/.local/share/vdb-agent-tools/loop-engineering/skill ~/.claude/skills/loop-engineering
```

These commands are examples for a trusted developer workstation. They are not run by VDB’s build, CI, release artifacts, or end-user installer.

## Security and privacy rules

Third-party agent tools run with the permissions of the host agent. Review upstream source, license, release history, hooks, scripts, network behavior, and generated files before installation. Do not install a skill solely because a search result recommends it, and do not place unreviewed community skills in a repository-wide mandatory path.

Claude-Mem can capture prompts, tool observations, code context, and summaries. Treat its local database as sensitive project data. Do not enable cloud synchronization for proprietary or personal data without an explicit decision. Use its privacy controls for secrets, keep its data directory owner-only, and never include Claude-Mem state in VDB commits or release archives.

Do not put API keys, tokens, private database contents, customer data, or credentials in prompts, progress notes, ADRs, screenshots, logs, or agent memory. Rotate a credential immediately if it appears in an agent transcript or generated artifact. Run VDB’s normal validation independently of any agent skill, because a skill’s success message is not evidence that the database is correct.

Loop tools must use worktree isolation, a verifiable stop condition, a turn/time bound, a maker/checker separation, a durable state file, and an escalation path. They may prepare a branch or draft pull request, but they must not auto-merge, force-push, delete data, change production configuration, or mutate VDB storage without explicit authorization and independent verification.

The `Taste Skill` and `Apple Design` skill are review aids only. They must not cause a future GUI project to bypass accessibility, privacy, security, or functional tests. gstack’s browser-oriented skills must never be used to upload VDB data to external services unless the owner explicitly authorizes that action.

## Update policy

Review this guide whenever a tool is added, removed, upgraded, moved from optional to required, or found to access new permissions or external services. Record the date, source URL, revision, installation scope, security review, and rollback procedure in a progress note or ADR. Keep the repository optional and buildable without these tools.

A future agent must not silently replace the tool list with a different project that merely has a similar name. When an ambiguous request such as “taste,” “Apple design,” “loop engineering,” or “skill finder” appears, resolve the exact source, inspect it, and record the choice before installation.

## References

1. [Claude Code Quickstart](https://code.claude.com/docs/en/quickstart)
2. [Claude Code plugins](https://code.claude.com/docs/en/plugins)
3. [gstack](https://github.com/garrytan/gstack)
4. [Superpowers](https://github.com/obra/superpowers)
5. [Claude-Mem](https://github.com/thedotmack/claude-mem)
6. [Vercel Skills](https://github.com/vercel-labs/skills)
7. [Loop Engineering](https://github.com/iamarvindh/claude-loop-engineering)
8. [Taste Skill](https://github.com/Leonxlnx/taste-skill)
9. [Apple Design Skill](https://github.com/dickwu/apple-design-skill)
10. [Bun installation](https://bun.com/docs/installation)

_Last reviewed: 2026-08-24._
_Last reviewed by: Manus AI._
_Last review evidence: local environment inventory, upstream repository inspection, and successful Claude Code plugin/skill installation._
