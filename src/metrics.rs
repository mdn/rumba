use std::{
    future::{ready, Ready},
    net::UdpSocket,
    sync::Arc,
    time::Instant,
};

use actix_http::HttpMessage;
use actix_web::{error::ErrorInternalServerError, web::Data, Error, FromRequest, HttpRequest};
use anyhow::anyhow;
use cadence::{
    BufferedUdpMetricSink, Counted, CountedExt, Metric, NopMetricSink, QueuingMetricSink,
    StatsdClient, Timed,
};

use crate::{settings::SETTINGS, tags::Tags};

#[derive(Clone, Debug)]
pub struct MetricsData {
    /// Metric reporting
    pub client: Arc<StatsdClient>,
}

#[derive(Debug, Clone)]
pub struct MetricTimer {
    pub label: String,
    pub start: Instant,
    pub tags: Tags,
}

#[derive(Debug, Clone)]
pub struct Metrics {
    client: Option<Arc<StatsdClient>>,
    tags: Option<Tags>,
    timer: Option<MetricTimer>,
}

impl Drop for Metrics {
    fn drop(&mut self) {
        let tags = self.tags.clone().unwrap_or_default();
        if let Some(client) = self.client.as_ref() {
            if let Some(timer) = self.timer.as_ref() {
                let lapse = (Instant::now() - timer.start).as_millis() as u64;
                trace!("⌚ Ending timer at nanos: {:?} : {:?}", &timer.label, lapse; &tags);
                let mut tagged = client.time_with_tags(&timer.label, lapse);
                // Include any "hard coded" tags.
                // tagged = tagged.with_tag("version", env!("CARGO_PKG_VERSION"));
                let tags = timer.tags.tags.clone();
                let keys = tags.keys();
                for tag in keys {
                    tagged = tagged.with_tag(tag, tags.get(tag).unwrap())
                }
                match tagged.try_send() {
                    Err(e) => {
                        // eat the metric, but log the error
                        warn!("⚠️ Metric {} error: {:?} ", &timer.label, e);
                    }
                    Ok(v) => {
                        trace!("⌚ {:?}", v.as_metric_str());
                    }
                }
            }
        }
    }
}

impl From<&HttpRequest> for Metrics {
    fn from(req: &HttpRequest) -> Self {
        let exts = req.extensions();
        let def_tags = Tags::from_request_head(req.head());
        let tags = exts.get::<Tags>().unwrap_or(&def_tags);
        Metrics {
            client: match req.app_data::<Data<MetricsData>>() {
                Some(v) => Some(Arc::clone(&v.client)),
                None => {
                    warn!("⚠️ metric error: No App State");
                    None
                }
            },
            tags: Some(tags.clone()),
            timer: None,
        }
    }
}

impl FromRequest for Metrics {
    type Error = Error;
    type Future = Ready<Result<Metrics, Error>>;

    fn from_request(req: &HttpRequest, _: &mut actix_http::Payload) -> Self::Future {
        ready(Ok(req.into()))
    }
}

impl From<Arc<StatsdClient>> for Metrics {
    fn from(client: Arc<cadence::StatsdClient>) -> Self {
        Metrics {
            client: Some(Arc::clone(&client)),
            tags: None,
            timer: None,
        }
    }
}

impl From<&MetricsData> for Metrics {
    fn from(state: &MetricsData) -> Self {
        Metrics {
            client: Some(Arc::clone(&state.client)),
            tags: None,
            timer: None,
        }
    }
}

impl Metrics {
    pub fn sink() -> Arc<StatsdClient> {
        Arc::new(StatsdClient::builder("", NopMetricSink).build())
    }

    pub fn noop() -> Self {
        Self {
            client: Some(Self::sink()),
            timer: None,
            tags: None,
        }
    }

    pub fn start_timer(&mut self, label: &str, tags: Option<Tags>) {
        let mut mtags = self.tags.clone().unwrap_or_default();
        if let Some(t) = tags {
            mtags.extend(t.tags)
        }

        trace!("⌚ Starting timer... {:?}", &label; &mtags);
        self.timer = Some(MetricTimer {
            label: label.to_owned(),
            start: Instant::now(),
            tags: mtags,
        });
    }

    // increment a counter with no tags data.
    pub fn incr(&self, label: &str) {
        self.incr_with_tags(label, None)
    }

