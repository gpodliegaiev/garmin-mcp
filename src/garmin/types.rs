//! Rust types for Garmin API responses.
//!
//! Garmin responses are large (~70 fields per activity, many nested objects
//! for sleep). These structs keep only the fields that are useful to an AI
//! assistant, which makes tool results much smaller.
//! serde ignores any JSON field that has no matching struct field.

use rmcp::schemars::{self, JsonSchema};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Sport groups accepted by the activity list's `activityType` filter.
///
/// Garmin filters only by *parent* type: `running` also returns treadmill
/// runs, and strength training is under `fitness_equipment`. Exact subtypes
/// like `strength_training` are rejected by Garmin, so they aren't listed.
///
/// An `enum` is a type whose value is exactly one of a fixed set of
/// *variants*. Using one here (instead of a free-form `String`) means:
/// - the AI client sees the allowed values in the tool's JSON schema
///   (`JsonSchema`), and
/// - anything else is rejected while reading the tool arguments
///   (`Deserialize`), before we ever call Garmin.
///
/// `rename_all = "snake_case"` maps `FitnessEquipment` <-> `"fitness_equipment"`
/// both when reading tool input and when writing the URL (`Serialize`).
/// `Clone, Copy` let the value be duplicated freely; it is tiny.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActivityTypeFilter {
    /// Running, including treadmill, trail and track running.
    Running,
    /// Cycling, including indoor cycling.
    Cycling,
    /// Walking.
    Walking,
    /// Hiking.
    Hiking,
    /// Swimming (pool and open water).
    Swimming,
    /// Gym activities: strength training, stair climbing, elliptical, etc.
    FitnessEquipment,
    /// Activities that fit no other group.
    Other,
    /// Multisport, e.g. triathlon.
    MultiSport,
    /// Skiing, snowboarding and other winter sports.
    WinterSports,
    /// Rowing, paddling, surfing and other water sports.
    WaterSports,
    /// Diving.
    Diving,
    /// Team sports, e.g. football, basketball.
    TeamSports,
    /// Racket sports, e.g. tennis, padel.
    RacketSports,
}

/// One activity (run, walk, strength session, ...) from the activity list.
///
/// The same struct is used in both directions:
/// - *Deserialize* reads Garmin's JSON, whose field names are camelCase
///   (`startTimeLocal`).
/// - *Serialize* writes our tool output, with snake_case names
///   (`start_time_local`), matching the Rust field names.
///
/// `rename_all(deserialize = "camelCase")` applies the camelCase names only
/// when reading; when writing, the Rust names are used as they are.
///
/// Fields wrapped in `Option<T>` may be missing: many only exist for some
/// sports (e.g. `total_reps` for strength training). `#[serde(default)]` on
/// the struct means "a missing field becomes its default" (`None` for an
/// `Option`), and `skip_serializing_if` leaves `None` fields out of the
/// output entirely instead of writing `null`.
///
/// Units are Garmin's: meters, seconds, meters per second.
// `Default` is derived so serde's `default` attribute (below) can fill in
// missing fields: `0` for numbers, `""` for strings, `None` for options.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all(deserialize = "camelCase"), default)]
pub struct Activity {
    /// Garmin's unique id for the activity.
    pub activity_id: u64,
    /// Name shown in Garmin Connect, e.g. "Morning Run".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activity_name: Option<String>,
    /// Sport, e.g. `running`, `walking`, `strength_training`.
    // Garmin sends `"activityType": {"typeId": 1, "typeKey": "running", ...}`.
    // `deserialize_with` names a function (below) that pulls out just the
    // `typeKey` string, so the output is simply `"activity_type": "running"`.
    #[serde(deserialize_with = "type_key")]
    pub activity_type: String,
    /// Start time in the activity's local time zone, "YYYY-MM-DD HH:MM:SS".
    pub start_time_local: String,
    /// Place name, if the activity had GPS.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location_name: Option<String>,

