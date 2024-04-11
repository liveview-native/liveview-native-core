use core::{
    cell::{Cell, RefCell},
    cmp::{self, Ordering},
    fmt,
    hash::{Hash, Hasher},
    mem,
    ops::Deref,
    ptr, slice, str,
};
use std::sync::{OnceLock, RwLock};

use fxhash::FxHashMap;

#[cfg_attr(rustfmt, rustfmt_skip)]
#[allow(nonstandard_style, non_upper_case_globals)]
pub mod symbols {
    // During the build step, `build.rs` will output the generated atoms to `OUT_DIR` to avoid
    // adding it to the source directory, so we just directly include the generated code here.
    include!(concat!(env!("OUT_DIR"), "/strings.rs"));
}

static SYMBOL_TABLE: OnceLock<SymbolTable> = OnceLock::new();

struct SymbolTable(RwLock<Interner>);
unsafe impl Sync for SymbolTable {}
impl SymbolTable {
    fn new() -> Self {
        Self(RwLock::new(Interner::new()))
    }
}

/// A symbol is an interned string.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Symbol(SymbolIndex);

impl Symbol {
    pub const fn new(n: u32) -> Self {
        Symbol(SymbolIndex::new(n))
    }

    /// Maps a string to its interned representation.
    pub fn intern(string: &str) -> Self {
        with_interner(|interner| interner.intern(string))
    }

    /// Converts this to an InternedString, which is more useful for comparisons/sorting/etc.
    #[inline]
    pub fn as_str(self) -> InternedString {
        InternedString(self)
    }

    #[inline]
    pub fn as_u32(self) -> u32 {
        self.0.as_u32()
    }

