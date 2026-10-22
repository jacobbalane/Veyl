//! Windows owns multicast networking; this module never opens a listening socket.
use super::{DiscoveryEvent, Service};
use std::{
    collections::BTreeMap,
    ffi::c_void,
    io,
    net::{Ipv4Addr, SocketAddrV4},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex, OnceLock,
    },
};
use tokio::sync::mpsc;
use windows_sys::Win32::{
    Foundation::{DNS_REQUEST_PENDING, ERROR_CANCELLED},
    NetworkManagement::Dns::*,
};

// Only the start/cancel caller accesses this handle, under its mutex. DNS owns its opaque value.
struct Cancel(DNS_SERVICE_CANCEL);
unsafe impl Send for Cancel {}
struct Context {
    name: Vec<u16>,
    tx: mpsc::Sender<DiscoveryEvent>,
    cancel: Mutex<Cancel>,
    resolve: bool,
    done: AtomicBool,
}
// Tokens are opaque to Windows. Looking up an Arc also protects overlapping callbacks during
// cancellation: no callback ever dereferences a context that another callback might release.
static CONTEXTS: OnceLock<Mutex<BTreeMap<usize, Arc<Context>>>> = OnceLock::new();
static NEXT_TOKEN: AtomicUsize = AtomicUsize::new(1);
fn contexts() -> &'static Mutex<BTreeMap<usize, Arc<Context>>> {
    CONTEXTS.get_or_init(Mutex::default)
}
fn callback_context(raw: *const c_void, terminal: bool) -> Option<Arc<Context>> {
    let mut registry = contexts().lock().unwrap_or_else(|error| error.into_inner());
    if terminal {
        registry.remove(&(raw as usize))
    } else {
        registry.get(&(raw as usize)).cloned()
    }
}
pub struct NativeRequest(Arc<Context>);
impl NativeRequest {
    pub fn browse(tx: mpsc::Sender<DiscoveryEvent>) -> io::Result<Self> {
        Self::start("_veyl._tcp.local", false, tx)
    }
    pub fn resolve(instance: &str, tx: mpsc::Sender<DiscoveryEvent>) -> io::Result<Self> {
        if instance.encode_utf16().count() > 255
            || !instance
                .trim_end_matches('.')
                .to_ascii_lowercase()
                .ends_with("._veyl._tcp.local")
            || instance.contains('\0')
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid service instance.",
            ));
        }
        Self::start(instance, true, tx)
    }
    fn start(name: &str, resolve: bool, tx: mpsc::Sender<DiscoveryEvent>) -> io::Result<Self> {
        let context = Arc::new(Context {
            name: name.encode_utf16().chain(Some(0)).collect(),
            tx,
            cancel: Mutex::new(Cancel(DNS_SERVICE_CANCEL::default())),
            resolve,
            done: AtomicBool::new(false),
        });
        // Registry ownership keeps the name and handle alive through the terminal callback.
        let token = NEXT_TOKEN
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map_err(|_| io::Error::other("Discovery token limit reached."))?;
        contexts().lock().unwrap().insert(token, context.clone());
        let raw = token as *mut c_void;
        let mut cancel = context.cancel.lock().unwrap();
        let status = unsafe {
            if resolve {
                let request = DNS_SERVICE_RESOLVE_REQUEST {
                    Version: DNS_QUERY_REQUEST_VERSION1,
                    InterfaceIndex: 0,
                    QueryName: context.name.as_ptr().cast_mut(),
                    pResolveCompletionCallback: Some(resolved),
                    pQueryContext: raw,
                };
                DnsServiceResolve(&request, &mut cancel.0)
            } else {
                let request = DNS_SERVICE_BROWSE_REQUEST {
                    Version: DNS_QUERY_REQUEST_VERSION1,
                    InterfaceIndex: 0,
                    QueryName: context.name.as_ptr(),
                    Anonymous: DNS_SERVICE_BROWSE_REQUEST_0 {
                        pBrowseCallback: Some(browsed),
                    },
                    pQueryContext: raw,
                };
                DnsServiceBrowse(&request, &mut cancel.0)
            }
        };
        drop(cancel);
        if status != DNS_REQUEST_PENDING {
            // A synchronous failure does not schedule a callback.
            callback_context(raw, true);
            return Err(io::Error::from_raw_os_error(status));
        }
        Ok(Self(context))
    }
    pub fn is_done(&self) -> bool {
        self.0.done.load(Ordering::Acquire)
    }
}
impl Drop for NativeRequest {
    fn drop(&mut self) {
        if self.is_done() {
            return;
        }
        let cancel = self.0.cancel.lock().unwrap();
        // Cancellation is asynchronous. Its final callback releases the registry's reference.
        unsafe {
            if self.0.resolve {
                DnsServiceResolveCancel(&cancel.0);
            } else {
                DnsServiceBrowseCancel(&cancel.0);
            }
        }
    }
}

