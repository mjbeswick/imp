//! libcurl through FFI. One POST, streamed into a callback.

use std::ffi::{c_char, c_long, c_void, CStr, CString};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;

const CURLOPT_URL: u32 = 10_002;
const CURLOPT_WRITEFUNCTION: u32 = 20_011;
const CURLOPT_WRITEDATA: u32 = 10_001;
const CURLOPT_HTTPHEADER: u32 = 10_023;
const CURLOPT_POST: u32 = 47;
const CURLOPT_POSTFIELDS: u32 = 10_015;
const CURLOPT_POSTFIELDSIZE: u32 = 60;
const CURLOPT_TIMEOUT_MS: u32 = 155;
const CURLOPT_CONNECTTIMEOUT_MS: u32 = 156;
const CURLOPT_NOSIGNAL: u32 = 99;
const CURLOPT_ERRORBUFFER: u32 = 10_010;
const CURLOPT_USERAGENT: u32 = 10_018;
const CURLOPT_ACCEPT_ENCODING: u32 = 10_102;
const CURLINFO_RESPONSE_CODE: u32 = 0x20_0002;
const CURLINFO_CONTENT_TYPE: u32 = 0x10_0012;
const CURLOPT_FOLLOWLOCATION: u32 = 52;
const CURLOPT_MAXREDIRS: u32 = 68;
const CURLOPT_PROTOCOLS_STR: u32 = 10_318;
const CURLOPT_REDIR_PROTOCOLS_STR: u32 = 10_319;

const CURLOPT_NOPROGRESS: u32 = 43;
const CURLOPT_XFERINFOFUNCTION: u32 = 20_219;
const CURLOPT_XFERINFODATA: u32 = 10_057;
const CURLOPT_LOW_SPEED_LIMIT: u32 = 19;
const CURLOPT_LOW_SPEED_TIME: u32 = 20;

const CURLE_OK: i32 = 0;
const CURLE_ABORTED_BY_CALLBACK: i32 = 42;

#[repr(C)]
struct CurlSlist {
    data: *mut c_char,
    next: *mut CurlSlist,
}

#[link(name = "curl")]
extern "C" {
    fn curl_global_init(flags: c_long) -> i32;
    fn curl_easy_init() -> *mut c_void;
    fn curl_easy_cleanup(handle: *mut c_void);
    fn curl_easy_setopt(handle: *mut c_void, option: u32, ...) -> i32;
    fn curl_easy_perform(handle: *mut c_void) -> i32;
    fn curl_easy_getinfo(handle: *mut c_void, info: u32, ...) -> i32;
    fn curl_easy_strerror(code: i32) -> *const c_char;
    fn curl_slist_append(list: *mut CurlSlist, value: *const c_char) -> *mut CurlSlist;
    fn curl_slist_free_all(list: *mut CurlSlist);
}

static INIT: Once = Once::new();

fn init() {
    INIT.call_once(|| unsafe {
        curl_global_init(3); // CURL_GLOBAL_DEFAULT
    });
}

/// A successful response: status plus whatever the callback consumed.
pub struct Response {
    pub status: u16,
    pub content_type: String,
}

/// Headers are ordinary `Name: value` strings; `Content-Type` is added for you.
/// Setting `cancel` aborts the request within about a second.
pub fn post_stream<F>(
    url: &str,
    headers: &[&str],
    body: &[u8],
    cancel: &AtomicBool,
    on_chunk: F,
) -> Result<Response, String>
where
    F: FnMut(&[u8]),
{
    request_stream(url, headers, Some(body), 600_000, false, Some(cancel), on_chunk)
}

/// One GET, collecting the body. Used for small JSON documents such as a
/// provider's model list.
pub fn get(url: &str, headers: &[&str], timeout_ms: u64) -> Result<(u16, String), String> {
    let mut body = Vec::new();
    let response = request_stream(url, headers, None, timeout_ms, false, None, |chunk| {
        body.extend_from_slice(chunk);
    })?;
    Ok((response.status, String::from_utf8_lossy(&body).into_owned()))
}

