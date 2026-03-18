#[cfg(target_family = "wasm")]
mod http {
    use crate::thread;
    use wasm_bindgen::prelude::*;

    fn trace_stage(stage: &str) {
        let payload = serde_json::to_string(stage)
            .unwrap_or_else(|_| "\"core-http-trace-encode-error\"".to_owned());
        let script = format!(
            "self.fetch('/wasm-smoke/trace?stage=' + encodeURIComponent('core-http:' + {payload}), {{ cache: 'no-store' }}).catch(() => undefined);"
        );
        let _ = js_sys::eval(&script);
    }

    fn to_io_err(err: JsValue) -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::Other, format!("{err:?}"))
    }

    async fn _get_bytes(url: String) -> std::io::Result<Vec<u8>> {
        use wasm_bindgen_futures::JsFuture;
        use web_sys::Response;

        // let window = js_sys::global().dyn_into::<web_sys::WorkerGlobalScope>().unwrap();
        let window = web_sys::window().unwrap();

        trace_stage("fetch.begin");
        let resp_value = JsFuture::from(window.fetch_with_str(&url))
            .await
            .map_err(to_io_err)?;

        assert!(resp_value.is_instance_of::<Response>());
        let resp: Response = resp_value.dyn_into().unwrap();
        let status = resp.status();
        trace_stage(&format!("fetch.response.status.{status}"));
        if !resp.ok() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("http fetch failed for {url} with status {status}"),
            ));
        }
        let js = JsFuture::from(resp.array_buffer().map_err(to_io_err)?)
            .await
            .map_err(to_io_err)?;
        trace_stage("fetch.array_buffer.complete");
        Ok(js_sys::Uint8Array::new(&js).to_vec())
    }

    pub async fn get_bytes(url: &str) -> std::io::Result<Vec<u8>> {
        let url = url.to_owned();
        if thread::is_web_worker_thread() {
            trace_stage("get_bytes.worker.dispatch");
            thread::run_async_fn_in_main_thread(move || _get_bytes(url))
                .await
                .expect("failed to run task in the main thread! Maybe main thread crashed or not initialized?")
        } else {
            trace_stage("get_bytes.main.direct");
            _get_bytes(url).await
        }
    }

    pub fn get_bytes_blocking(url: &str) -> std::io::Result<Vec<u8>> {
        let url = url.to_owned();
        if thread::is_web_worker_thread() {
            thread::run_async_fn_in_main_thread_blocking(move || _get_bytes(url)).expect("failed to run task in the main thread! Maybe main thread crashed or not initialized?")
        } else {
            panic!("can't do blocking requests from main browser thread");
        }
    }
}

#[cfg(target_family = "wasm")]
pub use http::{get_bytes, get_bytes_blocking};
