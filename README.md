# garmin-mcp

An [MCP](https://modelcontextprotocol.io) server, written in Rust, that gives AI assistants such as Claude Code and Claude Desktop read access to your Garmin Connect data through the [Garmin Connect personal API](https://developer.garmin.com/gc-developer-program/personal/personal-api/).

## Tools

| Tool | What it returns | Arguments |
|---|---|---|
| `list_activities` | Your most recent activities, newest first: name, sport, start time, duration, distance, heart rate, calories, training effect, and for strength training, sets and reps. | `start` (default 0), `limit` (default 5, max 100), `activity_type` (optional, see below) |
| `get_sleep` | One night's sleep: bed and wake times, deep/light/REM/awake durations, sleep score, average heart rate, respiration and stress, and recommended sleep. | `date` as `YYYY-MM-DD` (default today) |

Each field name includes its unit (`duration_min`, `distance_km`, `elevation_gain_m`, `calories_kcal`). Heart rate is in bpm. Speed is given in the usual way for each sport:

| Sport | Distance | Speed |
|---|---|---|
| Running, walking, hiking | `distance_km` | `avg_pace_per_km`, e.g. `"6:23"` |
| Swimming | `distance_m` | `avg_pace_per_100m`, e.g. `"2:05"` |
| Cycling and other sports | `distance_km` | `avg_speed_kmh` |

Fields that don't apply to an activity are left out. Examples: distance for strength training, moving time for indoor activities. Sleep durations are in minutes.

**`activity_type` values:** `running`, `cycling`, `walking`, `hiking`, `swimming`, `fitness_equipment`, `other`, `multi_sport`, `winter_sports`, `water_sports`, `diving`, `team_sports`, `racket_sports`.

Garmin filters by sport group, not by exact type:
- `running` includes treadmill runs.
- Strength training, stair climbing and elliptical are under `fitness_equipment`.

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
├── server.rs         MCP tools (list_activities, get_sleep)
└── garmin/
    ├── auth.rs       bearer token fetch and cache
    ├── client.rs     authenticated HTTP requests and API calls
    └── types.rs      response structs (Activity, SleepSummary, ActivityTypeFilter)
```

To add a tool:
1. Add the API call to `garmin/client.rs`.
2. Add its response types to `garmin/types.rs`.
3. Add a `#[tool]` method in `server.rs`.

The code is commented heavily, as a Rust learning resource.
