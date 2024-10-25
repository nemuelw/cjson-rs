mod bindings;
use bindings::*;
use std::ffi::{c_char, c_void, CStr, CString, NulError};

pub const CJSON_VERSION_MAJOR: u32 = bindings::CJSON_VERSION_MAJOR;
pub const CJSON_VERSION_MINOR: u32 = bindings::CJSON_VERSION_MINOR;
pub const CJSON_VERSION_PATCH: u32 = bindings::CJSON_VERSION_PATCH;

/// Get the version of the underlying cJSON library.
///
/// Example:
/// ```rust
/// use cjson_rs::cjson_version;
///
/// fn main() {
///     assert_eq!(cjson_version(), "1.7.18");
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_version() -> String {
    let cjson_version = unsafe { cJSON_Version() };
    unsafe {
        CStr::from_ptr(cjson_version)
            .to_str()
            .unwrap_or_default()
            .to_string()
    }
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
/// - `valueint`: Writing to this is deprecated. Use the `cjson_set_number_helper` method instead.
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
    InvalidTypeError(String),
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
            JsonError::InvalidTypeError(err) => write!(f, "InvalidType error: {}", err),
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
    fn is_type_invalid(&self) -> bool {
        unsafe { cJSON_IsInvalid(self as *const Json as *const cJSON) == 1 }
    }

    // check whether the Json object is of type False
    fn is_type_false(&self) -> bool {
        unsafe { cJSON_IsFalse(self as *const Json as *const cJSON) == 1 }
    }

    // check whether the Json object is of type True
    fn is_type_true(&self) -> bool {
        unsafe { cJSON_IsTrue(self as *const Json as *const cJSON) == 1 }
    }

    // check whether the Json object is of type Bool
    fn is_type_bool(&self) -> bool {
        unsafe { cJSON_IsBool(self as *const Json as *const cJSON) == 1 }
    }

    // check whether the Json object is of type Null
    fn is_type_null(&self) -> bool {
        unsafe { cJSON_IsNull(self as *const Json as *const cJSON) == 1 }
    }

    // check whether the Json object is of type Number
    fn is_type_number(&self) -> bool {
        unsafe { cJSON_IsNumber(self as *const Json as *const cJSON) == 1 }
    }

    // check whether the Json object is of type String
    fn is_type_string(&self) -> bool {
        unsafe { cJSON_IsString(self as *const Json as *const cJSON) == 1 }
    }

    // check whether the Json object is of type Array
    fn is_type_array(&self) -> bool {
        unsafe { cJSON_IsArray(self as *const Json as *const cJSON) == 1 }
    }

    // check whether the Json object is of type Object
    fn is_type_object(&self) -> bool {
        unsafe { cJSON_IsObject(self as *const Json as *const cJSON) == 1 }
    }

    // check whether the Json object is of type Raw
    fn is_type_raw(&self) -> bool {
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
    fn is_type_invalid(&self) -> bool;
    fn is_type_false(&self) -> bool;
    fn is_type_true(&self) -> bool;
    fn is_type_bool(&self) -> bool;
    fn is_type_null(&self) -> bool;
    fn is_type_number(&self) -> bool;
    fn is_type_string(&self) -> bool;
    fn is_type_array(&self) -> bool;
    fn is_type_object(&self) -> bool;
    fn is_type_raw(&self) -> bool;
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
    /// Check whether the Json object is of type `Invalid`.
    ///
    /// Returns:
    /// - `bool` - indicating whether or not the Json object is of type `Invalid`.
    fn is_type_invalid(&self) -> bool {
        match unsafe { self.as_mut() } {
            Some(json) => json.is_type_invalid(),
            None => false,
        }
    }

    /// Check whether the Json object is of type `False`.
    ///
    /// Returns:
    /// - `bool` - indicating whether or not the Json object is of type `False`.
    fn is_type_false(&self) -> bool {
        match unsafe { self.as_mut() } {
            Some(json) => json.is_type_false(),
            None => false,
        }
    }

    /// Check whether the Json object is of type `True`.
    ///
    /// Returns:
    /// - `bool` - indicating whether or not the Json object is of type `True`.
    fn is_type_true(&self) -> bool {
        match unsafe { self.as_mut() } {
            Some(json) => json.is_type_true(),
            None => false,
        }
    }

    /// Check whether the Json object is of type `Bool`.
    ///
    /// Returns:
    /// - `bool` - indicating whether or not the Json object is of type `Bool`.
    fn is_type_bool(&self) -> bool {
        match unsafe { self.as_mut() } {
            Some(json) => json.is_type_bool(),
            None => false,
        }
    }

    /// Check whether the Json object is of type `Null`.
    ///
    /// Returns:
    /// - `bool` - indicating whether or not the Json object is of type `Null`.
    fn is_type_null(&self) -> bool {
        match unsafe { self.as_mut() } {
            Some(json) => json.is_type_null(),
            None => false,
        }
    }

    /// Check whether the Json object is of type `Number`.
    ///
    /// Returns:
    /// - `bool` - indicating whether or not the Json object is of type `Number`.
    fn is_type_number(&self) -> bool {
        match unsafe { self.as_mut() } {
            Some(json) => json.is_type_number(),
            None => false,
        }
    }

    /// Check whether the Json object is of type `String`.
    ///
    /// Returns:
    /// - `bool` - indicating whether or not the Json object is of type `String`.
    fn is_type_string(&self) -> bool {
        match unsafe { self.as_mut() } {
            Some(json) => json.is_type_string(),
            None => false,
        }
    }

    /// Check whether the Json object is of type `Array`.
    ///
    /// Returns:
    /// - `bool` - indicating whether or not the Json object is of type `Array`.
    fn is_type_array(&self) -> bool {
        match unsafe { self.as_mut() } {
            Some(json) => json.is_type_array(),
            None => false,
        }
    }

    /// Check whether the Json object is of type `Object`.
    ///
    /// Returns:
    /// - `bool` - indicating whether or not the Json object is of type `Object`.
    fn is_type_object(&self) -> bool {
        match unsafe { self.as_mut() } {
            Some(json) => json.is_type_object(),
            None => false,
        }
    }

    /// Check whether the Json object is of type `Raw`.
    ///
    /// Returns:
    /// - `bool` - indicating whether or not the Json object is of type `Raw`.
    fn is_type_raw(&self) -> bool {
        match unsafe { self.as_mut() } {
            Some(json) => json.is_type_raw(),
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
    ///     let json: *mut Json = cjson_create_object();
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
    ///     let json: *mut Json = cjson_create_object();
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
    ///     let json: *mut Json = cjson_create_object();
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
    ///     let json: *mut Json = cjson_create_object();
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
    ///     let json: *mut Json = cjson_create_object();
    ///     json.delete();
    /// }
    /// ```
    fn delete(&self) {
        unsafe { self.as_mut().map(|json| json.delete()) };
    }
}

/// Remove all unnecessary whitespace and formatting from a JSON string.
///
/// Args:
/// - `json: String` - The JSON string to be minified.
///
/// Returns:
/// - `Ok(())` - if the operation gets performed.
/// - `Err(JsonError::CStringError(NulError))` - if the provided string contains a null byte.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let mut json_str: String = "{\n\t\"name\": \"Nemuel\",\n\t\"age\": 20\n}".to_string();
///     cjson_minify(&mut json_str).unwrap();
///     assert_eq!(json_str, r#"{"name":"Nemuel","age":20}"#);
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_minify(json: &mut String) -> Result<(), JsonError> {
    match CString::new((*json).as_bytes()) {
        Ok(c_str) => {
            let c_str_mut_ptr = c_str.as_ptr() as *mut i8;
            unsafe { cJSON_Minify(c_str_mut_ptr) };
            let minified = unsafe { CStr::from_ptr(c_str_mut_ptr) };
            *json = minified.to_string_lossy().into_owned();
            Ok(())
        }
        Err(err) => Err(JsonError::CStringError(err)),
    }
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

/// Get error message associated with the last parsing operation that failed.
///
/// Returns:
/// - `Some(String)` - if an error message exists.
/// - `None` - if there is no error message.
pub fn cjson_get_error_ptr() -> Option<String> {
    let c_str = unsafe { cJSON_GetErrorPtr() };
    if !c_str.is_null() {
        let c_str_ref = unsafe { CStr::from_ptr(c_str) };
        Some(c_str_ref.to_str().unwrap_or_default().to_string())
    } else {
        None
    }
}

/// Create Json item of type `Raw`.
///
/// Args:
/// - `raw: String` - Raw string (JSON or otherwise).
///
/// Returns:
/// - `Ok(*mut Json)` - a mutable pointer to the created Json item of type `Raw`.
/// - `Err(JsonError::CStringError(NulError))` - if the provided string contains a null byte.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let json = cjson_create_raw("\"count\": 5".to_string()).unwrap();
///     println!("{}", json.print().unwrap()); // output: "count": 5
/// }
/// ```
pub fn cjson_create_raw(raw: String) -> Result<*mut Json, JsonError> {
    match CString::new(raw) {
        Ok(c_str) => {
            let json = unsafe { cJSON_CreateRaw(c_str.as_ptr()) as *mut Json };
            Ok(json)
        }
        Err(err) => Err(JsonError::CStringError(err)),
    }
}

/// Create Json item of type `Null`.
///
/// Returns:
/// - `*mut Json` - a mutable pointer to the created Json item of type `Null`.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let json = cjson_create_null();
///     assert_eq!(json.is_type_null(), true);
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_create_null() -> *mut Json {
    unsafe { cJSON_CreateNull() as *mut Json }
}

/// Create a new JSON object (instance of the Json struct).
///
/// Example:
/// ```rust
/// use cjson_rs::{cjson_create_object, Json};
///
/// fn main() {
///     let json: *mut Json = cjson_cjson_create_object();
/// }
/// ```
pub fn cjson_create_object() -> *mut Json {
    unsafe { cJSON_CreateObject() as *mut Json }
}

/// Create Json item of type `True`.
///
/// Returns:
/// - `*mut Json` - a mutable pointer to the created Json item of type `True`.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let json = cjson_create_true();
///     assert_eq!(json.is_type_true(), true);
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_create_true() -> *mut Json {
    unsafe { cJSON_CreateTrue() as *mut Json }
}

/// Create Json item of type `False`.
///
/// Returns:
/// - `*mut Json` - a mutable pointer to the created Json item of type `False`.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let json = cjson_create_false();
///     assert_eq!(json.is_type_false(), true);
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_create_false() -> *mut Json {
    unsafe { cJSON_CreateFalse() as *mut Json }
}

/// Create Json item of type `Bool`.
///
/// Args:
/// - `boolean: bool`: Boolean value for the Json item to create (true or false).
///
/// Returns:
/// - `*mut Json` - a mutable pointer to the created Json item of type `Bool`.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let json = cjson_create_bool(true);
///     assert_eq!(json.is_type_bool(), true);
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_create_bool(boolean: bool) -> *mut Json {
    unsafe { cJSON_CreateBool(if boolean { 1 } else { 0 }) as *mut Json }
}

/// Create Json item of type `Number`.
///
/// Args:
/// - `num: f64`: Numeric value for the Json item to create.
///
/// Returns:
/// - `*mut Json` - a mutable pointer to the created Json item of type `Number`.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let json = cjson_create_number(254.0);
///     assert_eq!(json.is_type_number(), true);
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_create_number(num: f64) -> *mut Json {
    unsafe { cJSON_CreateNumber(num) as *mut Json }
}

/// Set the number value for a Json item of type `Number` to the specified value.
///
/// Args:
/// - `item: *mut Json` - Mutable pointer to Json item of type `Number` whose number value is to be set.
/// - `number: f64` - The number value to set for the Json item of type `Number`.
///
/// Returns:
/// - `Ok(f64)` - if the operation happens successfully.
/// - `Err(JsonError::InvalidTypeError(String))` - if the provided Json item is not of type `Number`.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let number_item = cjson_create_number(1.0);
///     assert_eq!(cjson_get_number_value(number_item).unwrap(), 1.0);
///
///     let new_number_value = cjson_set_number_helper(number_item, 2.0).unwrap();
///     assert_eq!(new_number_value, 2.0);
///
///     assert_eq!(cjson_get_number_value(number_item).unwrap(), 2.0);
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_set_number_helper(object: *mut Json, number: f64) -> Result<f64, JsonError> {
    if !object.is_type_number() {
        Err(JsonError::InvalidTypeError(
            "cannot set number value for a non-number Json item".to_string(),
        ))
    } else {
        Ok(unsafe { cJSON_SetNumberHelper(object as *mut cJSON, number) })
    }
}

/// Create Json item of type `String` (copies the string).
///
/// Args:
/// - `string: String`: String value for the Json item to create.
///
/// Returns:
/// - `*mut Json` - a mutable pointer to the created Json item of type `String`.
/// - `Err(JsonError::CStringError(NulError))` - if the provided string contains a null byte.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let json = cjson_create_string("Nemuel".to_string()).unwrap();
///     assert_eq!(json.is_type_string(), true);
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_create_string(string: String) -> Result<*mut Json, JsonError> {
    match CString::new(string) {
        Ok(c_str) => {
            let json = unsafe { cJSON_CreateString(c_str.as_ptr()) as *mut Json };
            Ok(json)
        }
        Err(err) => Err(JsonError::CStringError(err)),
    }
}

/// Set the string value of a Json item of type `String` to the specified value.
///
/// Args:
/// - `item: *mut Json` - Mutable pointer to Json item of type `String` whose string value is to be set.
/// - `valuestring: &str` - String slice specifying the value to set for the Json item of type `String`.
///
/// Returns:
/// - `Ok(String)` - if the operation happens successfully.
/// - `Err(JsonError::InvalidTypeError(String))` - if the provided Json item is not of type `String`.
/// - `Err(JsonError::CStringError(NulError))` - if the provided string contains a null byte.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let string_item = cjson_create_string("Nemuel".to_string()).unwrap();
///     assert_eq!(cjson_get_string_value(string_item).unwrap(), "Nemuel");
///
///     let new_string_value = cjson_set_value_string(string_item, "Wainaina").unwrap();
///     assert_eq!(new_string_value, "Wainaina");
///
///     assert_eq!(cjson_get_string_value(string_item).unwrap(), "Wainaina");
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_set_value_string(object: *mut Json, valuestring: &str) -> Result<String, JsonError> {
    if !object.is_type_string() {
        return Err(JsonError::InvalidTypeError(
            "cannot set string value for a non-string Json item".to_string(),
        ));
    }

    match CString::new(valuestring) {
        Ok(c_str) => {
            let c_str_ptr = unsafe { cJSON_SetValuestring(object as *mut cJSON, c_str.as_ptr()) };
            let str = unsafe { CStr::from_ptr(c_str_ptr).to_string_lossy().into_owned() };
            Ok(str)
        }
        Err(err) => Err(JsonError::CStringError(err)),
    }
}

/// Create Json item of type `Array`.
///
/// Returns:
/// - `*mut Json` - a mutable pointer to the created Json item of type `Array`.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let json = cjson_create_array();
///     assert_eq!(json.is_type_array(), true);
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_create_array() -> *mut Json {
    unsafe { cJSON_CreateArray() as *mut Json }
}

/// Create Json item of type `String`.
///
/// It points directly to the string. This means the `valuestring` field of the Json struct will
/// not be deleted by `cjson_delete`, and you are therefore responsible for its lifetime (useful
/// for constants).
///
/// Args:
/// - `string: String`: String value for the Json item to create.
///
/// Returns:
/// - `*mut Json` - a mutable pointer to the created Json item of type `String`.
/// - `Err(JsonError::CStringError(NulError))` - if the provided string contains a null byte.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let json = cjson_create_string_reference("Nemuel".to_string()).unwrap();
///     assert_eq!(json.is_type_string(), true);
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_create_string_reference(string: String) -> Result<*mut Json, JsonError> {
    match CString::new(string) {
        Ok(c_str) => {
            let json = unsafe { cJSON_CreateStringReference(c_str.as_ptr()) as *mut Json };
            Ok(json)
        }
        Err(err) => Err(JsonError::CStringError(err)),
    }
}

/// Create Json item of type `Array` that doesn't "own" its content.
///
/// Args:
/// - `child: *mut Json`: Json item of type `Array` to create a reference to.
///
/// Returns:
/// - `*mut Json` - a reference to the provided Json item of type `Array`.
/// - `Err(JsonError::InvalidTypeError(String))` - if the provided Json item is not of type `Array`.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let child = cjson_create_array();
///     let reference = cjson_create_array_reference(child).unwrap();
///     assert_eq!(reference.is_type_array(), true);
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_create_array_reference(child: *mut Json) -> Result<*mut Json, JsonError> {
    if !child.is_type_array() {
        Err(JsonError::InvalidTypeError(
            "cannot create array reference to a non-array Json item".to_string(),
        ))
    } else {
        let reference = unsafe { cJSON_CreateArrayReference(child as *mut cJSON) as *mut Json };
        Ok(reference)
    }
}

/// Create Json item of type `Array` that doesn't "own" its content.
///
/// Args:
/// - `child: *mut Json`: Json item of type `Array` to create a reference to.
///
/// Returns:
/// - `*mut Json` - a reference to the provided Json item of type `Array`.
/// - `Err(JsonError::InvalidTypeError(String))` - if the provided Json item is not of type `Array`.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let child = cjson_create_object();
///     let reference = cjson_create_object_reference(child).unwrap();
///     assert_eq!(reference.is_type_object(), true);
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_create_object_reference(child: *mut Json) -> Result<*mut Json, JsonError> {
    if !child.is_type_object() {
        Err(JsonError::InvalidTypeError(
            "cannot create object reference to a non-object Json item".to_string(),
        ))
    } else {
        let reference = unsafe { cJSON_CreateObjectReference(child as *mut cJSON) as *mut Json };
        Ok(reference)
    }
}

/// Create Json item of type `Array` containing integers.
///
/// Args:
/// - `numbers: *const i32` - Pointer to a signed 32-bit integer (start of the numbers array).
/// - `count: i32` - Number of array elements to include in the `Array` being created (typically just the
/// size of the `numbers` array).
///
/// Returns:
/// - `*mut Json` - a mutable pointer to the created Json item of type `Array` containing integers.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let numbers: [i32; 5] = [1, 2, 3, 4, 5];
///     let json = cjson_create_int_array(&numbers[0], numbers.len() as i32);
///     assert_eq!(json.is_type_array(), true);
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_create_int_array(numbers: *const i32, count: i32) -> *mut Json {
    unsafe { cJSON_CreateIntArray(numbers, count) as *mut Json }
}

/// Create Json item of type `Array` containing single-precision floating-point values.
///
/// Args:
/// - `numbers: *const i32` - Pointer to a signed single-precision floating-point value (start of the
/// numbers array).
/// - `count: i32` - Number of array elements to include in the `Array` being created (typically just the
/// size of the `numbers` array).
///
/// Returns:
/// - `*mut Json` - a mutable pointer to the created Json item of type `Array` containing single-precision
/// floating-point values.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let numbers: [f32; 5] = [1.0, 2.0, 3.0, 4.0, 5.0];
///     let json = cjson_create_float_array(&numbers[0], numbers.len() as i32);
///     assert_eq!(json.is_type_array(), true);
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_create_float_array(numbers: *const f32, count: i32) -> *mut Json {
    unsafe { cJSON_CreateFloatArray(numbers, count) as *mut Json }
}

/// Create Json item of type `Array` containing double-precision floating-point values.
///
/// Args:
/// - `numbers: *const i64` - Pointer to a signed double-precision floating-point value (start of the
/// numbers array).
/// - `count: i32` - Number of array elements to include in the `Array` being created (typically just the
/// size of the `numbers` array).
///
/// Returns:
/// - `*mut Json` - a mutable pointer to the created Json item of type `Array` containing double-precision
/// floating-point values.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let numbers: [f64; 5] = [1.0, 2.0, 3.0, 4.0, 5.0];
///     let json = cjson_create_double_array(&numbers[0], numbers.len() as i32);
///     assert_eq!(json.is_type_array(), true);
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_create_double_array(numbers: *const f64, count: i32) -> *mut Json {
    unsafe { cJSON_CreateDoubleArray(numbers, count) as *mut Json }
}

/// Create Json item of type `Array` containing string values.
///
/// Args:
/// - `strings: &[&str]` - Reference to an array of string slices.
/// - `count: i32` - Number of array elements to include in the `Array` being created (typically just the
/// size of the `strings` array).
///
/// Returns:
/// - `*mut Json` - a mutable pointer to the created Json item of type `Array` containing string values.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let strings = ["Alice", "Bob", "Chloe"];
///     let arr = cjson_create_string_array(&strings, strings.len() as i32).unwrap();
///     assert_eq!(arr.is_type_array(), true);
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_create_string_array(strings: &[&str], count: i32) -> Result<*mut Json, JsonError> {
    let mut c_strings: Vec<CString> = Vec::with_capacity(strings.len());

    for &s in strings {
        match CString::new(s) {
            Ok(c_str) => c_strings.push(c_str),
            Err(err) => return Err(JsonError::CStringError(err)),
        }
    }

    let pointers: Vec<*const c_void> = c_strings
        .iter()
        .map(|s| s.as_ptr() as *const c_void)
        .collect();

    let array = unsafe {
        cJSON_CreateStringArray(pointers.as_ptr() as *const *const c_char, count) as *mut Json
    };
    Ok(array)
}

/// Get the size of Json item of type `Array`.
///
/// Args:
/// - `array: *mut Json` - The Json item of type `Array` whose size we want.
///
/// Returns:
/// - `Ok(i32)` - if the size of the Json item of type `Array` is successfully determined.
/// - `Err(JsonError::InvalidTypeError(String))` - if the `array` value provided is not of type `Array`.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let strings = ["Alice", "Bob", "Chloe", "Dan", "Eyal"];
///     let arr = cjson_create_string_array(&strings, strings.len() as i32).unwrap();
///     assert_eq!(cjson_get_array_size(arr).unwrap(), 5);
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_get_array_size(array: *mut Json) -> Result<i32, JsonError> {
    if !array.is_type_array() {
        Err(JsonError::InvalidTypeError(
            "cannot get array size for a non-array Json item".to_string(),
        ))
    } else {
        Ok(unsafe { cJSON_GetArraySize(array as *const cJSON) })
    }
}

/// Get the item at the provided index of a Json item of type `Array`.
///
/// Args:
/// - `array: *mut Json` - The Json item of type `Array` from which we want to get an item.
/// - `index: i32` - Index of the item we want to get from the Json item of type `Array`.
///
/// Returns:
/// - `Ok(*mut Json)` - mutable pointer to the item at the specified index.
/// - `Err(JsonError::InvalidTypeError(String))` - if the `array` value provided is not of type `Array`.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let strings = ["Alice", "Bob", "Chloe", "Dan", "Eyal"];
///     let arr = cjson_create_string_array(&strings, strings.len() as i32).unwrap();
///     match cjson_get_array_item(arr, 2) {
///         Ok(item) => {
///             println!("{}", item.print().unwrap()); // output: "Chloe"
///         }
///         Err(err) => eprintln!("{}", err),
///     }
/// }
/// ```
pub fn cjson_get_array_item(array: *mut Json, index: i32) -> Result<*mut Json, JsonError> {
    if !array.is_type_array() {
        Err(JsonError::InvalidTypeError(
            "cannot get array item from a non-array Json item".to_string(),
        ))
    } else {
        Ok(unsafe { cJSON_GetArrayItem(array as *const cJSON, index) as *mut Json })
    }
}

/// Add an item to Json item of type `Array`.
///
/// Args:
/// - `array: *mut Json` - The Json item of type `Array` where the item will be added.
/// - `item: *mut Json` - The item to add to the Json item of type `Array`.
///
/// Returns:
/// - `Ok(bool)` - indicating success or failure in adding the item to the Json item of type `Array`.
/// - `Err(JsonError::InvalidTypeError(String))` - if the `array` value provided is not of type `Array`.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let numbers = [1, 2, 3, 4];
///     let arr = cjson_create_int_array(&numbers[0], 4);
///     let item = cjson_create_number(5.0);
///     let success = cjson_add_item_to_array(arr, item).unwrap();
///     assert_eq!(success, true);
///     assert_eq!(cjson_get_array_size(arr).unwrap(), 5);
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_add_item_to_array(array: *mut Json, item: *mut Json) -> Result<bool, JsonError> {
    if !array.is_type_array() {
        Err(JsonError::InvalidTypeError(
            "cannot add item to a non-array Json item".to_string(),
        ))
    } else {
        let result = unsafe { cJSON_AddItemToArray(array as *mut cJSON, item as *mut cJSON) };
        if result == 1 {
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

/// Add an item to Json item of type `Array` while maintaining a reference to the original item rather
/// than copying it.
///
/// Args:
/// - `array: *mut Json` - The Json item of type `Array` where the item will be added.
/// - `item: *mut Json` - The item to add to the Json item of type `Array`.
///
/// Returns:
/// - `Ok(bool)` - boolean value indicating success or failure of the operation.
/// - `Err(JsonError::InvalidTypeError(String))` - if the `array` value provided is not of type `Array`.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let numbers = [1, 2, 3, 4];
///     let arr = cjson_create_int_array(&numbers[0], 4);
///     let item = cjson_create_number(5.0);
///     let success = cjson_add_item_reference_to_array(arr, item).unwrap();
///     assert_eq!(success, true);
///     assert_eq!(cjson_get_array_size(arr).unwrap(), 5);
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_add_item_reference_to_array(
    array: *mut Json,
    item: *mut Json,
) -> Result<bool, JsonError> {
    if !array.is_type_array() {
        Err(JsonError::InvalidTypeError(
            "cannot add item to a non-array Json item".to_string(),
        ))
    } else {
        let result =
            unsafe { cJSON_AddItemReferenceToArray(array as *mut cJSON, item as *mut cJSON) };
        if result == 1 {
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

/// Insert an item at a specific index in a Json item of type `Array`.
///
/// Args:
/// - `array: *mut Json` - The Json item of type `Array` where the item will be added.
/// - `which: i32` - Index specifying where to insert the new item in the array.
/// - `newitem: *mut Json` - The item to add to the Json item of type `Array`.
///
/// Returns:
/// - `Ok(bool)` - indicating success or failure of the operation.
/// - `Err(JsonError::InvalidTypeError(String))` - if the `array` value provided is not of type `Array`.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let numbers = [1, 3, 4, 5];
///     let arr = cjson_create_int_array(&numbers[0], 4);
///     let item = cjson_create_number(2.0);
///     let success = cjson_insert_item_in_array(arr, 1, item).unwrap();
///     assert_eq!(success, true);
///     let new_item = cjson_get_array_item(arr, 1).unwrap();
///     assert_eq!(cjson_get_number_value(new_item).unwrap(), 2.0);
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_insert_item_in_array(
    array: *mut Json,
    which: i32,
    newitem: *mut Json,
) -> Result<bool, JsonError> {
    if !array.is_type_array() {
        Err(JsonError::InvalidTypeError(
            "cannot insert item in a non-array Json item".to_string(),
        ))
    } else {
        let result =
            unsafe { cJSON_InsertItemInArray(array as *mut cJSON, which, newitem as *mut cJSON) };
        if result == 1 {
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

/// Replace item at a specific index in Json item of type `Array`.
///
/// Args:
/// - `array: *mut Json` - The Json item of type `Array` in which the replacement will happen.
/// - `which: i32` - Index specifying the item to be replaced.
/// - `newitem: *mut Json` - The item replacing the one at the specified index.
///
/// Returns:
/// - `Ok(bool)` - indicating success or faanilure of the operation.
/// - `Err(JsonError::InvalidTypeError(String))` - if the `array` value provided is not of type `Array`.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let names = ["Alice", "Bob", "Chloe", "Dan"];
///     let array = cjson_create_string_array(&names, 4).unwrap();
///
///     let original_item = cjson_get_array_item(array, 3).unwrap();
///     assert_eq!(
///         cjson_get_string_value(original_item).unwrap(),
///         "Dan"
///     );
///
///     let newitem = cjson_create_string("Diana".to_string()).unwrap();
///     let success = cjson_replace_item_in_array(array, 3, newitem).unwrap();
///     assert_eq!(success, true);
///
///     let new_item = cjson_get_array_item(array, 3).unwrap();
///     assert_eq!(
///         cjson_get_string_value(new_item).unwrap(),
///         "Diana"
///     );
///
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_replace_item_in_array(
    array: *mut Json,
    which: i32,
    newitem: *mut Json,
) -> Result<bool, JsonError> {
    if !array.is_type_array() {
        Err(JsonError::InvalidTypeError(
            "cannot replace item in a non-array Json item".to_string(),
        ))
    } else {
        let result =
            unsafe { cJSON_ReplaceItemInArray(array as *mut cJSON, which, newitem as *mut cJSON) };
        if result == 1 {
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

/// Detach item at a specific index from Json item of type `Array`.
///
/// Args:
/// - `array: *mut Json` - The Json item of type `Array` from which an item is to be detached.
/// - `which: i32` - The index of the item to be detached from the array.
///
/// Returns:
/// - `Ok(*mut Json)` - a mutable pointer to the detached item if the operation is successful.
/// - `Err(JsonError::InvalidTypeError(String))` - if the `array` value provided is not of type `Array`.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let names = ["Alice", "Bob", "Chloe"];
///     let array = cjson_create_string_array(&names, 3).unwrap();
///     assert_eq!(cjson_get_array_size(array).unwrap(), 3);
///
///     let detached_item = cjson_detach_item_from_array(array, 2).unwrap();
///
///     assert_eq!(cjson_get_string_value(detached_item).unwrap(), "Chloe");
///     assert_eq!(cjson_get_array_size(array).unwrap(), 2);
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_detach_item_from_array(array: *mut Json, which: i32) -> Result<*mut Json, JsonError> {
    if !array.is_type_array() {
        Err(JsonError::InvalidTypeError(
            "cannot detach item from a non-array Json item".to_string(),
        ))
    } else {
        Ok(unsafe { cJSON_DetachItemFromArray(array as *mut cJSON, which) as *mut Json })
    }
}

/// Delete item at a specific index from Json item of type `Array`.
///
/// Args:
/// - `array: *mut Json` - The Json item of type `Array` from which an item is to be deleted.
/// - `which: i32` - The index of the item to be deleted from the array.
///
/// Returns:
/// - `Ok()` - if the operation is successful.
/// - `Err(JsonError::InvalidTypeError(String))` - if the `array` value provided is not of type `Array`.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let names = ["Alice", "Bob", "Chloe"];
///     let array = cjson_create_string_array(&names, 3).unwrap();
///     assert_eq!(cjson_get_array_size(array).unwrap(), 3);
///
///     cjson_delete_item_from_array(array, 2).unwrap();
///
///     assert_eq!(cjson_get_array_size(array).unwrap(), 2);
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_delete_item_from_array(array: *mut Json, which: i32) -> Result<(), JsonError> {
    if !array.is_type_array() {
        Err(JsonError::InvalidTypeError(
            "cannot delete item from a non-array Json item".to_string(),
        ))
    } else {
        unsafe { cJSON_DeleteItemFromArray(array as *mut cJSON, which) };
        Ok(())
    }
}

/// Get the string value of a Json item of type `String`.
///
/// Args:
/// - `item: *mut Json` - Mutable pointer to the Json item of type `String` whose string value we
/// want to get.
///
/// Returns:
/// - `Ok(String)` - if the string value is successfully gotten.
/// - `Err(JsonError::InvalidTypeError(String))` - if the Json item provided is not of type `String`.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let json = cjson_create_string("Nemuel".to_string()).unwrap();
///     assert_eq!(cjson_get_string_value(json).unwrap(), "Nemuel".to_string());
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_get_string_value(item: *mut Json) -> Result<String, JsonError> {
    if !item.is_type_string() {
        return Err(JsonError::InvalidTypeError(
            "cannot get string value from a non-string Json item".to_string(),
        ));
    }

    let c_str = unsafe { cJSON_GetStringValue(item as *mut cJSON) };
    Ok(unsafe {
        CStr::from_ptr(c_str)
            .to_str()
            .unwrap_or_default()
            .to_string()
    })
}

/// Get the number value of a Json item of type `Number`.
///
/// Args:
/// - `item: *mut Json` - Mutable pointer to the Json item of type `Number` whose number value we
/// want to get.
///
/// Returns:
/// - `Ok(i32)` - if the number value is successfully gotten.
/// - `Err(JsonError::InvalidTypeError(String))` - if the Json item provided is not of type `Number`.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let json = cjson_create_number(254.0);
///     assert_eq!(cjson_get_number_value(json).unwrap(), 254.0);
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_get_number_value(item: *mut Json) -> Result<f64, JsonError> {
    if !item.is_type_number() {
        Err(JsonError::InvalidTypeError(
            "cannot get number value from a non-number Json item".to_string(),
        ))
    } else {
        Ok(unsafe { cJSON_GetNumberValue(item as *const cJSON) })
    }
}

/// Add Json item of type `Null` to Json item of type `Object`.
///
/// Args:
/// - `object: *mut Json` - Json item of type `Object` to add the Json item of type `Null` to.
/// - `name: &str` - Key to set for the item being added.
///
/// Returns:
/// - `Ok(*mut Json)` - a mutable pointer to the Json item of type `Null` that has been added.
/// - `Err(JsonError::InvalidTypeError(String))` - if the Json item provided is not of type `Object`.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let object = cjson_create_object();
///     cjson_add_null_to_object(object, "test").unwrap();
///     let test_null_item = cjson_get_object_item(object, "test").unwrap();
///     assert_eq!(test_null_item.is_type_null(), true);
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_add_null_to_object(object: *mut Json, name: &str) -> Result<*mut Json, JsonError> {
    if !object.is_type_object() {
        return Err(JsonError::InvalidTypeError(
            "cannot add item to a non-object Json item".to_string(),
        ));
    }

    match CString::new(name) {
        Ok(c_str) => {
            let result =
                unsafe { cJSON_AddNullToObject(object as *mut cJSON, c_str.as_ptr()) as *mut Json };
            Ok(result)
        }
        Err(err) => Err(JsonError::CStringError(err)),
    }
}

/// Add Json item of type `True` to Json item of type `Object`.
///
/// Args:
/// - `object: *mut Json` - Json item of type `Object` to add the Json item of type `True` to.
/// - `name: &str` - Key to set for the item being added.
///
/// Returns:
/// - `Ok(*mut Json)` - a mutable pointer to the Json item of type `True` that has been added.
/// - `Err(JsonError::InvalidTypeError(String))` - if the Json item provided is not of type `Object`.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let object = cjson_create_object();
///     cjson_add_true_to_object(object, "test").unwrap();
///     let test_true_item = cjson_get_object_item(object, "test").unwrap();
///     assert_eq!(test_true_item.is_type_true(), true);
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_add_true_to_object(object: *mut Json, name: &str) -> Result<*mut Json, JsonError> {
    if !object.is_type_object() {
        return Err(JsonError::InvalidTypeError(
            "cannot add item to a non-object Json item".to_string(),
        ));
    }

    match CString::new(name) {
        Ok(c_str) => {
            let result =
                unsafe { cJSON_AddTrueToObject(object as *mut cJSON, c_str.as_ptr()) as *mut Json };
            Ok(result)
        }
        Err(err) => Err(JsonError::CStringError(err)),
    }
}

/// Add Json item of type `False` to Json item of type `Object`.
///
/// Args:
/// - `object: *mut Json` - Json item of type `Object` to add the Json item of type `False` to.
/// - `name: &str` - Key to set for the item being added.
///
/// Returns:
/// - `Ok(*mut Json)` - a mutable pointer to the Json item of type `False` that has been added.
/// - `Err(JsonError::InvalidTypeError(String))` - if the Json item provided is not of type `Object`.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let object = cjson_create_object();
///     cjson_add_false_to_object(object, "test").unwrap();
///     let test_false_item = cjson_get_object_item(object, "test").unwrap();
///     assert_eq!(test_false_item.is_type_false(), true);
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_add_false_to_object(object: *mut Json, name: &str) -> Result<*mut Json, JsonError> {
    if !object.is_type_object() {
        return Err(JsonError::InvalidTypeError(
            "cannot add item to a non-object Json item".to_string(),
        ));
    }

    match CString::new(name) {
        Ok(c_str) => {
            let result = unsafe {
                cJSON_AddFalseToObject(object as *mut cJSON, c_str.as_ptr()) as *mut Json
            };
            Ok(result)
        }
        Err(err) => Err(JsonError::CStringError(err)),
    }
}

/// Add Json item of type `Bool` to Json item of type `Object`.
///
/// Args:
/// - `object: *mut Json` - Json item of type `Object` to add the Json item of type `Bool` to.
/// - `name: &str` - Key to set for the item being added.
/// - `boolean: bool` - Boolean value for the Json item being added (true or false).
///
/// Returns:
/// - `Ok(*mut Json)` - a mutable pointer to the Json item of type `Bool` that has been added.
/// - `Err(JsonError::InvalidTypeError(String))` - if the Json item provided is not of type `Object`.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let object = cjson_create_object();
///     cjson_add_bool_to_object(object, "test", true).unwrap();
///     let test_bool_item = cjson_get_object_item(object, "test").unwrap();
///     assert_eq!(test_bool_item.is_type_bool(), true);
///     assert_eq!(test_bool_item.is_type_true(), true);
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_add_bool_to_object(
    object: *mut Json,
    name: &str,
    boolean: bool,
) -> Result<*mut Json, JsonError> {
    if !object.is_type_object() {
        return Err(JsonError::InvalidTypeError(
            "cannot add item to a non-object Json item".to_string(),
        ));
    }

    match CString::new(name) {
        Ok(c_str) => {
            let result = unsafe {
                cJSON_AddBoolToObject(
                    object as *mut cJSON,
                    c_str.as_ptr(),
                    if boolean { 1 } else { 0 },
                ) as *mut Json
            };
            Ok(result)
        }
        Err(err) => Err(JsonError::CStringError(err)),
    }
}

/// Add Json item of type `Number` to Json item of type `Object`.
///
/// Args:
/// - `object: *mut Json` - Json item of type `Object` to add the Json item of type `Number` to.
/// - `name: &str` - Key to set for the item being added.
/// - `number: f64` - Number value for the Json item being added.
///
/// Returns:
/// - `Ok(*mut Json)` - a mutable pointer to the Json item of type `Number` that has been added.
/// - `Err(JsonError::InvalidTypeError(String))` - if the Json item provided is not of type `Object`.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let object = cjson_create_object();
///     cjson_add_number_to_object(object, "kenya", 254.0).unwrap();
///     let test_number_item = cjson_get_object_item(object, "kenya").unwrap();
///     assert_eq!(test_number_item.is_type_number(), true);
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_add_number_to_object(
    object: *mut Json,
    name: &str,
    number: f64,
) -> Result<*mut Json, JsonError> {
    if !object.is_type_object() {
        return Err(JsonError::InvalidTypeError(
            "cannot add item to a non-object Json item".to_string(),
        ));
    }

    match CString::new(name) {
        Ok(c_str) => {
            let result = unsafe {
                cJSON_AddNumberToObject(object as *mut cJSON, c_str.as_ptr(), number) as *mut Json
            };
            Ok(result)
        }
        Err(err) => Err(JsonError::CStringError(err)),
    }
}

/// Add Json item of type `String` to Json item of type `Object`.
///
/// Args:
/// - `object: *mut Json` - Json item of type `Object` to add the Json item of type `String` to.
/// - `name: &str` - Key to set for the item being added.
/// - `string: &str` - String value for the Json item being added.
///
/// Returns:
/// - `Ok(*mut Json)` - a mutable pointer to the Json item of type `String` that has been added.
/// - `Err(JsonError::InvalidTypeError(String))` - if the Json item provided is not of type `Object`.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let object = cjson_create_object();
///     cjson_add_string_to_object(object, "name", "Nemuel").unwrap();
///     let test_string_item = cjson_get_object_item(object, "name").unwrap();
///     assert_eq!(test_string_item.is_type_string(), true);
///     assert_eq!(
///         cjson_get_string_value(test_string_item).unwrap(), "Nemuel"
///     );
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_add_string_to_object(
    object: *mut Json,
    name: &str,
    string: &str,
) -> Result<*mut Json, JsonError> {
    if !object.is_type_object() {
        return Err(JsonError::InvalidTypeError(
            "cannot add item to a non-object Json item".to_string(),
        ));
    }

    match CString::new(name) {
        Ok(name_c_str) => match CString::new(string) {
            Ok(string_c_str) => {
                let result = unsafe {
                    cJSON_AddStringToObject(
                        object as *mut cJSON,
                        name_c_str.as_ptr(),
                        string_c_str.as_ptr(),
                    ) as *mut Json
                };
                Ok(result)
            }
            Err(err) => Err(JsonError::CStringError(err)),
        },
        Err(err) => Err(JsonError::CStringError(err)),
    }
}

/// Add Json item of type `Raw` to Json item of type `Object`.
///
/// Args:
/// - `object: *mut Json` - Json item of type `Object` to add the Json item of type `Raw` to.
/// - `name: &str` - Key to set for the item being added.
/// - `raw: &str` - Raw string (JSON or otherwise).
///
/// Returns:
/// - `Ok(*mut Json)` - a mutable pointer to the Json item of type `Raw` that has been added.
/// - `Err(JsonError::InvalidTypeError(String))` - if the Json item provided is not of type `Object`.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let object: *mut Json = cjson_create_object();
///     cjson_add_raw_to_object(object, "name", "Nemuel").unwrap();
///     let test_raw_item = cjson_get_object_item(object, "name").unwrap();
///     assert_eq!(test_raw_item.is_type_raw(), true);
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_add_raw_to_object(
    object: *mut Json,
    name: &str,
    raw: &str,
) -> Result<*mut Json, JsonError> {
    if !object.is_type_object() {
        return Err(JsonError::InvalidTypeError(
            "cannot add item to a non-object Json item".to_string(),
        ));
    }

    match CString::new(name) {
        Ok(name_c_str) => match CString::new(raw) {
            Ok(raw_c_str) => {
                let result = unsafe {
                    cJSON_AddRawToObject(
                        object as *mut cJSON,
                        name_c_str.as_ptr(),
                        raw_c_str.as_ptr(),
                    ) as *mut Json
                };
                Ok(result)
            }
            Err(err) => Err(JsonError::CStringError(err)),
        },
        Err(err) => Err(JsonError::CStringError(err)),
    }
}

/// Add Json item of type `Object` to Json item of type `Object`.
///
/// Args:
/// - `object: *mut Json` - Json item of type `Object` to add the Json item of type `Object` to.
/// - `name: &str` - Key to set for the item being added.
///
/// Returns:
/// - `Ok(*mut Json)` - a mutable pointer to the Json item of type `Object` that has been added.
/// - `Err(JsonError::InvalidTypeError(String))` - if the Json item provided is not of type `Object`.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let object: *mut Json = cjson_create_object();
///     cjson_add_object_to_object(object, "test").unwrap();
///     let test_object_item = cjson_get_object_item(object, "test").unwrap();
///     assert_eq!(test_object_item.is_type_object(), true);
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_add_object_to_object(object: *mut Json, name: &str) -> Result<*mut Json, JsonError> {
    if !object.is_type_object() {
        return Err(JsonError::InvalidTypeError(
            "cannot add item to a non-object Json item".to_string(),
        ));
    }

    match CString::new(name) {
        Ok(c_str) => {
            let result = unsafe {
                cJSON_AddObjectToObject(object as *mut cJSON, c_str.as_ptr()) as *mut Json
            };
            Ok(result)
        }
        Err(err) => Err(JsonError::CStringError(err)),
    }
}

/// Add Json item of type `Array` to Json item of type `Object`.
///
/// Args:
/// - `object: *mut Json` - Json item of type `Object` to add the Json item of type `Array` to.
/// - `name: &str` - Key to set for the item being added.
///
/// Returns:
/// - `Ok(*mut Json)` - a mutable pointer to the Json item of type `Array` that has been added.
/// - `Err(JsonError::InvalidTypeError(String))` - if the Json item provided is not of type `Object`.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let object: *mut Json = cjson_create_object();
///     cjson_add_array_to_object(object, "test").unwrap();
///     let test_array_item = cjson_get_object_item(object, "test").unwrap();
///     assert_eq!(test_array_item.is_type_array(), true);
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_add_array_to_object(object: *mut Json, name: &str) -> Result<*mut Json, JsonError> {
    if !object.is_type_object() {
        return Err(JsonError::InvalidTypeError(
            "cannot add item to a non-object Json item".to_string(),
        ));
    }

    match CString::new(name) {
        Ok(c_str) => {
            let result = unsafe {
                cJSON_AddArrayToObject(object as *mut cJSON, c_str.as_ptr()) as *mut Json
            };
            Ok(result)
        }
        Err(err) => Err(JsonError::CStringError(err)),
    }
}

/// Add Json item of any type to Json item of type `Object`.
///
/// Args:
/// - `object: *mut Json` - Json item of type `Object` to add the Json item to.
/// - `string: &str` - Key to set for the item being added.
/// - `item: *mut Json` - Json item to be added.
///
/// Returns:
/// - `Ok(bool)` - a boolean value indicating whether or not the operation was successful.
/// - `Err(JsonError::InvalidTypeError(String))` - if the Json item to be added to is not of type `Object`.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let test_item = cjson_create_null();
///     let object = cjson_create_object();
///     assert_eq!(
///         cjson_add_item_to_object(object, "test", test_item).unwrap(),
///         true
///     );
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_add_item_to_object(
    object: *mut Json,
    string: &str,
    item: *mut Json,
) -> Result<bool, JsonError> {
    if !object.is_type_object() {
        return Err(JsonError::InvalidTypeError(
            "cannot add item to a non-object Json item".to_string(),
        ));
    }

    match CString::new(string) {
        Ok(c_str) => {
            let success = unsafe {
                cJSON_AddItemToObject(object as *mut cJSON, c_str.as_ptr(), item as *mut cJSON)
            };
            if success == 1 {
                Ok(true)
            } else {
                Ok(false)
            }
        }
        Err(err) => Err(JsonError::CStringError(err)),
    }
}

/// Add an item to Json item of type `Object` while maintaining a reference to the original item rather
/// than copying it.
///
/// Args:
/// - `object: *mut Json` - Json item of type `Object` to add the Json item to.
/// - `string: &str` - Key to set for the item being added.
/// - `item: *mut Json` - Json item to be added.
///
/// Returns:
/// - `Ok(bool)` - a boolean value indicating whether or not the operation was successful.
/// - `Err(JsonError::InvalidTypeError(String))` - if the Json item to be added to is not of type `Object`.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let test_item = cjson_create_null();
///     let object = cjson_create_object();
///     assert_eq!(
///         cjson_add_item_reference_to_object(object, "test", test_item).unwrap(),
///         true
///     );
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_add_item_reference_to_object(
    object: *mut Json,
    string: &str,
    item: *mut Json,
) -> Result<bool, JsonError> {
    if !object.is_type_object() {
        return Err(JsonError::InvalidTypeError(
            "cannot add item to a non-object Json item".to_string(),
        ));
    }

    match CString::new(string) {
        Ok(c_str) => {
            let success = unsafe {
                cJSON_AddItemReferenceToObject(
                    object as *mut cJSON,
                    c_str.as_ptr(),
                    item as *mut cJSON,
                )
            };
            if success == 1 {
                Ok(true)
            } else {
                Ok(false)
            }
        }
        Err(err) => Err(JsonError::CStringError(err)),
    }
}

/// Add Json item of any type to Json item of type `Object` with a name/key that is a constant
/// or reference.
///
/// Args:
/// - `object: *mut Json` - Json item of type `Object` to add the Json item to.
/// - `name: &str` - Key to set for the item being added.
/// - `item: *mut Json` - Json item to be added.
///
/// Returns:
/// - `Ok(bool)` - a boolean value indicating whether or not the operation was successful.
/// - `Err(JsonError::InvalidTypeError(String))` - if the Json item to be added to is not of type `Object`.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let test_item = cjson_create_null();
///     let object = cjson_create_object();
///     assert_eq!(
///         cjson_add_item_to_object_cs(object, "test", test_item).unwrap(),
///         true
///     );
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_add_item_to_object_cs(
    object: *mut Json,
    name: &str,
    item: *mut Json,
) -> Result<bool, JsonError> {
    if !object.is_type_object() {
        return Err(JsonError::InvalidTypeError(
            "cannot add item to a non-object Json item".to_string(),
        ));
    }

    match CString::new(name) {
        Ok(c_str) => {
            let success = unsafe {
                cJSON_AddItemToObjectCS(object as *mut cJSON, c_str.as_ptr(), item as *mut cJSON)
            };
            if success == 1 {
                Ok(true)
            } else {
                Ok(false)
            }
        }
        Err(err) => Err(JsonError::CStringError(err)),
    }
}

/// Check whether or not the Json item of type `Object` contains an item with the specified key.
///
/// Args:
/// - `object: *mut Json` - The Json item of type `Object` to inspect for an item with the specified key.
/// - `string: &str` - String slice specifying the key of the item being looked for.
///
/// Returns:
/// - `Ok(bool)` - a boolean value indicating whether or not an item with the specified key exists within
/// the object if the lookup is successful.
/// - `Err(JsonError::CStringError(NulError))` - if the provided string slice (representing the key)
/// contains a null byte.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let object = cjson_create_object();
///     cjson_add_string_to_object(object, "name", "Nemuel").unwrap();
///
///     assert_eq!(cjson_has_object_item(object, "name").unwrap(), true);
///     assert_eq!(cjson_has_object_item(object, "age").unwrap(), false);
///
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_has_object_item(object: *mut Json, string: &str) -> Result<bool, JsonError> {
    match CString::new(string) {
        Ok(c_str) => {
            let result = unsafe { cJSON_HasObjectItem(object as *const cJSON, c_str.as_ptr()) };
            if result == 1 {
                Ok(true)
            } else {
                Ok(false)
            }
        }
        Err(err) => Err(JsonError::CStringError(err)),
    }
}

/// Get item within the object with the specified key.
///
/// Args:
/// - `object: *mut Json` - Json item of type `Object` from which we want to get an item.
/// - `string: &str` - Key of the Json item that we want to get.
///
/// Returns:
/// - `Ok(*mut Json)` - a mutable pointer to the Json item with the provided key if gotten successfully.
/// - `Err(JsonError::CStringError(NulError))` - if the provided string slice contains a null byte.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let object = cjson_create_object();
///     cjson_add_string_to_object(object, "name", "Nemuel").unwrap();
///
///     let item = cjson_get_object_item(object, "name").unwrap();
///     assert_eq!(item.is_type_string(), true);
///     assert_eq!(cjson_get_string_value(item).unwrap(), "Nemuel");
///
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_get_object_item(object: *mut Json, string: &str) -> Result<*mut Json, JsonError> {
    match CString::new(string) {
        Ok(c_str) => {
            let result =
                unsafe { cJSON_GetObjectItem(object as *const cJSON, c_str.as_ptr()) as *mut Json };
            Ok(result)
        }
        Err(err) => Err(JsonError::CStringError(err)),
    }
}

/// Get item within the object with the specified key, with a case-sensitive comparison of keys.
///
/// Args:
/// - `object: *mut Json` - Json item of type `Object` from which we want to get an item.
/// - `string: &str` - Key of the Json item that we want to get.
///
/// Returns:
/// - `Ok(*mut Json)` - a mutable pointer to the Json item with the provided key if gotten successfully.
/// - `Err(JsonError::CStringError(NulError))` - if the provided string slice contains a null byte.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let object = cjson_create_object();
///     cjson_add_string_to_object(object, "name", "Nemuel").unwrap();
///
///     let item = cjson_get_object_item_case_sensitive(object, "Name").unwrap();
///     assert_eq!(item.is_null(), true);
///     let item = cjson_get_object_item_case_sensitive(object, "name").unwrap();
///     assert_eq!(item.is_null(), false);
///     assert_eq!(item.is_type_string(), true);
///
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_get_object_item_case_sensitive(
    object: *mut Json,
    string: &str,
) -> Result<*mut Json, JsonError> {
    match CString::new(string) {
        Ok(c_str) => {
            let result = unsafe {
                cJSON_GetObjectItemCaseSensitive(object as *const cJSON, c_str.as_ptr())
                    as *mut Json
            };
            Ok(result)
        }
        Err(err) => Err(JsonError::CStringError(err)),
    }
}

/// Replace item with specified key in Json item of type `Object`.
///
/// Args:
/// - `object: *mut Json` - Json item of type `Object` within which the replacement is to happen.
/// - `string: &str` - The key of the Json item to be replaced.
/// - `newitem: *mut Json` - Item replacing the original one.
///
/// Returns:
/// - `Ok(bool)` - a boolean value indicating whether or not the operation was successful.
/// - `Err(JsonError::InvalidTypeError(String))` - if the Json item being operated on is not of type
/// `Object`.
/// - `Err(JsonError::CStringError(NulError))` - if the provided string slice contains a null byte.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let object = cjson_create_object();
///     let original_item = cjson_create_string("Nemuel".to_string()).unwrap();
///     cjson_add_item_to_object(object, "name", original_item).unwrap();
///
///     let new_item = cjson_create_string("Wainaina".to_string()).unwrap();
///     let result = cjson_replace_item_in_object(object, "name", new_item).unwrap();
///     assert_eq!(result, true);
///     assert_eq!(
///         cjson_get_string_value(cjson_get_object_item(object, "name").unwrap()).unwrap(),
///         "Wainaina"
///     );
///
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_replace_item_in_object(
    object: *mut Json,
    string: &str,
    newitem: *mut Json,
) -> Result<bool, JsonError> {
    if !object.is_type_object() {
        return Err(JsonError::InvalidTypeError(
            "cannot replace item in a non-object Json item".to_string(),
        ));
    }

    match CString::new(string) {
        Ok(c_str) => {
            let result = unsafe {
                cJSON_ReplaceItemInObject(
                    object as *mut cJSON,
                    c_str.as_ptr(),
                    newitem as *mut cJSON,
                )
            };
            if result == 1 {
                Ok(true)
            } else {
                Ok(false)
            }
        }
        Err(err) => Err(JsonError::CStringError(err)),
    }
}

/// Replace item with specified key in Json item of type `Object`, with a case-sensitive comparison of
/// keys.
///
/// Args:
/// - `object: *mut Json` - Json item of type `Object` within which the replacement is to happen.
/// - `string: &str` - The key of the Json item to be replaced.
/// - `newitem: *mut Json` - Item replacing the original one.
///
/// Returns:
/// - `Ok(bool)` - a boolean value indicating whether or not the operation was successful.
/// - `Err(JsonError::InvalidTypeError(String))` - if the Json item being operated on is not of type
/// `Object`.
/// - `Err(JsonError::CStringError(NulError))` - if the provided string slice contains a null byte.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let object = cjson_create_object();
///     let original_item = cjson_create_string("Nemuel".to_string()).unwrap();
///     cjson_add_item_to_object(object, "name", original_item).unwrap();
///
///     let new_item = cjson_create_string("Wainaina".to_string()).unwrap();
///     let mut result = cjson_replace_item_in_object_case_sensitive(object, "Name", new_item).unwrap();
///     assert_eq!(result, false);
///     result = cjson_replace_item_in_object_case_sensitive(object, "name", new_item).unwrap();
///     assert_eq!(result, true);
///     assert_eq!(
///         cjson_get_string_value(cjson_get_object_item(object, "name").unwrap()).unwrap(),
///         "Wainaina"
///     );
///
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_replace_item_in_object_case_sensitive(
    object: *mut Json,
    string: &str,
    newitem: *mut Json,
) -> Result<bool, JsonError> {
    if !object.is_type_object() {
        return Err(JsonError::InvalidTypeError(
            "cannot replace item in a non-object Json item".to_string(),
        ));
    }

    match CString::new(string) {
        Ok(c_str) => {
            let result = unsafe {
                cJSON_ReplaceItemInObjectCaseSensitive(
                    object as *mut cJSON,
                    c_str.as_ptr(),
                    newitem as *mut cJSON,
                )
            };
            if result == 1 {
                Ok(true)
            } else {
                Ok(false)
            }
        }
        Err(err) => Err(JsonError::CStringError(err)),
    }
}

/// Detach item from Json item of type `Object`.
///
/// Args:
/// - `object: *mut Json` - Mutable pointer to the Json item of type `Object` from which an item is to
/// be detached.
/// - `string: &str` - The key value for the item that is to be detached from the object.
///
/// Returns:
/// - `Ok(*mut Json)` - a mutable pointer to the detached item if the operation happens.
/// - `Err(JsonError::InvalidTypeError(String))` - if the Json item to be operated on is not of type
/// `Object`.
/// - `Err(JsonError::CStringError(NulError))` - if the provided string slice contains a null byte.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let object = cjson_create_object();
///     let string_item = cjson_create_string("Nemuel".to_string()).unwrap();
///
///     cjson_add_item_to_object(object, "name", string_item).unwrap();
///     assert_eq!(cjson_has_object_item(object, "name").unwrap(), true);
///
///     let detached_item = cjson_detach_item_from_object(object, "name").unwrap();
///     assert_eq!(detached_item.is_type_string(), true);
///     assert_eq!(cjson_has_object_item(object, "name").unwrap(), false);
///
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_detach_item_from_object(
    object: *mut Json,
    string: &str,
) -> Result<*mut Json, JsonError> {
    if !object.is_type_object() {
        return Err(JsonError::InvalidTypeError(
            "cannot detach item from a non-object Json item".to_string(),
        ));
    }

    match CString::new(string) {
        Ok(c_str) => {
            let detached_item = unsafe {
                cJSON_DetachItemFromObject(object as *mut cJSON, c_str.as_ptr()) as *mut Json
            };
            Ok(detached_item)
        }
        Err(err) => Err(JsonError::CStringError(err)),
    }
}

/// Detach Json item from its parent via pointer (thus maintaining access to the detached item).
///
/// Args:
/// - `parent: *mut Json` - Mutable pointer to the parent Json item from which an item is to be detached.
/// - `item: *mut Json` - Mutable pointer to the Json item that is to be detached from its parent.
///
/// Returns:
/// - `*mut Json` - a mutable pointer to the detached item.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let parent = cjson_create_object();
///     let item = cjson_create_string("Nemuel".to_string()).unwrap();
///
///     cjson_add_item_to_object(parent, "name", item).unwrap();
///     assert_eq!(cjson_has_object_item(parent, "name").unwrap(), true);
///
///     let detached_item = cjson_detach_item_via_pointer(parent, item);
///     assert_eq!(detached_item.is_type_string(), true);
///     assert_eq!(cjson_has_object_item(parent, "name").unwrap(), false);
///
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_detach_item_via_pointer(parent: *mut Json, item: *mut Json) -> *mut Json {
    unsafe { cJSON_DetachItemViaPointer(parent as *mut cJSON, item as *mut cJSON) as *mut Json }
}

/// Replace a Json item from its parent via pointer with a new item.
///
/// Args:
/// - `parent: *mut Json` - Mutable pointer to the parent Json item in which an item is to be replaced.
/// - `item: *mut Json` - Mutable pointer to the Json item that is to be replaced with another one.
/// - `replacement: *mut Json` - Mutable pointer to the Json item that is to replace the original one.
///
/// Returns:
/// - `bool` - a boolean value indicating success or failure of the operation.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let parent = cjson_create_array();
///     let item = cjson_create_string("Nemuel".to_string()).unwrap();
///     cjson_add_item_to_array(parent, item).unwrap();
///     assert_eq!(parent.print().unwrap(), r#"["Nemuel"]"#);
///
///     let replacement = cjson_create_string("Wainaina".to_string()).unwrap();
///     cjson_replace_item_via_pointer(parent, item, replacement);
///     assert_eq!(parent.print().unwrap(), r#"["Wainaina"]"#);
///
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_replace_item_via_pointer(
    parent: *mut Json,
    item: *mut Json,
    replacement: *mut Json,
) -> bool {
    let result = unsafe {
        cJSON_ReplaceItemViaPointer(
            parent as *mut cJSON,
            item as *mut cJSON,
            replacement as *mut cJSON,
        )
    };
    if result == 1 {
        true
    } else {
        false
    }
}

/// Create a copy of a Json item.
///
/// Args:
/// - `item: *mut Json` - Mutable pointer to the Json item to be duplicated.
/// - `recurse: bool` - Boolean value specifying whether or not to duplicate nested structures as well.
///
/// Returns:
/// - `*mut Json` - a mutable pointer to the newly created duplicate Json item.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let original = cjson_create_string("Nemuel".to_string()).unwrap();
///
///     let copy = cjson_duplicate(original, false);
///
///     let result = cjson_compare(original, copy, true);
///     assert_eq!(result, true);
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_duplicate(item: *mut Json, recurse: bool) -> *mut Json {
    unsafe { cJSON_Duplicate(item as *const cJSON, if recurse { 1 } else { 0 }) as *mut Json }
}

/// Check whether 2 Json items are equivalent in structure and value.
///
/// Args:
/// - `a: *mut Json` - Mutable pointer to the first Json item.
/// - `b: *mut Json` - Mutable pointer to the second Json item.
/// - `case_sensitive: bool` - Boolean value specifying whether or not to do case-sensitive comparison
/// for string values.
///
/// Returns:
/// - `bool` - a boolean value (true or false) indicating whether or not the 2 Json items are equivalent.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let item1 = cjson_create_string("Nemuel".to_string()).unwrap();
///     let item2 = cjson_create_string("Nemuel".to_string()).unwrap();
///     let result = cjson_compare(item1, item2, true);
///     assert_eq!(result, true);
///     println!("Test passed"); // output: Test passed
/// }
/// ```
pub fn cjson_compare(a: *mut Json, b: *mut Json, case_sensitive: bool) -> bool {
    let result = unsafe {
        cJSON_Compare(
            a as *const cJSON,
            b as *const cJSON,
            if case_sensitive { 1 } else { 0 },
        )
    };
    if result == 1 {
        true
    } else {
        false
    }
}

/// Deallocate/free the memory allocated for a Json item along with all its nested structures if any.
///
/// NOTE: The pointers to the parent item and all its nested structures (if any) are themselves not
/// set to NULL, raising a dangling pointers issue.
///
/// Args:
/// - `item: *mut Json` - Mutable pointer to the Json item whose memory is to be deallocated/freed.
///
/// Example:
/// ```rust
/// use cjson_rs::*;
///
/// fn main() {
///     let mut object = cjson_create_object();
///     cjson_add_string_to_object(object, "name", "Nemuel").unwrap();
///
///     cjson_delete(&mut object);
/// }
/// ```
pub fn cjson_delete(item: &mut *mut Json) {
    unsafe {
        cJSON_Delete(*item as *mut cJSON);
    }
}

/// Allocate a specified amount of memory.
///
/// Args:
/// - `size: usize` - Amount of memory to allocate.
///
/// Returns:
/// - `*mut c_void` - a mutable pointer to the allocated memory.
pub fn cjson_malloc(size: usize) -> *mut c_void {
    unsafe { cJSON_malloc(size) }
}

/// Deallocate/free the memory at the specified location.
///
/// NOTE: The pointer to the memory location is itself not set to NULL, raising a dangling pointer issue.
///
/// Args:
/// - `item: *mut c_void` - Mutable pointer to the memory which is to be deallocated/freed.
pub fn cjson_free(item: *mut c_void) {
    unsafe {
        cJSON_free(item);
    }
}
