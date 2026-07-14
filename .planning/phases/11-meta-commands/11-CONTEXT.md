# Phase 11: Meta-Commands - Context

**Gathered:** 2026-07-14
**Status:** Ready for planning

<domain>
## Phase Boundary

Ship the two frictionless-PS7 **meta-commands** last, generated against the now
final and complete `Cli` arg surface:

- **`box config`** (CFG-01) — `show | get <key> | set <key> <value> | path`
  reads, edits, and locates `%APPDATA%\box\config.toml`, with `--json` support
  on `show` (dogfooding the spine).
- **`box completions`** (CMP-01) — emits a static shell-completion script
  generated from the live final `Cli`, so it reflects **every** global
  `--json`/`--clip`/`--no-color` flag and **every** Phase-8/9/10 depth flag.

Both are additive plumbing over settled subsystems: the config schema (D-13,
Phase 10 — nested `[hash]`/`[weather]` tables), the CLI>env>config>builtin
resolver (SPINE-05, Phase 6), the `--json` spine (Phase 7), and the 0/1/2 exit
contract (v1). Nothing here rebuilds those — `box config` **locks against** the
existing schema, and `completions` **generates against** the existing `Cli`.

No new *tool* commands (milestone charter). This is the final v2.0 phase.

</domain>

<decisions>
## Implementation Decisions

### box config — subcommand shape & dispatch (CFG-01)
- **D-01:** `box config` introduces the CLI's **first nested subcommand** (every
  other command is flag-based). Shape: a `ConfigArgs` carrying a
  `#[command(subcommand)]` `ConfigCommand` enum with `Show`, `Get { key }`,
  `Set { key, value }`, `Path` variants. It slots into `cli.rs`'s `Commands`
  enum and `main.rs`'s dispatch match like every other command, flowing through
  the existing `init_config → init_color → init_output → dispatch → flush_clip`
  pipeline and the 0/1/2 exit downcast.

### box config — `set` write strategy (CFG-01)
- **D-02:** `box config set` persists via a **typed-struct round-trip**, NOT
  `toml_edit`. Add `#[derive(Serialize)]` to `Config` + `HashConfig` +
  `WeatherConfig` with `#[serde(skip_serializing_if = "Option::is_none")]` on the
  leaf fields; `set` does load-or-default → mutate the target field →
  `toml::to_string` → **atomic temp-write + rename**, creating the
  `%APPDATA%\box\` parent dir if absent. **No new crate** (reuses the existing
  `toml = "1.1.2"` + `serde` derive). User comments/formatting are dropped on
  write — accepted for a machine-managed personal-toolbox config.
  - **`toml_edit` explicitly rejected:** the advisor **empirically verified**
    (throwaway build against the pinned versions) that adding `toml_edit 0.23`
    pulls a *duplicate* `winnow 0.7` + `toml_datetime 0.7` alongside `toml 1.x`'s
    own `winnow 1.0`/`toml_datetime 1.1` — a direct hit against CLAUDE.md's
    no-duplicate-crate / lean ethos, buying only comment-preservation that a
    tiny fully-typed `deny_unknown_fields` config doesn't need.
  - **Empty-table note:** `toml::to_string` of an all-`None` sub-table emits an
    empty `[hash]`/`[weather]` header. It re-parses cleanly to all-`None`, so it
    is harmless; suppress it with `skip_serializing_if` on the sub-table fields
    or leave it as a section hint — planner's discretion.

### box config — `set` safety & settable-key surface (CFG-01)
- **D-03:** `set` **validates before it persists.** Because `box` is *stricter
  at read time* than git/gh/cargo — `#[serde(deny_unknown_fields)]` + typed enums
  turn any stray key or bad value into `BoxError::Config` → **exit 2 for every
  command at startup** — the writer must be the strict gate. `set` reconstructs
  the resulting document and round-trips it through the **same**
  `toml::from_str::<Config>` the startup path uses; on any parse / invalid-enum /
  unknown-key failure it writes **nothing** and exits 2. This makes a
  self-inflicted exit-2 lockout **structurally impossible**. (The existing
  `malformed_maps_to_config_error` test in `config.rs` is the exact harness.)
- **D-04:** **Closed settable-key set** — only `hash.default_algo`,
  `weather.location`, `weather.units` are settable (the D-13 schema). An unknown
  key (e.g. `foo.bar`) is **rejected at set time** with the list of known keys
  (exit 2) and **never written** — `box` must be *more* strict than gh's
  warn-then-write here, because a written unknown key would brick startup. The
  closed key registry doubles as the surface for `set`/`get`, `--help`, and the
  "did you mean" error text.
