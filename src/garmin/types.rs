//! Rust types for Garmin API responses.
//!
//! Garmin responses are large (~70 fields per activity, many nested objects
//! for sleep). These structs keep only the fields that are useful to an AI
//! assistant, which makes tool results much smaller.
//! serde ignores any JSON field that has no matching struct field.

use rmcp::schemars::{self, JsonSchema};
use serde::{Deserialize, Deserializer, Serialize};

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

/// One activity (run, walk, strength session, ...) as Garmin sends it.
///
/// Only used for reading (`Deserialize`). It is converted into the
/// [`ActivitySummary`] the tool returns, which uses friendlier units.
///
/// `rename_all = "camelCase"` matches Garmin's field names
/// (`startTimeLocal` <-> `start_time_local`). Fields wrapped in `Option<T>`
/// may be missing: many only exist for some sports (e.g. `total_reps` for
/// strength training). `#[serde(default)]` on the struct means "a missing
/// field becomes its default" (`None` for an `Option`).
///
/// Units are Garmin's: meters, seconds, meters per second.
// `Default` is derived so serde's `default` attribute (above) can fill in
// missing fields: `0` for numbers, `""` for strings, `None` for options.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Activity {
    activity_id: u64,
    activity_name: Option<String>,
    // Garmin sends `"activityType": {"typeId": 1, "typeKey": "running", ...}`.
    // `deserialize_with` names a function (below) that pulls out just the
    // `typeKey` string.
    #[serde(deserialize_with = "type_key")]
    activity_type: String,
    start_time_local: String,
    location_name: Option<String>,
    /// Seconds.
    duration: Option<f64>,
    /// Seconds.
    moving_duration: Option<f64>,
    /// Meters.
    distance: Option<f64>,
    /// Meters per second.
    average_speed: Option<f64>,
    /// Meters.
    elevation_gain: Option<f64>,
    steps: Option<u64>,
    // Garmin writes `averageHR`, not `averageHr`, so the automatic camelCase
    // rename would not match; `rename` sets it by hand.
    #[serde(rename = "averageHR")]
    average_hr: Option<f64>,
    #[serde(rename = "maxHR")]
    max_hr: Option<f64>,
    avg_power: Option<f64>,
    calories: Option<f64>,
    aerobic_training_effect: Option<f64>,
    anaerobic_training_effect: Option<f64>,
    training_effect_label: Option<String>,
    activity_training_load: Option<f64>,
    difference_body_battery: Option<i64>,
    moderate_intensity_minutes: Option<u32>,
    vigorous_intensity_minutes: Option<u32>,
    total_sets: Option<u32>,
    total_reps: Option<u32>,
}

/// One activity, in the shape and units the tool returns.
///
/// Only used for writing (`Serialize`). Each unit is in the field name.
/// `skip_serializing_if` leaves `None` fields out of the output entirely
/// instead of writing `null`, so each activity only shows what applies to
/// its sport.
#[derive(Debug, Serialize)]
pub struct ActivitySummary {
    /// Garmin's unique id for the activity.
    pub activity_id: u64,
    /// Name shown in Garmin Connect, e.g. "Morning Run".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activity_name: Option<String>,
    /// Sport, e.g. `running`, `walking`, `strength_training`.
    pub activity_type: String,
    /// Start time in the activity's local time zone, "YYYY-MM-DD HH:MM:SS".
    pub start_time_local: String,
    /// Place name, if the activity had GPS.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location_name: Option<String>,

    /// Total duration in minutes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_min: Option<f64>,
    /// Time spent actually moving, in minutes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moving_duration_min: Option<f64>,
    /// Distance in kilometers (all sports except swimming).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance_km: Option<f64>,
    /// Distance in meters (swimming only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance_m: Option<f64>,
    /// Average pace as "minutes:seconds" per km (running, walking, hiking).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_pace_per_km: Option<String>,
    /// Average pace as "minutes:seconds" per 100 m (swimming).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_pace_per_100m: Option<String>,
    /// Average speed in km/h (cycling and other distance sports).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_speed_kmh: Option<f64>,
    /// Total climb in meters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elevation_gain_m: Option<f64>,
    /// Step count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steps: Option<u64>,

