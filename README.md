# purge-deps

`purge-deps` is a command-line tool designed to delete all files and folders related to dependencies in JavaScript-based monorepo projects. It provides a convenient way to clean up generated files created by package managers.

## Features

- By default, it targets files and folders related to JavaScript package managers.
- If you're not working with JavaScript, you can specify custom targets using the `overwrite` option.

## Installation

You can install `purge-deps` globally using npm:

```bash
npm install -g purge-deps
```

## Usage

You can run the tool using:

```bash
npx purge
```

## Options
```bash
-h, help: Show help information.
-p, path <path>: Specify the path to delete files and folders. Default is the current directory (.).
-e, extends <targets>: Add to the list of targets to delete.
-o, overwrite <targets>: Replace the list of targets to delete.
```

## Default Targets
The default targets include:

- node-modules
- pnpm-lock.yaml
- yarn.lock
- package-lock.json

## Example

To use purge-deps:

```bash
npx purge-deps -p /path/to/your/project -e target,Cargo.lock
```