    /// Total duration in seconds.
    // Rounded to 0.1 s on output; Garmin's extra digits are milliseconds and
    // float noise, irrelevant for any health evaluation.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "round_1")]
    pub duration: Option<f64>,
    /// Time spent actually moving, in seconds.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "round_1")]
    pub moving_duration: Option<f64>,
    /// Distance in meters (0 for activities without distance).
    // Rounded to centimeters: this is Garmin's real precision, the longer
    // tail (e.g. `4380.18017578125`) is float32 conversion noise.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "round_2")]
    pub distance: Option<f64>,
    /// Average speed in meters per second.
    // Rounded to 3 decimals, Garmin's real precision (`1.1710000038` is
    // float noise for `1.171`). Kept that fine because pace is derived from it.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "round_3")]
    pub average_speed: Option<f64>,
    /// Total climb in meters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elevation_gain: Option<f64>,
    /// Step count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steps: Option<u64>,

    /// Average heart rate in beats per minute.
    // Garmin writes `averageHR`, not `averageHr`, so the automatic camelCase
    // rename would not match; `rename(deserialize = ...)` sets it by hand.
    #[serde(
        rename(deserialize = "averageHR"),
        skip_serializing_if = "Option::is_none"
    )]
    pub average_hr: Option<f64>,
    /// Maximum heart rate in beats per minute.
    #[serde(rename(deserialize = "maxHR"), skip_serializing_if = "Option::is_none")]
    pub max_hr: Option<f64>,
    /// Average power in watts (only with a power meter or running power).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_power: Option<f64>,
    /// Active calories burned (kcal).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calories: Option<f64>,

    /// Aerobic training effect, 0.0 to 5.0.
    // Garmin's training effect scale has 0.1 steps; `2.4000000953674316` is
    // float noise for `2.4`, so rounding loses nothing.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "round_1")]
    pub aerobic_training_effect: Option<f64>,
    /// Anaerobic training effect, 0.0 to 5.0.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "round_1")]
    pub anaerobic_training_effect: Option<f64>,
    /// Garmin's label for the main benefit, e.g. `TEMPO`, `ANAEROBIC_CAPACITY`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub training_effect_label: Option<String>,
    /// Garmin's training load score for this activity.
    // Rounded to 0.1; Garmin itself only shows whole numbers.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "round_1")]
    pub activity_training_load: Option<f64>,
    /// Change in Body Battery during the activity (usually negative).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub difference_body_battery: Option<i64>,
    /// Minutes of moderate intensity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moderate_intensity_minutes: Option<u32>,
    /// Minutes of vigorous intensity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vigorous_intensity_minutes: Option<u32>,

    /// Strength training: number of sets.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_sets: Option<u32>,
    /// Strength training: number of repetitions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_reps: Option<u32>,
}

/// Writes an `Option<f64>` rounded to `decimals` places (or `null`).
///
/// Garmin stores many numbers as 32-bit floats. Converted to the 64-bit
/// floats we use, `2.4` turns into `2.4000000953674316`: the extra digits are
/// conversion noise, not real measurement detail. Rounding to the value's
/// real precision gives shorter, cleaner output without losing information.
///
/// `S: Serializer` is generic like `D: Deserializer` below: "any output
/// format". `S::Ok` and `S::Error` are that format's success and error types.
fn serialize_rounded<S: Serializer>(
    value: &Option<f64>,
    decimals: i32,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match value {
        // `*v` *dereferences* the `&f64` borrowed from the `Option` to get the
        // number itself. `10f64.powi(n)` is 10 to the power `n`.
        Some(v) => {
            let factor = 10f64.powi(decimals);
            serializer.serialize_some(&((*v * factor).round() / factor))
        }
        None => serializer.serialize_none(),
    }
}

// `#[serde(serialize_with = "...")]` needs a function with exactly the shape
// `fn(&T, S) -> Result<S::Ok, S::Error>`, so the number of decimals can't be
// an extra argument there. These three small wrappers fix it per field.

/// Rounds to 1 decimal place when serializing.
fn round_1<S: Serializer>(value: &Option<f64>, serializer: S) -> Result<S::Ok, S::Error> {
    serialize_rounded(value, 1, serializer)
}

/// Rounds to 2 decimal places when serializing.
fn round_2<S: Serializer>(value: &Option<f64>, serializer: S) -> Result<S::Ok, S::Error> {
    serialize_rounded(value, 2, serializer)
}

/// Rounds to 3 decimal places when serializing.
fn round_3<S: Serializer>(value: &Option<f64>, serializer: S) -> Result<S::Ok, S::Error> {
    serialize_rounded(value, 3, serializer)
}

