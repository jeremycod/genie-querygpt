# Genie QueryGPT (Rust)

A natural language to SQL system that converts plain English queries into validated SQL for the Genie database schema.

## Overview

QueryGPT translates natural language prompts into SQL queries by:
1. Understanding the user's intent
2. Generating a structured ReportSpec (internal DSL)
3. Compiling and validating against the schema
4. Rendering production-ready SQL

**Key Features:**
- 🤖 **LLM-powered planning** with OpenAI integration
- 📊 **Full schema awareness** with rich metadata and descriptions
- ✅ **Validation pipeline** ensures only valid SQL is generated
- 🔄 **Retry loop** with compiler diagnostics feedback to LLM
- 🧪 **Deterministic testing** with fixture-based planner
- 📝 **Example-driven learning** guides LLM with query patterns

## Quick Start

### Prerequisites
- Rust 1.92.0+
- OpenAI API key (optional, for LLM-powered queries)

### Installation

```bash
# Build the project
cargo build

# Run tests
cargo test

# Run the CLI
cargo run -p querygpt-cli -- plan --prompt "show all campaigns" --yes
```

### Using OpenAI Integration

```bash
# Set your API key
export OPENAI_API_KEY=sk-your-key-here

# Generate SQL from natural language
cargo run -p querygpt-cli -- plan --prompt "show active offers ordered by name" --openai --yes
```

## Example Queries

### Basic Queries
```bash
# Simple select
cargo run -p querygpt-cli -- plan --prompt "show all offers" --openai --yes

# With filter
cargo run -p querygpt-cli -- plan --prompt "show active campaigns" --openai --yes

# With ordering
cargo run -p querygpt-cli -- plan --prompt "list offers ordered by name" --openai --yes
```

### Advanced Queries
```bash
# Date filtering
cargo run -p querygpt-cli -- plan --prompt "show offers starting after 2024-01-01" --openai --yes

# Geographic filtering
cargo run -p querygpt-cli -- plan --prompt "show offers available in US" --openai --yes

# Multiple filters
cargo run -p querygpt-cli -- plan --prompt "show active offers starting after 2024" --openai --yes
```

### Interactive Mode
```bash
# Without --yes flag, you'll be prompted to approve the generated SQL
cargo run -p querygpt-cli -- plan --prompt "show all campaigns" --openai
```

## Architecture

### Project Structure

```
genie-querygpt/
├── crates/
│   ├── querygpt-core/      # Core library
│   │   ├── dsl/             # ReportSpec DSL and IntermediatePlan
│   │   ├── schema/          # Schema registry and cards
│   │   ├── planner/         # LLM integration and orchestration
│   │   ├── compile/         # ReportSpec → IntermediatePlan compiler
│   │   └── sql/             # SQL renderer
│   └── querygpt-cli/        # Command-line interface
├── config/
│   └── workspaces/          # Schema definitions
│       ├── *.index.json         # Workspace index
│       └── *.schema_cards.json  # Entity schemas with metadata
└── docs/                    # Documentation
```

### Data Flow

```
Natural Language
      ↓
  LLM Planner (OpenAI / Fixtures)
      ↓
  ReportSpec (validated JSON)
      ↓
  Compiler (with diagnostics)
      ↓
  IntermediatePlan
      ↓
  SQL Renderer
      ↓
  Production SQL
```

### Key Components

#### 1. Schema System
- **SchemaRegistry**: Loads workspace schemas from JSON config files
- **SchemaCards**: Entity definitions with columns, types, descriptions
- **SchemaSummary**: Simplified schema view for LLM context
- **Join Graph**: Valid table relationships and join conditions

#### 2. Planner System
- **LlmPlanner**: OpenAI integration with structured prompts
- **FixturePlanner**: Deterministic test planner with hardcoded responses
- **PlannerContext**: Schema summary + examples + constraints
- **Orchestrator**: Coordinates planning, compilation, and confirmation

#### 3. Validation Pipeline
- **ReportSpec**: High-level query specification (workspace, select, filters, order_by)
- **Compiler**: Validates ReportSpec against schema, generates diagnostics
- **IntermediatePlan**: Validated query plan with resolved joins and fields
- **Retry Loop**: LLM receives diagnostics and can fix errors (max 3 attempts)

#### 4. User Interaction
- **InteractiveConfirmation**: Shows diff and prompts user for approval
- **AutoApproveConfirmation**: Skips confirmation (with --yes flag)
- **FlowLogger**: Traces execution with [flow] logs

## Configuration

### Workspace Schema

Schemas are defined in `config/workspaces/`:

**Index File** (`campaigns_offers.index.json`):
```json
{
  "workspace": "campaigns_offers",
  "description": "Campaigns & Offers reporting workspace",
  "schema_cards_path": "config/workspaces/campaigns_offers.schema_cards.json",
  "entities": ["offers_latest", "campaigns_latest", ...]
}
```

