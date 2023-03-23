use self::countries::COUNTRIES;

pub mod countries;

pub fn country_iso_to_name(input: &str) -> Option<String> {
    return COUNTRIES.get(input).map(|v| v.to_string());
}

pub fn normalize_uri(input: &str) -> String {
    input.to_lowercase().trim().to_string()
}
