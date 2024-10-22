mod bindings;
use bindings::*;

pub const VERSION_MAJOR: u32 = bindings::CJSON_VERSION_MAJOR;
pub const VERSION_MINOR: u32 = bindings::CJSON_VERSION_MINOR;
pub const VERSION_PATCH: u32 = bindings::CJSON_VERSION_PATCH;
pub const IS_REFERENCE: u32 = 256;
pub const STRING_IS_CONST: u32 = 512;
pub const NESTING_LIMIT: u32 = 1000;
pub const CIRCULAR_LIMIT: u32 = 10000;

pub enum JsonValueType {
    Invalid = 0,
    False = 1,
    True = 2,
    Null = 4,
    Number = 8,
    String = 16,
    Array = 32,
    Object = 64,
    Raw = 128,
}

/// Get the version of the underlying cJSON library
///
/// Example:
/// ```rust
/// use cjson_rs::cjson_version;
///
/// fn main() {
///     assert_eq!(cjson_version(), "1.7.18");
/// }
/// ```
pub fn cjson_version() -> String {
    format!("{}.{}.{}", VERSION_MAJOR, VERSION_MINOR, VERSION_PATCH)
}

/// Struct for managing custom memory allocation and deallocation functions
///
/// Fields:
/// - `malloc_fn`: Optional function pointer for custom memory allocation
/// - `free_fn`: Option function pointer for custom memory deallocation
///
/// To create a new instance of `Hooks`, use its `new` method:
/// ```rust
/// let hooks = Hooks::new(Some(custom_malloc), Some(custom_free));
/// ```
///
/// To initialize hooks, use its `init` method:
/// ```rust
/// hooks.init();
/// ```
pub struct Hooks {
    pub malloc_fn: Option<fn(sz: usize) -> *mut libc::c_void>,
    pub free_fn: Option<fn(*mut libc::c_void)>,
}

impl Hooks {
    /// Create new instance of the Hooks struct
    ///
    /// If no functions are provided (passing `None` to the constructor), the
    /// cJSON library will use the default `malloc` and `free` functions from C.
    ///  
    /// Example:
    /// ```rust
    /// use cjson_rs::Hooks;
    /// use libc::{malloc, free};
    ///
    /// fn custom_malloc(size: usize) -> *mut libc::c_void {
    ///     println!("allocating memory ...");
    ///     unsafe { malloc(size) }
    /// }
    ///
    /// fn custom_free(ptr: *mut libc::c_void) {
    ///    println!("freeing memory ...");
    ///    unsafe { free(ptr); }
    /// }
    ///
    /// fn main() {
    ///     let _: Hooks = Hooks::new(Some(custom_malloc), Some(custom_free));
    ///     let _: Hooks = Hooks::new(None, None); // the default C functions will be used
    /// }
    ///
    /// ```
    pub fn new(
        malloc_fn: Option<fn(usize) -> *mut libc::c_void>,
        free_fn: Option<fn(*mut libc::c_void)>,
    ) -> Hooks {
        Hooks { malloc_fn, free_fn }
    }

    // map Hooks instance to cJSON_Hooks instance
    fn to_cjson_hooks(&self) -> cJSON_Hooks {
        cJSON_Hooks {
            malloc_fn: self.malloc_fn.map(|f| {
                static mut RUST_MALLOC_FN: Option<fn(usize) -> *mut libc::c_void> = None;
                unsafe { RUST_MALLOC_FN = Some(f) };

                unsafe extern "C" fn c_malloc(sz: usize) -> *mut libc::c_void {
                    if let Some(f) = RUST_MALLOC_FN {
                        let boxed = Box::new(f(sz));
                        Box::into_raw(boxed) as *mut libc::c_void
                    } else {
                        std::ptr::null_mut()
                    }
                }

                c_malloc as unsafe extern "C" fn(usize) -> *mut libc::c_void
            }),
            free_fn: self.free_fn.map(|f| {
                static mut RUST_FREE_FN: Option<fn(*mut libc::c_void)> = None;
                unsafe { RUST_FREE_FN = Some(f) };

                unsafe extern "C" fn c_free(ptr: *mut libc::c_void) {
                    if let Some(f) = RUST_FREE_FN {
                        if !ptr.is_null() {
                            f(ptr)
                        }
                    }
                }

                c_free as unsafe extern "C" fn(*mut libc::c_void)
            }),
        }
    }

    /// Initialize the custom memory management hooks
    ///
    /// Usage:
    /// ```rust
    /// let hooks = Hooks::new(Some(custom_malloc), Some(custom_free));
    /// hooks.init();
    /// ```
    pub fn init(&self) {
        unsafe {
            cJSON_InitHooks(&mut self.to_cjson_hooks());
        }
    }
}

/// Rust binding for the underlying `cJSON` struct from the C library
///
/// Fields:
/// - `next`: pointer to the next `cJSON` object in a linked list
/// - `prev`: pointer to the previous `cJSON` object in a linked list
/// - `child`: array or object item will have a child pointing to a chain of items in the array or object
/// - `type`: the type of the JSON value eg. `JsonValueType.Number`, `JsonValueType.Array`
/// - `valuestring`: pointer to the string value if the type is a string (and raw)
/// - `valueint`: writing to this is deprecated. Use the `set_number_value` method instead
/// - `valuedouble`: double precision floating point value if the type is `JsonValueType.Number`
/// - `string`: pointer to the key string (used when this `cJSON` object is part of an object)
pub struct CJson {
    pub next: *mut CJson,
    pub prev: *mut CJson,
    pub child: *mut CJson,
    pub type_: i32,
    pub valuestring: *mut i8,
    pub valueint: i32,
    pub valuedouble: f64,
    pub string: *mut i8,
}
