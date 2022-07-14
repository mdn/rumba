use std::collections::BTreeMap;

use actix_http::HttpMessage;
use actix_web::{
    dev::{Payload, RequestHead},
    http::header::USER_AGENT,
    Error, FromRequest, HttpRequest,
};
use futures_util::future::{self, Ready};
use serde::{
    ser::{SerializeMap, Serializer},
    Serialize,
};
use serde_json::value::Value;
use slog::{Key, Record, KV};
use woothee::parser::{Parser, WootheeResult};

// List of valid user-agent attributes to keep, anything not in this
// list is considered 'Other'. We log the user-agent on connect always
// to retain the full string, but for DD more tags are expensive so we
// limit to these.
const VALID_UA_BROWSER: &[&str] = &["Chrome", "Firefox", "Safari", "Opera"];

// See dataset.rs in https://github.com/woothee/woothee-rust for the
// full list (WootheeResult's 'os' field may fall back to its 'name'
// field). Windows has many values and we only care that its Windows
const VALID_UA_OS: &[&str] = &["Firefox OS", "Linux", "Mac OSX"];

pub fn parse_user_agent(agent: &str) -> (WootheeResult<'_>, &str, &str) {
    let parser = Parser::new();
    let wresult = parser.parse(agent).unwrap_or_else(|| WootheeResult {
        name: "",
        category: "",
        os: "",
        os_version: "".into(),
        browser_type: "",
        version: "",
        vendor: "",
    });

    // Determine a base os/browser for metrics' tags
    let metrics_os = if wresult.os.starts_with("Windows") {
        "Windows"
    } else if VALID_UA_OS.contains(&wresult.os) {
        wresult.os
    } else {
        "Other"
    };
    let metrics_browser = if VALID_UA_BROWSER.contains(&wresult.name) {
        wresult.name
    } else {
        "Other"
    };
    (wresult, metrics_os, metrics_browser)
}

#[derive(Clone, Debug, Default)]
pub struct Tags {
    pub tags: BTreeMap<String, String>,
    pub extra: BTreeMap<String, String>,
}

impl Serialize for Tags {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_map(Some(self.tags.len()))?;
        for tag in self.tags.clone() {
            if !tag.1.is_empty() {
                seq.serialize_entry(&tag.0, &tag.1)?;
            }
        }
        seq.end()
    }
}

fn insert_if_not_empty(label: &str, val: &str, tags: &mut BTreeMap<String, String>) {
    if !val.is_empty() {
        tags.insert(label.to_owned(), val.to_owned());
    }
}

// Tags are extra data to be recorded in metric and logging calls.
// If additional tags are required or desired, you will need to add them to the
// mutable extensions, e.g.
// ```
//      let mut tags = request.extensions_mut().get::<Tags>();
//      tags.insert("SomeLabel".to_owned(), "whatever".to_owned());
// ```
// how you get the request (or the response, and it's set of `extensions`) to whatever
// function requires it, is left as an exercise for the reader.
impl Tags {
    pub fn from_request_head(req_head: &RequestHead) -> Tags {
        // Return an Option<> type because the later consumers (HandlerErrors) presume that
        // tags are optional and wrapped by an Option<> type.
        let mut tags = BTreeMap::new();
        let mut extra = BTreeMap::new();
        if let Some(ua) = req_head.headers().get(USER_AGENT) {
            if let Ok(uas) = ua.to_str() {
                // if you wanted to parse out the user agent using some out-of-scope user agent parser like woothee
                let (ua_result, metrics_os, metrics_browser) = parse_user_agent(uas);
                insert_if_not_empty("ua.os.family", metrics_os, &mut tags);
                insert_if_not_empty("ua.browser.family", metrics_browser, &mut tags);
                insert_if_not_empty("ua.name", ua_result.name, &mut tags);
                insert_if_not_empty("ua.os.ver", &ua_result.os_version.to_owned(), &mut tags);
                insert_if_not_empty("ua.browser.ver", ua_result.version, &mut tags);
                extra.insert("ua".to_owned(), uas.to_string());
            }
        }
        tags.insert("uri.method".to_owned(), req_head.method.to_string());
        // `uri.path` causes too much cardinality for influx but keep it in
        // extra for sentry
        extra.insert("uri.path".to_owned(), req_head.uri.to_string());
        Tags { tags, extra }
    }

    pub fn with_tags(tags: BTreeMap<String, String>) -> Tags {
        if tags.is_empty() {
            return Tags::default();
        }
        Tags {
            tags,
            extra: BTreeMap::new(),
        }
    }

    pub fn get(&self, label: &str) -> String {
        let none = "None".to_owned();
        self.tags.get(label).map(String::from).unwrap_or(none)
    }

    pub fn extend(&mut self, tags: BTreeMap<String, String>) {
        self.tags.extend(tags);
    }

    pub fn tag_tree(self) -> BTreeMap<String, String> {
        let mut result = BTreeMap::new();

        for (k, v) in self.tags {
            result.insert(k.clone(), v.clone());
        }
        result
    }

    pub fn extra_tree(self) -> BTreeMap<String, Value> {
        let mut result = BTreeMap::new();

        for (k, v) in self.extra {
            result.insert(k.clone(), Value::from(v));
        }
        result
    }
}

impl FromRequest for Tags {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let tags = {
            let exts = req.extensions();
            match exts.get::<Tags>() {
                Some(t) => t.clone(),
                None => Tags::from_request_head(req.head()),
            }
        };

        future::ok(tags)
    }
}

impl From<Tags> for BTreeMap<String, String> {
    fn from(val: Tags) -> Self {
        let mut result = BTreeMap::new();

        for (k, v) in val.tags {
            result.insert(k.clone(), v.clone());
        }

        result
    }
}

impl KV for Tags {
    fn serialize(&self, _rec: &Record<'_>, serializer: &mut dyn slog::Serializer) -> slog::Result {
        for (key, val) in &self.tags {
            serializer.emit_str(Key::from(key.clone()), val)?;
        }
        Ok(())
    }
}