    pub fn incr_with_tags(&self, label: &str, tags: Option<Tags>) {
        if let Some(client) = self.client.as_ref() {
            let mut tagged = client.incr_with_tags(label);
            let mut mtags = self.tags.clone().unwrap_or_default();
            if let Some(tags) = tags {
                mtags.extend(tags.tags);
            }
            for key in mtags.tags.keys().clone() {
                if let Some(val) = mtags.tags.get(key) {
                    tagged = tagged.with_tag(key, val.as_ref());
                }
            }
            // Include any "hard coded" tags.
            // incr = incr.with_tag("version", env!("CARGO_PKG_VERSION"));
            match tagged.try_send() {
                Err(e) => {
                    // eat the metric, but log the error
                    warn!("⚠️ Metric {} error: {:?} ", label, e; mtags);
                }
                Ok(v) => trace!("☑️ {:?}", v.as_metric_str()),
            }
        }
    }

    pub fn count(&self, label: &str, count: i64) {
        self.count_with_tags(label, count, None)
    }

    pub fn count_with_tags(&self, label: &str, count: i64, tags: Option<Tags>) {
        if let Some(client) = self.client.as_ref() {
            let mut tagged = client.count_with_tags(label, count);
            let mut mtags = self.tags.clone().unwrap_or_default();
            if let Some(tags) = tags {
                mtags.extend(tags.tags);
            }
            for key in mtags.tags.keys().clone() {
                if let Some(val) = mtags.tags.get(key) {
                    tagged = tagged.with_tag(key, val.as_ref());
                }
            }
            // Include any "hard coded" tags.
            // incr = incr.with_tag("version", env!("CARGO_PKG_VERSION"));
            match tagged.try_send() {
                Err(e) => {
                    // eat the metric, but log the error
                    warn!("⚠️ Metric {} error: {:?} ", label, e; mtags);
                }
                Ok(v) => trace!("☑️ {:?}", v.as_metric_str()),
            }
        }
    }
}

pub fn metrics_from_req(req: &HttpRequest) -> Result<Arc<StatsdClient>, Error> {
    Ok(req
        .app_data::<Data<MetricsData>>()
        .ok_or_else(|| ErrorInternalServerError("Could not get state"))
        .expect("Could not get state in metrics_from_req")
        .client
        .clone())
}

/// Create a cadence StatsdClient from the given options
pub fn metrics_from_opts() -> Result<StatsdClient, anyhow::Error> {
    let builder = if let Some(statsd_host) = SETTINGS.metrics.statsd_host.as_ref() {
        let socket = UdpSocket::bind("0.0.0.0:0")
            .map_err(|e| anyhow!(format!("Could not bind UDP port {:?}", e)))?;
        socket
            .set_nonblocking(true)
            .map_err(|e| anyhow!(format!("Could not init UDP port {:?}", e)))?;

        let host = (statsd_host.as_str(), SETTINGS.metrics.statsd_port);
        let udp_sink = BufferedUdpMetricSink::from(host, socket)
            .map_err(|e| anyhow!(format!("Could not generate UDP sink {:?}", e)))?;
        let sink = QueuingMetricSink::from(udp_sink);
        StatsdClient::builder(SETTINGS.metrics.statsd_label.as_ref(), sink)
    } else {
        StatsdClient::builder(SETTINGS.metrics.statsd_label.as_ref(), NopMetricSink)
    };
    Ok(builder
        .with_error_handler(|err| {
            warn!("⚠️ Metric send error:  {:?}", err);
        })
        .build())
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_tags() {
        use actix_web::dev::RequestHead;
        use actix_web::http::{header, uri::Uri};

        use std::collections::BTreeMap;

        let mut rh = RequestHead::default();
        let path = "/1.5/42/storage/meta/global";
        rh.uri = Uri::from_static(path);
        rh.headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:72.0) Gecko/20100101 Firefox/72.0",
            ),
        );

        let tags = Tags::from_request_head(&rh);

        let mut result = BTreeMap::<String, String>::new();
        result.insert("ua.os.ver".to_owned(), "NT 10.0".to_owned());
        result.insert("ua.os.family".to_owned(), "Windows".to_owned());
        result.insert("ua.browser.ver".to_owned(), "72.0".to_owned());
        result.insert("ua.name".to_owned(), "Firefox".to_owned());
        result.insert("ua.browser.family".to_owned(), "Firefox".to_owned());
        result.insert("uri.method".to_owned(), "GET".to_owned());

        assert_eq!(tags.tags, result)
    }

    #[test]
    fn no_empty_tags() {
        use actix_web::dev::RequestHead;
        use actix_web::http::{header, uri::Uri};

        let mut rh = RequestHead::default();
        let path = "/1.5/42/storage/meta/global";
        rh.uri = Uri::from_static(path);
        rh.headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static("Mozilla/5.0 (curl) Gecko/20100101 curl"),
        );

        let tags = Tags::from_request_head(&rh);
        assert!(!tags.tags.contains_key("ua.os.ver"));
    }
}
