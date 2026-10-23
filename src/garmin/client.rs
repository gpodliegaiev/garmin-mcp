//! HTTP client for the Garmin personal API.
//!
//! `GarminClient` adds the bearer token to every request, retries once with a
//! new token if Garmin says the current one is no longer accepted, and turns
//! JSON responses into Rust types (see `types.rs`).

use std::time::Duration;

use anyhow::Context;
use reqwest::StatusCode;
// `NaiveDate` is a calendar date with no time zone, e.g. 2026-10-23.
use chrono::NaiveDate;
use serde::Serialize;
// `DeserializeOwned` is the trait for "a type serde can build from JSON
// without borrowing from the input text". Used as a bound on `get_json`.
use serde::de::DeserializeOwned;

// `super::` means "the parent module" (`garmin`), so this is
// `crate::garmin::auth::TokenManager`.
use super::{
    auth::TokenManager,
    types::{Activity, DailySleep, SleepSummary},
};
use crate::config::Config;

/// Give up on a request if Garmin does not answer within this time.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Path of the activity search endpoint.
const ACTIVITIES_PATH: &str = "/activitylist-service/activities/search/activities";

/// Path of the daily sleep endpoint. Requires a `date=YYYY-MM-DD` parameter.
const DAILY_SLEEP_PATH: &str = "/sleep-service/sleep/dailySleep";

/// Authenticated access to the Garmin API.
pub struct GarminClient {
    /// Connection pool used for every request.
    http: reqwest::Client,
    /// Root URL, e.g. `https://personal.connectapi.garmin.com`.
    base_url: String,
    /// Provides (and caches) the bearer token.
    tokens: TokenManager,
}

impl GarminClient {
    /// Builds a client from the loaded configuration.
    ///
    /// Takes `&Config` (a borrow) because it only needs to read the config,
    /// not take ownership of it.
    pub fn new(config: &Config) -> anyhow::Result<Self> {
        // One `reqwest::Client` for the whole program, so connections are
        // reused between requests.
        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .context("could not create HTTP client")?;

        Ok(Self {
            // `http.clone()` gives the token manager its own handle to the
            // same connection pool.
            tokens: TokenManager::new(http.clone(), &config.base_url, config.api_key.clone()),
            http,
            base_url: config.base_url.clone(),
        })
    }

    /// Lists recent activities, newest first.
    ///
    /// - `start`: how many activities to skip (for paging).
    /// - `limit`: how many to return.
    pub async fn list_activities(&self, start: u32, limit: u32) -> anyhow::Result<Vec<Activity>> {
        // `&[...]` is a borrowed slice of (name, value) pairs; reqwest turns
        // them into `?start=0&limit=5`. Garmin returns a JSON array, which
        // serde turns into a `Vec<Activity>` (a growable list).
        self.get_json(ACTIVITIES_PATH, &[("start", start), ("limit", limit)])
            .await
    }

    /// Gets the sleep summary for one night.
    ///
    /// `date` is the calendar day Garmin files the sleep under (the day you
    /// woke up). Returns `Ok(None)` when nothing was recorded for that day.
    pub async fn daily_sleep(&self, date: NaiveDate) -> anyhow::Result<Option<SleepSummary>> {
        // `NaiveDate`'s `Display` format is `YYYY-MM-DD`, exactly what Garmin
        // expects, so `.to_string()` gives the query value.
        let date = date.to_string();
        // `date.as_str()` borrows the `String` as a `&str` for the query.
        let raw: DailySleep = self
            .get_json(DAILY_SLEEP_PATH, &[("date", date.as_str())])
            .await?;
        // Convert Garmin's nested format into our flat summary (or `None`).
        Ok(raw.into_summary())
    }

    /// Sends an authenticated `GET` request and parses the JSON response
    /// into `T`.
    ///
    /// `<T: DeserializeOwned>` makes this function *generic*: the caller picks
    /// the result type (e.g. `Vec<Activity>`), and the compiler checks that
    /// serde knows how to build it from JSON.
    ///
    /// `query` can be anything serde can turn into URL parameters, such as
    /// `&[("start", 0), ("limit", 5)]` or `&[("date", "2026-10-23")]`. That is
    /// what the second generic parameter `Q: Serialize` expresses. `?Sized`
    /// relaxes a default rule so `Q` may be a slice like `[(&str, u32)]`,
    /// whose size isn't known at compile time (we only use it via `&Q`).
    async fn get_json<T: DeserializeOwned, Q: Serialize + ?Sized>(
        &self,
        path: &str,
        query: &Q,
    ) -> anyhow::Result<T> {
        let token = self.tokens.get_token().await?;
        let mut response = self.send_get(path, query, &token).await?;

        // 401 means Garmin no longer accepts this token. Drop it, get a new
        // one, and try exactly once more.
        if response.status() == StatusCode::UNAUTHORIZED {
            eprintln!("garmin: token rejected (401), refreshing and retrying");
            self.tokens.invalidate(&token).await;
            let token = self.tokens.get_token().await?;
            response = self.send_get(path, query, &token).await?;
        }

        let status = response.status();
        if !status.is_success() {
            // Include part of the body: Garmin's error message is often there.
            // `.unwrap_or_default()` uses an empty string if reading fails.
            let body = response.text().await.unwrap_or_default();
            // `.chars().take(500)` keeps at most 500 characters (not bytes,
            // so we never cut a multi-byte character in half).
            let snippet: String = body.chars().take(500).collect();
            anyhow::bail!("GET {path} failed: HTTP {status}: {snippet}");
        }

        // Parse the body as JSON into `T`. `::<T>` (the "turbofish") tells
        // `.json` which type to produce. This fails if the JSON doesn't match
        // `T`, e.g. a field has an unexpected type.
        response
            .json::<T>()
            .await
            .with_context(|| format!("GET {path}: unexpected response format"))
    }

    /// Sends one `GET` request with the given bearer token.
    async fn send_get<Q: Serialize + ?Sized>(
        &self,
        path: &str,
        query: &Q,
        token: &str,
    ) -> anyhow::Result<reqwest::Response> {
        self.http
            .get(format!("{}{path}", self.base_url))
            .query(query)
            // `bearer_auth` sets `Authorization: Bearer <token>`.
            .bearer_auth(token)
            .send()
            .await
            // `with_context` takes a closure, so the message is only built
            // if there actually is an error.
            .with_context(|| format!("GET {path} failed (network error)"))
    }
}
