# purge-deps

`purge-deps` is a command-line tool designed to delete all files and folders related to dependencies in JavaScript-based monorepo projects. It provides a convenient way to clean up generated files created by package managers.

## Installation

You can install `purge-deps` globally using npm

```bash
npm install -g purge-deps
```

## Usage

```bash
npx purge-deps [options]
```

## Options
```bash
-h or help: Displays the usage information.
-p or path <path>: Specifies the path to delete files and folders. Default: .
-t or targets <targets>: Replaces the targets to delete. Multiple targets can be separated by commas. Default: ["node_modules", "pnpm-lock.yaml", "yarn.lock", "package-lock.json"]
-e or extends <targets>: Adds to the targets to delete. Multiple targets can be separated by commas. Default: none
-i or ignore <folders>: Specifies folders to ignore. Multiple folders can be separated by commas. Default: [".changeset", ".husky", ".git", ".github", "src"]
-gi or gitignore <true|false>: Enables or disables reading from the .gitignore file. Default: true
```

## Default Targets
The default targets include:

- node_modules
- pnpm-lock.yaml
- yarn.lock
- package-lock.json

## Examples

```bash
# Basic usage (delete default targets in the current directory)
npx purge-deps [options]

# Delete target files in a specific path
npx purge-deps -p ./path

# Delete a specific target
npx purge-deps -t "test.txt,build"

# Extends targets
npx purge-deps -e "test1.txt,test2.txt,dist"

# Ignore specific folders
npx purge-deps -i "node_modules,build"

# Disable usage of the .gitignore file
npx purge-deps -gi false
```
