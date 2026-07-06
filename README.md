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

## Project Structure

```
my-project/
├── jsraft.toml          # Project configuration
├── jsraft.lock          # Lock file
├── src/
│   └── index.ts        # Entry point
└── dist/               # Build output
```

## Configuration

### jsraft.toml

```toml
name = "my-project"
version = "0.1.0"

[dependencies]
lodash = "^4.17.0"

[dev_dependencies]
typescript = "^5.0.0"

[build]
entry = ["src/index.js"]
outdir = "dist"
format = "esm"
minify = true
sourcemap = true

[lint]
categories = ["correctness", "suspicious", "pedantic"]
max_warnings = 100

[fmt]
print_width = 80
tab_width = 2
use_tabs = false
semicolons = true
single_quotes = false
trailing_commas = true
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

## Roadmap

### Phase 1: Core Runtime
- [x] QuickJS engine integration
- [x] Basic event loop (Tokio)
- [x] Module system (ESM loader)
- [x] Console API
- [x] File system APIs (Deno-compatible)
- [x] Timer APIs (setTimeout, setInterval)
- [x] Process APIs

### Phase 2: Tooling
- [x] TypeScript transpilation (Oxc)
- [x] Module resolver (oxc_resolver)
- [x] Linter (oxc_linter)
- [x] Formatter (Oxc codegen)
- [x] Bundler (custom)
- [x] Package manager (npm compatible)

### Phase 3: Ecosystem
- [ ] HTTP server (axum)
- [ ] WebSocket support
- [ ] Native addon support (NAPI)
- [ ] Permissions/security model
- [ ] Web Workers
- [ ] Test runner
- [ ] REPL

### Phase 4: Performance
- [ ] V8 upgrade path (rusty_v8)
- [ ] Source maps
- [ ] Snapshot support
- [ ] io_uring (Linux)
- [ ] Memory-mapped file reads

## License

MIT
