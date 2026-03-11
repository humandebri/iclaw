//! where: iclaw/core/src/providers/instructions.rs | what: prompt-guided tool instructions builder | why: non-native-tool providers still need a deterministic tool prompt format

use crate::tools::ToolSpec;
use std::fmt::Write;

pub fn build_tool_instructions_text(tools: &[ToolSpec]) -> String {
    let mut instructions = String::new();
    instructions.push_str("## Tool Use Protocol\n\n");
    instructions.push_str("To use a tool, wrap a JSON object in <tool_call></tool_call> tags:\n\n");
    instructions.push_str("<tool_call>\n");
    instructions.push_str(r#"{"name": "tool_name", "arguments": {"param": "value"}}"#);
    instructions.push_str("\n</tool_call>\n\n");
    instructions.push_str("You may use multiple tool calls in a single response. ");
    instructions.push_str("After tool execution, results appear in <tool_result> tags. ");
    instructions
        .push_str("Continue reasoning with the results until you can give a final answer.\n\n");
    instructions.push_str("### Available Tools\n\n");

    for tool in tools {
        writeln!(&mut instructions, "**{}**: {}", tool.name, tool.description)
            .expect("writing to String cannot fail");
        let parameters =
            serde_json::to_string(&tool.parameters).unwrap_or_else(|_| "{}".to_string());
        writeln!(&mut instructions, "Parameters: `{parameters}`")
            .expect("writing to String cannot fail");
        instructions.push('\n');
    }

    instructions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instructions_embed_tool_names() {
        let tools = vec![ToolSpec {
            name: "memory_store".into(),
            description: "Store a value".into(),
            parameters: serde_json::json!({"type": "object"}),
        }];
        let text = build_tool_instructions_text(&tools);
        assert!(text.contains("memory_store"));
        assert!(text.contains("Tool Use Protocol"));
    }
}
