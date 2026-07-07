# JSRaft

A minimal JavaScript runtime + compiler toolchain + package ecosystem, built in Rust.

## Features

- **Runtime**: Execute JavaScript and TypeScript files with QuickJS engine
- **Bundler**: Experimental single-file bundling by resolving and concatenating modules
- **Linter**: Experimental lightweight lint checks
- **Formatter**: Basic whitespace and line-ending formatting
- **Package Manager**: Early npm registry client and lockfile scaffolding

## Installation

```bash
cargo install --path crates/jsraft-cli
```

## Usage

### Run a file
```bash
jsraft run src/index.js
jsraft run src/index.ts
```

### Start a REPL
```bash
jsraft repl
```

### Bundle your project
```bash
jsraft build --entry src/index.js --outdir dist
jsraft build --format esm --minify
```

### Lint your code
```bash
jsraft lint src/
jsraft lint --fix src/
```

### Format your code
```bash
jsraft fmt src/
jsraft fmt --check src/
```

### Manage packages
```bash
jsraft init
jsraft install
jsraft install lodash
jsraft install typescript --dev
jsraft remove lodash
jsraft search "react"
```

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│                    JSRaft CLI                             │
│  run │ repl │ build │ lint │ fmt │ install │ completions │
├──────────────────────────────────────────────────────────┤
│              Shared Oxc AST Layer                         │
│   oxc_parser → AST → oxc_transformer → oxc_codegen      │
├──────────┬──────────┬───────────┬────────────┬───────────┤
│ Runtime  │ Bundler  │ Linter    │ Formatter  │ Pkg Mgr   │
│ rquickjs │ minimal  │ simple    │ simple     │ npm API   │
│ tokio    │ concat   │ checks    │ whitespace │ oxc_res   │
├──────────┴──────────┴───────────┴────────────┴───────────┤
│              Extension System (ops + JS modules)          │
│   console │ fs │ net │ path │ process │ timers           │
├──────────────────────────────────────────────────────────┤
│              JS Engine: QuickJS (rquickjs)                │
├──────────────────────────────────────────────────────────┤
│              Event Loop: Tokio                            │
└──────────────────────────────────────────────────────────┘
```

## License

MIT
