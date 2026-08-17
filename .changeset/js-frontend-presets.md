---
"purge-deps": patch
---

Default to `js` + `frontend` presets, add `--preset` / `--dry-run`, and treat `-i` as ignore extend.

Breaking notes:

- A no-flag run now deletes frontend caches (`.next`, `.turbo`, JS-package `dist`/`build`, and so on), not only `node_modules` and lockfiles. Use `--preset js` for the old lockfile-oriented scope.
- `.turbo` is a target, not an ignore name.
- `-i` extends the default ignore list. Use `--replace-ignore` to replace it.
- Bun lockfiles (`bun.lock`, `bun.lockb`) are part of the `js` preset.
- `.gitignore` uses globs from `{path}/.gitignore`. Target names are still deleted even when they appear in that file.
- The npm package includes platform binaries (macOS, Linux, Windows) selected by a Node wrapper.
