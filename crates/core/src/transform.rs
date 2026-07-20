use crate::{JsRaftError, Result};
use oxc_allocator::Allocator;
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use oxc_transformer::{TransformOptions, Transformer};
use std::path::Path;

/// Transform TypeScript/TSX source to JavaScript.
pub fn transform_typescript(path: &Path, source: &str) -> Result<String> {
    if !is_typescript_path(path) {
        return Ok(source.to_string());
    }

    let allocator = Allocator::default();
    let source_type = SourceType::from_path(path)
        .map_err(|e| JsRaftError::TypeScriptError(format!("{}: {e}", path.display())))?;
    let parse = Parser::new(&allocator, source, source_type).parse();
    if !parse.diagnostics.is_empty() {
        let diagnostics = parse
            .diagnostics
            .into_iter()
            .map(|d| format!("{d:?}"))
            .collect::<Vec<_>>()
            .join("\n");
        return Err(JsRaftError::TypeScriptError(format!(
            "{}:\n{diagnostics}",
            path.display()
        )));
    }

    let mut program = parse.program;
    let semantic = SemanticBuilder::new()
        .with_excess_capacity(2.0)
        .with_enum_eval(true)
        .build(&program);
    if !semantic.diagnostics.is_empty() {
        let diagnostics = semantic
            .diagnostics
            .into_iter()
            .map(|d| format!("{d:?}"))
            .collect::<Vec<_>>()
            .join("\n");
        return Err(JsRaftError::TypeScriptError(format!(
            "{}:\n{diagnostics}",
            path.display()
        )));
    }

    let scoping = semantic.semantic.into_scoping();
    let transformed = Transformer::new(&allocator, path, &TransformOptions::default())
        .build_with_scoping(scoping, &mut program);
    if !transformed.diagnostics.is_empty() {
        let diagnostics = transformed
            .diagnostics
            .into_iter()
            .map(|d| format!("{d:?}"))
            .collect::<Vec<_>>()
            .join("\n");
        return Err(JsRaftError::TypeScriptError(format!(
            "{}:\n{diagnostics}",
            path.display()
        )));
    }

    Ok(Codegen::new()
        .with_options(CodegenOptions::default())
        .build(&program)
        .code)
}

/// Whether a path should be treated as TypeScript input.
pub fn is_typescript_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("ts" | "tsx")
    )
}