/// Reads Garmin's `activityType` object and returns only its `typeKey`.
///
/// Used through `#[serde(deserialize_with = "type_key")]` above.
///
/// The signature looks scary but is standard serde boilerplate:
/// - `'de` is a *lifetime*: a name for "how long the input data lives".
///   serde uses it so deserialized values may borrow from the input.
/// - `D: Deserializer<'de>` means "any JSON reader"; it is a *generic*
///   parameter, so this one function works with whatever reader serde uses.
/// - `D::Error` is that reader's error type.
fn type_key<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    // A tiny struct that exists only inside this function, describing the
    // part of `activityType` we care about. Other fields are ignored.
    #[derive(Deserialize)]
    struct ActivityType {
        #[serde(rename = "typeKey")]
        type_key: String,
    }

    // Deserialize the object into that struct, then return just the string.
    Ok(ActivityType::deserialize(deserializer)?.type_key)
}

/// Garmin's daily sleep response, as it arrives.
///
/// Only used for reading (`Deserialize`); it is converted into the flatter
/// [`SleepSummary`] before being sent to the client. Every field is an
/// `Option` because, for a day without sleep data, Garmin still answers
/// with the same object but with every field set to `null`.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DailySleep {
    /// Day the sleep is filed under, "YYYY-MM-DD". `None` means no data.
    calendar_date: Option<String>,
    /// Total time asleep, in seconds.
    sleep_time_seconds: Option<u32>,
    /// Time spent napping during the day, in seconds.
    nap_time_seconds: Option<u32>,
    /// Bedtime as milliseconds since 1970, already shifted to local time.
    sleep_start_timestamp_local: Option<i64>,
    /// Wake-up time, same format as the start.
    sleep_end_timestamp_local: Option<i64>,
    deep_sleep_seconds: Option<u32>,
    light_sleep_seconds: Option<u32>,
    rem_sleep_seconds: Option<u32>,
    awake_sleep_seconds: Option<u32>,
    awake_count: Option<u32>,
    avg_heart_rate: Option<f64>,
    average_respiration_value: Option<f64>,
    avg_sleep_stress: Option<f64>,
    /// Short code describing the night, e.g. `POSITIVE_HIGHLY_RECOVERING`.
    sleep_score_feedback: Option<String>,
    /// Short code with an extra insight, e.g. `POSITIVE_RESTFUL_EVENING`.
    sleep_score_insight: Option<String>,
    /// Sleep score and per-stage scores (nested object).
    sleep_scores: Option<SleepScores>,
    /// How much sleep Garmin recommended for this night (nested object).
    sleep_need: Option<SleepNeed>,
}

/// The `sleepScores` object. Garmin includes more sub-scores; we keep these.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct SleepScores {
    overall: Option<Score>,
    deep_percentage: Option<Score>,
    light_percentage: Option<Score>,
    rem_percentage: Option<Score>,
}

/// One score inside `sleepScores`: a number plus a rating word.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Score {
    /// The number (0–100 for the overall score, a percentage for stages).
    value: Option<u32>,
    /// Rating such as `EXCELLENT`, `GOOD`, `FAIR`, `POOR`.
    qualifier_key: Option<String>,
}

/// The `sleepNeed` object.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct SleepNeed {
    /// Recommended sleep for the night, in minutes.
    actual: Option<u32>,
}

/// One night's sleep, in the flat shape the tool returns.
///
/// Only used for writing (`Serialize`). Fields Garmin did not provide are
/// written as `null`.
#[derive(Debug, Serialize)]
pub struct SleepSummary {
    /// Day the sleep is filed under (the day you woke up), "YYYY-MM-DD".
    pub calendar_date: String,
    /// Bedtime, local time, "YYYY-MM-DD HH:MM:SS".
    pub sleep_start_local: Option<String>,
    /// Wake-up time, local time, "YYYY-MM-DD HH:MM:SS".
    pub sleep_end_local: Option<String>,
    /// Total time asleep, in seconds.
    pub total_sleep_seconds: Option<u32>,
    pub deep_sleep_seconds: Option<u32>,
    pub light_sleep_seconds: Option<u32>,
    pub rem_sleep_seconds: Option<u32>,
    pub awake_seconds: Option<u32>,
    pub nap_seconds: Option<u32>,
    /// Number of times awake during the night.
    pub awake_count: Option<u32>,
    /// Garmin sleep score, 0–100.
    pub sleep_score: Option<u32>,
    /// Rating of the sleep score, e.g. `EXCELLENT`.
    pub sleep_score_rating: Option<String>,
    /// Share of sleep in each stage, in percent.
    pub deep_percent: Option<u32>,
    pub light_percent: Option<u32>,
    pub rem_percent: Option<u32>,
    /// Average heart rate during sleep, bpm.
    pub avg_heart_rate: Option<f64>,
    /// Average breaths per minute.
    pub avg_respiration: Option<f64>,
    /// Average stress level during sleep (0–100, lower is calmer).
    pub avg_sleep_stress: Option<f64>,
    /// Recommended sleep for the night, in minutes.
    pub sleep_need_minutes: Option<u32>,
    /// Garmin's feedback code for the night.
    pub feedback: Option<String>,
    /// Garmin's insight code for the night.
    pub insight: Option<String>,
}