    /// Average heart rate in beats per minute.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub average_hr: Option<f64>,
    /// Maximum heart rate in beats per minute.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_hr: Option<f64>,
    /// Average power in watts (only with a power meter or running power).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_power_w: Option<f64>,
    /// Active calories burned (kcal).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calories_kcal: Option<f64>,

    /// Aerobic training effect, 0.0 to 5.0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aerobic_training_effect: Option<f64>,
    /// Anaerobic training effect, 0.0 to 5.0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anaerobic_training_effect: Option<f64>,
    /// Garmin's label for the main benefit, e.g. `TEMPO`, `ANAEROBIC_CAPACITY`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub training_effect_label: Option<String>,
    /// Garmin's training load score for this activity.
    #[serde(skip_serializing_if = "Option::is_none")]
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

/// How speed is usually expressed for a sport.
///
/// A plain `enum` with no data in its variants, used only inside this file.
enum SpeedStyle {
    /// Minutes per kilometer: running, walking, hiking.
    PacePerKm,
    /// Minutes per 100 meters: swimming.
    PacePer100m,
    /// Kilometers per hour: cycling and everything else.
    KmPerHour,
}

impl SpeedStyle {
    /// Picks the style from Garmin's activity `typeKey`.
    ///
    /// Matches on parts of the name so all variants are covered, e.g.
    /// `treadmill_running`, `trail_running`, `virtual_run`, `lap_swimming`,
    /// `open_water_swimming`, `casual_walking`.
    fn for_activity(type_key: &str) -> Self {
        if type_key.contains("swim") {
            Self::PacePer100m
        } else if type_key.contains("run") || type_key.contains("walk") || type_key.contains("hik")
        {
            Self::PacePerKm
        } else {
            Self::KmPerHour
        }
    }
}

impl Activity {
    /// Converts Garmin's raw activity into the tool's [`ActivitySummary`].
    ///
    /// Takes `self` by value, so strings are moved, not copied. Rounding only
    /// removes detail that is irrelevant for health evaluation; Garmin stores
    /// many values as 32-bit floats, so e.g. `2.4` arrives as
    /// `2.4000000953674316` and the extra digits are only conversion noise.
    pub fn into_summary(self) -> ActivitySummary {
        // Distance and speed only make sense if the activity covered ground.
        // Strength training reports `0.0` for both; treat that as "none".
        // `.filter(|d| *d > 0.0)` turns `Some(0.0)` into `None`; the `*`
        // dereferences the `&f64` the closure receives.
        let distance = self.distance.filter(|d| *d > 0.0);
        let speed = self.average_speed.filter(|v| *v > 0.0);
        // Indoor activities (e.g. indoor cycling) report a moving time of 0
        // because Garmin doesn't track it there. 0 would look like "never
        // moved", so leave the field out instead.
        let moving_duration = self.moving_duration.filter(|s| *s > 0.0);

        let style = SpeedStyle::for_activity(&self.activity_type);

        // Swimming keeps meters; everything else uses km with 3 decimals,
        // which is still meter precision.
        // `matches!(value, pattern)` is `true` if the value fits the pattern.
        let is_swim = matches!(style, SpeedStyle::PacePer100m);
        let (distance_km, distance_m) = if is_swim {
            (None, distance.map(|m| round(m, 0)))
        } else {
            (distance.map(|m| round(m / 1000.0, 3)), None)
        };

        // Exactly one of the three speed fields is filled, depending on the
        // sport. A `match` must handle every variant of the enum; the
        // compiler checks that none is forgotten.
        // Pace = time per distance = distance / speed (in seconds).
        let (avg_pace_per_km, avg_pace_per_100m, avg_speed_kmh) = match style {
            SpeedStyle::PacePerKm => (speed.map(|v| format_pace(1000.0 / v)), None, None),
            SpeedStyle::PacePer100m => (None, speed.map(|v| format_pace(100.0 / v)), None),
            // m/s to km/h: multiply by 3600 s/h and divide by 1000 m/km.
            SpeedStyle::KmPerHour => (None, None, speed.map(|v| round(v * 3.6, 1))),
        };

        ActivitySummary {
            activity_id: self.activity_id,
            activity_name: self.activity_name,
            activity_type: self.activity_type,
            start_time_local: self.start_time_local,
            location_name: self.location_name,
            // Seconds to minutes. 2 decimals = 0.6 s precision.
            duration_min: self.duration.map(|s| round(s / 60.0, 2)),
            moving_duration_min: moving_duration.map(|s| round(s / 60.0, 2)),
            distance_km,
            distance_m,
            avg_pace_per_km,
            avg_pace_per_100m,
            avg_speed_kmh,
            elevation_gain_m: self.elevation_gain,
            steps: self.steps,
            average_hr: self.average_hr,
            max_hr: self.max_hr,
            avg_power_w: self.avg_power,
            calories_kcal: self.calories,
            // Training effect uses 0.1 steps, so 1 decimal loses nothing.
            aerobic_training_effect: self.aerobic_training_effect.map(|v| round(v, 1)),
            anaerobic_training_effect: self.anaerobic_training_effect.map(|v| round(v, 1)),
            training_effect_label: self.training_effect_label,
            // Garmin itself only shows whole numbers for load.
            activity_training_load: self.activity_training_load.map(|v| round(v, 1)),
            difference_body_battery: self.difference_body_battery,
            moderate_intensity_minutes: self.moderate_intensity_minutes,
            vigorous_intensity_minutes: self.vigorous_intensity_minutes,
            total_sets: self.total_sets,
            total_reps: self.total_reps,
        }
    }
}

/// Rounds `value` to `decimals` decimal places.
fn round(value: f64, decimals: i32) -> f64 {
    // `10f64.powi(n)` is 10 to the power `n` (`f64` is the number type, and
    // `10f64` is the literal 10 written as an `f64`).
    let factor = 10f64.powi(decimals);
    (value * factor).round() / factor
}

/// Formats a number of seconds as "minutes:seconds", e.g. `383.4` -> "6:23".
fn format_pace(seconds: f64) -> String {
    // Round to whole seconds first, so 359.6 s becomes "6:00", not "5:60".
    // `as u64` converts the float to an unsigned integer.
    let total = seconds.round() as u64;
    // `/` on integers divides and drops the remainder; `%` is the remainder.
    // `{:02}` pads with a leading zero to 2 digits ("6:05", not "6:5").
    format!("{}:{:02}", total / 60, total % 60)
}

/// Reads Garmin's `activityType` object and returns only its `typeKey`.
///
/// Used through `#[serde(deserialize_with = "type_key")]` on [`Activity`].
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
    /// Total time asleep, in minutes.
    pub total_sleep_minutes: Option<f64>,
    /// Time in each sleep stage, in minutes.
    pub deep_sleep_minutes: Option<f64>,
    pub light_sleep_minutes: Option<f64>,
    pub rem_sleep_minutes: Option<f64>,
    pub awake_minutes: Option<f64>,
    /// Time spent napping during the day, in minutes.
    pub nap_minutes: Option<f64>,
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
            // Garmin gives seconds; `minutes` converts (see below). Passing
            // the function name `minutes` to `.map` is shorthand for
            // `.map(|s| minutes(s))`.
            total_sleep_minutes: self.sleep_time_seconds.map(minutes),
            deep_sleep_minutes: self.deep_sleep_seconds.map(minutes),
            light_sleep_minutes: self.light_sleep_seconds.map(minutes),
            rem_sleep_minutes: self.rem_sleep_seconds.map(minutes),
            awake_minutes: self.awake_sleep_seconds.map(minutes),
            nap_minutes: self.nap_time_seconds.map(minutes),
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

/// Converts whole seconds to minutes, rounded to 0.1 (6 s).
fn minutes(seconds: u32) -> f64 {
    // `f64::from` converts the integer to a float without any loss.
    round(f64::from(seconds) / 60.0, 1)
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

    /// Parses one activity from JSON and converts it to the output shape.
    fn summary(json: &str) -> ActivitySummary {
        // `serde_json::from_str` parses text into our struct. `.unwrap()`
        // panics (fails the test) if parsing returns an error.
        let activity: Activity = serde_json::from_str(json).unwrap();
        activity.into_summary()
    }

    #[test]
    fn strength_training_has_no_distance_or_speed() {
        let out = summary(STRENGTH_JSON);
        assert_eq!(out.activity_type, "strength_training");
        assert_eq!(out.duration_min, Some(70.65));
        assert_eq!(out.average_hr, Some(124.0));
        assert_eq!(out.total_reps, Some(285));
        // Distance 0 means "no distance", so all distance/speed fields are
        // empty. `avg_power` was missing from the JSON.
        assert_eq!(out.distance_km, None);
        assert_eq!(out.avg_pace_per_km, None);
        assert_eq!(out.avg_speed_kmh, None);
        assert_eq!(out.avg_power_w, None);
    }

    #[test]
    fn walking_gets_km_and_pace_per_km() {
        // Raw values exactly as Garmin sends them.
        let out = summary(
            r#"{
            "activityId": 1,
            "activityType": {"typeKey": "walking"},
            "duration": 3741.52587890625,
            "distance": 4380.18017578125,
            "averageSpeed": 1.1710000038146973,
            "aerobicTrainingEffect": 2.4000000953674316,
            "activityTrainingLoad": 62.689056396484375,
            "averageHR": 83.0
        }"#,
        );
        assert_eq!(out.duration_min, Some(62.36));
        assert_eq!(out.distance_km, Some(4.38));
        // 1000 m / 1.171 m/s = 853.97 s = 14 min 14 s.
        assert_eq!(out.avg_pace_per_km.as_deref(), Some("14:14"));
        assert_eq!(out.avg_speed_kmh, None);
        // Float noise removed.
        assert_eq!(out.aerobic_training_effect, Some(2.4));
        assert_eq!(out.activity_training_load, Some(62.7));
        // Not rounded: already clean.
        assert_eq!(out.average_hr, Some(83.0));
    }

