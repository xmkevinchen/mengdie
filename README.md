# 梦蝶 Mengdie

AI-native knowledge memory for development workflows.

> 昔者庄周梦为胡蝶，栩栩然胡蝶也。自喻适志与！不知周也。俄然觉，则蘧蘧然周也。不知周之梦为胡蝶与，胡蝶之梦为周与？周与胡蝶，则必有分矣。此之谓物化。
>
> — 《庄子·齐物论》

> Once upon a time, Zhuang Zhou dreamed he was a butterfly — a butterfly fluttering about, happy with itself, doing as it pleased. It did not know it was Zhuang Zhou. Suddenly he awoke, and there he was, solid and unmistakable Zhuang Zhou. But he did not know whether he was Zhuang Zhou who had dreamed he was a butterfly, or a butterfly dreaming he was Zhuang Zhou. Between Zhuang Zhou and the butterfly there must be some distinction. This is called the transformation of things.
>
> — *Zhuangzi*, "Discussion on Making All Things Equal"

AI produces knowledge, knowledge feeds AI — who is the dreamer?

## What It Does

Mengdie is a local MCP server that gives your AI coding tools persistent memory across conversations. Knowledge flows in from your AI workflows, gets filtered by actual usage patterns, and feeds back as context — creating a spiral of improving AI output.

```
AI tools produce knowledge (decisions, findings, patterns)
    ↓
Mengdie ingests and indexes (embedding + FTS)
    ↓
Dreaming filters by real usage (frequently recalled = worth keeping)
    ↓
Next AI session gets prior context automatically
    ↓
Better output → richer knowledge → ...
```

## Install

### Pre-built binary (recommended)

```bash
# macOS / Linux
curl -fsSL https://github.com/anthropics/mengdie/releases/latest/download/install.sh | sh
```

### From source

```bash
cargo install mengdie
```

> First run downloads the embedding model (~90MB) to `~/.cache/fastembed/`. Takes 5-10 seconds once.

## Setup

Register as a Claude Code MCP server (one time):

```bash
claude mcp add mengdie -- mengdie-mcp
```

That's it. Three tools appear in Claude Code:

| Tool | What it does |
|------|-------------|
| `mengdie_search` | Search memories by semantic + keyword similarity |
| `mengdie_ingest` | Store a new memory with metadata and entity tags |
| `mengdie_invalidate` | Mark a memory as no longer valid |

## Usage

### In Claude Code (automatic)

Once registered, Claude Code can call `mengdie_search` / `mengdie_ingest` / `mengdie_invalidate` in any conversation. AI tools that support MCP (like the [AE plugin](https://github.com/anthropics/agentic-engineering)) will automatically use Mengdie if available.

### CLI

```bash
# Search memories
mengdie search "ink terminal renderer"
mengdie search "auth middleware" --global    # search across all projects
mengdie search "API timeout" --min-score 0.5

# Batch import existing documents
mengdie import --dir ./docs/discussions/

# Run Dreaming (promote frequently-recalled memories to long-term)
mengdie dream
mengdie dream --min-recall 5 --window-days 7

# View stats
mengdie stats
```

### Example output

```
$ mengdie stats
Mengdie Stats:
  Total memories:    47
  Valid (active):    42
  Long-term:         12
  Recalled (≥1x):    28
  Context injection rate: 73.2% (30/41 non-empty)
  Conflict detection rate: 4.3% (2/47 ingestions)
```

## How It Works

### Storage

- All data in `~/.mengdie/db.sqlite` (single file, easy to back up)
- Per-project isolation via git-inferred `project_id` (from git remote URL)
- Global search available with `scope: "global"`

### Search

Hybrid search combining:
1. **FTS5** full-text search (keyword matching)
2. **Vector similarity** (semantic matching via [all-MiniLM-L6-v2](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2), 384 dimensions)
3. **Reciprocal Rank Fusion** merges both result sets

### Dreaming

Inspired by [Zhuangzi's butterfly dream](https://en.wikipedia.org/wiki/Zhuangzi_(book)#%22The_Butterfly_Dream%22) — knowledge that proves useful in practice gets promoted to long-term memory.

- Tracks every `mengdie_search` call (recall count + relevance score)
- Daily promotion pass: memories with high recall + high relevance → long-term
- Memories that are never recalled naturally decay
- No AI judgment needed — usage behavior decides what's worth keeping

### Contradiction Detection

When new knowledge conflicts with existing memories:
- Entity-tag overlap triggers comparison
- Temporal validity tracking (`valid_from` / `valid_until`)
- Conflicts are flagged, not auto-resolved — you decide

## Integration with AI Tools

Mengdie is a standalone MCP server. Any tool that speaks MCP can use it.

### AE Plugin (Agentic Engineering)

If you use the [AE plugin](https://github.com/anthropics/agentic-engineering) for Claude Code:
- `ae:analyze` searches Mengdie for prior research before analysis
- `ae:discuss` / `ae:plan` / `ae:review` / `ae:retrospect` — integration planned

AE does **not** bundle Mengdie. Install Mengdie separately; AE will use it if available, degrade gracefully if not.

### Claude Code (direct)

Any Claude Code conversation can call the MCP tools directly. Useful for ad-hoc knowledge capture:

```
You: remember that the auth service uses JWT with RS256, not HS256
Claude: [calls mengdie_ingest with structured metadata]
```

## Architecture

```
src/
  core/
    db.rs            # SQLite connection, schema, migrations
    search.rs        # Hybrid FTS5 + vector + RRF merge
    embeddings.rs    # fastembed (all-MiniLM-L6-v2, local inference)
    vector.rs        # Cosine similarity, score normalization
    ingest.rs        # Parse → embed → store pipeline
    contradiction.rs # Entity-tag overlap + temporal validity
    dreaming.rs      # Usage-driven promotion (recall × relevance)
    parser.rs        # YAML frontmatter + entity extraction
    watcher.rs       # File system watcher for AE output
    mcp_tools.rs     # MCP tool implementations
    project.rs       # project_id inference from git remote
    metrics.rs       # Observability counters
  bin/
    mcp_server.rs    # MCP stdio server (spawned by Claude Code)
    cli.rs           # CLI (dream, import, search, stats)
```

## Limitations

- **Single instance per machine.** One `mengdie-mcp` process, one SQLite database. Don't run multiple instances (write lock contention).
- **Local only.** No cloud sync, no team sharing (yet). Your memories stay on your machine.
- **Embedding model size.** First run downloads ~90MB model. Subsequent starts are instant.
- **No auto-cleanup.** Invalid/stale memories must be explicitly invalidated via `mengdie_invalidate` or manual review.
- **Project isolation is git-based.** Non-git directories get a path-hash project_id. Moving a repo changes its project_id (memories still exist, just not auto-matched).

## Data & Privacy

- All data stored locally at `~/.mengdie/db.sqlite`
- Embedding model runs locally (no API calls for inference)
- No telemetry, no network requests
- Back up by copying `~/.mengdie/`

## License

MIT
