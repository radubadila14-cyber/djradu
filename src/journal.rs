// =============================================================================
// journal.rs — THE MANIFEST & JOURNAL OF RADU ARCHITECTUL'S TRADING SYSTEM
// =============================================================================
//
// This file is the brain of the project. It documents:
//   - WHO is building this and WHY
//   - The ARCHITECTURE decisions and roadmap
//   - STEP-BY-STEP instructions for Radu
//   - The BEHAVIOR and rules the system must follow
//   - The TELEMETRY philosophy
//
// =============================================================================

// ─────────────────────────────────────────────────────────────────────────────
// WHO
// ─────────────────────────────────────────────────────────────────────────────
//
// Builder:       Radu Architectul — professional trader, learning Rust
// Mentor:        Daniel (TravelMasterAI.com) — fast-forwarding the process
// AI Professor:  GitHub Copilot (Claude Opus 4.6) — explains everything
//
// ─────────────────────────────────────────────────────────────────────────────
// THE VISION
// ─────────────────────────────────────────────────────────────────────────────
//
// An autonomous AI-powered trading system that:
//   1. Ingests real-time market data
//   2. Measures EVERYTHING via telemetry (Alpha, Beta, Theta)
//   3. Makes decisions via trained ML models (Quant 3.5B → Lambda Labs GPU)
//   4. Executes trades with full audit trail
//   5. Provides a mobile app (Flutter/Dart) for monitoring
//   6. Scales to 10,000 concurrent connections / 1M accounts
//
// ─────────────────────────────────────────────────────────────────────────────
// ARCHITECTURE
// ─────────────────────────────────────────────────────────────────────────────
//
//  ┌──────────────────────────────────────────────────────────────────────┐
//  │                    RADU TRADING SYSTEM                              │
//  │                                                                      │
//  │  ┌─────────────┐  ┌──────────────┐  ┌────────────────┐              │
//  │  │  Rust Core   │  │ Telemetry    │  │ ML Pipeline    │              │
//  │  │  (this repo) │  │ (measure)    │  │ (predict)      │              │
//  │  └──────┬───────┘  └──────┬───────┘  └───────┬────────┘              │
//  │         │                 │                   │                       │
//  │  ┌──────▼─────────────────▼───────────────────▼────────┐             │
//  │  │              Event Bus (append-only log)            │             │
//  │  └──────┬──────────────┬──────────────┬────────────────┘             │
//  │         │              │              │                               │
//  │  ┌──────▼───┐  ┌───────▼────┐  ┌─────▼──────┐                       │
//  │  │ Postgres  │  │   Redis    │  │  Grafana   │                       │
//  │  │ (Railway) │  │ (Railway)  │  │ (monitor)  │                       │
//  │  └──────────┘  └────────────┘  └────────────┘                       │
//  │                                                                      │
//  │  ┌──────────────────────────────────────────────────────┐            │
//  │  │  NMM App (Flutter/Dart) — NewMoneyMedia               │            │
//  │  │  Mobile dashboard for trading + monitoring             │            │
//  │  └──────────────────────────────────────────────────────┘            │
//  └──────────────────────────────────────────────────────────────────────┘
//
// ─────────────────────────────────────────────────────────────────────────────
// CRATE STRUCTURE (planned)
// ─────────────────────────────────────────────────────────────────────────────
//
//  radu-trading-system/
//  ├── src/                        ← Current Rust backend
//  │   ├── main.rs                 ← Entry point
//  │   ├── config.rs               ← Environment config
//  │   ├── db.rs                   ← PostgreSQL connection
//  │   ├── cache.rs                ← Redis connection
//  │   ├── models.rs               ← Data structures
//  │   ├── quant.rs                ← Quant API client
//  │   └── journal.rs              ← THIS FILE (manifest)
//  │
//  ├── crates/                     ← Future workspace crates
//  │   ├── telemetry-core/         ← Telemetry engine (Alpha/Beta/Theta)
//  │   └── apps/
//  │       └── newmoneymedia/      ← NMM Flutter/Dart app
//  │
//  └── telemetry-schemas/          ← JSON Schemas for events
//
// ─────────────────────────────────────────────────────────────────────────────
// TELEMETRY PHILOSOPHY
// ─────────────────────────────────────────────────────────────────────────────
//
// Telemetry is NOT optional. It is the nervous system.
//
// WHAT WE MEASURE:
//   1. Market Data       — ticks, L2 depth, bars, venue status
//   2. Strategy Decisions — features, model output, confidence, regime
//   3. Order Lifecycle   — new → ack → partial fill → fill → cancel/reject
//   4. Execution Quality — slippage, fill ratio, adverse selection
//   5. Risk              — exposure, limits, margin, kill-switch triggers
//   6. Portfolio          — positions, PnL, fees, funding
//   7. Latency           — p50/p95/p99/p99.9 per hop
//   8. Infrastructure    — CPU, memory, event loop lag, reconnects
//   9. Audit / Lineage   — immutable event log, correlation IDs
//
// GREEK LETTERS (trading risk measures):
//   Alpha (α) — excess return above benchmark (your edge)
//   Beta (β)  — sensitivity to market movements (your exposure)
//   Theta (θ) — time decay of options positions
//   Delta (δ) — rate of change of price
//   Gamma (γ) — rate of change of delta
//   Vega (ν)  — sensitivity to volatility
//
// KEY IDs (for causality tracking):
//   trace_id       — ties market snapshot → decision → order → fills
//   decision_id    — unique per strategy decision
//   order_id       — unique per order
//   model_id       — which model version made the call
//
// THE GOLDEN RULE: If you can't replay it, you can't learn from it.
//   → Build replayability into every event from day one.
//
// ─────────────────────────────────────────────────────────────────────────────
// STEP-BY-STEP GUIDE FOR RADU
// ─────────────────────────────────────────────────────────────────────────────
//
// ┌─────────────────────────────────────────────────────────────────────────┐
// │ VS CODE BASICS                                                         │
// ├─────────────────────────────────────────────────────────────────────────┤
// │                                                                         │
// │ Step 1: OPEN YOUR PROJECT                                               │
// │   File → Open Folder → C:\Users\Radu\Documents\radu-trading-system     │
// │                                                                         │
// │ Step 2: THE SIDEBAR (left side)                                         │
// │   📁 Explorer    — shows your files (click to open them)                │
// │   🔍 Search      — find text across all files                           │
// │   🔀 Source Ctrl — Git: commit, push, pull                              │
// │   🐛 Debug       — run your code with breakpoints                      │
// │   📦 Extensions  — install add-ons                                      │
// │                                                                         │
// │ Step 3: THE TERMINAL (bottom panel)                                     │
// │   View → Terminal (or press Ctrl+`)                                     │
// │   This is where you run: cargo run, cargo build, cargo test             │
// │                                                                         │
// │ Step 4: COPILOT CHAT (right side)                                       │
// │   The chat panel where you talk to me (Opus 4.6)                        │
// │   TIP: For loading photographs or graphs, switch the agent              │
// │   dropdown at the top of the chat to "Sonnet 4.6"                      │
// │                                                                         │
// │ Step 5: COMMAND PALETTE                                                 │
// │   Press Ctrl+Shift+P → type any command                                │
// │   This is your Swiss Army knife. Examples:                              │
// │     "Git: Clone" — clone a repo                                         │
// │     "Terminal: New" — open new terminal                                 │
// │     "Preferences: Settings" — configure VS Code                         │
// │                                                                         │
// └─────────────────────────────────────────────────────────────────────────┘
//
// ┌─────────────────────────────────────────────────────────────────────────┐
// │ RAILWAY SETUP                                                           │
// ├─────────────────────────────────────────────────────────────────────────┤
// │                                                                         │
// │ Step 1: Go to https://railway.app and sign in                           │
// │                                                                         │
// │ Step 2: Click your PostgreSQL service                                   │
// │         → "Connect" tab                                                 │
// │         → Copy the PUBLIC URL (not the internal one!)                   │
// │         → The public one has a host like:                               │
// │           xxxxxxxx.railway.app (NOT railway.internal)                   │
// │                                                                         │
// │ Step 3: Same for Redis service → Connect → copy public REDIS_URL       │
// │                                                                         │
// │ Step 4: Paste both into .env file in your project                      │
// │                                                                         │
// │ CURRENT STATUS:                                                         │
// │   PostgreSQL: ✓ Credentials saved (need PUBLIC url)                    │
// │   Redis: ✗ Need connection string                                      │
// │                                                                         │
// └─────────────────────────────────────────────────────────────────────────┘
//
// ┌─────────────────────────────────────────────────────────────────────────┐
// │ RUNNING THE SYSTEM                                                      │
// ├─────────────────────────────────────────────────────────────────────────┤
// │                                                                         │
// │ Step 1: Open terminal (Ctrl+`)                                          │
// │ Step 2: Type: cargo run                                                 │
// │ Step 3: Watch the logs — you should see:                                │
// │         "RADU TRADING SYSTEM — Starting up..."                          │
// │         "PostgreSQL connected!"                                         │
// │         "Redis connected and healthy!"                                  │
// │         "ALL SYSTEMS GO!"                                               │
// │                                                                         │
// │ If it fails: Read the error message. Common issues:                     │
// │   - "DATABASE_URL must be set" → check your .env file                  │
// │   - "Failed to connect to PostgreSQL" → wrong URL or not public        │
// │   - "Failed to connect to Redis" → need Redis URL from Railway         │
// │                                                                         │
// └─────────────────────────────────────────────────────────────────────────┘
//
// ┌─────────────────────────────────────────────────────────────────────────┐
// │ GIT & GITHUB                                                            │
// ├─────────────────────────────────────────────────────────────────────────┤
// │                                                                         │
// │ Step 1: Open terminal, run:                                             │
// │         git init                                                        │
// │         git add .                                                       │
// │         git commit -m "Initial commit: trading system scaffold"         │
// │                                                                         │
// │ Step 2: On GitHub.com → New Repository → name it                       │
// │         Copy the URL (like https://github.com/radu/trading.git)         │
// │                                                                         │
// │ Step 3: Back in terminal:                                               │
// │         git remote add origin YOUR_GITHUB_URL                           │
// │         git push -u origin main                                         │
// │                                                                         │
// │ IMPORTANT: .env is in .gitignore — your passwords will NEVER be         │
// │ uploaded to GitHub. This is correct and intentional.                    │
// │                                                                         │
// └─────────────────────────────────────────────────────────────────────────┘
//
// ─────────────────────────────────────────────────────────────────────────────
// BEHAVIOR RULES (for the AI assistant)
// ─────────────────────────────────────────────────────────────────────────────
//
// 1. Always explain what you're doing and why — Radu is learning.
// 2. Use Step 1 / Step 2 / Step 3 format for instructions.
// 3. Research official sources for math — don't ask Radu.
// 4. For photographs/graphs: tell Radu to switch to Sonnet 4.6 agent.
// 5. Write all architecture decisions in this journal.
// 6. Never commit secrets to Git.
// 7. Telemetry is never optional — instrument everything.
// 8. Build for replayability — if you can't replay it, you can't learn from it.
// 9. All behavior and manifest updates go in THIS FILE.
//