- **D-05:** The roadmap/REQUIREMENTS `color` config key is **OUT of scope** for
  Phase 11. No `color` field exists in `Config` today — color is resolved
  entirely through `--no-color` / `NO_COLOR` / non-TTY with no config tier. Adding
  it would require a new schema field **and** wiring a fourth precedence tier
  into the color gate (deciding CLI vs `NO_COLOR` env vs config order, the
  auto/always/never tri-state) — new runtime behavior, not config-spine
  plumbing. Under the D-03 design a key is only settable if it has a backing
  `Config` field to round-trip against, so `color` *cannot* be a known key
  without the schema change. → Treat the roadmap mention as forward-looking;
  see Deferred Ideas.

### box config — `show` / `get` view semantics (CFG-01)
- **D-06:** `show` presents the **EFFECTIVE resolved config** (built-in defaults
  filled in, env tier applied), with **full human/JSON parity** — human `show`
  and `show --json` render the same picture. SC1 already locks `show --json` to
  "the effective config"; `box` owns its config and every command reads through
  the `cli.or(env).or(cfg).unwrap_or(builtin)` resolver, so `show` must display
  exactly what those commands will consume (anything else lets `config show`
  lie about what `box weather` will do). `show --json` **dogfoods `emit_json`**
  → one clean nested snake_case doc, no ANSI/BOM/progress. The **env tier is
  included** in "effective" (it sits in the precedence chain).
- **D-07:** `box config get <key>` — three cases:
  1. **Unset key WITH a builtin default** (`hash.default_algo`→`blake3`,
     `weather.units`→`metric`) → print the **resolved default**, exit 0. (This is
     the whole point of "effective".)
  2. **Unset key with NO default** (`weather.location`) → print **nothing** to
     stdout, **exit 1** (git-style scriptable "not set" signal); the `--json`
     variant emits `null`.
  3. **Unknown / misspelled key** → **exit 2** (usage error) with a stderr
     message + the known-key list. Keeps "no value" (exit 1) distinct from "no
     such key" (exit 2).

### box config — `path` (CFG-01)
- **D-08:** `box config path` prints the resolved `config_path()`
  (`%APPDATA%\box\config.toml`) to stdout, exit 0, **whether or not the file
  exists** (locate, never read). Reuses `core::config::config_path()` verbatim
  (make it `pub`).

### box completions — shell surface (CMP-01)
- **D-09:** `box completions <shell>` takes a **required positional
  `shell: clap_complete::Shell`** (a `ValueEnum`) accepting
  `bash|zsh|fish|powershell|elvish` — the **rustup pattern**. This satisfies the
  roadmap's literal `box completions powershell` **verbatim as a superset**, and
  because `Shell` already derives `ValueEnum` you get argument validation and a
  `--help` listing for free. Generate via
  `clap_complete::generate(shell, &mut Cli::command(), "box", &mut io::stdout())`
  — driven by `clap::CommandFactory` on the live final `Cli`, so it
  auto-reflects every Phase-6..10 flag. (Only `powershell` is charter-tested;
  the other shells are best-effort — see Deferred.)

### box completions — output purity & registration hint (CMP-01)
- **D-10:** The generated script goes to **STDOUT ONLY** — the same purity
  discipline as `--json` — so `box completions powershell > _box.ps1` and piping
  into `$PROFILE` stay uncontaminated. **No per-run stderr chatter** (it fires
  even under redirection). The registration one-liner lives in two **inert**
  places: (1) a `#`-prefixed **PowerShell comment header** prepended before the
  generated script (PS comments are inert, so the artifact still executes and is
  self-documenting even after redirection to a file), and (2) the subcommand's
  `long_about` / `--help`.

### install.ps1 — completion registration (CMP-01)
- **D-11:** `install.ps1` **prints a one-line hint** by default and does **NOT**
  silently mutate `$PROFILE` (unprompted profile edits are exactly the
  far-reaching change the dev profile flags). Provide an **opt-in
  `-RegisterCompletions` switch** that appends
  `box completions powershell | Out-String | Invoke-Expression` wrapped in a
  `# box completions` sentinel marker — **idempotent** (guard with
  `Select-String -Quiet -Pattern '# box completions' $PROFILE` before appending;
  ensure `$PROFILE` + its parent dir exist). The Invoke-Expression-from-live-command
  form (not dot-sourcing a saved file) means completions regenerate each shell
  start and auto-track future `Cli` changes across `box` upgrades.

### Dependencies
- **D-12:** Add **`clap_complete`** (pinned to match `clap 4.6`) — the one new
  crate for the phase, already earmarked in STATE.md. `box config` needs **no
  new crate** (existing `toml` + `serde` derive cover write + validation).

### Claude's Discretion
- Exact `ConfigArgs` / `ConfigCommand` enum shape, and where the closed
  settable-key registry lives (a match in `commands/config` vs a small table in
  `core/config`).
