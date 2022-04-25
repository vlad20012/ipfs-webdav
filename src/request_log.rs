use hyper::{Method, Request, Response, Uri};
use std::time::{Duration, SystemTime};
use webdav_handler::body::Body;

pub struct RequestLog {
    method: Method,
    uri: Uri,
    time_start: SystemTime,
}

impl RequestLog {
    pub fn on_request<T>(req: &Request<T>) -> RequestLog {
        let method = req.method().clone();
        let uri = req.uri().clone();
        let time_start = SystemTime::now();
        log::debug!("Request: {} {}", method, uri);

        RequestLog {
            method,
            uri,
            time_start,
        }
    }

    pub fn on_response(self, resp: &Response<Body>) {
        let elapsed = match self.time_start.elapsed() {
            Ok(elapsed) => elapsed,
            Err(_) => {
                // The system time is likely jumped backward, ignoring it
                Duration::from_secs(0)
            }
        };
        log::debug!(
            "   Resp: {} {} = {} ({:?})",
            self.method,
            self.uri,
            resp.status(),
            elapsed
        );
    }
}
