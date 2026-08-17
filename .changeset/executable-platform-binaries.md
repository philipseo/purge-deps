---
"purge-deps": patch
---

Fix `EACCES` when running the CLI. pnpm strips the executable bit from every packed file that is not listed in `bin`, so the platform binaries shipped under `bin/<platform>-<arch>/` arrived without `+x`. They are now declared in `publishConfig.executableFiles`, and the stale committed `bin/purge-deps` is no longer published.
