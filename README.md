# purge-deps

`purge-deps` deletes JavaScript dependency leftovers and frontend build caches in a repo or monorepo. React Native / Expo leftovers are opt-in.

Run `--dry-run` first if you want to see what would be removed.

## Installation

```bash
npm install -g purge-deps
npx purge-deps --dry-run
```

The npm package ships prebuilt binaries for macOS (arm64/x64), Linux (x64/arm64), and Windows (x64).

## Usage

```bash
npx purge-deps [options]
```

With no flags, it walks the current directory using the `js` and `frontend` presets. Names in the ignore list are skipped. `.gitignore` globs under the search path are skipped unless the basename is also a target. `.git` is never deleted.

## Options

```text
-h, --help                         Show help
-p, --path <path>                  Directory to walk (default: .)
    --preset <presets>             Comma-separated presets (default: js,frontend)
-t, --targets <targets>            Replace delete targets; ignores presets
-e, --extends <targets>            Add names to the active preset targets
-i, --ignore <folders>             Add names to the default ignore list
    --replace-ignore <folders>     Replace the ignore list entirely
-g, --gitignore, --gi <true|false> Read `{path}/.gitignore` (default: true)
    --dry-run                      List matches without deleting them
```

`-t` and `-e` cannot be combined. `-i` and `--replace-ignore` cannot be combined.

`-t` and `-e` match basenames exactly. `-e dist` deletes every `dist` folder, including ones the `frontend` preset would leave alone (no `package.json`, or under `android/` / `ios/`).

## Presets

| Preset     | Default | What it matches |
|------------|---------|-----------------|
| `js`       | yes     | `node_modules`, `pnpm-lock.yaml`, `yarn.lock`, `package-lock.json`, `bun.lock`, `bun.lockb` |
| `frontend` | yes     | `.next`, `.nuxt`, `.output`, `.nitro`, `.svelte-kit`, `.vite`, `.turbo`, `.parcel-cache`, `.vercel/output` |
| `mobile`   | no      | `.expo`, `.expo-shared`, `ios/Pods`, `ios/build`, `android/build`, `android/app/build`, `android/.gradle`, `android/.cxx`, `android/app/.cxx` |

`frontend` also uses path-aware rules so generic folder names are not deleted everywhere:

- `dist` / `build` — only when the parent directory has a `package.json`, and the path is not under `android/` or `ios/`
- `out` — only when a sibling `.next` or `next.config.*` exists
- `.vercel/output` — not the whole `.vercel` directory

`mobile` does not delete the `ios/` or `android/` source trees themselves.

## Defaults

| Option     | Default |
|------------|---------|
| `path`     | `.` |
| `preset`   | `js,frontend` |
| `ignore`   | `.changeset`, `.git`, `.github`, `.husky`, `src` |
| `gitignore`| `true` |

## Examples

```bash
# Preview default js+frontend matches
npx purge-deps --dry-run

# Delete default targets in a specific path
npx purge-deps -p ./apps

# JavaScript leftovers only (no .next / dist / .turbo)
npx purge-deps --preset js

# Include Expo / React Native leftovers
npx purge-deps --preset js,frontend,mobile

# Replace targets entirely (presets are ignored)
npx purge-deps -t "coverage,.cache"

# Add extra names on top of the active presets
npx purge-deps -e "tmp,.cache"

# Also skip vendor/ (keeps the default ignore list)
npx purge-deps -i vendor

# Ignore only vendor/
npx purge-deps --replace-ignore vendor

# Do not read .gitignore
npx purge-deps -g false
```