impl DailySleep {
    /// Flattens Garmin's response into a [`SleepSummary`].
    ///
    /// Takes `self` by value (no `&`): the raw struct is *consumed*, so its
    /// strings can be moved into the summary instead of copied.
    ///
    /// Returns `None` if Garmin had no sleep data for the day.
    pub fn into_summary(self) -> Option<SleepSummary> {
        // `?` also works on `Option`: if `calendar_date` is `None`, this
        // function returns `None` right here.
        let calendar_date = self.calendar_date?;

        // Destructure the nested objects, using empty defaults when missing,
        // so the code below doesn't need to check each level for `None`.
        // `unwrap_or_default()` gives `SleepScores::default()` if `None`.
        let scores = self.sleep_scores.unwrap_or_default();
        let overall = scores.overall.unwrap_or_default();

        Some(SleepSummary {
            calendar_date,
            // `.and_then(f)` runs `f` only if there is a value, and `f` may
            // itself return `None` (here: if the timestamp is out of range).
            sleep_start_local: self.sleep_start_timestamp_local.and_then(local_time),
            sleep_end_local: self.sleep_end_timestamp_local.and_then(local_time),
            total_sleep_seconds: self.sleep_time_seconds,
            deep_sleep_seconds: self.deep_sleep_seconds,
            light_sleep_seconds: self.light_sleep_seconds,
            rem_sleep_seconds: self.rem_sleep_seconds,
            awake_seconds: self.awake_sleep_seconds,
            nap_seconds: self.nap_time_seconds,
            awake_count: self.awake_count,
            sleep_score: overall.value,
            sleep_score_rating: overall.qualifier_key,
            // `.and_then(|s| s.value)` pulls `value` out of an
            // `Option<Score>`, giving `None` if either level is missing.
            deep_percent: scores.deep_percentage.and_then(|s| s.value),
            light_percent: scores.light_percentage.and_then(|s| s.value),
            rem_percent: scores.rem_percentage.and_then(|s| s.value),
            avg_heart_rate: self.avg_heart_rate,
            avg_respiration: self.average_respiration_value,
            avg_sleep_stress: self.avg_sleep_stress,
            sleep_need_minutes: self.sleep_need.and_then(|n| n.actual),
            feedback: self.sleep_score_feedback,
            insight: self.sleep_score_insight,
        })
    }
}

