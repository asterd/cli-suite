use std::fmt;
use std::path::Path;

pub struct Request {
    pub id: u64,
}

pub struct Response {
    pub body: String,
}

impl Response {
    pub fn ok(id: u64) -> Self {
        Self {
            body: format!("ok:{id}"),
        }
    }
}

/// Process one request.
#[inline]
pub fn process_request(req: Request) -> Response {
    Response::ok(req.id)
}

pub struct Parser;

impl Parser {
    /// Parse with the parser type.
    #[must_use]
    pub fn parse(input: &str) -> usize {
        input.len()
    }
}

pub struct OtherParser;

impl OtherParser {
    pub fn parse(input: &str) -> usize {
        input.trim().len()
    }
}

fn helper() -> String {
    format!("{}", Path::new(".").display())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_request_returns_body() {
        let response = process_request(Request { id: 7 });
        assert_eq!(response.body, "ok:7");
    }
}

impl fmt::Display for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}
