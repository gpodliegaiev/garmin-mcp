# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

`garmin-mcp` is an MCP (Model Context Protocol) server in Rust (edition 2024) that exposes tools for reading Garmin Connect data, so an AI can evaluate the user's training and health. Early stage. Tools: `list_activities`, `get_activity`, `get_sleep`.

## Commands

```sh
cargo build                 # build
cargo run                   # run server on stdio (waits for an MCP client)
cargo clippy --all-targets  # lint
cargo fmt                   # format
cargo test                  # all tests
cargo test <name>           # single test by name substring
```

## Architecture

- Built on the `rmcp` crate (feature `transport-io`) with a `tokio` multi-thread runtime.
- `src/main.rs` — entry point; loads `Config`, builds `GarminClient`, constructs `GarminServer` and serves it over stdio transport.
- `src/config.rs` — `Config` from env vars / `.env` (`GARMIN_API_KEY` required, `GARMIN_BASE_URL` optional). Custom `Debug` hides the key.
- `src/garmin/auth.rs` — `TokenManager`: `POST /personalapitoken-service/personal-api/personal-api-key/token` with `Authorization: ApiKey <key>` returns a bearer JWT as plain text, valid 24h. Cached behind a tokio `Mutex`, refreshed 5 min before expiry. Uses `SystemTime`, not `Instant` (macOS `Instant` pauses during sleep).
- `src/garmin/client.rs` — `GarminClient`: authenticated GETs with `Authorization: Bearer <token>`; on 401 invalidates the token and retries once. API calls (`list_activities`, `activity_detail`, `daily_sleep`) return typed structs via generic `get_json<T, Q>` (`Q` = any `Serialize` query, e.g. `&[("date", "2026-10-23")]`).
- Response types all follow one pattern: a raw struct (Deserialize from Garmin camelCase, Garmin units, every field `Option`, unknown Garmin fields ignored) → `into_summary()` → an output struct (Serialize as snake_case, unit in every field name, `None` skipped).
- `src/garmin/types.rs` — shared pieces: `ActivityTypeFilter`, `SpeedStyle`, and `pub(super)` helpers used by both activity modules: `round`, `format_pace` ("m:ss"), `format_time` ("m:ss" or "h:mm:ss"), `positive` / `rounded` / `count` (0 → `None`), `to_minutes` (s → min, 2 dp), `only_true` (keep a flag only if `true`), `benefit` (strip training-effect message number), `event_type` (drop `uncategorized`).   - Activity conversions: durations → minutes (2 dp); distance → km (3 dp), but meters for swimming; speed → `*_pace_per_km` "m:ss" for run/walk/hike, `*_pace_per_100m` for swim, `*_speed_kmh` otherwise (chosen by `SpeedStyle::for_activity` from the `typeKey`). Zero values (strength, indoor) mean "not tracked" → omitted. Kept even when 0: training effect, `difference_body_battery`; altitude and temperature can be ≤ 0, so they are not filtered.
  - Garmin stores many values as float32, so they arrive with noise (`2.4000000953674316`). Round only to real precision (training effect 1 dp, training load 1 dp, cadence and running dynamics 1 dp) via `round()`. Never round below what matters for health evaluation; leave already-clean fields (HR, power, calories, sleep metrics) unrounded. Sleep durations are converted seconds → minutes (1 dp).
  - `ActivityTypeFilter` enum = values accepted by the activity list's `activityType` param. Garmin accepts only *parent* types (`running` includes `treadmill_running`; strength training and `stair_climbing` are `fitness_equipment`, but `floor_climbing` and `breathwork` are `other`); subtypes, wrong case or unknown values get HTTP 400 `[]`. Used directly as tool input (`JsonSchema`) and as query value (`Serialize`), via the `ActivitiesQuery` struct in `client.rs`.
  - `ActivitiesQuery` (`client.rs`) holds every activity-search param in Garmin units: dates as `NaiveDate` (chrono `serde` feature → `YYYY-MM-DD`), distance in m, duration in s, elevation in m, all whole numbers (Garmin returns 404 for decimals). `ListActivitiesRequest::into_query` in `server.rs` converts the tool's units (km, minutes) into it, clamps `limit`, and rejects bad dates, negative values and reversed ranges. A reversed range gets no error from Garmin, just an empty list.
  - `SpeedStyle::convert` (m/s → pace/km, pace/100 m or km/h) and `SpeedStyle::distance` (m → km, or m for swimming) are used by both activity outputs.
