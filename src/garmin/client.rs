//! HTTP client for the Garmin personal API.
//!
//! `GarminClient` adds the bearer token to every request, retries once with a
//! new token if Garmin says the current one is no longer accepted, and turns
//! JSON responses into Rust types (see `types.rs`).

use std::time::Duration;

use anyhow::Context;
use reqwest::StatusCode;
// `DeserializeOwned` is the trait for "a type serde can build from JSON
// without borrowing from the input text". Used as a bound on `get_json`.
use serde::de::DeserializeOwned;

// `super::` means "the parent module" (`garmin`), so this is
// `crate::garmin::auth::TokenManager`.
use super::{auth::TokenManager, types::Activity};
use crate::config::Config;

/// Give up on a request if Garmin does not answer within this time.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Path of the activity search endpoint.
const ACTIVITIES_PATH: &str = "/activitylist-service/activities/search/activities";

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

    /// Sends an authenticated `GET` request and parses the JSON response
    /// into `T`.
    ///
    /// `<T: DeserializeOwned>` makes this function *generic*: the caller picks
    /// the result type (e.g. `Vec<Activity>`), and the compiler checks that
    /// serde knows how to build it from JSON.
    ///
    /// `query: &[(&str, u32)]` is a slice of query parameters. If we later
    /// need text parameters, this can be made generic too.
    async fn get_json<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, u32)],
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
    async fn send_get(
        &self,
        path: &str,
        query: &[(&str, u32)],
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
