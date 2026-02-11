//! getaddrinfo sandbox IPC protocol. Shared memory layout and serialization.
//! Re-exports from ironlung-protocol and adds parent-side helpers.

use core::ptr;

use libc;

pub use ironlung_protocol::{
    serialize_addrinfo_list, AddrInfoNodeHeader, DATA_OFFSET, GetAddrInfoHints, GetAddrInfoRequest,
    GetAddrInfoResponse, NODE_LEN, NODE_MAX_SIZE, REQUEST_SIZE, RESPONSE_OFFSET, RESPONSE_SIZE,
    SERV_LEN, AI_ADDR_MAX, AI_CANONNAME_MAX,
};

const NODE_HEADER_SIZE: usize = core::mem::size_of::<AddrInfoNodeHeader>();

/// Build request from getaddrinfo arguments. Caller must pass hints as NULL or stack-local
/// (only scalar fields are copied).
pub unsafe fn build_request(
    req: *mut GetAddrInfoRequest,
    node: *const libc::c_char,
    service: *const libc::c_char,
    hints: *const libc::addrinfo,
) {
    ptr::write_bytes(req, 0, 1);
    let r = &mut *req;

    if !node.is_null() {
        let mut i = 0usize;
        while i < NODE_LEN - 1 {
            let c = *node.add(i);
            r.node[i] = c as u8;
            if c == 0 {
                break;
            }
            i += 1;
        }
        r.node[NODE_LEN - 1] = 0;
    }

    if !service.is_null() {
        let mut i = 0usize;
        while i < SERV_LEN - 1 {
            let c = *service.add(i);
            r.service[i] = c as u8;
            if c == 0 {
                break;
            }
            i += 1;
        }
        r.service[SERV_LEN - 1] = 0;
    }

    if hints.is_null() {
        r.hints = GetAddrInfoHints {
            ai_flags: 0,
            ai_family: libc::AF_UNSPEC,
            ai_socktype: 0,
            ai_protocol: 0,
        };
    } else {
        let h = &*hints;
        r.hints = GetAddrInfoHints {
            ai_flags: h.ai_flags,
            ai_family: h.ai_family,
            ai_socktype: h.ai_socktype,
            ai_protocol: h.ai_protocol,
        };
    }
}