    #[inline]
    pub fn as_usize(self) -> usize {
        self.0.as_usize()
    }
}
impl From<&str> for Symbol {
    #[inline]
    fn from(s: &str) -> Self {
        Self::intern(s)
    }
}
impl From<String> for Symbol {
    #[inline]
    fn from(s: String) -> Self {
        Self::intern(s.as_str())
    }
}
impl fmt::Debug for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}({:?})", self, self.0)
    }
}
impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.as_str(), f)
    }
}
impl<T: Deref<Target = str>> PartialEq<T> for Symbol {
    fn eq(&self, other: &T) -> bool {
        self.as_str() == other.deref()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct SymbolIndex(u32);
impl From<SymbolIndex> for u32 {
    #[inline]
    fn from(v: SymbolIndex) -> u32 {
        v.as_u32()
    }
}
impl From<SymbolIndex> for usize {
    #[inline]
    fn from(v: SymbolIndex) -> usize {
        v.as_usize()
    }
}
impl SymbolIndex {
    // shave off 256 indices at the end to allow space for packing these indices into enums
    pub const MAX_AS_U32: u32 = 0xFFFF_FF00;

    pub const MAX: SymbolIndex = SymbolIndex::new(0xFFFF_FF00);

    #[inline]
    const fn new(n: u32) -> Self {
        assert!(n <= Self::MAX_AS_U32, "out of range value used");

        SymbolIndex(n)
    }

    #[inline]
    pub fn as_u32(self) -> u32 {
        self.0
    }

    #[inline]
    pub fn as_usize(self) -> usize {
        self.0 as usize
    }
}

/// An `Interner` stores unique strings in an arena that lives for the duration of its owning thread,
/// which allows those strings to be treated as immortal (i.e. static). It further allocates `Symbol`
/// for each unique string, which is a small, copyable handle that can be much more efficiently compared
/// for equality, and can be used to get access to the original string data it represents.
#[derive(Default)]
pub struct Interner {
    arena: ByteArena,
    pub symbols: FxHashMap<&'static str, Symbol>,
    pub strings: Vec<&'static str>,
}
impl Interner {
    pub fn new() -> Self {
        let mut this = Interner::default();
        for (sym, s) in symbols::__SYMBOLS {
            this.symbols.insert(s, *sym);
            this.strings.push(s);
        }
        this
    }

    pub fn intern(&mut self, string: &str) -> Symbol {
        if let Some(&symbol) = self.symbols.get(string) {
            return symbol;
        }

        let symbol = Symbol::new(self.strings.len() as u32);

        // `from_utf8_unchecked` is safe since we just allocated a `&str` which is known to be
        // UTF-8.
        let string: &str =
            unsafe { str::from_utf8_unchecked(self.arena.alloc_slice(string.as_bytes())) };
        // It is safe to extend the arena allocation to `'static` because we only access
        // these while the arena is still alive.
        let string: &'static str = unsafe { &*(string as *const str) };
        self.strings.push(string);
        self.symbols.insert(string, symbol);
        symbol
    }

    #[inline]
    pub fn get(&self, symbol: Symbol) -> &str {
        self.strings[symbol.0.as_usize()]
    }
}

// If an interner exists, return it. Otherwise, prepare a fresh one.
#[inline]
fn with_interner<T, F: FnOnce(&mut Interner) -> T>(f: F) -> T {
    let symbol_table = SYMBOL_TABLE.get_or_init(|| SymbolTable::new());
    let mut r = symbol_table
        .0
        .write()
        .expect("unable to acquire write lock for symbol table");
    f(&mut *r)
}

#[inline]
fn with_read_only_interner<T, F: FnOnce(&Interner) -> T>(f: F) -> T {
    let symbol_table = SYMBOL_TABLE.get_or_init(|| SymbolTable::new());
    let r = symbol_table
        .0
        .read()
        .expect("unable to acquire read lock for symbol table");
    f(&*r)
}

/// Represents a string stored in the global string interner, and is thus thread-safe
#[derive(Clone, Copy, Eq)]
#[repr(transparent)]
pub struct InternedString(Symbol);

impl InternedString {
    #[inline(always)]
    pub fn as_symbol(self) -> Symbol {
        self.0
    }

    /// Interns the given string
    pub fn intern(string: &str) -> Self {
        with_interner(|interner| Self(interner.intern(string)))
    }

    #[inline]
    pub fn as_str(self) -> &'static str {
        with_read_only_interner(|interner| unsafe {
            mem::transmute::<&str, &str>(interner.get(self.0))
        })
    }
}
impl Hash for InternedString {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state)
    }
}
impl PartialOrd<InternedString> for InternedString {
    fn partial_cmp(&self, other: &InternedString) -> Option<Ordering> {
        if self.0 == other.0 {
            return Some(Ordering::Equal);
        }
        self.as_str().partial_cmp(other.as_str())
    }
}
impl Ord for InternedString {
    fn cmp(&self, other: &InternedString) -> Ordering {
        if self.0 == other.0 {
            return Ordering::Equal;
        }
        self.as_str().cmp(&other.as_str())
    }
}
impl<T: Deref<Target = str>> PartialEq<T> for InternedString {
    fn eq(&self, other: &T) -> bool {
        self.as_str() == other.deref()
    }
}
impl PartialEq<Symbol> for InternedString {
    fn eq(&self, other: &Symbol) -> bool {
        self.0 == *other
    }
}
impl PartialEq<InternedString> for InternedString {
    fn eq(&self, other: &InternedString) -> bool {
        self.0 == other.0
    }
}
impl<'a> PartialEq<&'a InternedString> for InternedString {
    fn eq(&self, other: &&InternedString) -> bool {
        self.0 == other.0
    }
}
impl PartialEq<InternedString> for str {
    fn eq(&self, other: &InternedString) -> bool {
        self == other.as_str()
    }
}
impl<'a> PartialEq<InternedString> for &'a str {
    fn eq(&self, other: &InternedString) -> bool {
        *self == other.as_str()
    }
}
impl PartialEq<InternedString> for String {
    fn eq(&self, other: &InternedString) -> bool {
        self.as_str() == other.as_str()
    }
}
impl<'a> PartialEq<InternedString> for &'a String {
    fn eq(&self, other: &InternedString) -> bool {
        self.as_str() == other.as_str()
    }
}
impl<'a, const N: usize> PartialEq<InternedString> for smallstr::SmallString<[u8; N]> {
    fn eq(&self, other: &InternedString) -> bool {
        self.as_str() == other.as_str()
    }
}
impl From<&str> for InternedString {
    #[inline]
    fn from(string: &str) -> Self {
        Self::intern(string)
    }
}
impl From<String> for InternedString {
    #[inline]
    fn from(string: String) -> Self {
        Self::intern(string.as_str())
    }
}
impl<'a> From<std::borrow::Cow<'a, str>> for InternedString {
    #[inline]
    fn from(string: std::borrow::Cow<'a, str>) -> Self {
        Self::intern(string.as_ref())
    }
}
impl From<Symbol> for InternedString {
    #[inline(always)]
    fn from(sym: Symbol) -> Self {
        Self(sym)
    }
}
impl<const N: usize> From<smallstr::SmallString<[u8; N]>> for InternedString {
    #[inline]
    fn from(s: smallstr::SmallString<[u8; N]>) -> Self {
        Self::intern(s.as_str())
    }
}
impl From<InternedString> for String {
    #[inline]
    fn from(val: InternedString) -> String {
        val.to_string()
    }
}
impl<const N: usize> From<InternedString> for smallstr::SmallString<[u8; N]> {
    fn from(val: InternedString) -> smallstr::SmallString<[u8; N]> {
        smallstr::SmallString::from_str(val.as_str())
    }
}
impl fmt::Debug for InternedString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}
impl fmt::Display for InternedString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}

// Arena

pub struct ByteArena {
    /// A pointer to the next object to be allocated.
    ptr: Cell<*mut u8>,

    /// A pointer to the end of the allocated area. When this pointer is
    /// reached, a new chunk is allocated.
    end: Cell<*mut u8>,

    /// A vector of arena chunks.
    chunks: RefCell<Vec<TypedArenaChunk<u8>>>,
}

unsafe impl Send for ByteArena {}

impl Default for ByteArena {
    #[inline]
    fn default() -> ByteArena {
        Self {
            ptr: Cell::new(0 as *mut u8),
            end: Cell::new(0 as *mut u8),
            chunks: Default::default(),
        }
    }
}

