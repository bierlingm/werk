# User Testing

Testing surface: tools, URLs, setup steps, isolation notes, known quirks.

**What belongs here:** How to test the CLI manually, known test data patterns, environment setup for validation.
**What does NOT belong here:** Automated test infrastructure (see services.yaml commands).

---

## Testing Surface

### CLI Binary
- Build: `cargo build -p werk`
- Run from temp dir: `cd $(mktemp -d) && cargo run -p werk -- <subcommand>`

### Test Workspace Setup
```bash
tmpdir=$(mktemp -d)
cd "$tmpdir"
cargo run -p werk -- init
cargo run -p werk -- add "write the novel" "have an outline"
# capture the returned ID for further commands
```

### JSON Output Validation
```bash
cargo run -p werk -- show <id> --json | jq .
cargo run -p werk -- tree --json | jq .
```

### TOON Output Validation (new)
```bash
cargo run -p werk -- show <id> --toon
# Verify TOON syntax: key-value blocks, indentation, proper encoding
```

## Known Quirks
- ID prefix matching requires 4+ characters
- `$EDITOR` integration in note commands requires terminal
- Color output respects NO_COLOR env var
