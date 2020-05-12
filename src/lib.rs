#![feature(proc_macro_hygiene)]

use skyline::{hook, install_hook};
use skyline::nn::ro;
use skyline::libc::{c_void, c_int, size_t};
use skyline::from_c_str;

use parking_lot::Mutex;

use skyline::nro::NroInfo;

type Callback = fn(&NroInfo);

static HOOKS: Mutex<Vec<Callback>> = Mutex::new(Vec::new());

#[hook(replace = ro::LoadModule)]
pub fn handle_load_module(
    out_module: *mut ro::Module, 
    image: *const c_void, 
    buffer: *mut c_void, 
    buffer_size: size_t, 
    _flag: c_int
) -> c_int {
    // use Flag_Now to ensure we aren't lazy loading NROs
    // causes slower load times but is necessary for hooking
    let ret = original!()(out_module, image, buffer, buffer_size, ro::BindFlag_BindFlag_Now as i32);

    let name = unsafe { from_c_str(&(*out_module).Name as *const u8) };
    println!("[NRO hook] Loaded {}.", name);
    let nro_info = NroInfo::new(&name, unsafe { &mut *out_module });
    for hook in HOOKS.lock().iter() {
        hook(&nro_info)
    }

    ret
}

#[skyline::main(name = "nro_hook")]
pub fn main() {
    println!("[NRO hook] Installing NRO hook...");
    install_hook!(handle_load_module);
    println!("[NRO hook] NRO hook installed.");
}

#[no_mangle]
pub extern "Rust" fn add_nro_load_hook(callback: Callback) {
    let mut hooks = HOOKS.lock();

    hooks.push(callback);
}