/// Deserialize addrinfo list from buffer. Allocates via malloc.
/// Returns head pointer, or null on error. Caller frees via freeaddrinfo.
pub unsafe fn deserialize_addrinfo_list(base: *const u8, len: usize) -> *mut libc::addrinfo {
    extern "C" {
        fn malloc(size: libc::size_t) -> *mut core::ffi::c_void;
    }
    let mut offset = 0usize;
    let mut head: *mut libc::addrinfo = core::ptr::null_mut();
    let mut tail: *mut *mut libc::addrinfo = core::ptr::null_mut();

    while offset + NODE_HEADER_SIZE <= len {
        let hdr = &*(base.add(offset) as *const AddrInfoNodeHeader);
        let addrlen = hdr.ai_addrlen as usize;
        if addrlen > AI_ADDR_MAX {
            break;
        }
        let node_size = NODE_HEADER_SIZE + AI_ADDR_MAX + AI_CANONNAME_MAX + 8;
        if offset + node_size > len {
            break;
        }

        let ai = malloc(core::mem::size_of::<libc::addrinfo>()) as *mut libc::addrinfo;
        if ai.is_null() {
            if !head.is_null() {
                extern "C" {
                    fn freeaddrinfo(res: *mut libc::addrinfo);
                }
                freeaddrinfo(head);
            }
            return core::ptr::null_mut();
        }
        ptr::write(ai, libc::addrinfo {
            ai_flags: hdr.ai_flags,
            ai_family: hdr.ai_family,
            ai_socktype: hdr.ai_socktype,
            ai_protocol: hdr.ai_protocol,
            ai_addrlen: hdr.ai_addrlen,
            ai_addr: core::ptr::null_mut(),
            ai_canonname: core::ptr::null_mut(),
            ai_next: core::ptr::null_mut(),
        });

        let ai_addr_buf = base.add(offset + NODE_HEADER_SIZE);
        if addrlen > 0 {
            let addr = malloc(addrlen) as *mut libc::sockaddr;
            if addr.is_null() {
                extern "C" {
                    fn freeaddrinfo(res: *mut libc::addrinfo);
                }
                freeaddrinfo(head);
                return core::ptr::null_mut();
            }
            ptr::copy_nonoverlapping(ai_addr_buf, addr as *mut u8, addrlen);
            (*ai).ai_addr = addr;
        }

        let canon_buf = base.add(offset + NODE_HEADER_SIZE + AI_ADDR_MAX);
        let mut canonlen = 0usize;
        while canonlen < AI_CANONNAME_MAX {
            if *canon_buf.add(canonlen) == 0 {
                break;
            }
            canonlen += 1;
        }
        canonlen += 1;
        if canonlen > 1 {
            let canon = malloc(canonlen) as *mut libc::c_char;
            if canon.is_null() {
                extern "C" {
                    fn freeaddrinfo(res: *mut libc::addrinfo);
                }
                freeaddrinfo(head);
                return core::ptr::null_mut();
            }
            ptr::copy_nonoverlapping(canon_buf, canon as *mut u8, canonlen);
            (*ai).ai_canonname = canon;
        }

        if head.is_null() {
            head = ai;
            tail = &mut (*ai).ai_next;
        } else {
            *tail = ai;
            tail = &mut (*ai).ai_next;
        }

        let next_off = ptr::read(
            base.add(offset + NODE_HEADER_SIZE + AI_ADDR_MAX + AI_CANONNAME_MAX) as *const usize,
        );
        if next_off == 0 {
            break;
        }
        offset = next_off;
    }
    head
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use core::ptr;

    #[test]
    fn test_protocol_layout() {
        assert!(REQUEST_SIZE >= 256, "request has node+service+hints");
        assert!(NODE_MAX_SIZE <= 1024, "single node bounded");
        assert_eq!(NODE_HEADER_SIZE, core::mem::size_of::<AddrInfoNodeHeader>());
    }

    #[test]
    #[ignore] // Requires LD_PRELOAD; can hang as unit test due to allocator/dlsym
    fn test_deserialize_addrinfo_roundtrip() {
        extern "C" {
            fn freeaddrinfo(res: *mut libc::addrinfo);
        }
        // Build a minimal serialized buffer: one node, AF_INET, sockaddr_in (16 bytes), canonname "x\0"
        let mut buf = [0u8; NODE_MAX_SIZE];
        let hdr = AddrInfoNodeHeader {
            ai_flags: 0,
            ai_family: libc::AF_INET,
            ai_socktype: libc::SOCK_STREAM,
            ai_protocol: 0,
            ai_addrlen: 16,
            next_offset: 0,
        };
        unsafe {
            ptr::copy_nonoverlapping(
                &hdr as *const _ as *const u8,
                buf.as_mut_ptr(),
                NODE_HEADER_SIZE,
            );
        }
        // ai_addr: 16 bytes (sockaddr_in), zeroed
        // ai_canonname: "x\0" at offset NODE_HEADER_SIZE + AI_ADDR_MAX
        let canon_offset = NODE_HEADER_SIZE + AI_ADDR_MAX;
        buf[canon_offset] = b'x';
        buf[canon_offset + 1] = 0;
        // next_offset = 0 at end
        let next_offset = canon_offset + AI_CANONNAME_MAX;
        unsafe {
            ptr::write(buf[next_offset..].as_mut_ptr() as *mut usize, 0usize);
        }

        let result = unsafe { deserialize_addrinfo_list(buf.as_ptr(), buf.len()) };
        assert!(!result.is_null(), "deserialize should return non-null");
        let ai = unsafe { &*result };
        assert_eq!(ai.ai_family, libc::AF_INET);
        assert_eq!(ai.ai_socktype, libc::SOCK_STREAM);
        assert_eq!(ai.ai_addrlen, 16);
        assert!(!ai.ai_addr.is_null());
        assert!(!ai.ai_canonname.is_null());
        assert_eq!(unsafe { *ai.ai_canonname as u8 }, b'x');
        assert!(ai.ai_next.is_null());

        unsafe { freeaddrinfo(result) };
    }
}
