mod bindings;
use bindings::*;
use std::ffi::{CStr, CString, NulError};

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
    CStringError(NulError),
    EmptyStringError,
    NullPointer,
    ParseError,
    PrintError,
    PrintBufferedError,
    PrintPreallocatedError,
}

impl std::fmt::Display for JsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JsonError::CStringError(err) => write!(f, "CString error: {}", err),
            JsonError::EmptyStringError => write!(f, "you provided an empty string"),
            JsonError::NullPointer => write!(f, "the JSON pointer is null"),
            JsonError::ParseError => write!(f, "failed to parse the JSON string"),
            JsonError::PrintError => write!(f, "failed to print the JSON object"),
            JsonError::PrintBufferedError => {
                write!(f, "failed to print the JSON object to allocated buffer")
            }
            JsonError::PrintPreallocatedError => {
                write!(f, "failed to print the JSON object to preallocated buffer")
            }
        }
    }
}

impl std::error::Error for JsonError {}

impl Json {
    // check whether the Json object is of type Invalid
    fn is_invalid(&self) -> bool {
        unsafe { cJSON_IsInvalid(self as *const Json as *const cJSON) == 1 }
    }

    // check whether the Json object is of type False
    fn is_false(&self) -> bool {
        unsafe { cJSON_IsFalse(self as *const Json as *const cJSON) == 1 }
    }

    // check whether the Json object is of type True
    fn is_true(&self) -> bool {
        unsafe { cJSON_IsTrue(self as *const Json as *const cJSON) == 1 }
    }

    // check whether the Json object is of type Bool
    fn is_bool(&self) -> bool {
        unsafe { cJSON_IsBool(self as *const Json as *const cJSON) == 1 }
    }

    // check whether the Json object is of type Null
    fn is_null(&self) -> bool {
        unsafe { cJSON_IsNull(self as *const Json as *const cJSON) == 1 }
    }

    // check whether the Json object is of type Number
    fn is_number(&self) -> bool {
        unsafe { cJSON_IsNumber(self as *const Json as *const cJSON) == 1 }
    }

    // check whether the Json object is of type String
    fn is_string(&self) -> bool {
        unsafe { cJSON_IsString(self as *const Json as *const cJSON) == 1 }
    }

    // check whether the Json object is of type Array
    fn is_array(&self) -> bool {
        unsafe { cJSON_IsArray(self as *const Json as *const cJSON) == 1 }
    }

    // check whether the Json object is of type Object
    fn is_object(&self) -> bool {
        unsafe { cJSON_IsObject(self as *const Json as *const cJSON) == 1 }
    }

    // check whether the Json object is of type Raw
    fn is_raw(&self) -> bool {
        unsafe { cJSON_IsRaw(self as *const Json as *const cJSON) == 1 }
    }

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

    // generate a string representation of the JSON object with dynamic buffer resizing
    fn print_buffered(&self, prebuffer: i32, fmt: bool) -> Result<String, JsonError> {
        let c_str = unsafe {
            cJSON_PrintBuffered(
                self as *const Json as *const cJSON,
                prebuffer,
                if fmt { 1 } else { 0 },
            )
        };
        if !c_str.is_null() {
            let c_str_ref = unsafe { CStr::from_ptr(c_str) };
            Ok(c_str_ref.to_str().unwrap_or_default().to_string())
        } else {
            Err(JsonError::PrintBufferedError)
        }
    }

