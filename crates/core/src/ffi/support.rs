use std::marker::PhantomData;
use std::ptr;
use std::slice;
use std::str;

#[repr(C)]
pub struct RustResult {
    pub is_ok: bool,
    pub ok_result: *mut std::ffi::c_void,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct RustSlice<'a, T> {
    pub start: *const T,
    pub len: usize,
    _marker: PhantomData<&'a T>,
}
impl<'a, T> RustSlice<'a, T> {
    pub fn from_slice(slice: &'a [T]) -> Self {
        Self {
            start: slice.as_ptr(),
            len: slice.len(),
            _marker: PhantomData,
        }
    }

    pub fn as_slice(&self) -> &'a [T] {
        unsafe { slice::from_raw_parts(self.start, self.len) }
    }
}
impl<'a, T> Default for RustSlice<'a, T> {
    fn default() -> Self {
        Self {
            start: ptr::null(),
            len: 0,
            _marker: PhantomData,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct RustStr<'a> {
    pub start: *const u8,
    pub len: usize,
    _marker: PhantomData<&'a str>,
}
impl<'a> RustStr<'a> {
    pub fn from_str(str: &'a str) -> Self {
        let bytes = str.as_bytes();
        Self {
            start: bytes.as_ptr(),
            len: bytes.len(),
            _marker: PhantomData,
        }
    }

    pub fn to_str(&self) -> &'a str {
        let slice = unsafe { slice::from_raw_parts(self.start, self.len) };
        unsafe { str::from_utf8_unchecked(slice) }
    }
}
impl<'a> Default for RustStr<'a> {
    fn default() -> Self {
        Self {
            start: ptr::null(),
            len: 0,
            _marker: PhantomData,
        }
    }
}

#[export_name = "__liveview_native_core$RustStr$eq"]
pub extern "C" fn ruststr_eq(lhs: RustStr, rhs: RustStr) -> bool {
    lhs.to_str() == rhs.to_str()
}

#[export_name = "__liveview_native_core$RustStr$lt"]
pub extern "C" fn ruststr_lt(lhs: RustStr, rhs: RustStr) -> bool {
    lhs.to_str() < rhs.to_str()
}

#[repr(C)]
pub struct RustString {
    pub ptr: *mut u8,
    pub len: usize,
    pub capacity: usize,
}
impl RustString {
    pub fn from_string(str: String) -> Self {
        let (ptr, len, capacity) = String::into_raw_parts(str);
        Self { ptr, len, capacity }
    }

    pub fn to_string(self) -> String {
        unsafe { String::from_raw_parts(self.ptr, self.len, self.capacity) }
    }
}

#[export_name = "__liveview_native_core$RustString$drop"]
pub extern "C" fn drop_rust_string(string: RustString) {
    drop(string.to_string());
}