/// One GET for the `fetch` tool: redirects followed over http and https,
/// the body kept up to `cap` bytes. Returns status, content type, and body.
pub fn fetch(url: &str, timeout_ms: u64, cap: usize) -> Result<(u16, String, String), String> {
    let mut body = Vec::new();
    let response = request_stream(url, &[], None, timeout_ms, true, None, |chunk| {
        if body.len() < cap {
            body.extend_from_slice(chunk);
        }
    })?;
    Ok((
        response.status,
        response.content_type,
        String::from_utf8_lossy(&body).into_owned(),
    ))
}

fn request_stream<F>(
    url: &str,
    headers: &[&str],
    body: Option<&[u8]>,
    timeout_ms: u64,
    follow: bool,
    cancel: Option<&AtomicBool>,
    mut on_chunk: F,
) -> Result<Response, String>
where
    F: FnMut(&[u8]),
{
    init();

    unsafe {
        let handle = curl_easy_init();
        if handle.is_null() {
            return Err("curl_easy_init failed".into());
        }

        let url_c = CString::new(url).map_err(|_| "url contains a nul byte")?;
        let agent = CString::new("imp/0.1").unwrap();
        let mut errbuf = [0 as c_char; 256];
        let mut slist: *mut CurlSlist = std::ptr::null_mut();

        for h in headers {
            let c = CString::new(*h).map_err(|_| "header contains a nul byte")?;
            slist = curl_slist_append(slist, c.as_ptr());
            // `c` is only needed for the duration of the call; curl copies.
        }

        let mut chunk_fn = &mut on_chunk;
        let userdata = &mut chunk_fn as *mut &mut F as *mut c_void;

        let rc = curl_easy_setopt(handle, CURLOPT_URL, url_c.as_ptr());
        if rc != CURLE_OK {
            curl_easy_cleanup(handle);
            curl_slist_free_all(slist);
            return Err(format!("setopt URL failed: {rc}"));
        }
        setopt_ptr(handle, CURLOPT_WRITEFUNCTION, write_cb::<F> as *const c_void);
        setopt_ptr(handle, CURLOPT_WRITEDATA, userdata);
        setopt_ptr(handle, CURLOPT_HTTPHEADER, slist as *const c_void);
        if let Some(body) = body {
            setopt_long(handle, CURLOPT_POST, 1);
            setopt_ptr(handle, CURLOPT_POSTFIELDS, body.as_ptr() as *const c_void);
            setopt_long(handle, CURLOPT_POSTFIELDSIZE, body.len() as c_long);
        }
        setopt_long(handle, CURLOPT_TIMEOUT_MS, timeout_ms as c_long);
        setopt_long(handle, CURLOPT_CONNECTTIMEOUT_MS, 15_000);
        setopt_long(handle, CURLOPT_NOSIGNAL, 1);
        // A stalled stream ends: under one byte a second for a minute.
        setopt_long(handle, CURLOPT_LOW_SPEED_LIMIT, 1);
        setopt_long(handle, CURLOPT_LOW_SPEED_TIME, 60);
        if let Some(flag) = cancel {
            // curl calls this about once a second, bytes or not.
            setopt_ptr(handle, CURLOPT_XFERINFOFUNCTION, progress_cb as *const c_void);
            setopt_ptr(handle, CURLOPT_XFERINFODATA, flag as *const AtomicBool as *const c_void);
            setopt_long(handle, CURLOPT_NOPROGRESS, 0);
        }
        setopt_str(handle, CURLOPT_USERAGENT, agent.as_ptr());
        setopt_ptr(handle, CURLOPT_ACCEPT_ENCODING, std::ptr::null());
        setopt_ptr(handle, CURLOPT_ERRORBUFFER, errbuf.as_mut_ptr() as *const c_void);
        if follow {
            // A fetch may follow redirects, but only ever over http or https.
            let web = CString::new("http,https").unwrap();
            setopt_str(handle, CURLOPT_PROTOCOLS_STR, web.as_ptr());
            setopt_str(handle, CURLOPT_REDIR_PROTOCOLS_STR, web.as_ptr());
            setopt_long(handle, CURLOPT_FOLLOWLOCATION, 1);
            setopt_long(handle, CURLOPT_MAXREDIRS, 5);
        }

        let code = curl_easy_perform(handle);
        let mut status: c_long = 0;
        curl_easy_getinfo(handle, CURLINFO_RESPONSE_CODE, &mut status as *mut c_long);
        let mut kind: *const c_char = std::ptr::null();
        curl_easy_getinfo(handle, CURLINFO_CONTENT_TYPE, &mut kind as *mut *const c_char);
        let content_type = if kind.is_null() {
            String::new()
        } else {
            CStr::from_ptr(kind).to_string_lossy().into_owned()
        };

        curl_slist_free_all(slist);
        curl_easy_cleanup(handle);

        if code == CURLE_ABORTED_BY_CALLBACK {
            return Err("cancelled".into());
        }
        if code != CURLE_OK {
            let detail = read_errbuf(&errbuf);
            let base = CStr::from_ptr(curl_easy_strerror(code))
                .to_string_lossy()
                .into_owned();
            return Err(if detail.is_empty() {
                base
            } else {
                format!("{base}: {detail}")
            });
        }

        Ok(Response {
            status: status as u16,
            content_type,
        })
    }
}