    // generate a string representation of the JSON object using a preallocated buffer
    fn print_preallocated(&self, buffer: *mut i8, length: i32, format: bool) -> bool {
        unsafe {
            cJSON_PrintPreallocated(
                self as *const Json as *mut cJSON,
                buffer,
                length,
                if format { 1 } else { 0 },
            ) == 1
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
    fn is_invalid(&self) -> bool;
    fn is_false(&self) -> bool;
    fn is_true(&self) -> bool;
    fn is_bool(&self) -> bool;
    fn is_null(&self) -> bool;
    fn is_number(&self) -> bool;
    fn is_string(&self) -> bool;
    fn is_array(&self) -> bool;
    fn is_object(&self) -> bool;
    fn is_raw(&self) -> bool;
    fn print(&self) -> Result<String, JsonError>;
    fn print_buffered(&self, prebuffer: i32, fmt: bool) -> Result<String, JsonError>;
    fn print_preallocated(
        &self,
        buffer: *mut i8,
        length: i32,
        format: bool,
    ) -> Result<(), JsonError>;
    fn print_unformatted(&self) -> Result<String, JsonError>;
    fn delete(&self);
}

impl JsonPtrExt for *mut Json {
    /// Check whether the Json object is of type `Invalid`
    ///
    /// Returns:
    /// - `bool` - indicating whether or not the Json object is of type `Invalid`
    fn is_invalid(&self) -> bool {
        match unsafe { self.as_mut() } {
            Some(json) => json.is_invalid(),
            None => false,
        }
    }

    /// Check whether the Json object is of type `False`
    ///
    /// Returns:
    /// - `bool` - indicating whether or not the Json object is of type `False`
    fn is_false(&self) -> bool {
        match unsafe { self.as_mut() } {
            Some(json) => json.is_false(),
            None => false,
        }
    }

    /// Check whether the Json object is of type `True`
    ///
    /// Returns:
    /// - `bool` - indicating whether or not the Json object is of type `True`
    fn is_true(&self) -> bool {
        match unsafe { self.as_mut() } {
            Some(json) => json.is_true(),
            None => false,
        }
    }

    /// Check whether the Json object is of type `Bool`
    ///
    /// Returns:
    /// - `bool` - indicating whether or not the Json object is of type `Bool`
    fn is_bool(&self) -> bool {
        match unsafe { self.as_mut() } {
            Some(json) => json.is_bool(),
            None => false,
        }
    }

    /// Check whether the Json object is of type `Null`
    ///
    /// Returns:
    /// - `bool` - indicating whether or not the Json object is of type `Null`
    fn is_null(&self) -> bool {
        match unsafe { self.as_mut() } {
            Some(json) => json.is_null(),
            None => false,
        }
    }

    /// Check whether the Json object is of type `Number`
    ///
    /// Returns:
    /// - `bool` - indicating whether or not the Json object is of type `Number`
    fn is_number(&self) -> bool {
        match unsafe { self.as_mut() } {
            Some(json) => json.is_number(),
            None => false,
        }
    }

    /// Check whether the Json object is of type `String`
    ///
    /// Returns:
    /// - `bool` - indicating whether or not the Json object is of type `String`
    fn is_string(&self) -> bool {
        match unsafe { self.as_mut() } {
            Some(json) => json.is_string(),
            None => false,
        }
    }

    /// Check whether the Json object is of type `Array`
    ///
    /// Returns:
    /// - `bool` - indicating whether or not the Json object is of type `Array`
    fn is_array(&self) -> bool {
        match unsafe { self.as_mut() } {
            Some(json) => json.is_array(),
            None => false,
        }
    }

    /// Check whether the Json object is of type `Object`
    ///
    /// Returns:
    /// - `bool` - indicating whether or not the Json object is of type `Object`
    fn is_object(&self) -> bool {
        match unsafe { self.as_mut() } {
            Some(json) => json.is_object(),
            None => false,
        }
    }

    /// Check whether the Json object is of type `Raw`
    ///
    /// Returns:
    /// - `bool` - indicating whether or not the Json object is of type `Raw`
    fn is_raw(&self) -> bool {
        match unsafe { self.as_mut() } {
            Some(json) => json.is_raw(),
            None => false,
        }
    }

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

    /// Generate a string representation of the JSON object with dynamic buffer resizing.
    ///
    /// Args:
    /// - `prebuffer: i32`: Size of buffer to start with.
    /// - `fmt: bool`: Whether or not to have the output formatted/pretty-printed.
    ///
    /// Returns:
    /// - `Ok(String)` - if the buffer allocation and string generation go well.
    /// - `Err(JsonError::NullPointer)` - if the pointer is null.
    /// - `Err(JsonError::PrintBufferedError)` - if an error occurs during allocation and/or string
    /// generation.
    ///
    /// Example:
    /// ```rust
    /// use cjson_rs::*;
    ///
    /// fn main() {
    ///     let json: *mut Json = create_object();
    ///     match json.print_buffered(8, false) {
    ///         Ok(result) => assert_eq!(result, "{}"),
    ///         Err(err) => eprintln!("{}", err),
    ///     }
    /// }
    /// ```
    fn print_buffered(&self, prebuffer: i32, fmt: bool) -> Result<String, JsonError> {
        match unsafe { self.as_mut() } {
            Some(json) => json.print_buffered(prebuffer, fmt),
            None => Err(JsonError::NullPointer),
        }
    }

    /// Generate a string representation of the JSON object into a preallocated buffer.
    ///
    /// Args:
    /// - `buffer: *mut i8`: Preallocated buffer where the generated string will be stored.
    /// - `length: i32`: Number of bytes to write (preferably equal to the size of the allocated buffer).
    /// - `format: bool`: Whether or not to have the output formatted/pretty-printed.
    ///
    /// Returns:
    /// - `Ok(())` - if all goes well.
    /// - `Err(NullPointer)` - if the pointer is null.
    /// - `Err(PrintPreallocatedError)` - if an error occurs during the string generation or copying
    /// into the buffer.
    ///
    /// Example:
    /// ```rust
    /// use cjson_rs::*;
    /// use libc::malloc;
    /// use std::ffi::CStr;
    ///
    /// fn main() {
    ///     let json: *mut Json = create_object();
    ///     let buffer: *mut i8 = unsafe { malloc(8) as *mut i8 };
    ///     match json.print_preallocated(buffer, 8, false) {
    ///         Ok(_) => unsafe {
    ///             let c_str = CStr::from_ptr(buffer);
    ///            let result = c_str.to_str().unwrap_or_default().to_string();
    ///             assert_eq!(result, "{}");
    ///             println!("Test passed!");
    ///         },
    ///         Err(err) => eprintln!("{}", err),
    ///     }
    /// }
    /// ```
    fn print_preallocated(
        &self,
        buffer: *mut i8,
        length: i32,
        format: bool,
    ) -> Result<(), JsonError> {
        match unsafe { self.as_mut() } {
            Some(json) => {
                if json.print_preallocated(buffer, length, format) {
                    Ok(())
                } else {
                    Err(JsonError::PrintPreallocatedError)
                }
            }
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
pub fn cjson_create_object() -> *mut Json {
    unsafe { cJSON_CreateObject() as *mut Json }
}

/// Parse a JSON string into a Json object.
///
/// Args:
/// - `value: String`: The JSON string to be parsed. Providing an empty string will result in
/// JsonError::EmptyStringError.
///
/// Returns:
/// - `Ok(*mut Json)` - if the parsing happens successfully.
/// - `Err(JsonError::EmptyStringError)` - if the provided `value` string is empty (can't parse an
/// empty string).
/// - `Err(JsonError::CStringError(NulError))` - if the provided string contains a null byte.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let value  = "{\"name\":\"Nemuel\", \"age\":20}".to_string();
///     match parse_json(value) {
///         Ok(json) => println!("{}", json.print().unwrap()),
///         Err(err) => eprintln!("{}", err),
///     }
/// }
/// ```
///
/// Output:
/// ```json
/// {
///     "name": "Nemuel",
///     "age":  20
/// }
/// ```
pub fn cjson_parse_json(value: String) -> Result<*mut Json, JsonError> {
    if value.is_empty() {
        return Err(JsonError::EmptyStringError);
    }

    match CString::new(value) {
        Ok(c_str) => {
            let json = unsafe { cJSON_Parse(c_str.as_ptr()) };
            if json.is_null() {
                Err(JsonError::ParseError)
            } else {
                Ok(json as *mut Json)
            }
        }
        Err(err) => Err(JsonError::CStringError(err)),
    }
}

/// Parse a specific length of a JSON string into a Json object.
///
/// Args:
/// - `value: String`: The JSON string to be parsed. Providing an empty string will result in
/// JsonError::EmptyStringError.
/// - `buffer_length: usize`: Length of the JSON string to be parsed.
///
/// Returns:
/// - `Ok(*mut Json)` - if the parsing happens successfully.
/// - `Err(JsonError::EmptyStringError)` - if the provided `value` string is empty (can't parse an
/// empty string).
/// - `Err(JsonError::CStringError(NulError))` - if the provided string contains a null byte.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let value = "{\"rps\":500} more text".to_string();
///     match parse_json_with_length(value, 11) {
///         Ok(json) => println!("{}", json.print().unwrap()),
///         Err(err) => eprintln!("{}", err),
///     }
/// }
/// ```
///
/// Output:
/// ```json
/// {
///     "rps": 500
/// }
/// ```
pub fn cjson_parse_json_with_length(
    value: String,
    buffer_length: usize,
) -> Result<*mut Json, JsonError> {
    if value.is_empty() {
        return Err(JsonError::EmptyStringError);
    }

    match CString::new(value) {
        Ok(c_str) => {
            let json = unsafe { cJSON_ParseWithLength(c_str.as_ptr(), buffer_length) };
            if json.is_null() {
                Err(JsonError::ParseError)
            } else {
                Ok(json as *mut Json)
            }
        }
        Err(err) => Err(JsonError::CStringError(err)),
    }
}
