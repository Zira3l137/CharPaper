# Releasing

Two kinds of builds come out of GitHub Actions, both on the
[Releases page](https://github.com/Zira3l137/CharPaper/releases):

| Build   | Made when                     | Release name       | Notes cover                    |
|---------|-------------------------------|--------------------|--------------------------------|
| Nightly | every push to `main`          | `nightly`, replaced each time | commits since the last version |
| Version | a tag like `v0.2.0` is pushed | `v0.2.0`, kept forever | commits since the previous version |

Each one carries the app for every platform (`charpaper-<label>-windows-x64.zip`), the
Blender add-on (`charpaper_exporter-<label>.zip`) and `SHA256SUMS.txt`.

The notes are written from commit messages, so they read only as well as the messages do.
`feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `build` and `ci` each get a section;
`chore` commits are left out.

## Nightly

Nothing to do: push to `main`. To rebuild without pushing, open
Actions → Nightly → Run workflow.

## A version

Versions follow [semver](https://semver.org). Before 1.0, bump the minor number
(`0.1.0` → `0.2.0`) for anything that breaks existing suites or settings and the patch
number (`0.2.0` → `0.2.1`) for fixes only. A version with a suffix, like `0.3.0-rc.1`,
becomes a pre-release.

1. Make sure `main` builds: the latest Nightly run is green.
2. Set the new version in both places. The release workflow refuses a tag that matches
   neither or only one of them.
   - `Cargo.toml`, `[workspace.package]` → `version = "0.2.0"`
   - `tools/blender/charpaper_exporter/blender_manifest.toml` → `version = "0.2.0"`
3. Update the workspace's own entries in `Cargo.lock`. This touches nothing else and
   compiles nothing; without it the `--locked` build in CI fails.
   ```
   cargo update --workspace
   ```
4. Commit, tag and push the commit and the tag together:
   ```
   git commit -am "chore(release): 0.2.0"
   git tag -a v0.2.0 -m "CharPaper 0.2.0"
   git push --atomic origin main v0.2.0
   ```
5. Watch Actions → Release. When it finishes, the release is on the Releases page.

To preview the notes before tagging, with [git-cliff](https://git-cliff.org) installed
(`cargo install git-cliff`): `git cliff --unreleased --tag v0.2.0`.

### When a release goes wrong

If the workflow fails before publishing, or the published build is broken, remove the
release and the tag, fix `main`, then repeat from step 4 with the same version:

```
gh release delete v0.2.0 --cleanup-tag --yes
git tag -d v0.2.0
```

Without `gh`: delete the release on its page, then `git push origin :refs/tags/v0.2.0`.
Once people have downloaded a version, ship the fix as the next patch version instead.

## Adding a platform

1. Add an entry to the matrix in `.github/workflows/build.yml`. A commented Linux entry is
   already there; its system libraries install in the step guarded by `runner.os`.
2. Give the target its flags in `.cargo/config.toml`, as Windows has.

Nightly and Release both build through `build.yml`, so neither needs changing.