**Schema Cards** (`campaigns_offers.schema_cards.json`):
```json
{
  "entities": [
    {
      "name": "offers_latest",
      "description": "Latest version of each offer",
      "columns": [
        {
          "name": "id",
          "data_type": "varchar",
          "nullable": false,
          "description": "Offer identifier"
        },
        {
          "name": "countries",
          "data_type": "varchar[]",
          "nullable": true,
          "description": "Countries where offer is available"
        }
        ...
      ]
    }
  ],
  "join_graph": { ... }
}
```

### Example Queries

The CLI includes example queries to guide the LLM (see `crates/querygpt-cli/src/main.rs:196-316`):
- Simple select queries
- Boolean filters (active/deleted)
- Date comparisons (gte, lte)
- Ordering (asc, desc)

## CLI Reference

### Commands

```bash
querygpt plan [OPTIONS] --prompt <PROMPT>
```

### Options

- `--prompt <PROMPT>` - Natural language query (required)
- `--yes` - Auto-approve without confirmation
- `--openai` - Use OpenAI instead of fixture planner
- `--max-attempts <N>` - Maximum retry attempts (default: 3)
- `--explain` - Show detailed explanation (not yet implemented)

### Environment Variables

- `OPENAI_API_KEY` - OpenAI API key for LLM integration

## Testing

### Unit Tests
```bash
# Run all tests
cargo test

# Run core library tests only
cargo test -p querygpt-core

# Run with output
cargo test -- --nocapture
```

### End-to-End Testing

```bash
# Test with fixture planner (deterministic, no API key needed)
cargo run -p querygpt-cli -- plan --prompt "show all campaigns" --yes

# Test with OpenAI (requires API key)
export OPENAI_API_KEY=sk-your-key-here
cargo run -p querygpt-cli -- plan --prompt "show active offers" --openai --yes
```

See `docs/testing_guide.md` for comprehensive test scenarios.

## Development Status

### ✅ Completed (Phase 1)

- [x] Core DSL (ReportSpec, IntermediatePlan)
- [x] Schema registry and cards system
- [x] Compiler with validation and diagnostics
- [x] SQL renderer
- [x] LLM integration (OpenAI)
- [x] Orchestration flow with retry loop
- [x] Interactive confirmation with diff display
- [x] CLI interface
- [x] Full schema integration (all fields, descriptions)
- [x] Example-driven learning
- [x] Comprehensive test suite (58+ tests)

### 🚧 Planned (Future Phases)

- [ ] Timeout handling for LLM calls
- [ ] Better error messages and logging
- [ ] Safety guardrails (secret redaction, prompt size caps)
- [ ] Performance optimizations (circuit breaker, backoff)
- [ ] Geographic/array filter support (countries overlaps)
- [ ] JOIN support across multiple tables
- [ ] Aggregation functions (COUNT, SUM, AVG)
- [ ] GROUP BY support
- [ ] HTTP API server (querygpt-server)
- [ ] Worker for MV refreshes (querygpt-worker)

### 🔬 Experimental

- [ ] Multi-table queries
- [ ] Complex nested filters
- [ ] Derived field support
- [ ] JSONB path extraction

## Contributing

### Code Standards

This project follows Rust best practices:

```bash
# Format code
cargo fmt

# Lint code
cargo clippy --all-targets --all-features -- -D warnings

# Run tests
cargo test
```

See `CLAUDE.md` for detailed development guidelines.

### Workflow

1. Make changes in small, focused commits
2. Run `cargo fmt` → `cargo clippy` → `cargo test`
3. Create PR with clear description
4. All tests must pass

## Documentation

- `docs/testing_guide.md` - End-to-end testing guide with 9 test scenarios
- `docs/step_4_llm_integration_plan.md` - LLM integration design document
- `docs/step_4_progress.md` - Implementation progress tracker
- `docs/github_issues_summary_brief.md` - Roadmap and issue tracking
- `CLAUDE.md` - Development guidelines for contributors

## Examples

### Example 1: Simple Query
```bash
$ cargo run -p querygpt-cli -- plan --prompt "show all offers" --yes

🧪 Using fixture planner for testing
[flow] prompt received: show all offers
[flow] planner.suggest_report_spec (attempt 1)
[flow] compiler.compile_report_spec → OK
[flow] confirm spec → approved

✅ Success! Generated SQL:
SELECT o.id,
       o.name
FROM offers_latest o
```

### Example 2: With Filters
```bash
$ cargo run -p querygpt-cli -- plan --prompt "show active campaigns" --yes

✅ Success! Generated SQL:
SELECT c.id,
       c.name
FROM campaigns_latest c
WHERE campaign_deleted = false
```

### Example 3: With Ordering
```bash
$ cargo run -p querygpt-cli -- plan --prompt "list campaigns ordered by name" --yes

✅ Success! Generated SQL:
SELECT c.id,
       c.name
FROM campaigns_latest c
ORDER BY c.name ASC
```

## License

[License information here]

## Contact

[Contact information here]
