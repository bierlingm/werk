# Tool & Framework Evaluations

Tracking potential tools and frameworks for integration with werk-cli and sd-core.

## Incur

**Status:** Evaluation

**What it is:** TypeScript framework for building CLIs that work for both AI agents and humans (dual-mode CLI builder).

**Key Features:**
- Type-first design: Define interfaces with TypeScript, framework generates everything
- Agent-ready: Automatically outputs structured data (JSON, YAML, JSONL, Markdown)
- Human-friendly: Terminal formatting for humans + structured formats for LLMs
- Token awareness: `--token-count` flag for LLM context management
- Multiple output formats: `--format json|yaml|toon|md`
- Schema generation: `--schema` for programmatic parsing
- MCP integration: Register as MCP server
- Shell completions support

**Potential Use Cases:**
- werk-cli command output formatting (structured for agents, readable for humans)
- sd-core CLI interface standardization
- Agent-facing command construction
- Token-efficient output for LLM processing

**Links:**
- `npx incur --help` for command reference
- `npx incur gen` for type generation

**Next Steps:**
- Evaluate if dual-mode output aligns with werk-cli architecture
- Compare with existing CLI patterns in sd-core
- Consider integration for agent-facing commands
