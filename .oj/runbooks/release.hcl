# Release quench: prep, validate, and publish a new version.
#
# Bumps version, validates locally, pushes, monitors CI, tags,
# waits for release build, and updates the homebrew tap.
#
# Examples:
#   oj run release
#   oj run release --bump minor
#   oj run release --bump major

command "release" {
  args = "[--bump <level>]"
  run  = { pipeline = "release" }

  defaults = {
    bump = "patch"
  }
}

pipeline "release" {
  name      = "release"
  vars      = ["bump"]
  workspace = "ephemeral"

  locals {
    repo        = "$(git -C ${invoke.dir} rev-parse --show-toplevel)"
    prev_version = "$(grep '^version = ' ${invoke.dir}/Cargo.toml | head -1 | sed 's/version = \"\\(.*\\)\"/\\1/')"
    tag_version = "$(git -C ${invoke.dir} tag --sort=-v:refname | grep '^v' | head -1 | sed 's/^v//' || echo '0.0.0')"
  }

  notify {
    on_start = "Release: starting"
    on_done  = "Release: published"
    on_fail  = "Release: failed"
  }

  step "init" {
    run = <<-SHELL
      # Hard prerequisites
      BRANCH=$(git -C "${local.repo}" branch --show-current)
      test "$BRANCH" = "main" || { echo "Not on main branch (on $BRANCH)" >&2; exit 1; }
      git -C "${local.repo}" diff --quiet || { echo "Uncommitted changes in working tree" >&2; exit 1; }
      git -C "${local.repo}" diff --cached --quiet || { echo "Staged changes present" >&2; exit 1; }

      # Create worktree from main
      git -C "${local.repo}" worktree add -b "release-${workspace.nonce}" "${workspace.root}" HEAD
      mkdir -p .cargo
      echo "[build]" > .cargo/config.toml
      echo "target-dir = \"${local.repo}/target\"" >> .cargo/config.toml
    SHELL
    on_done = { step = "prep" }
  }

  step "prep" {
    run     = { agent = "prep" }
    on_done = { step = "validate" }
  }

  step "validate" {
    run     = "make check"
    on_done = { step = "release" }
    on_fail = { step = "fix" }
  }

  step "fix" {
    run     = { agent = "fix" }
    on_done = { step = "validate" }
  }

  step "release" {
    run     = { agent = "release" }
    on_done = { step = "cleanup" }
  }

  step "cleanup" {
    run = <<-SHELL
      git -C "${local.repo}" worktree remove --force "${workspace.root}" 2>/dev/null || true
      git -C "${local.repo}" branch -D "release-${workspace.nonce}" 2>/dev/null || true
    SHELL
  }
}

agent "prep" {
  run     = "claude --model opus --dangerously-skip-permissions --disallowed-tools ExitPlanMode,EnterPlanMode,AskUserQuestion"
  on_idle = { action = "done" }
  on_dead = { action = "escalate" }

  prompt = <<-PROMPT
    Prepare a release for quench.

    ## Context

    - Highest git tag: v${local.tag_version}  (this is the source of truth)
    - Current Cargo.toml version: ${local.prev_version}  (may already be bumped — ignore it)
    - Bump level: ${var.bump}

    ## Tasks

    1. **Bump version** in the workspace root `Cargo.toml`:
       - Compute the new version by applying the bump level to the highest
         **git tag** (v${local.tag_version}), NOT the Cargo.toml or CHANGELOG.
       - "patch": increment patch on the tag (e.g. v0.4.0 → 0.4.1)
       - "minor": increment minor, reset patch (e.g. v0.4.0 → 0.5.0)
       - "major": increment major, reset minor+patch (e.g. v0.4.0 → 1.0.0)
       - Only git tags determine the last released version.

    2. **Generate changelog** entry:
       - Run `git log v${local.tag_version}..HEAD --oneline` to see changes since last release
       - If a CHANGELOG.md exists, prepend a new section for this version
       - If no CHANGELOG.md exists, skip this step

    3. **Commit** the version bump (and changelog if updated):
       - Stage only the changed files: Cargo.toml, Cargo.lock, and CHANGELOG.md if it exists
       - `git commit -m "chore: bump version to <new_version>"`

    When done, say "I'm done".
  PROMPT
}

