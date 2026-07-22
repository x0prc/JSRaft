use anyhow::{Context, Result};
use jsraft_core::{extensions, JsRaftError, RuntimePermissions};
use rquickjs::{AsyncContext, AsyncRuntime, CatchResultExt, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Debug, serde::Deserialize)]
struct TestSummary {
    passed: usize,
    failed: usize,
    failures: Vec<TestFailure>,
}

#[derive(Debug, serde::Deserialize)]
struct TestFailure {
    name: String,
    error: String,
}

/// Run JavaScript test files.
pub async fn execute(paths: &[PathBuf], fail_fast: bool) -> Result<()> {
    let files = discover_tests(paths)?;
    if files.is_empty() {
        println!("No test files found");
        return Ok(());
    }

    let mut total_passed = 0;
    let mut total_failed = 0;

    for file in &files {
        println!("\n{}", file.display());
        let summary = run_test_file(file).await?;

        for failure in &summary.failures {
            println!("  FAIL {}", failure.name);
            println!("       {}", failure.error);
        }

        if summary.passed > 0 {
            println!("  passed: {}", summary.passed);
        }
        if summary.failed > 0 {
            println!("  failed: {}", summary.failed);
        }

        total_passed += summary.passed;
        total_failed += summary.failed;

        if fail_fast && total_failed > 0 {
            break;
        }
    }

    println!("\nTest result: {total_passed} passed; {total_failed} failed");
    if total_failed > 0 {
        anyhow::bail!("test run failed");
    }

    Ok(())
}

fn discover_tests(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let roots = if paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        paths.to_vec()
    };

    let mut files = BTreeSet::new();
    for root in roots {
        if root.is_file() {
            if is_test_file(&root) {
                files.insert(root.canonicalize()?);
            }
            continue;
        }

        if !root.is_dir() {
            anyhow::bail!("Test path not found: {}", root.display());
        }

        for pattern in test_patterns(&root) {
            for entry in glob::glob(&pattern).with_context(|| format!("Invalid glob: {pattern}"))? {
                let path = entry?;
                if path.is_file() {
                    files.insert(path.canonicalize()?);
                }
            }
        }
    }

    Ok(files.into_iter().collect())
}

fn test_patterns(root: &Path) -> Vec<String> {
    let root = root.display();
    vec![
        format!("{root}/**/*.test.js"),
        format!("{root}/**/*.spec.js"),
        format!("{root}/**/*.test.mjs"),
        format!("{root}/**/*.spec.mjs"),
    ]
}

fn is_test_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    name.ends_with(".test.js")
        || name.ends_with(".spec.js")
        || name.ends_with(".test.mjs")
        || name.ends_with(".spec.mjs")
}

async fn run_test_file(path: &Path) -> Result<TestSummary> {
    let source = std::fs::read_to_string(path)?;
    let rt = AsyncRuntime::new()
        .map_err(|e| JsRaftError::Runtime(format!("Failed to create runtime: {e}")))?;
    let ctx = AsyncContext::full(&rt)
        .await
        .map_err(|e| JsRaftError::Runtime(format!("Failed to create context: {e}")))?;

    let json = ctx
        .with(|ctx| {
            extensions::console::register(&ctx)?;
            let permissions = RuntimePermissions::default();
            extensions::fs::register(&ctx, &permissions)?;
            extensions::path::register(&ctx)?;
            extensions::process::register(&ctx, &permissions)?;
            extensions::timers::register(&ctx)?;

            let _: Value = ctx
                .eval(test_bootstrap().as_bytes())
                .catch(&ctx)
                .map_err(|e| JsRaftError::JsError(format!("{}: {e}", path.display())))?;

            let source = format!(
                "{}\n//# sourceURL={}\n",
                strip_esm_syntax(&source),
                path.display()
            );
            let _: Value = ctx
                .eval(source.as_bytes())
                .catch(&ctx)
                .map_err(|e| JsRaftError::JsError(format!("{}: {e}", path.display())))?;

            let json: String = ctx
                .eval(test_runner().as_bytes())
                .catch(&ctx)
                .map_err(|e| JsRaftError::JsError(format!("{}: {e}", path.display())))?;

            Ok::<String, JsRaftError>(json)
        })
        .await?;

    Ok(serde_json::from_str(&json)?)
}

fn test_bootstrap() -> &'static str {
    r#"
globalThis.__jsraft_tests = [];
globalThis.test = function test(name, fn) {
  __jsraft_tests.push({ name: String(name), fn });
};
globalThis.assert = {
  ok(value, message) {
    if (!value) throw new Error(message || `Expected truthy value, got ${value}`);
  },
  equal(actual, expected, message) {
    if (actual != expected) throw new Error(message || `Expected ${actual} to equal ${expected}`);
  },
  strictEqual(actual, expected, message) {
    if (actual !== expected) throw new Error(message || `Expected ${actual} to strictly equal ${expected}`);
  },
  throws(fn, message) {
    let threw = false;
    try { fn(); } catch (_) { threw = true; }
    if (!threw) throw new Error(message || "Expected function to throw");
  }
};
"#
}

fn test_runner() -> &'static str {
    r#"
(function runTests() {
  const result = { passed: 0, failed: 0, failures: [] };
  for (const t of __jsraft_tests) {
    try {
      t.fn();
      result.passed++;
      console.log(`  PASS ${t.name}`);
    } catch (error) {
      result.failed++;
      result.failures.push({ name: t.name, error: String(error && error.message ? error.message : error) });
    }
  }
  return JSON.stringify(result);
})()
"#
}

fn strip_esm_syntax(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    for line in source.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("import ") || trimmed.starts_with("export {") || trimmed.starts_with("export *") {
            output.push('\n');
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("export default ") {
            let indent_len = line.len() - trimmed.len();
            output.push_str(&line[..indent_len]);
            output.push_str("const __default = ");
            output.push_str(rest);
            output.push('\n');
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("export ") {
            let indent_len = line.len() - trimmed.len();
            output.push_str(&line[..indent_len]);
            output.push_str(rest);
            output.push('\n');
            continue;
        }
        output.push_str(line);
        output.push('\n');
    }
    output
}
