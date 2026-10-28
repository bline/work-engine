---
name: skill-release
description: Deploy a committed revision of the repository's skills to the Claude and Codex skill directories as an immutable release, and inspect, roll back or retire what agents can discover. Use only when the user decides to release skill changes to active agents.
---

# Skill release

Active agents read skills from `~/.claude/skills` and `~/.codex/skills`. Those
entries are symlinks into a release, not into the working tree, so editing
`skills/` changes nothing an agent reads until a release is deployed.

- A **release** is a read-only export of one commit's `skills/` tree at
  `~/.local/share/work-engine/skill-releases/releases/<commit>`, with a
  `RELEASE.json` recording the commit, the `skills/` tree id and the install
  lists.
- `current` in that directory is a symlink to the active release. Deploy and
  rollback replace it atomically, which switches every agent at once.
- [`skills/catalog.json`](../catalog.json) records each skill's kind, state
  and install targets. It is read from the commit being deployed, so a change
  to what is installed is released like any other skill change. Commits from
  before the catalog are read through their older
  `skills/skill-release/install-manifest.json`.

Run from the repository root; `<skill-dir>` is this skill's directory.

```bash
python3 <skill-dir>/scripts/skill_release.py check          # validate the working tree's catalog; changes nothing
python3 <skill-dir>/scripts/skill_release.py plan [REF]     # what deploying REF would change; changes nothing
python3 <skill-dir>/scripts/skill_release.py deploy [REF] [--replace-copies] [--no-announce]
python3 <skill-dir>/scripts/skill_release.py status [--verify]   # deployed commit, link state, undeployed changes
python3 <skill-dir>/scripts/skill_release.py rollback       # switch back to the previous release
python3 <skill-dir>/scripts/skill_release.py retire TARGET NAME...   # move unmanaged entries to the backup root
```

## The catalog

Each entry has a `kind` (`workflow`, `tool`, `research`, `other-project`), a
`state` (`active`, `stable` meaning unchanged but in use, `dormant`, `retired`)
and its `install` targets. `plan` and `deploy` refuse a commit whose catalog
breaks these rules, and `check` applies them to the working tree:

1. **Entries and skill directories correspond one to one.** A directory without
   a `SKILL.md` is not a skill and does not belong in `skills/`. An
   uncatalogued one would ship with no stated status.
2. **A retired skill is not installed.** Its listing would mislead every worker.
3. **An installed skill that is `dormant`, `research` or `other-project`
   carries an `install_reason`.** Every installed description costs each worker
   about 177 bytes of context, so installing one outside current use is a
   recorded choice.
4. **An `active` or `stable` skill with no install targets carries a
   `not_installed_reason`**, so leaving it out is visibly deliberate.

An entry may also carry `release_exclude`, a list of skill-relative paths
(such as `pilot/`) that releases leave out. Use it for data the skill's scripts
read through the repository path rather than the skill directory: shipping it
would copy it into every release without any agent reading it there.
`RELEASE.json` records the exclusions and the number of files excluded, and
verification compares a release with its commit less those paths. Each path must
name something, and `SKILL.md` cannot be excluded.

Rule 3 allows dormant installs that the layout plan's rule 2 would have
excluded. On 2026-10-27 the user first kept every installed skill installed,
since recording dormant ones as `stable` would misstate their use, and then
uninstalled the five with no load recorded since 2026-10-01.

## Generated command lines

Agents spent whole model calls on `--help`: 1,119 help-only calls between
2026-10-01 and 2026-10-27, 80% of them for five repository scripts. Markdown
blocks between `<!-- cli-synopsis script=... -->` and `<!-- /cli-synopsis -->`
hold those scripts' usage lines, generated from their `--help` output, where the
command is taught. `scripts/cli_synopsis.py write [files]` regenerates them and
`check` reports stale ones; `skill_release.py check` includes it, and a test runs
it on this repository, so a parser change without a regenerated block fails.
Keep blocks in `AGENTS.md` to the commands nearly every session uses: it is
re-sent with every model call. Files under `reviews/` and `records/` are records kept as written,
so the default check skips them; name one to check it.

## Releasing

1. Commit the skill changes, after `check` passes. Deploy accepts only commits.
2. Run `plan` and read it with the user. `LISTING CHANGES` marks a changed
   `name` or `description`, which changes the skill list every new worker
   receives. A running experiment that fixes worker-visible text treats that
   as a treatment change.
3. Deploy only on the user's decision. `--replace-copies` is required when a
   real directory sits where a managed link belongs; the directory is moved to
   `~/cbm-work/skill-release-retired/`, outside every skill directory, never
   deleted.
4. A successful deploy announces the full commit and affected installed skills
   on the repository coordination board by default. New reads follow current
   links; agents must explicitly re-read instructions already loaded. Use
   `--no-announce` to skip publication. A pure same-current redeploy with no
   link changes is not announced. Exit 2 means deployment succeeded but the
   announcement is unconfirmed: inspect the returned message UUID on the board
   before manual recovery. Publication failure does not undo deployment.

Rollback does not announce automatically; post the rolled-back commit and
affected skills to the coordination board manually when appropriate.

Releases are kept after they stop being current: daemons started from a
release keep running its files, and rollback reuses it. Never edit a release in
place; a modified release fails verification.

## Writing skills that release correctly

Write a path an agent will run as `<skill-dir>/scripts/x.py`, or
`<skill-dir>/../other-skill/scripts/z.py` for a sibling skill, and say once that
`<skill-dir>` is the skill's directory. An agent then runs the released copy:
through a skill-directory symlink, `..` resolves inside the release. A path
written from the repository root (`skills/x/scripts/y.py`) runs the working tree
instead, mixing released instructions with unreleased scripts. Markdown links
such as `references/y.md` already resolve relative to their file. Repository
data (`docs/`, `metrics/`, `campaigns/`) stays repository-relative.

Scripts run through a symlinked skill directory. A script that finds a sibling
skill from an unresolved `__file__` looks in the agent directory, so that
sibling must be installed there too; resolve the path first to look in the
release. A Node entry-point check must compare real paths, since Node resolves
symlinks for `import.meta.url` but not for `process.argv[1]`.

Node scripts in a release resolve packages through a `node_modules` link to the
repository's `node_modules`; `RELEASE.json` records the `package-lock.json`
digest at deploy time.