unsafe fn setopt_long(handle: *mut c_void, option: u32, value: c_long) -> i32 {
    curl_easy_setopt(handle, option, value)
}

unsafe fn setopt_ptr(handle: *mut c_void, option: u32, value: *const c_void) -> i32 {
    curl_easy_setopt(handle, option, value)
}

unsafe fn setopt_str(handle: *mut c_void, option: u32, value: *const c_char) -> i32 {
    curl_easy_setopt(handle, option, value)
}

fn read_errbuf(buf: &[c_char; 256]) -> String {
    let bytes: Vec<u8> = buf
        .iter()
        .take_while(|c| **c != 0)
        .map(|c| *c as u8)
        .collect();
    String::from_utf8_lossy(&bytes).trim().to_string()
}

/// Non-zero aborts the transfer.
extern "C" fn progress_cb(flag: *mut c_void, _: i64, _: i64, _: i64, _: i64) -> i32 {
    let flag = unsafe { &*(flag as *const AtomicBool) };
    i32::from(flag.load(Ordering::Relaxed))
}

extern "C" fn write_cb<F>(data: *mut c_char, size: usize, nmemb: usize, userdata: *mut c_void) -> usize
where
    F: FnMut(&[u8]),
{
    let len = size.saturating_mul(nmemb);
    if data.is_null() || userdata.is_null() {
        return 0;
    }
    unsafe {
        let slot = &mut *(userdata as *mut &mut F);
        let bytes = std::slice::from_raw_parts(data as *const u8, len);
        slot(bytes);
    }
    len
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_aborts_a_stalled_request() {
        let server = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/", server.local_addr().unwrap());
        std::thread::spawn(move || {
            let (mut conn, _) = server.accept().unwrap();
            use std::io::Write;
            let _ = conn.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\ndata: hi\n\n");
            std::thread::sleep(std::time::Duration::from_secs(30));
        });
        let cancel = std::sync::Arc::new(AtomicBool::new(false));
        let flag = cancel.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(300));
            flag.store(true, Ordering::Relaxed);
        });
        let start = std::time::Instant::now();
        let mut got = Vec::new();
        let r = post_stream(&url, &[], b"{}", &cancel, |b| got.extend_from_slice(b));
        assert_eq!(r.err().as_deref(), Some("cancelled"));
        assert!(start.elapsed() < std::time::Duration::from_secs(2), "{:?}", start.elapsed());
        assert!(String::from_utf8_lossy(&got).contains("hi"));
    }

    #[test]
    fn error_buffer_reads_until_nul() {
        let mut buf = [0 as c_char; 256];
        for (i, b) in b"dns failure".iter().enumerate() {
            buf[i] = *b as c_char;
        }
        assert_eq!(read_errbuf(&buf), "dns failure");
    }

    #[test]
    fn error_buffer_can_be_empty() {
        let buf = [0 as c_char; 256];
        assert_eq!(read_errbuf(&buf), "");
    }
}