unsafe extern "system" fn browsed(status: u32, raw: *const c_void, records: *const DNS_RECORDW) {
    // Windows supplies these pointers for this callback; copy all data before freeing the list.
    let Some(context) = callback_context(raw, status == ERROR_CANCELLED) else {
        if !records.is_null() {
            DnsFree(records.cast(), DnsFreeRecordList);
        }
        return;
    };
    if status == ERROR_CANCELLED {
        context.done.store(true, Ordering::Release);
        if !records.is_null() {
            DnsFree(records.cast(), DnsFreeRecordList);
        }
        return;
    }
    if status == 0 {
        let mut record = records;
        for _ in 0..512 {
            if record.is_null() {
                break;
            }
            if (*record).wType == DNS_TYPE_PTR {
                if let Some(name) = wide((*record).Data.PTR.pNameHost) {
                    let _ = context.tx.try_send(DiscoveryEvent::Instance {
                        name,
                        gone: (*record).dwTtl == 0,
                    });
                }
            }
            record = (*record).pNext;
        }
    } else {
        let _ = context.tx.try_send(DiscoveryEvent::Failed(status));
    }
    if !records.is_null() {
        DnsFree(records.cast(), DnsFreeRecordList);
    }
}
unsafe extern "system" fn resolved(
    status: u32,
    raw: *const c_void,
    instance: *const DNS_SERVICE_INSTANCE,
) {
    let Some(context) = callback_context(raw, true) else {
        if !instance.is_null() {
            DnsServiceFreeInstance(instance);
        }
        return;
    };
    if status == 0 && !instance.is_null() {
        if let Some(service) = copy_service(&*instance) {
            let _ = context.tx.try_send(DiscoveryEvent::Resolved(service));
        }
    }
    if !instance.is_null() {
        DnsServiceFreeInstance(instance);
    }
    context.done.store(true, Ordering::Release);
}
unsafe fn copy_service(instance: &DNS_SERVICE_INSTANCE) -> Option<Service> {
    if instance.ip4Address.is_null() || instance.dwPropertyCount > 64 {
        return None;
    }
    let name = wide(instance.pszInstanceName)?;
    let mut properties = BTreeMap::new();
    if instance.dwPropertyCount > 0 {
        if instance.keys.is_null() || instance.values.is_null() {
            return None;
        }
        for i in 0..instance.dwPropertyCount as usize {
            let key = wide(*instance.keys.add(i))?;
            let value = wide(*instance.values.add(i))?;
            if properties.insert(key.to_ascii_lowercase(), value).is_some() {
                return None;
            }
        }
    }
    // IP4_ADDRESS stores network bytes in memory. wPort is the service's host-order port.
    let ip = Ipv4Addr::from((*instance.ip4Address).to_ne_bytes());
    Some(Service {
        instance: name,
        address: SocketAddrV4::new(ip, instance.wPort),
        properties,
    })
}
unsafe fn wide(value: *const u16) -> Option<String> {
    if value.is_null() {
        return None;
    }
    let mut length = 0;
    while length <= 1024 {
        if *value.add(length) == 0 {
            return String::from_utf16(std::slice::from_raw_parts(value, length)).ok();
        }
        length += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn terminal_callback_retires_token_while_inflight_callbacks_keep_owned_context() {
        let (tx, _) = mpsc::channel(1);
        let context = Arc::new(Context {
            name: vec![0],
            tx,
            cancel: Mutex::new(Cancel(DNS_SERVICE_CANCEL::default())),
            resolve: false,
            done: AtomicBool::new(false),
        });
        let token = NEXT_TOKEN.fetch_add(1, Ordering::Relaxed);
        contexts().lock().unwrap().insert(token, context.clone());
        let raw = token as *const c_void;
        let inflight = callback_context(raw, false).unwrap();
        let weak = Arc::downgrade(&context);
        unsafe {
            browsed(ERROR_CANCELLED, raw, std::ptr::null());
        }
        assert!(context.done.load(Ordering::Acquire));
        assert!(callback_context(raw, false).is_none());
        // A late callback safely ignores an already retired token.
        unsafe {
            browsed(0, raw, std::ptr::null());
        }
        drop(context);
        assert!(weak.upgrade().is_some());
        drop(inflight);
        assert!(weak.upgrade().is_none());
    }
    #[test]
    fn copies_owned_utf16_txt_ipv4_and_host_order_port() {
        let mut name: Vec<_> = "Desk._veyl._tcp.local"
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let mut keys: Vec<Vec<_>> = ["v", "id", "name", "model"]
            .map(|s| s.encode_utf16().chain(Some(0)).collect())
            .into();
        let mut values: Vec<Vec<_>> = ["1", "1234abcd", "Desk phone", "Test"]
            .map(|s| s.encode_utf16().chain(Some(0)).collect())
            .into();
        let mut key_ptrs: Vec<_> = keys.iter_mut().map(|s| s.as_mut_ptr()).collect();
        let mut value_ptrs: Vec<_> = values.iter_mut().map(|s| s.as_mut_ptr()).collect();
        let mut ip = u32::from_ne_bytes([192, 168, 1, 23]);
        let instance = DNS_SERVICE_INSTANCE {
            pszInstanceName: name.as_mut_ptr(),
            ip4Address: &mut ip,
            wPort: 47820,
            dwPropertyCount: 4,
            keys: key_ptrs.as_mut_ptr(),
            values: value_ptrs.as_mut_ptr(),
            ..Default::default()
        };
        let copied = unsafe { copy_service(&instance).unwrap() };
        assert_eq!(copied.address.to_string(), "192.168.1.23:47820");
        assert_eq!(copied.properties["name"], "Desk phone");
        let mut invalid = instance;
        invalid.dwPropertyCount = 65;
        assert!(unsafe { copy_service(&invalid) }.is_none());
    }
}