- `src/garmin/activity.rs`: activity list entry `Activity` → `ActivitySummary`. Garmin's richest activity record (`SummarizedActivityDTO`, ~110 keys per real entry, ~230 in the spec); every workout metric is output, for full AI evaluation. Left out on purpose: owner/profile, privacy, social (likes, comments), media flags, GPS coordinates, device manufacturer, ids/UUIDs, duplicate flags (`pr` vs `isPR`) and repeats (`startTimeGMT`, `beginTimestamp`, `maxDoubleCadence`, `minActivityLapDuration`). Quirks:
  - Names with `_N` suffixes (`hrTimeInZone_1`, `powerTimeInZone_1`, `maxAvgPower_60`, `fastestSplit_1609`) need explicit `rename`. HR zones are always sent (0.0 if empty); time below zone 1 is in no zone. Output as `hr_zones_min` / `power_zones_min` objects, omitted when every zone is 0.
  - `maxAvgPower_*` (power curve) and `fastestSplit_*` (seconds, names are meters: `1609` = mile, `91` = 100 yd) are read with `#[serde(flatten)]` into sub-structs.
  - Odd units per the spec: `poolLength` cm, `getlactateThresholdSpeed` (sic) and `concentricMeanVelocity` cm/ms, `maxDepth` cm, `surfaceInterval` ms, `caloriesEstimated` kJ; exercise-set `duration` ms, `maxWeight` and `volume` g.
  - `summarizedExerciseSets` → `exercises` (per-exercise sets, reps, max weight, volume); body-weight exercises have weight 0 → omitted.
  - `elapsed_duration_min` is only output when it differs from `duration_min` (i.e. there were pauses); `laps` only when > 1; `event_type` `uncategorized` is dropped.
  - `splitSummaries` use the same `RawSplit` / `SplitSummary` as details (list splits have step and ski fields, detail splits have HR).
