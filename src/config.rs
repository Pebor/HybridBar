use crate::{environment, math};
use heapless::Vec;
use json::JsonValue;
use std::{fs, i32, sync::RwLock};

lazy_static! {
    /// Caches the config.
    // "Why not Mutex" : https://onesignal.com/blog/thread-safety-rust/#:~:text=They%20have%20one%20important%20difference,exclusive%20access%20for%20write%20locks.
    // TODO: Make read_config_cached() - if possible.
    pub static ref CONFIG: RwLock<JsonValue> = RwLock::new(JsonValue::Null);
}

/// Gets the root home path to Hybrid.
pub fn get_path() -> String {
    format!(
        "/home/{}/.config/HybridBar/",
        execute!(&heapless::String::<6>::from("whoami"))
    )
}

/// Returns the set update-rate.
pub fn get_update_rate() -> u64 {
    let mut update_rate = 100;
    if let Some(c_update_rate) = try_get("hybrid", "update_rate", false, false) {
        update_rate = math::clamp_i32(c_update_rate.1, 5, 10_000)
    }

    update_rate
        .try_into()
        .expect("[ERROR] Cannot convert update_rate into u64!\n")
}

/// Caches the config so we don't have to re-parse it every time.
/// Works as a fix for issue #13
pub fn cache() {
    *CONFIG.write().unwrap() = read_config_raw();
}

/// Parses and returns the config.
fn read_config_raw() -> JsonValue {
    let mut conf_path = get_path();
    conf_path.push_str(&environment::try_get_var("HYBRID_CONFIG", "config.json"));
    json::parse(
        &fs::read_to_string(&conf_path)
            .unwrap_or_else(|_| panic!("[ERROR] Failed reading config file from '{conf_path}'!\n")),
    )
    .unwrap_or_else(|_| panic!("[ERROR] Failed parsing config from '{conf_path}'!\n"))
}

/// Tries to fetch a value from the config. Supported types are `String` and `i32`.
/// Panics if `is_string` is `true` and the `as_i32` function fails.
/// If the specified root/key wasn't found, a `None` value is returned.
pub fn try_get(
    root: &str,
    key: &str,
    is_string: bool,
    with_custom_variables: bool,
) -> Option<(String, i32)> {
    let config = &CONFIG.read().unwrap()[root];
    let default_string = String::default();
    if config.has_key(key) {
        let grabbed_value = &config[key];

        // If the desired value isn't a string, try and get it as a 32-bit integer.
        if !is_string {
            return Some((
                default_string,
                grabbed_value
                    .as_i32()
                    .unwrap_or_else(|| panic!("[ERROR] Failed parsing {root}:{key} as i32!\n")),
            ));
        }

        // Convert it to a string-value.
        if with_custom_variables {
            Some((with_variables(grabbed_value.to_string()), 0))
        } else {
            Some((grabbed_value.to_string(), 0))
        }
    } else {
        // The key wasn't found, so just return None.
        None
    }
}

/// Same as `try_get`, but if the value is `None` then the return-value becomes `"", 0` (default).
/// NOTE: This function should NOT act as a replacement for `try_get`, but rather as one where you
/// can actually use default values and want to avoid explicitly `.unwrap_or_else()` or other long
/// code.
pub fn get_or_default(
    root: &str,
    key: &str,
    is_string: bool,
    with_custom_variables: bool,
) -> (String, i32) {
    try_get(root, key, is_string, with_custom_variables).unwrap_or_else(|| (String::default(), 0))
}

/// Gets all the custom variables.
fn get_custom_variables() -> Vec<(String, String), 64> {
    let cfg = &CONFIG.read().unwrap()["variables"];
    // 0.3.0: Only allow for 64 variables.
    let mut vector: Vec<(String, String), 64> = Vec::new();
    for entry in cfg.entries() {
        vector
            .push((entry.0.to_string(), entry.1.to_string()))
            .expect("[ERROR] You cannot have more than `64` variables!\n");
    }

    vector
}

/// Replaces any variable-matching patterns in the `String` with the variables value.
fn with_variables(input: String) -> String {
    let mut result = input;
    for variable in get_custom_variables() {
        // Only replace if `result` actually contains the defined variable.
        if result.contains(&variable.0) {
            result = result.replace(&variable.0, &variable.1);
        }
    }

    result
}
