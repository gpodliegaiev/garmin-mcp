# garmin-mcp

An [MCP](https://modelcontextprotocol.io) server, written in Rust, that gives AI assistants such as Claude Code and Claude Desktop read access to your Garmin Connect data through the [Garmin Connect personal API](https://developer.garmin.com/gc-developer-program/personal/personal-api/).

## Tools

| Tool | What it returns | Arguments |
|---|---|---|
| `list_activities` | Your most recent activities, newest first, with every metric Garmin recorded: time, distance, pace, elevation, heart rate and time in each HR zone, power (zones, power curve, normalized power, FTP, TSS), cadence, running dynamics, fastest splits, training effect and load, VO2 max, body battery, respiration, stress, calories, sweat loss, swim/paddle/dive/MTB metrics, strength exercises (sets, reps, weight, volume per exercise), and split totals. | `start` (default 0), `limit` (default 5, max 100), and optional filters: `activity_type` (see below), `start_date` / `end_date` (`YYYY-MM-DD`), `min_distance_km` / `max_distance_km`, `min_duration_min` / `max_duration_min`, `min_elevation_gain_m` / `max_elevation_gain_m` |
| `get_activity` | One activity by id. Most metrics are already in `list_activities`; this adds the time zone, min and recovery heart rate, average moving pace, stamina, and heart rate per split type. | `activity_id` (from `list_activities`) |
| `get_fitness_age` | Garmin Fitness Age as of a date: chronological, current, previous and achievable fitness age, and each input (vigorous days and minutes per week, resting heart rate, BMI or body fat) with Garmin's target, the fitness age reachable by hitting it, and its priority. Inputs without recent data are marked stale. | `date` as `YYYY-MM-DD` (default today); returns the latest values on or before it |
| `get_daily_summary` | All-day wellness per day: steps, floors, calories, resting and min/max heart rate, intensity minutes, stress (level and minutes per category), Body Battery with every event that charged or drained it, respiration, SpO2, hydration. For a single day, also Body Battery at wake and during sleep, 7-day resting HR, time per activity level, and the activity behind each Body Battery event. | `start_date` / `end_date` as `YYYY-MM-DD` (default the last 7 days, up to 31 days apart) |
| `get_hrv` | Overnight heart rate variability per night: last night's average and 5-minute high, 7-day average, personal baseline (balanced range, low limit), HRV status and feedback code, in ms. For a single day, also the measured window and every 5-minute reading. | `start_date` / `end_date` as `YYYY-MM-DD` (default the last 7 nights, up to 366 days apart) |
| `get_sleep` | One night's sleep with every metric Garmin recorded: bed and wake times, deep/light/REM/awake durations and naps, sleep score with each factor's rating and optimal range, heart rate, respiration, SpO2 and breathing disruption (when tracked), stress, sleep need with its adjustments, recommended bedtime for this and the next night, and sleep-timing alignment. | `date` as `YYYY-MM-DD` (default today) for one night, or `start_date` / `end_date` for a list of nights (default the last 7, up to 31 days apart) |

Each field name includes its unit (`duration_min`, `distance_km`, `elevation_gain_m`, `calories_kcal`). Heart rate is in bpm. Speed is given in the usual way for each sport:

| Sport | Distance | Speed |
|---|---|---|
| Running, walking, hiking | `distance_km` | `avg_pace_per_km`, e.g. `"6:23"` |
| Swimming | `distance_m` | `avg_pace_per_100m`, e.g. `"2:05"` |
| Cycling and other sports | `distance_km` | `avg_speed_kmh` |

Every tool returns all useful fields Garmin sends. Fields that don't apply or weren't recorded are left out. Examples: distance for strength training, moving time for indoor activities, power zones without a power meter. Sleep durations are in minutes.

**Filters** can be combined, and all ranges are inclusive. Dates refer to each activity's local start date. Distance is in km for every sport, including swimming (1.5 means 1500 m). Duration is timer time, excluding pauses, the same as `duration_min` in the result (not `moving_duration_min`). Only `limit` activities come back, so raise it to get everything in a range.

**`activity_type` values:** `running`, `cycling`, `walking`, `hiking`, `swimming`, `fitness_equipment`, `other`, `multi_sport`, `winter_sports`, `water_sports`, `diving`, `team_sports`, `racket_sports`.

Garmin filters by sport group, not by exact type:
- `running` includes treadmill runs.
- Strength training, stair climbing and elliptical are under `fitness_equipment`.
- Floor climbing and breathwork are under `other`.

**Sleep dates:** Garmin files a night under the day you woke up, so last night's sleep is today's date.

## Requirements

- Rust 1.88 or newer (install with [rustup](https://rustup.rs)).
- A Garmin Connect personal API key (see [Garmin's guide](https://developer.garmin.com/gc-developer-program/personal/personal-api/)).

## Setup

```sh
git clone git@github.com:gpodliegaiev/garmin-mcp.git
cd garmin-mcp
cp .env.example .env    # then set GARMIN_API_KEY in .env
cargo build --release
```

The binary is written to `target/release/garmin-mcp`. Rebuild after pulling changes.

### Configuration

| Variable | Required | Default |
|---|---|---|
| `GARMIN_API_KEY` | yes | none |
| `GARMIN_BASE_URL` | no | `https://personal.connectapi.garmin.com` |

The server reads variables from the environment and from a `.env` file. Environment variables take priority.

> **Note:** `.env` is only found in the directory the server is started from, or one of its parents. MCP clients often start servers from another directory. If the server exits with `GARMIN_API_KEY is not set`, pass the key through the client's config instead, as shown below.

`.env` is git-ignored. Never commit your API key.

## Connecting a client

### Claude Code

To make the server available in every project, passing the key directly:

```sh
claude mcp add -s user garmin -e GARMIN_API_KEY=<your-key> -- /path/to/garmin-mcp/target/release/garmin-mcp
```

To use it only when Claude Code is started inside this repository (the key is read from `.env`):

```sh
claude mcp add garmin -- /path/to/garmin-mcp/target/release/garmin-mcp
```

Check the connection with `claude mcp list`, or with `/mcp` inside a session. Then ask, for example, "What were my last 3 runs?" or "How did I sleep last night?"

### Claude Desktop

Add the server to `~/Library/Application Support/Claude/claude_desktop_config.json` and restart the app:

```json
{
  "mcpServers": {
    "garmin": {
      "command": "/path/to/garmin-mcp/target/release/garmin-mcp",
      "env": { "GARMIN_API_KEY": "<your-key>" }
    }
  }
}
```

With either client, a key passed this way is stored in plain text in the client's config file.

## How it works

1. On the first tool call, the server exchanges your API key for a bearer token:
   `POST /personalapitoken-service/personal-api/personal-api-key/token` with the header `Authorization: ApiKey <key>`.
2. The token is valid for 24 hours. It is kept in memory and refreshed 5 minutes before it expires.
3. Every API request is sent with `Authorization: Bearer <token>`. If Garmin answers `401`, the server fetches a new token and retries once.
4. Responses are reduced to the useful fields, which makes results about 80% smaller than Garmin's raw JSON.

The server talks to the client over stdin/stdout. Logs go to stderr.

## Development

```sh
cargo build                 # debug build
cargo run                   # run on stdio (waits for an MCP client)
cargo test                  # unit tests (no network needed)
cargo clippy --all-targets  # lint
cargo fmt                   # format
```

### Project layout

```
src/
├── main.rs           entry point: loads config, starts the server on stdio
├── config.rs         reads GARMIN_API_KEY / GARMIN_BASE_URL
├── server.rs         MCP tools, one #[tool] method each (see the table above)
└── garmin/
    ├── activity.rs         activity list entries (Activity)
    ├── activity_detail.rs  one activity's full details (ActivityDetail)
    ├── auth.rs             bearer token fetch and cache
    ├── client.rs           authenticated HTTP requests and API calls
    ├── daily_summary.rs    daily wellness summaries (DailySummaries, UserDailySummary)
    ├── fitness_age.rs      Fitness Age and its inputs (FitnessAge)
    ├── hrv.rs              heart rate variability per night (HrvRange, HrvNight)
    ├── sleep.rs            one night's sleep (DailySleep)
    └── types.rs            shared unit helpers, ActivityTypeFilter
```

To add a tool:
1. Add the API call to `garmin/client.rs`.
2. Add its response types in a new file under `garmin/`. Output every useful field Garmin returns (see CLAUDE.md, "Response fields").
3. Add a `#[tool]` method in `server.rs`.

The code is commented heavily, as a Rust learning resource.

### Garmin API docs

`docs/` holds a local copy of Garmin's [swagger docs](https://developer.garmin.com/gc-developer-program/personal/swagger-docs/):
- `garmin-api.md`: a readable index of every endpoint, marking which ones this server implements and the MCP tool for each.
- `garmin-openapi.json`: the full OpenAPI spec.

To refresh both, run `python3 docs/update_garmin_spec.py`. After adding a tool, run it with `--offline` to update only the implementation status from the local spec.
