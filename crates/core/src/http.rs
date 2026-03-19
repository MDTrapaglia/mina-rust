fn http_fetch_status_error(url: &str, status: u16) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("http fetch failed for {url} with status {status}"),
    )
}

#[cfg(target_family = "wasm")]
mod http {
    use crate::thread;
    use wasm_bindgen::prelude::*;

    fn to_io_err(err: JsValue) -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::Other, format!("{err:?}"))
    }

    async fn _get_bytes(url: String) -> std::io::Result<Vec<u8>> {
        use wasm_bindgen_futures::JsFuture;
        use web_sys::Response;

        // let window = js_sys::global().dyn_into::<web_sys::WorkerGlobalScope>().unwrap();
        let window = web_sys::window().unwrap();

        let resp_value = JsFuture::from(window.fetch_with_str(&url))
            .await
            .map_err(to_io_err)?;

        assert!(resp_value.is_instance_of::<Response>());
        let resp: Response = resp_value.dyn_into().unwrap();
        let status = resp.status();
        if !resp.ok() {
            return Err(super::http_fetch_status_error(&url, status));
        }
        let js = JsFuture::from(resp.array_buffer().map_err(to_io_err)?)
            .await
            .map_err(to_io_err)?;
        Ok(js_sys::Uint8Array::new(&js).to_vec())
    }

    pub async fn get_bytes(url: &str) -> std::io::Result<Vec<u8>> {
        let url = url.to_owned();
        if thread::is_web_worker_thread() {
            thread::run_async_fn_in_main_thread(move || _get_bytes(url)).await.expect("failed to run task in the main thread! Maybe main thread crashed or not initialized?")
        } else {
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

#[cfg(test)]
mod tests {
    use super::http_fetch_status_error;

    #[test]
    fn non_ok_fetch_uses_not_found_error_kind() {
        let err = http_fetch_status_error("https://example.invalid/missing", 404);

        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn non_ok_fetch_error_mentions_url_and_status() {
        let err = http_fetch_status_error("https://example.invalid/missing", 503);

        let message = err.to_string();
        assert!(message.contains("https://example.invalid/missing"));
        assert!(message.contains("503"));
    }
}
