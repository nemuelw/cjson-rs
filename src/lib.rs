mod bindings;
use bindings::*;

pub const VERSION_MAJOR: u32 = bindings::CJSON_VERSION_MAJOR;
pub const VERSION_MINOR: u32 = bindings::CJSON_VERSION_MINOR;
pub const VERSION_PATCH: u32 = bindings::CJSON_VERSION_PATCH;
pub const INVALID: u32 = bindings::cJSON_Invalid;
pub const FALSE: u32 = bindings::cJSON_False;
pub const TRUE: u32 = bindings::cJSON_True;
pub const NULL: u32 = bindings::cJSON_NULL;
pub const NUMBER: u32 = bindings::cJSON_Number;
pub const STRING: u32 = bindings::cJSON_String;
pub const ARRAY: u32 = bindings::cJSON_Array;
pub const OBJECT: u32 = bindings::cJSON_Object;
pub const RAW: u32 = bindings::cJSON_Raw;
pub const IS_REFERENCE: u32 = bindings::cJSON_IsReference;
pub const STRING_IS_CONST: u32 = bindings::cJSON_StringIsConst;
pub const NESTING_LIMIT: u32 = bindings::CJSON_NESTING_LIMIT;
pub const CIRCULAR_LIMIT: u32 = bindings::CJSON_CIRCULAR_LIMIT;

pub fn cjson_version() -> String {
    format!("{}.{}.{}", VERSION_MAJOR, VERSION_MINOR, VERSION_PATCH)
}
