#![feature(proc_macro_hygiene)]

use std::sync::atomic::{AtomicBool, Ordering};

use skyline::{hook, install_hooks};
use skyline::nn::ro;
use skyline::libc::{c_void, c_int, size_t};
use skyline::try_from_c_str;

use parking_lot::Mutex;

use skyline::nro::NroInfo;

type Callback = fn(&NroInfo);

static LOAD_HOOKS: Mutex<Vec<Callback>> = Mutex::new(Vec::new());
static UNLOAD_HOOKS: Mutex<Vec<Callback>> = Mutex::new(Vec::new());

static SHOULD_BIND_NOW: AtomicBool = AtomicBool::new(false);

#[hook(replace = ro::LoadModule)]
pub fn handle_load_module(
    out_module: *mut ro::Module, 
    image: *const c_void, 
    buffer: *mut c_void, 
    buffer_size: size_t, 
    mut flag: c_int
) -> c_int {
    if SHOULD_BIND_NOW.load(Ordering::SeqCst) {
        flag = 1;
    }
    let ret = original!()(out_module, image, buffer, buffer_size, flag);

    let name = unsafe { try_from_c_str(&(*out_module).Name as *const u8).unwrap_or_else(|_| String::from("unknown")) };
    println!("[NRO hook] Loaded {}. BindFlag: {}", name, match flag {
        1 => "Now",
        2 => "Lazy",
        _ => "Unknown"
    });
    let nro_info = NroInfo::new(&name, unsafe { &mut *out_module });
    for hook in LOAD_HOOKS.lock().iter() {
        hook(&nro_info)
    }

    ret
}

#[hook(replace = ro::UnloadModule)]
pub fn handle_unload_module(in_module: *mut ro::Module) -> c_int {
    let ret = original!()(in_module);

    let name = unsafe { try_from_c_str(&(*in_module).Name as *const u8).unwrap_or_else(|_| String::from("unknown")) };
    println!("[NRO hook] Unloaded {}.", name);
    let nro_info = NroInfo::new(&name, unsafe { &mut *in_module });
    for hook in UNLOAD_HOOKS.lock().iter() {
        hook(&nro_info);
    }

    ret
}

#[skyline::main(name = "nro_hook")]
pub fn main() {
    println!("[NRO hook] Installing NRO hooks...");
    install_hooks!(handle_load_module, handle_unload_module);
    println!("[NRO hook] NRO hooks installed.");
}

#[no_mangle]
pub extern "Rust" fn add_nro_load_hook(callback: Callback) {
    let mut hooks = LOAD_HOOKS.lock();

    hooks.push(callback);
}

#[no_mangle]
pub extern "Rust" fn add_nro_unload_hook(callback: Callback) {
    let mut hooks = UNLOAD_HOOKS.lock();

    hooks.push(callback);
}

#[no_mangle]
pub extern "Rust" fn nro_request_bind_now() {
    SHOULD_BIND_NOW.store(true, Ordering::SeqCst);
}