---
name: publish
description: Publish a new mengdie release to the public GitHub remote. Triggers when the user says "release vX.Y.Z", "publish vX.Y.Z", or "/publish". Squashes the current private main into a single commit on public-main, pushes to github main, tags, and watches the multi-platform release build.
---

# /publish — Mengdie Public Release Workflow

Mengdie has two remotes (Pattern A, single repo, dual remote):

- **`origin`** = Forgejo private — every dev commit lands here.
- **`github`** = public — **only release versions**, one squash commit per release.

This skill orchestrates the publish flow. Run it when a new version is ready
to go out. Default scope: the requested version only — do not modify main
history, do not bump version inside this skill (do that beforehand on main).

## Tag philosophy

**Version tags (`vX.Y.Z`) are `/publish`-only artifacts.** They are created
on `public-main` (the release branch) at publish time and pushed to GitHub.
They are NOT created on `main` (private dev). The Forgejo origin should
never carry new `vX.Y.Z` tags going forward — `git fetch origin` should
not contaminate the local tag namespace with private-side aliases.

Tag namespace rule: **`vX.Y.Z` = what users see on GitHub**, full stop.

One-time setup (per clone): disable tag fetch from Forgejo to prevent
historical pre-rule tags or accidental private tags from drifting into
the local namespace where they'd collide with publish-created tags:

```bash
git config remote.origin.tagopt --no-tags
```

(GitHub tags are always pulled explicitly via `git push github vX.Y.Z` and
`git fetch github --tags` if you need them locally; they are never
auto-pulled because GitHub is a push-only relationship for this repo.)

## Args

- `<version>` (positional, required) — release tag, e.g. `v0.0.3`.
  Format: `^v\d+\.\d+\.\d+(-[a-z0-9.-]+)?$`. Refuse if format mismatches.
- `--dry-run` (optional) — print every command without executing.

## Pre-checks (fail-fast, refuse to proceed if any fails)