- Whether to suppress empty `[hash]`/`[weather]` tables via `skip_serializing_if`
  on the sub-table fields, or accept the harmless empty-table scaffolding (both
  re-parse clean).
- Exact JSON shape of `config show --json` (recommend nested
  `{hash:{default_algo},weather:{location,units}}` effective doc, snake_case per
  house style; include keys whose value is a builtin default since it's
  "effective") and whether `get --json` wraps the single value or emits it bare.
- Whether config's usage errors reuse `BoxError::Config` or get a new typed
  variant — either is fine as long as `main.rs`'s exit-downcast routes set
  bad-value / unknown-key to **exit 2** while `get` unset-no-default stays
  **exit 1**.
- Location of the atomic-write helper (`core::fs` vs `core::config`).
- Precise hint/error wording and the known-key "did you mean" text; the exact PS
  comment-header lines.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

No external ADRs/specs exist beyond the `.planning/` corpus — the decisions above
plus these files fully define the phase.

### Phase scope & requirements
- `.planning/ROADMAP.md` §"Phase 11: Meta-Commands" — goal, success criteria
  SC1–SC4, and the two provisional plan splits (11-01 `config` / 11-02
  `completions`).
- `.planning/REQUIREMENTS.md` — **CFG-01** / **CMP-01** wording; §"Milestone
  Decisions" D-6 (config co-ships in Phase 6) and D-7 (`completions` is strictly
  last, generated from the final `Cli`); the "Config interactive wizard/TUI —
  Out of scope" boundary (`box config` stays flag-driven show/get/set/path).
- `.planning/PROJECT.md` §"Key Decisions" + §"Current Milestone" — the toolkit
  ethos and the "globally available, instantly usable from PowerShell 7" core
  value that D-09/D-10/D-11 serve.
- `.planning/STATE.md` §"Accumulated Context" — the D-1..D-38 decision log, the
  config-precedence / JSON-purity / exit-contract pitfalls, and the line
  earmarking `clap_complete` + `cli.rs` `+Completions/Config variants`.

### Config subsystem (box config locks against this)
- `src/core/config.rs` — the typed nested `Config`
  (`[hash] default_algo` / `[weather] location` / `[weather] units`),
  `config_path()` (`%APPDATA%` first, `dirs` fallback), `load()`, `init_config()`
  `OnceLock`, the `cli.or(env).or(cfg).unwrap_or(builtin)` resolver, the
  missing→default / malformed→exit-2 behavior to preserve, and the
  `malformed_maps_to_config_error` test — the exact round-trip harness D-03's
  set-validation reuses.

### CLI surface & dispatch (completions generates against this; both commands slot in)
- `src/cli.rs` — the final `Cli` / `Commands` surface `completions` renders via
  `Cli::command()`; where the new `Config(ConfigArgs)` + `Completions(...)`
  variants land.
- `src/main.rs` — the `init_config → init_color → init_output → dispatch →
  flush_clip` order and the exit-code downcast (`BoxError::Config` + the usage
  variants → exit 2) config's set errors slot into.
- `src/commands/mod.rs` — the `RunCommand` static-dispatch trait each new Args
  struct implements.
- `src/core/errors.rs` — `BoxError` variants + the typed exit-2 set; a config
  usage error (new or reused `Config`) plugs into the `main.rs` downcast here.

### Spine & template (dogfooded by config show --json)
- `src/core/output.rs` — `is_json_on()` / `emit_json` / `out_line` the
  `config show --json` and `config get` paths reuse.
- `src/commands/uuid/mod.rs` — the frozen `{results,count}` + `is_json_on()`
  fork pilot template the config `--json` path follows for house style.

### Distribution
- `install.ps1` — the build → copy-to-bin → add-to-PATH installer D-11's
  registration hint / opt-in `-RegisterCompletions` switch extends.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`core::config::config_path()`** — the `%APPDATA%`-first resolver; `config
  path` prints it, `config set` writes to it, both reuse it verbatim (make
  `pub`).
- **`core::config::Config` + `load()` / `toml::from_str::<Config>`** — the exact
  deserialize D-03's set-validation round-trips against; the typed `Algo` /
  `Units` enums already reject bad values, and `deny_unknown_fields` already
  rejects stray keys — the validation is *inherent*, `set` just has to run it
  before writing.
- **The config resolver** (`cli.or(env).or(cfg).unwrap_or(builtin)`) — `show` /
  `get` read the **same** resolution every command uses, so the effective view
  (D-06) can't drift from real behavior.
- **`core::output` spine** (`is_json_on` / `emit_json` / `out_line`) — `config
  show --json` dogfoods `emit_json`; `get` / human `show` print via `out_line`.
