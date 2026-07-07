# JSRaft

A minimal JavaScript runtime + compiler toolchain + package ecosystem, built in Rust.

## Features

- **Runtime**: Execute JavaScript and TypeScript files with QuickJS engine
- **Bundler**: Bundle your project into a single output file
- **Linter**: 650+ lint rules powered by Oxc
- **Formatter**: Prettier-compatible formatting
- **Package Manager**: npm-compatible dependency management

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
│  run │ build │ lint │ fmt │ install │ test │ fmt-check   │
├──────────────────────────────────────────────────────────┤
│              Shared Oxc AST Layer                         │
│   oxc_parser → AST → oxc_transformer → oxc_codegen      │
├──────────┬──────────┬───────────┬────────────┬───────────┤
│ Runtime  │ Bundler  │ Linter    │ Formatter  │ Pkg Mgr   │
│ rquickjs │ oxc      │ oxc_lint  │ oxc_fmt    │ npm API   │
│ tokio    │          │ 650+ rules│ Prettier   │ oxc_res   │
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
