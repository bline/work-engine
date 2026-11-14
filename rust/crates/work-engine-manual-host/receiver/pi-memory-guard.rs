//! Minimal Node N-API interoperability. Compile as a cdylib, load before importing
//! Pi or accepting any candidate bytes. No credential, filesystem or transport IO.
use std::ffi::{c_char, c_int, c_void};
type Env = *mut c_void;
type Value = *mut c_void;
type Info = *mut c_void;
unsafe extern "C" {
    fn prctl(option: c_int, ...) -> c_int;
    fn napi_throw_error(env: Env, code: *const c_char, message: *const c_char) -> c_int;
    fn napi_get_boolean(env: Env, value: bool, result: *mut Value) -> c_int;
    fn napi_create_function(
        env: Env,
        name: *const c_char,
        length: usize,
        callback: Option<unsafe extern "C" fn(Env, Info) -> Value>,
        data: *mut c_void,
        result: *mut Value,
    ) -> c_int;
    fn napi_set_named_property(env: Env, object: Value, name: *const c_char, value: Value)
    -> c_int;
}
const PR_SET_DUMPABLE: c_int = 4;
const PR_GET_DUMPABLE: c_int = 3;
unsafe fn fail(env: Env) -> Value {
    unsafe {
        napi_throw_error(
            env,
            std::ptr::null(),
            c"native memory protection unavailable".as_ptr(),
        );
    }
    std::ptr::null_mut()
}
unsafe extern "C" fn assert_protected(env: Env, _info: Info) -> Value {
    if unsafe { prctl(PR_GET_DUMPABLE, 0, 0, 0, 0) } != 0 {
        return unsafe { fail(env) };
    }
    let mut result = std::ptr::null_mut();
    if unsafe { napi_get_boolean(env, true, &mut result) } != 0 {
        return unsafe { fail(env) };
    }
    result
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn napi_register_module_v1(env: Env, exports: Value) -> Value {
    if unsafe { prctl(PR_SET_DUMPABLE, 0, 0, 0, 0) } != 0
        || unsafe { prctl(PR_GET_DUMPABLE, 0, 0, 0, 0) } != 0
    {
        return unsafe { fail(env) };
    }
    let mut callback = std::ptr::null_mut();
    if unsafe {
        napi_create_function(
            env,
            c"assertProtected".as_ptr(),
            15,
            Some(assert_protected),
            std::ptr::null_mut(),
            &mut callback,
        )
    } != 0
        || unsafe { napi_set_named_property(env, exports, c"assertProtected".as_ptr(), callback) }
            != 0
    {
        return unsafe { fail(env) };
    }
    exports
}