agent "fix" {
  run     = "claude --model opus --dangerously-skip-permissions --disallowed-tools ExitPlanMode,EnterPlanMode,AskUserQuestion"
  on_idle = { action = "gate", run = "make check", attempts = 3 }
  on_dead = { action = "escalate" }

  prompt = <<-PROMPT
    `make check` failed during release preparation. Fix whatever is broken.

    1. Run `make check` to see the failures
    2. Fix the issues (formatting, clippy, tests, build, audit, etc.)
    3. Commit your fixes with a descriptive message
    4. Run `make check` again to confirm it passes
    5. When everything passes, say "I'm done"
  PROMPT
}

agent "release" {
  run     = "claude --model opus --dangerously-skip-permissions --disallowed-tools ExitPlanMode,EnterPlanMode,AskUserQuestion"
  on_idle = { action = "nudge", message = "Keep working on the release. If waiting for CI, check status with `gh run list --limit 5` and `gh run view <id>`." }
  on_dead = {
    action = "gate"
    run    = "gh release view v$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = \"\\(.*\\)\"/\\1/') --json tagName --jq .tagName 2>/dev/null"
  }

  prompt = <<-PROMPT
    Publish the quench release. You are in a worktree branched from main.

    Read `scripts/release` for context on the existing manual release process.
    You are replacing that script — do the same work but use your judgment to
    iterate on failures instead of aborting.

    ## Step 1: Push to remotes

    Push this branch to main on both remotes:
    ```
    git push origin HEAD:main
    git push github HEAD:main
    ```

    If push fails (non-fast-forward), fetch and rebase:
    ```
    git fetch origin main
    git rebase origin/main
    ```
    Then push again.

    ## Step 2: Monitor CI

    Wait for CI checks to pass on the pushed commit:
    ```
    gh run list --branch main --limit 5
    gh run view <run-id>
    gh run watch <run-id>
    ```

    If CI fails:
    - `gh run view <run-id> --log-failed` to diagnose
    - Fix the issue, commit, and push again
    - Repeat until CI passes

    ## Step 3: Tag and push

    ```
    VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
    git tag -a "v$VERSION" -m "Release v$VERSION"
    git push origin "v$VERSION"
    git push github "v$VERSION"
    ```

    ## Step 4: Wait for release build

    The tag push triggers a GitHub Actions release workflow (builds binaries
    for linux-x86_64, macos-x86_64, macos-aarch64 and creates a GitHub Release):
    ```
    gh run list --branch "v$VERSION" --limit 5
    gh run watch <run-id>
    ```

    If the release build fails, use `gh run view <run-id> --log-failed` to
    investigate. Transient failures can be retried with `gh run rerun <run-id>`.

    ## Step 5: Update homebrew tap

    The homebrew tap formula is at `${local.repo}/../homebrew-tap/Formula/quench.rb`.

    1. Compute the source tarball SHA256:
       ```
       curl -sL "https://github.com/alfredjeanlab/quench/archive/refs/tags/v$VERSION.tar.gz" | shasum -a 256
       ```

    2. Update `Formula/quench.rb`:
       - Set `url` to `https://github.com/alfredjeanlab/quench/archive/refs/tags/v$VERSION.tar.gz`
       - Set `sha256` to the computed hash
       - Do NOT add or change a `version` field (Homebrew extracts it from the URL)

    3. Commit and push:
       ```
       cd ${local.repo}/../homebrew-tap
       git add Formula/quench.rb
       git commit -m "quench $VERSION"
       git push
       ```

    ## Done

    When the release is published and the homebrew tap is updated, say "I'm done".
  PROMPT
}