- `src/garmin/activity_detail.rs`: `GET /activity-service/activity/{id}` → `ActivityDetail` (raw: `activityTypeDTO`, `eventTypeDTO`, `timeZoneUnitDTO`, `summaryDTO`, `splitSummaries`; the `DTO` names need explicit `rename`) → `into_summary()` → flat `ActivityDetailSummary`, following the same unit and rounding rules. Quirks:
  - `startTimeLocal` here is `"YYYY-MM-DDTHH:MM:SS.0"`, so it is normalized to the list's format.
  - `avgElevation: -500.0` is a "no elevation" sentinel (indoor), so it is not output.
  - Many untracked metrics are `0.0`, so they are filtered out with `positive()`.
  - Nearly everything here is also in the list entry; only time zone, min/recovery HR, average moving speed, stamina, and per-split HR are extra.
  - `summaryDTO.totalSets` counts rest periods, so `activeSets` (equal to the list's `totalSets`) is output as `total_sets`.
  - Training-effect messages lose their trailing variant number (`IMPACTING_TEMPO_22` becomes `IMPACTING_TEMPO`).
  - An empty `description` (`""`) is dropped.
  - An unknown id returns 404 `{}`.
  - The response has no per-exercise strength sets; the list's `summarizedExerciseSets` does.
- `src/garmin/sleep.rs`: `dailySleep` → `DailySleep` (raw, nested, all `Option`) → `into_summary()` → flat `SleepSummary`. Quirks:
  - Needs `date=YYYY-MM-DD` (filed under the wake-up day); a day without data returns 200 with every field `null` (→ `None`), a malformed date returns 404.
  - `...TimestampLocal` fields are epoch ms pre-shifted to local time, so format them as UTC. Naps (`dailyNapDTOS`) have GMT text times plus an offset in minutes.
  - `sleepScores` factors have `qualifierKey` and `optimalStart`/`optimalEnd` (output as `[low, high]`; `totalDuration` in seconds → minutes); only overall and stage percentages have a `value`.
  - `sleepNeed` / `nextSleepNeed` / `sleepAlignment` times (`recommendedBedtimeStartMins`, `optimalSleepWindowStartMins`, `...MidpointMins`) are minutes after local midnight → "HH:MM" (wrapping for negative values).
  - "Nothing to report" values are dropped: `NOT_AVAILABLE` insight, `NO_CHANGE` adjustments, 0 s durations. SpO2 fields are only sent when SpO2 tracking was on.
- `src/server.rs` — `GarminServer`, the MCP handler:
  - Tools are async methods in the `#[tool_router] impl GarminServer` block, each annotated `#[tool(description = ...)]`.
  - Tool inputs are structs deriving `Deserialize` + `JsonSchema` (via `rmcp::schemars`), received as `Parameters<T>`. Doc comments on fields become the JSON schema descriptions shown to clients.
  - `#[tool_handler]` on the `ServerHandler` impl wires the router in; `get_info` declares capabilities, server info, and instructions.
- To add a tool: define a request struct, then add a `#[tool]` method to the `#[tool_router]` impl.

## Response fields

Every tool must return **all useful fields** Garmin sends, so the AI can do a full evaluation without another call. When adding or changing an endpoint:

1. Fetch real responses (several days or sports) and read the spec schema in `docs/garmin-openapi.json`. Nested objects are often untyped in the spec, so real data is the main source.
2. Output every field about the user's training, health or body, including nested objects, lists and sport-specific fields seen only in the spec.
3. Leave out only what isn't about the data itself: profile/owner/device ids, UUIDs, privacy and social fields, media flags, GPS coordinates, upload/file metadata, GMT copies of local times, and duplicates of fields already output. List what was left out in the module's `//!` doc.
4. Hide "nothing to report" values with `skip_serializing_if`: `null`, placeholder `0`, empty strings and lists, `false` flags, and codes like `uncategorized`, `NOT_AVAILABLE`, `NO_CHANGE`. Keep a `0` that is a real result (training effect 0.0, body battery change 0, awake count 0, 0.0 inside a zone breakdown, altitude/temperature ≤ 0).
5. Convert to readable units with the unit in the field name, following the conversion and rounding rules above.
6. Update the tool's `description` and the README tool table to name the field groups returned.

## Garmin API reference

Local copy of Garmin's swagger docs (the online page renders client-side and can't be fetched as a spec):

- `docs/garmin-api.md`: endpoint index with params, response schema names, and implementation status (✅, MCP tool, `GarminClient` method), found by scanning `src/` for endpoint paths. Check it first when adding a tool or param.
- `docs/garmin-openapi.json`: full OpenAPI 3.0 spec, including response schemas (`components.schemas`, e.g. `SummarizedActivityDTO`, `DailySleepDTO`).
- `python3 docs/update_garmin_spec.py` regenerates both from developer.garmin.com. After adding or removing an endpoint call, run it with `--offline` to refresh the status from the local spec.

The spec omits some behavior found by probing (see the quirks under Architecture), and it doesn't list the token endpoint.

## Code comments

The maintainer is new to Rust. Comment code generously so it can be read as a learning resource:

- Every function, struct, enum, and module gets a `///` doc comment saying what it does and why it exists, in plain language.
- Explain Rust-specific concepts where they appear, briefly: ownership/borrowing (`&`, `&mut`, moves), `Result`/`Option` and the `?` operator, `async`/`.await`, traits and `impl` blocks, derive macros, lifetimes, and pattern matching/destructuring (e.g. `Parameters(req)`).
- Explain what macros like `#[tool]`, `#[tool_router]`, `#[tool_handler]`, and `#[tokio::main]` do behind the scenes.
- Inside function bodies, add `//` comments for each non-obvious step, describing what it does and what data flows through it.
- Keep comments accurate when changing code; update or remove stale ones.

## Configuration

Copy `.env.example` to `.env` and set `GARMIN_API_KEY`. `.env` is git-ignored. Real env vars (e.g. from the MCP client config) override `.env`.

## Constraints

- **stdout is the MCP protocol channel.** Never `println!` or write anything else to stdout; log to stderr (`eprintln!`).
- Never log or print the API key or bearer token.