impl ByteArena {
    #[inline]
    fn align(&self, align: usize) {
        let final_address = ((self.ptr.get() as usize) + align - 1) & !(align - 1);
        self.ptr.set(final_address as *mut u8);
        assert!(self.ptr <= self.end);
    }

    #[inline(never)]
    #[cold]
    fn grow(&self, needed_bytes: usize) {
        unsafe {
            let mut chunks = self.chunks.borrow_mut();
            let (chunk, mut new_capacity);
            if let Some(last_chunk) = chunks.last_mut() {
                let used_bytes = self.ptr.get() as usize - last_chunk.start() as usize;
                new_capacity = last_chunk.storage.capacity();
                loop {
                    new_capacity = new_capacity.checked_mul(2).unwrap();
                    if new_capacity >= used_bytes + needed_bytes {
                        break;
                    }
                }
            } else {
                new_capacity = cmp::max(needed_bytes, PAGE);
            }
            chunk = TypedArenaChunk::<u8>::new(new_capacity);
            self.ptr.set(chunk.start());
            self.end.set(chunk.end());
            chunks.push(chunk);
        }
    }

    #[inline]
    pub unsafe fn alloc_raw(&self, bytes: usize, align: usize) -> *mut u8 {
        assert!(bytes != 0);

        self.align(align);

        let future_end = self.ptr.get().wrapping_offset(bytes as isize);
        if (future_end as *mut u8) >= self.end.get() {
            self.grow(bytes);
        }

        let ptr = self.ptr.get();
        // Set the pointer past ourselves
        self.ptr
            .set(self.ptr.get().wrapping_offset(bytes as isize) as *mut u8);

        ptr
    }

    /// Allocates a slice of objects that are copied into the `ByteArena`, returning a mutable
    /// reference to it. Will panic if passed a zero-sized type.
    ///
    /// Panics:
    ///
    ///  - Zero-sized types
    ///  - Zero-length slices
    #[inline]
    pub fn alloc_slice<T>(&self, slice: &[T]) -> &mut [T]
    where
        T: Copy,
    {
        assert!(!mem::needs_drop::<T>());
        assert!(mem::size_of::<T>() != 0);
        assert!(!slice.is_empty());

        unsafe {
            let mem = self.alloc_raw(slice.len() * mem::size_of::<T>(), mem::align_of::<T>())
                as *mut _ as *mut T;

            let arena_slice = slice::from_raw_parts_mut(mem, slice.len());
            arena_slice.copy_from_slice(slice);
            arena_slice
        }
    }
}

struct TypedArenaChunk<T> {
    /// The raw storage for the arena chunk.
    storage: Vec<T>,
}

impl<T> TypedArenaChunk<T> {
    #[inline]
    unsafe fn new(capacity: usize) -> TypedArenaChunk<T> {
        TypedArenaChunk {
            storage: Vec::with_capacity(capacity),
        }
    }

    /// Destroys this arena chunk.
    #[inline]
    #[allow(unused)]
    unsafe fn destroy(&mut self, len: usize) {
        // The branch on needs_drop() is an -O1 performance optimization.
        // Without the branch, dropping TypedArena<u8> takes linear time.
        if mem::needs_drop::<T>() {
            let mut start = self.start();
            // Destroy all allocated objects.
            for _ in 0..len {
                ptr::drop_in_place(start);
                start = start.offset(1);
            }
        }
    }

    // Returns a pointer to the first allocated object.
    #[inline]
    fn start(&self) -> *mut T {
        self.storage.as_ptr() as *mut T
    }

    // Returns a pointer to the end of the allocated space.
    #[inline]
    fn end(&self) -> *mut T {
        unsafe {
            if mem::size_of::<T>() == 0 {
                // A pointer as large as possible for zero-sized elements.
                !0 as *mut T
            } else {
                self.start().add(self.storage.capacity())
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
const PAGE: usize = 4 * 1024;

#[cfg(target_arch = "wasm32")]
const PAGE: usize = 64 * 1024;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interner_tests() {
        let mut i: Interner = Interner::default();
        // first one is zero:
        assert_eq!(i.intern("dog"), Symbol::new(0));
        // re-use gets the same entry:
        assert_eq!(i.intern("dog"), Symbol::new(0));
        // different string gets a different #:
        assert_eq!(i.intern("cat"), Symbol::new(1));
        assert_eq!(i.intern("cat"), Symbol::new(1));
        // dog is still at zero
        assert_eq!(i.intern("dog"), Symbol::new(0));
    }

    #[test]
    fn interned_keywords_no_gaps() {
        let mut i = Interner::new();
        // Should already be interned with matching indexes
        for (sym, s) in symbols::__SYMBOLS {
            assert_eq!(i.intern(&s), *sym)
        }
        // Should create a new symbol resulting in an index equal to the last entry in the table
        assert_eq!(i.intern("foo").as_u32(), (i.symbols.len() - 1) as u32);
    }
}
