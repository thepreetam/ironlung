//! Shared getaddrinfo sandbox IPC protocol. Types and serialization.
//! Used by both the main ironlung lib and the ironlung-sandbox binary.

#![no_std]

use core::ptr;

use libc;

/// Null-terminated hostname, max 256 bytes.
pub const NODE_LEN: usize = 256;
/// Null-terminated service/port, max 256 bytes.
pub const SERV_LEN: usize = 256;
/// Max bytes for ai_addr (sockaddr_storage).
pub const AI_ADDR_MAX: usize = 128;
/// Max bytes for ai_canonname.
pub const AI_CANONNAME_MAX: usize = 256;

/// Scalar hints fields only; pointers cannot cross process boundary.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GetAddrInfoHints {
    pub ai_flags: libc::c_int,
    pub ai_family: libc::c_int,
    pub ai_socktype: libc::c_int,
    pub ai_protocol: libc::c_int,
}

/// Request at SHM offset 0.
#[repr(C)]
pub struct GetAddrInfoRequest {
    pub node: [u8; NODE_LEN],
    pub service: [u8; SERV_LEN],
    pub hints: GetAddrInfoHints,
}

/// Response at fixed offset after request.
#[repr(C)]
pub struct GetAddrInfoResponse {
    pub result: libc::c_int,
    pub errno: libc::c_int,
    pub addrinfo_data_len: usize,
    pub addrinfo_data_offset: usize,
}

/// Fixed sizes for layout.
pub const REQUEST_SIZE: usize = core::mem::size_of::<GetAddrInfoRequest>();
pub const RESPONSE_SIZE: usize = core::mem::size_of::<GetAddrInfoResponse>();
pub const RESPONSE_OFFSET: usize = REQUEST_SIZE;
/// Start of serialized addrinfo data region (after response).
pub const DATA_OFFSET: usize = RESPONSE_OFFSET + RESPONSE_SIZE;

/// Serialized addrinfo node header (before variable-length ai_addr and canonname).
#[repr(C)]
pub struct AddrInfoNodeHeader {
    pub ai_flags: libc::c_int,
    pub ai_family: libc::c_int,
    pub ai_socktype: libc::c_int,
    pub ai_protocol: libc::c_int,
    pub ai_addrlen: libc::socklen_t,
    pub next_offset: usize,
}

const NODE_HEADER_SIZE: usize = core::mem::size_of::<AddrInfoNodeHeader>();

/// Size of one serialized node in worst case (header + max addr + max canonname + next_offset).
pub const NODE_MAX_SIZE: usize = NODE_HEADER_SIZE + AI_ADDR_MAX + AI_CANONNAME_MAX + 8;

/// Serialize addrinfo list into buffer. Returns total bytes written, or 0 on overflow.
/// Used by sandbox process. Caller must call freeaddrinfo on the list afterward.
pub unsafe fn serialize_addrinfo_list(
    mut head: *mut libc::addrinfo,
    base: *mut u8,
    capacity: usize,
) -> usize {
    let mut offset = 0usize;
    while !head.is_null() {
        if offset + NODE_MAX_SIZE > capacity {
            return 0;
        }
        let ai = &*head;
        let hdr = AddrInfoNodeHeader {
            ai_flags: ai.ai_flags,
            ai_family: ai.ai_family,
            ai_socktype: ai.ai_socktype,
            ai_protocol: ai.ai_protocol,
            ai_addrlen: ai.ai_addrlen,
            next_offset: 0,
        };
        ptr::copy_nonoverlapping(
            &hdr as *const _ as *const u8,
            base.add(offset),
            NODE_HEADER_SIZE,
        );
        offset += NODE_HEADER_SIZE;

        let addrlen = ai.ai_addrlen as usize;
        if addrlen > AI_ADDR_MAX {
            return 0;
        }
        if offset + addrlen + AI_CANONNAME_MAX + 8 > capacity {
            return 0;
        }
        if !ai.ai_addr.is_null() && addrlen > 0 {
            ptr::copy_nonoverlapping(ai.ai_addr as *const u8, base.add(offset), addrlen);
        }
        offset += AI_ADDR_MAX;

        let mut canonlen = 0usize;
        if !ai.ai_canonname.is_null() {
            while canonlen < AI_CANONNAME_MAX - 1 {
                let c = *ai.ai_canonname.add(canonlen);
                if c == 0 {
                    break;
                }
                *base.add(offset + canonlen) = c as u8;
                canonlen += 1;
            }
        }
        *base.add(offset + canonlen) = 0;
        offset += AI_CANONNAME_MAX;

        let node_end = offset + 8;
        let next_off = if ai.ai_next.is_null() {
            0usize
        } else {
            node_end
        };
        ptr::write(base.add(offset) as *mut usize, next_off);
        offset = node_end;

        head = ai.ai_next;
    }
    offset
}