- **`clap::CommandFactory` on `Cli`** — `Cli::command()` hands `completions` the
  live, final arg tree for free; no manual flag enumeration.
- **The uuid `{results,count}` / `is_json_on()` fork** — the frozen JSON house
  style config's `--json` document follows.

### Established Patterns
- **Exit contract 0/1/2** (D-06, v1): usage/config errors → **exit 2** (`set`
  bad value / unknown key); the **git-style `get` unset-no-default → exit 1**
  carve-out is a deliberate "not set" data signal, NOT a usage error.
- **JSON / output purity**: `--json` stdout carries exactly one document;
  `completions` stdout carries exactly the script — no ANSI/BOM/progress/hint
  contamination.
- **Config tolerance** (SPINE-05): missing → silent default; malformed → exit 2;
  a normal command never errors on config. D-03's validate-before-write makes a
  *self-authored* malformed file impossible.
- **Lean-dep / no-duplicate-crate ethos** (CLAUDE.md): drove the D-02
  `toml_edit` rejection (duplicate `winnow`/`toml_datetime`); `clap_complete`
  (D-12) is the one sanctioned new crate.
- **`RunCommand` static dispatch**: `config` and `completions` each get an Args
  struct with a `RunCommand::run` impl — no `Box<dyn>`.

### Integration Points
- **`src/cli.rs`** — add `Config(ConfigArgs)` + `Completions(CompletionsArgs)`
  to `Commands`; both appear in `box --help`. `ConfigArgs` carries the nested
  `ConfigCommand` subcommand enum (D-01).
- **`src/main.rs`** — add the two dispatch arms; they inherit the existing
  init/exit pipeline. `completions` still flows through `init_config` (harmless)
  but its real input is `Cli::command()`.
- **`src/core/errors.rs`** — add or reuse a config-usage `BoxError` variant and
  wire it into the exit-2 downcast (respecting the `get` exit-1 carve-out).
- **`src/core/config.rs`** — expose `config_path()`; add `Serialize` derives +
  `skip_serializing_if`; host the atomic-write + validate helper and/or the
  settable-key registry.
- **`Cargo.toml`** — add `clap_complete` (matches `clap 4.6`).
- **`install.ps1`** — registration hint + opt-in `-RegisterCompletions` (D-11).

</code_context>

<specifics>
## Specific Ideas

- `box config show` → effective config (human table + `--json` nested doc);
  `box config show --json | ConvertFrom-Json` yields one clean object.
- `box config get hash.default_algo` on an empty config → `blake3` (exit 0);
  `box config get weather.location` unset → empty stdout, exit 1.
- `box config set weather.units imperial` → validates, atomically writes
  `[weather] units = "imperial"`; a subsequent bare `box weather` then resolves
  to °F/mph (the SC2-style round-trip proof, mirroring `hash.default_algo`).
- `box config set weather.units kelvin` → **exit 2, nothing written** (invalid
  enum caught by the round-trip).
- `box config set nope.key 1` → **exit 2**, prints the known-key list, nothing
  written.
- `box config path` → `C:\Users\<user>\AppData\Roaming\box\config.toml`.
- `box completions powershell > _box.ps1` → clean PS7 script with a `#`-comment
  "how to register" header; `box completions bash` also works (best-effort).
- `install.ps1` prints: *"To enable tab-completion, add `box completions
  powershell | Out-String | Invoke-Expression` to your $PROFILE — or re-run with
  -RegisterCompletions."*

</specifics>

<deferred>
## Deferred Ideas

- **`color` config key** (D-05) — needs a new `Config` schema field **and** a new
  config precedence tier wired into the `--no-color`/`NO_COLOR`/TTY color gate
  (new runtime behavior). Out of Phase 11; its own follow-up item if genuinely
  wanted. The roadmap's key-list mention is forward-looking.
- **`config unset` / `config edit`** (open `$EDITOR`) — not in the SC surface
  (show/get/set/path only); revisit if a reset-to-default / bulk-edit need
  appears.
- **`config show --show-origin` / provenance markers** (per-key `file`/`env`/
  `default` source) — a clean superset of the effective view (D-06), kept in
  reserve behind an explicit flag; add later if "why is this the value?" becomes
  a common question. Must never pollute the default `--json` doc.
- **Interactive config wizard / TUI** — explicitly Out of Scope per
  REQUIREMENTS (config stays flag-driven).
- **Non-PowerShell completion shells** — `bash`/`zsh`/`fish`/`elvish` are
  *supported* by D-09 (all `clap_complete::Shell` variants) but only PowerShell
  is charter-tested; treat the others as best-effort, untested output.

None of the above pulled Phase 11 out of scope — discussion stayed within
CFG-01 + CMP-01.

</deferred>

---

*Phase: 11-meta-commands*
*Context gathered: 2026-07-14*
