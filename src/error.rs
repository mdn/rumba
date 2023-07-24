use serde::Serialize;

#[derive(Serialize)]
pub struct ErrorResponse<'a> {
    pub code: u16,
    pub error: &'a str,
    pub message: &'a str,
}