1. `git rev-parse --abbrev-ref HEAD` → `main`. If on another branch, stop.
2. `git status --porcelain` empty. If dirty, stop.
3. `git fetch --multiple origin github` succeeds.
   (Note the `--multiple` flag; `git fetch origin github` without it is
   interpreted as "fetch ref `github` from `origin`", not "fetch from two
   remotes" — that was the v0.0.2 bootstrap mistake.)
4. `git rev-list --count origin/main..main` is `0` and
   `git rev-list --count main..origin/main` is `0` — i.e. `main` is in
   sync with `origin/main`. If ahead, push first; if behind, pull. Either
   way: don't publish stale state.
5. `Cargo.toml` `version` field equals `<version>` with the leading `v`
   stripped. If mismatched: stop with diff. The version bump is a
   normal commit on main before invoking this skill.
6. `CHANGELOG.md` contains a heading `## <version>` or
   `## <version> — <date>`. If absent: stop. Public-facing release notes
   are required.
7. `cargo test --all-targets` passes. **Note: `--all-targets` is required**;
   plain `cargo test` only runs library unit tests, leaving the integration
   suites under `tests/` unexercised — they're what catches MCP wire-format
   and schema-migration regressions.
8. `cargo clippy --all-targets -- -D warnings` passes.
9. `gh release view <version> --repo xmkevinchen/mengdie` returns
   not-found (release not yet published). This is the **authoritative**
   "already released?" check — GitHub is the source of truth for what
   has been published. (Local tag presence alone is not a reliable
   signal: it could be a stale alias from a bootstrap scenario, a
   leftover from a rolled-back release, etc. See "Bootstrap special
   cases" below for handling tag-name collisions.)

## Squash + push + tag

Extract `<theme>` = first line under the `## <version>` heading in
`CHANGELOG.md` (typically `**Theme**: ...`).

```bash
# 1. (re)Sync private side — already covered in pre-check 4, but recover
#    if it changed between check and execute
git push origin main

# 2. Switch to the public mirror branch
git checkout public-main

# 3. Linear-only pull from github main (no-op for solo dev; safety net)
git pull --ff-only github main

# 4. Squash-merge main's current tree onto public-main
git merge --squash main

# 5. Commit (single linear release commit on public-main)
git commit -m "Release <version> — <theme>"

# 6. Push public-main → github main
git push github public-main:main

# 7. Tag the release on public-main (NOT on private main)
git tag -a <version> -m "<version>"

# 8. Push the tag — triggers .github/workflows/release.yml multi-platform
#    build + asset upload
git push github <version>

# 9. Return to private main for continued dev
git checkout main
```

## Watch the release build

```bash
# Find the just-triggered release.yml workflow run
RUN_ID=$(gh run list --repo xmkevinchen/mengdie --workflow release.yml \
  --limit 1 --json databaseId --jq '.[0].databaseId')

gh run watch "$RUN_ID" --repo xmkevinchen/mengdie --interval 30
```

Expected: 5-15 min. The matrix builds 5 platforms in parallel, each
uploads its archive via `gh release upload --clobber`.

## Verify outcome

```bash
gh release view <version> --repo xmkevinchen/mengdie \
  --json assets --jq '.assets | map(.name)'
```

Expected assets (exact 4):

- `mengdie-<version>-linux-amd64.tar.gz`
- `mengdie-<version>-linux-arm64.tar.gz`
- `mengdie-<version>-darwin-arm64.tar.gz`
- `mengdie-<version>-windows-amd64.zip`

(Intel macOS / `darwin-amd64` was dropped from the matrix after v0.0.2
publish — GitHub's macos-13 hosted runner pool reliably stalls jobs in
`queued`. Add it back when macOS Intel becomes practical or if a user
specifically asks for that target.)

If fewer than 4: dig into the failed matrix job
(`gh run view "$RUN_ID" --repo xmkevinchen/mengdie --log-failed`).

## Rollback

If the release is botched (CI fails, wrong assets, wrong content) and
you need to retract:

```bash
gh release delete <version> --repo xmkevinchen/mengdie --yes --cleanup-tag
git tag -d <version>
# If you want to also un-publish the squash commit on public-main:
git checkout public-main
git reset --hard HEAD~1
git push github public-main:main --force-with-lease
git checkout main
```

`--cleanup-tag` deletes the remote tag (which is what triggered release.yml);
local tag is deleted separately.

### Nuke-and-redo recovery

When `public-main` ends up in a stuck or wrong state (cherry-pick conflict
left mid-merge, force-push silently no-op'd because local HEAD didn't
actually advance, tag pointing at the wrong commit because of a script
mishap), **do not try to surgically fix `public-main`**. By construction
`public-main` is just a squash mirror of `main` plus a tag — it contains
zero unrecoverable state. Nuke it and rebuild:

```bash
# 1. Make sure main has every fix you want to publish, committed and
#    pushed to origin. (The fix on main is the source of truth.)
git checkout main
git push origin main

# 2. Delete the local public-main branch outright
git branch -D public-main

# 3. Recreate as a fresh orphan from main HEAD (working tree + index
#    populated automatically; no cherry-pick / merge / conflict path)
git checkout --orphan public-main
git commit -m "Initial public release — mengdie <version>"

# 4. Force-push the rebuilt orphan to GitHub
git push github public-main:main --force-with-lease

# 5. If a wrong-commit tag was already pushed, delete it cleanly first
#    (these can run in any order with step 4)
gh release delete <version> --repo xmkevinchen/mengdie --yes --cleanup-tag
git tag -d <version>

# 6. Retag on the new public-main HEAD
git tag -a <version> public-main -m "<version> — <theme>"
git push github <version>

# 7. Return to private main
git checkout main
```

**Why this is safe**: every byte on `public-main` is derived from
`main`'s current tree by the squash rule; there is no information on
`public-main` that isn't reproducible from `main`. Burning it down and
re-creating from main is therefore semantically identical to a "clean
publish from scratch" — strictly less risky than partial-state surgery.

**When to reach for nuke-and-redo instead of surgical rollback**:
- Any time you've been in `cherry-pick`, `merge`, or `rebase` on
  `public-main` and aren't 100% sure of the resulting state.
- After a `git push --force-with-lease` that printed
  `Everything up-to-date` when you expected to push new content (silent
  no-op: local HEAD didn't actually move forward).
- After a tag push where you later realize the tag points at the wrong
  commit.

**Operational tip**: when running this skill's destructive sequences,
chain commands with `&&` (or `set -e` at script top) so a single
failure aborts the whole sequence. Unconditional newline-separated
commands will keep marching past a `cherry-pick` conflict and leave a
half-applied state — the exact failure mode that produced the v0.0.2
re-deploy mishap.

## Bootstrap special cases

### Tag-name collision with a pre-existing Forgejo tag

**Symptom**: the requested `<version>` already exists as a tag on
Forgejo (or in the local tag namespace) pointing at a commit that is
NOT on `public-main` — typically a private-side merge commit from
before the dual-remote topology was set up. Pre-check 9 may pass (no
GitHub release yet) but `git tag <version>` later fails with
`fatal: tag already exists`.

**Pattern (rename Forgejo's tag, free the namespace)**:

```bash
# Inspect what the pre-existing tag points to
git cat-file -p <version> | head -10

# Create a clearly-named historical alias at the same SHA, preserving
# the annotated tag body so the original release notes survive
ORIG_SHA=$(git rev-list -n 1 <version>)
ORIG_BODY=$(git cat-file -p <version> | tail -n +6)   # skip object/type/tag/tagger/blank
git tag -a <version>-internal "$ORIG_SHA" -m "$(printf '%s pre-public-mirror Forgejo ship (renamed from %s)\n\nOriginal tag body:\n\n%s' "$version" "$version" "$ORIG_BODY")"
git push origin "$version"-internal

# Delete the colliding tag — Forgejo first, then local
# (DESTRUCTIVE — requires explicit operator approval)
git push origin :refs/tags/<version>
git tag -d <version>

# Now <version> is free; resume the normal /publish flow from
# "Squash + push + tag" step 4 onward.
```

**Why this works**: Forgejo's `<version>` tag is preserved under
`<version>-internal` (same SHA, same annotated body, recoverable via
`git push origin <version>-internal:refs/tags/<version>` if you ever
need to restore it). The freed name is now available for a new
`<version>` pointing at `public-main`'s squash commit.

**This case only applies to versions tagged before the tag-philosophy
rule was adopted** (e.g. mengdie's v0.0.1, v0.0.2). All versions
created via this skill ship with their tags exclusively on public-main
+ GitHub, so this collision class does not recur for them.

### GITHUB_TOKEN permission for release.yml

The `release.yml` workflow uses `gh release create` + `gh release upload`.
Both require `contents: write` on the workflow's `GITHUB_TOKEN`; the
default token permission is read-only. Failure mode is:

```
HTTP 403: Resource not accessible by integration
  https://api.github.com/repos/<owner>/<repo>/releases
```

The fix is a workflow-level `permissions:` block in `.github/workflows/release.yml`:

```yaml
permissions:
  contents: write
```

This was missing on the v0.0.2 bootstrap publish; the failure mode is
caught by pre-check 7 (cargo test) only AFTER tag push, so it's a
runtime failure on first deploy of any repo with this workflow. Verify
the block is present before invoking `/publish` on a fresh GitHub repo.

### History-import risk if you reuse a private-side tag

**Do not** `git push github <existing-private-tag>` to publish a
release — the tag points to a commit on private `main`, and pushing
the tag transfers the underlying commit plus its entire ancestor
chain. That would import the private R&D history into GitHub (the
exact noise the orphan squash was designed to exclude). Always
create the tag fresh on `public-main` after a squash-merge.

## Why this shape

- **One commit per release on public-main**: the user reads GitHub history
  as a stream of releases, not internal R&D iterations. Squash discards the
  noise; the linear chain preserves the release lineage.
- **Tag lives on public-main, not main**: the release event (tag push) is
  a publish action; tying it to public-main keeps the publish/dev concerns
  separated. Private main can advance freely without polluting tag history.
- **Pre-checks are fail-fast and explicit**: a botched release is hard to
  retract once binaries are downloaded. Better to refuse on a stale CHANGELOG
  than to publish wrong notes.
- **`gh release view` is the canonical "already released?" check**: more
  reliable than local tag presence because local tags can be stale
  aliases, half-rolled-back state, or bootstrap artifacts.

## What this skill does NOT do

- Does not bump `Cargo.toml` version — that is a normal dev commit on `main`.
- Does not write `CHANGELOG.md` entries — write the entry on `main` first,
  then invoke this skill.
- Does not strip files between main and public-main — anything that should
  be excluded from public is already gitignored on the maintainer side
  (`.ae/`, `CLAUDE.local.md`, `.claude/pipeline.local.yml`, `.claude/agents/`).
  If a NEW path needs stripping, add it to `.gitignore` (and `git rm` from
  history if it was previously tracked) — do not silently filter inside this
  skill.
- Does not create or modify Forgejo tags — Forgejo's tag namespace is left
  alone after the bootstrap (only `v0.0.1` + `v0.0.2-internal` should exist
  there as historical artifacts).