/// Formats one of Garmin's "local" timestamps as "YYYY-MM-DD HH:MM:SS".
///
/// Garmin's `...TimestampLocal` values are milliseconds since 1970 that have
/// already been shifted by the user's time-zone offset. Reading them as if
/// they were UTC therefore gives the local wall-clock time.
fn local_time(millis: i64) -> Option<String> {
    // `from_timestamp_millis` returns `None` for impossible values.
    chrono::DateTime::from_timestamp_millis(millis)
        .map(|dt| dt.naive_utc().format("%Y-%m-%d %H:%M:%S").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trimmed-down copy of a real strength-training entry from Garmin.
    // `r#"..."#` is a *raw string*: quotes inside need no escaping.
    const STRENGTH_JSON: &str = r#"{
        "activityId": 24824822921,
        "activityName": "Leg day",
        "activityType": {"typeId": 13, "typeKey": "strength_training", "parentTypeId": 29},
        "startTimeLocal": "2026-10-23 13:24:21",
        "duration": 4239.2,
        "distance": 0.0,
        "averageHR": 124.0,
        "maxHR": 159.0,
        "calories": 617.0,
        "totalSets": 24,
        "totalReps": 285,
        "ownerFullName": "ignored",
        "splitSummaries": []
    }"#;

    #[test]
    fn reads_garmin_json() {
        // `serde_json::from_str` parses text into our struct. `.unwrap()`
        // panics (fails the test) if parsing returns an error.
        let activity: Activity = serde_json::from_str(STRENGTH_JSON).unwrap();
        assert_eq!(activity.activity_id, 24824822921);
        assert_eq!(activity.activity_type, "strength_training");
        assert_eq!(activity.average_hr, Some(124.0));
        assert_eq!(activity.total_reps, Some(285));
        // Not in the JSON, so it falls back to `None`.
        assert_eq!(activity.avg_power, None);
    }

    #[test]
    fn rounds_float_noise_on_output() {
        // Raw values exactly as Garmin sends them.
        let json = r#"{
            "activityId": 1,
            "activityType": {"typeKey": "walking"},
            "duration": 3741.52587890625,
            "distance": 4380.18017578125,
            "averageSpeed": 1.1710000038146973,
            "aerobicTrainingEffect": 2.4000000953674316,
            "activityTrainingLoad": 62.689056396484375,
            "averageHR": 83.0
        }"#;
        let activity: Activity = serde_json::from_str(json).unwrap();
        let out = serde_json::to_value(&activity).unwrap();
        assert_eq!(out["duration"], 3741.5);
        assert_eq!(out["distance"], 4380.18);
        assert_eq!(out["average_speed"], 1.171);
        assert_eq!(out["aerobic_training_effect"], 2.4);
        assert_eq!(out["activity_training_load"], 62.7);
        // Fields without rounding are written unchanged.
        assert_eq!(out["average_hr"], 83.0);
        // Missing values are still skipped, not written as rounded `null`s.
        assert!(out.get("moving_duration").is_none());
    }

    #[test]
    fn writes_snake_case_and_skips_missing_fields() {
        let activity: Activity = serde_json::from_str(STRENGTH_JSON).unwrap();
        let out = serde_json::to_value(&activity).unwrap();
        assert_eq!(out["activity_type"], "strength_training");
        assert_eq!(out["average_hr"], 124.0);
        // `None` fields are left out, and Garmin fields we don't model
        // (like `ownerFullName`) never appear.
        assert!(out.get("avg_power").is_none());
        assert!(out.get("ownerFullName").is_none());
    }

    /// Trimmed-down copy of a real `dailySleep` response.
    const SLEEP_JSON: &str = r#"{
        "calendarDate": "2026-10-23",
        "sleepTimeSeconds": 29040,
        "sleepStartTimestampLocal": 1792717086000,
        "sleepEndTimestampLocal": 1792746786000,
        "deepSleepSeconds": 4500,
        "avgHeartRate": 51.0,
        "sleepScores": {
            "overall": {"value": 91, "qualifierKey": "EXCELLENT"},
            "deepPercentage": {"value": 15, "qualifierKey": "FAIR", "optimalStart": 16.0}
        },
        "sleepNeed": {"baseline": 490, "actual": 490}
    }"#;

    #[test]
    fn flattens_sleep_response() {
        let raw: DailySleep = serde_json::from_str(SLEEP_JSON).unwrap();
        // `.expect(msg)` is like `.unwrap()` but with a custom panic message.
        let sleep = raw.into_summary().expect("has data");
        assert_eq!(sleep.calendar_date, "2026-10-23");
        assert_eq!(sleep.sleep_score, Some(91));
        assert_eq!(sleep.sleep_score_rating.as_deref(), Some("EXCELLENT"));
        assert_eq!(sleep.deep_percent, Some(15));
        // Missing in the JSON, so `None`.
        assert_eq!(sleep.rem_percent, None);
        assert_eq!(sleep.sleep_need_minutes, Some(490));
        assert_eq!(
            sleep.sleep_start_local.as_deref(),
            Some("2026-10-23 00:58:06")
        );
        assert_eq!(
            sleep.sleep_end_local.as_deref(),
            Some("2026-10-23 09:13:06")
        );
    }

    #[test]
    fn all_null_sleep_response_means_no_data() {
        // What Garmin sends for a day without sleep data.
        let json = r#"{"calendarDate": null, "sleepTimeSeconds": null, "sleepScores": null}"#;
        let raw: DailySleep = serde_json::from_str(json).unwrap();
        assert!(raw.into_summary().is_none());
    }
}
