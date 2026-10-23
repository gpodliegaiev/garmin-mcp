//! Rust types for Garmin API responses.
//!
//! Garmin returns ~70 fields per activity. These structs keep only the ones
//! that are useful to an AI assistant, which makes tool results much smaller.
//! serde ignores any JSON field that has no matching struct field.

use serde::{Deserialize, Deserializer, Serialize};

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
    /// Time spent actually moving, in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moving_duration: Option<f64>,
    /// Distance in meters (0 for activities without distance).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance: Option<f64>,
    /// Average speed in meters per second.
    #[serde(skip_serializing_if = "Option::is_none")]
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
}