/// The Journal module — project manifest and documentation in code.
///
/// LEARNING NOTE (Radu):
/// This file is a Rust module, but its primary purpose is documentation.
/// The constants below capture your project's identity and can be
/// printed at startup or used in API responses.
pub const PROJECT_NAME: &str = "Radu Trading System";
pub const VERSION: &str = "0.1.0";
pub const BUILDER: &str = "Radu Architectul";
pub const MENTOR: &str = "Daniel (TravelMasterAI.com)";
pub const AI_PROFESSOR: &str = "GitHub Copilot — Claude Opus 4.6";

/// Print the project banner.
pub fn print_banner() {
    println!("╔══════════════════════════════════════════════════╗");
    println!("║          {} v{}          ║", PROJECT_NAME, VERSION);
    println!("║                                                  ║");
    println!("║  Builder:  {}                   ║", BUILDER);
    println!("║  Mentor:   {}       ║", MENTOR);
    println!("║  AI:       {}    ║", AI_PROFESSOR);
    println!("║                                                  ║");
    println!("║  Telemetry: α β θ δ γ ν                          ║");
    println!("║  Status:    INITIALIZING                         ║");
    println!("╚══════════════════════════════════════════════════╝");
}

/// Telemetry domains this system measures.
pub const TELEMETRY_DOMAINS: &[&str] = &[
    "Market Data",
    "Strategy Decisions",
    "Order Lifecycle",
    "Execution Quality",
    "Risk",
    "Portfolio / PnL",
    "Latency (p50/p95/p99)",
    "Infrastructure",
    "Audit / Lineage",
];