    #[test]
    fn treadmill_running_counts_as_running() {
        // 1000 / 2.778 = 360.0 s = exactly 6:00 per km.
        let out = summary(
            r#"{"activityId": 1, "activityType": {"typeKey": "treadmill_running"},
                "distance": 5000.0, "averageSpeed": 2.778}"#,
        );
        assert_eq!(out.avg_pace_per_km.as_deref(), Some("6:00"));
    }

    #[test]
    fn swimming_gets_meters_and_pace_per_100m() {
        // 100 / 0.8 = 125 s = 2:05 per 100 m.
        let out = summary(
            r#"{"activityId": 1, "activityType": {"typeKey": "lap_swimming"},
                "distance": 1500.0, "averageSpeed": 0.8}"#,
        );
        assert_eq!(out.distance_m, Some(1500.0));
        assert_eq!(out.distance_km, None);
        assert_eq!(out.avg_pace_per_100m.as_deref(), Some("2:05"));
        assert_eq!(out.avg_pace_per_km, None);
    }

    #[test]
    fn cycling_gets_km_per_hour() {
        // 7.03 m/s * 3.6 = 25.308 km/h.
        let out = summary(
            r#"{"activityId": 1, "activityType": {"typeKey": "indoor_cycling"},
                "distance": 20000.0, "averageSpeed": 7.03, "movingDuration": 0.0}"#,
        );
        assert_eq!(out.distance_km, Some(20.0));
        // Indoor: Garmin's moving time of 0 means "not tracked".
        assert_eq!(out.moving_duration_min, None);
        assert_eq!(out.avg_speed_kmh, Some(25.3));
        assert_eq!(out.avg_pace_per_km, None);
    }

    #[test]
    fn pace_rounds_seconds_without_showing_60() {
        assert_eq!(format_pace(359.6), "6:00");
        assert_eq!(format_pace(365.0), "6:05");
    }

    #[test]
    fn output_skips_missing_fields() {
        let out = serde_json::to_value(summary(STRENGTH_JSON)).unwrap();
        assert_eq!(out["activity_type"], "strength_training");
        // `None` fields are left out, and Garmin fields we don't model
        // (like `ownerFullName`) never appear.
        assert!(out.get("avg_power_w").is_none());
        assert!(out.get("distance_km").is_none());
        assert!(out.get("ownerFullName").is_none());
    }

    /// Trimmed-down copy of a real `dailySleep` response.
    const SLEEP_JSON: &str = r#"{
        "calendarDate": "2026-10-23",
        "sleepTimeSeconds": 29040,
        "napTimeSeconds": 1819,
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
        // 29040 s = 484 min, 4500 s = 75 min, 1819 s = 30.3 min.
        assert_eq!(sleep.total_sleep_minutes, Some(484.0));
        assert_eq!(sleep.deep_sleep_minutes, Some(75.0));
        assert_eq!(sleep.nap_minutes, Some(30.3));
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
