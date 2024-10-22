mod bindings;
use bindings::*;
use std::ffi::CStr;

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

/// Get the version of the underlying cJSON library.
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

/// Struct for managing custom memory allocation and deallocation functions.
///
/// Fields:
/// - `malloc_fn`: Optional function pointer for custom memory allocation.
/// - `free_fn`: Optional function pointer for custom memory deallocation.
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
    /// Create new instance of the Hooks struct.
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

    /// Initialize the custom memory management hooks.
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

/// Rust binding for the underlying `cJSON` struct from the C library.
///
/// Fields:
/// - `next`: Pointer to the next `cJSON` object in a linked list.
/// - `prev`: Pointer to the previous `cJSON` object in a linked list.
/// - `child`: An array or object item will have a child pointing to a chain of items in the array or object.
/// - `type`: The type of the JSON value eg. `JsonValueType.Number`, `JsonValueType.Array`.
/// - `valuestring`: Pointer to the string value if the type is a string (and raw).
/// - `valueint`: Writing to this is deprecated. Use the `set_number_value` method instead.
/// - `valuedouble`: Double precision floating point value if the type is `JsonValueType.Number`.
/// - `string`: Pointer to the key string (used when this `cJSON` object is part of an object).
#[repr(C)]
pub struct Json {
    pub next: *mut Json,
    pub prev: *mut Json,
    pub child: *mut Json,
    pub type_: i32,
    pub valuestring: *mut i8,
    pub valueint: i32,
    pub valuedouble: f64,
    pub string: *mut i8,
}

/// Errors that can occur when working with Json objects.
///
/// Each variant indicates a specific kind of error can occur in these operations.
#[derive(Debug)]
pub enum JsonError {
    NullPointer,
    PrintError,
}

impl std::fmt::Display for JsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JsonError::NullPointer => write!(f, "the JSON pointer is null"),
            JsonError::PrintError => write!(f, "failed to print the JSON object"),
        }
    }
}

impl std::error::Error for JsonError {}

impl Json {
    // generate a string representation of the JSON object
    fn print(&self) -> Result<String, JsonError> {
        let c_str = unsafe { cJSON_Print(self as *const Json as *const cJSON) };
        if !c_str.is_null() {
            let c_str_ref = unsafe { CStr::from_ptr(c_str) };
            Ok(c_str_ref.to_str().unwrap_or_default().to_string())
        } else {
            Err(JsonError::PrintError)
        }
    }

    // generate unformatted string representation of the JSON object
    fn print_unformatted(&self) -> Result<String, JsonError> {
        let c_str = unsafe { cJSON_PrintUnformatted(self as *const Json as *const cJSON) };
        if !c_str.is_null() {
            let c_str_ref = unsafe { CStr::from_ptr(c_str) };
            Ok(c_str_ref.to_str().unwrap_or_default().to_string())
        } else {
            Err(JsonError::PrintError)
        }
    }

    // delete a JSON entity and all its subentities
    fn delete(&self) {
        unsafe { cJSON_Delete(self as *const Json as *mut cJSON) };
    }
}

pub trait JsonPtrExt {
    fn print(&self) -> Result<String, JsonError>;
    fn print_unformatted(&self) -> Result<String, JsonError>;
    fn delete(&self);
}

impl JsonPtrExt for *mut Json {
    /// Generate a string representation of the JSON object eg.
    /// ```json
    /// {
    ///     "name": "Nemuel",
    ///     "age": 20
    /// }
    /// ```
    ///
    /// Returns:
    /// - `Ok(String)` - if the JSON object's string representation is successfully generated.
    /// - `Err(JsonError::NullPointer)` - if the pointer is null.
    /// - `Err(JsonError::PrintError)` - if the string generation fails.
    ///
    /// Example:
    /// ```rust
    /// use cjson_rs::*;
    ///
    /// fn main() {
    ///     let json: *mut Json = create_object();
    ///     match json.print() {
    ///         Ok(result) => assert_eq!(result, "{\n}"),
    ///         Err(err) => eprintln!("{}", err),
    ///     }
    /// }
    /// ```
    fn print(&self) -> Result<String, JsonError> {
        match unsafe { self.as_mut() } {
            Some(json) => json.print(),
            None => Err(JsonError::NullPointer),
        }
    }

    /// Generate an **unformatted** string representation of the JSON object eg.
    /// ```json
    /// {
    ///     "name": "Nemuel",
    ///     "age": 20
    /// }
    /// ```
    ///
    /// Returns:
    /// - `Ok(String)` - if the JSON object's string representation is successfully generated.
    /// - `Err(JsonError::NullPointer)` - if the pointer is null.
    /// - `Err(JsonError::PrintError)` - if the string generation fails.
    ///
    /// Example:
    /// ```rust
    /// use cjson_rs::*;
    ///
    /// fn main() {
    ///     let json: *mut Json = create_object();
    ///     match json.print() {
    ///         Ok(result) => assert_eq!(result, "{\n}"),
    ///         Err(err) => eprintln!("{}", err),
    ///     }
    /// }
    /// ```
    fn print_unformatted(&self) -> Result<String, JsonError> {
        match unsafe { self.as_mut() } {
            Some(json) => json.print_unformatted(),
            None => Err(JsonError::NullPointer),
        }
    }

    /// Delete a JSON entity and all its subentities.
    ///
    /// Example:
    /// ```rust
    /// use cjson_rs::*;
    ///
    /// fn main() {
    ///     let json: *mut Json = create_object();
    ///     json.delete();
    /// }
    /// ```
    fn delete(&self) {
        unsafe { self.as_mut().map(|json| json.delete()) };
    }
}

/// Create a new JSON object (instance of the Json struct).
///
/// Example:
/// ```rust
/// use cjson_rs::{create_object, Json};
///
/// fn main() {
///     let json: *mut Json = create_object();
/// }
/// ```
pub fn create_object() -> *mut Json {
    unsafe { cJSON_CreateObject() as *mut Json }
}
