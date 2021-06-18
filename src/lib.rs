#![deny(clippy::all)]

#[macro_use]
extern crate napi_derive;
extern crate tauri_hotkey;
use napi::{
  threadsafe_function::{ThreadSafeCallContext, ThreadsafeFunctionCallMode},
  CallContext, Env, JsBoolean, JsFunction, JsObject, JsString, JsUndefined, Property, Result,
};
use tauri_hotkey::{parse_hotkey, Hotkey, HotkeyManager};

#[cfg(all(
  unix,
  not(target_env = "musl"),
  not(target_arch = "aarch64"),
  not(target_arch = "arm"),
  not(debug_assertions)
))]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[cfg(all(windows, target_arch = "x86_64"))]
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn hotkey_to_napi_err(hotkey_err: tauri_hotkey::Error) -> napi::Error {
  napi::Error::from_reason(hotkey_err.to_string())
}

#[module_exports]
fn init(mut exports: JsObject, env: Env) -> Result<()> {
  let hotkey_manager_class = env.define_class(
    "HotkeyManager",
    hotkey_manager_constructor,
    &[
      Property::new(&env, "register")?.with_method(register_hotkey),
      Property::new(&env, "unregister")?.with_method(unregister_hotkey),
      Property::new(&env, "unregisterAll")?.with_method(unregister_all),
      Property::new(&env, "checkIsRegistered")?.with_method(check_hotkey_is_registered),
    ],
  )?;
  exports.set_named_property("HotkeyManager", hotkey_manager_class)?;
  Ok(())
}

#[js_function(1)]
fn hotkey_manager_constructor(ctx: CallContext) -> Result<JsUndefined> {
  let manager = HotkeyManager::new();
  let mut this: JsObject = ctx.this_unchecked();
  ctx.env.wrap(&mut this, manager)?;
  ctx.env.get_undefined()
}

#[js_function(2)]
fn register_hotkey(ctx: CallContext) -> Result<JsUndefined> {
  let hotkey_utf8 = ctx.get::<JsString>(0)?.into_utf8()?;
  let hotkey_str = hotkey_utf8.as_str()?;
  let callback_js_func = ctx.get::<JsFunction>(1)?;
  let mut this: JsObject = ctx.this_unchecked();
  let manager = ctx.env.unwrap::<HotkeyManager>(&mut this)?;

  let hotkey: Hotkey = parse_hotkey(&hotkey_str).map_err(hotkey_to_napi_err)?;

  let ts_callback =
    ctx
      .env
      .create_threadsafe_function(&callback_js_func, 0, |ctx: ThreadSafeCallContext<()>| {
        Ok(vec![ctx.env.get_undefined()?])
      })?;

  manager
    .register(hotkey, move || {
      ts_callback.call(Ok(()), ThreadsafeFunctionCallMode::NonBlocking);
    })
    .map_err(hotkey_to_napi_err)?;

  ctx.env.get_undefined()
}

#[js_function(1)]
fn unregister_hotkey(ctx: CallContext) -> Result<JsUndefined> {
  let hotkey_utf8 = ctx.get::<JsString>(0)?.into_utf8()?;
  let hotkey_str = hotkey_utf8.as_str()?;
  let mut this: JsObject = ctx.this_unchecked();
  let manager = ctx.env.unwrap::<HotkeyManager>(&mut this)?;

  let hotkey: Hotkey = parse_hotkey(&hotkey_str).map_err(hotkey_to_napi_err)?;
  manager.unregister(&hotkey).map_err(hotkey_to_napi_err)?;

  ctx.env.get_undefined()
}

#[js_function(0)]
fn unregister_all(ctx: CallContext) -> Result<JsUndefined> {
  let mut this: JsObject = ctx.this_unchecked();
  let manager = ctx.env.unwrap::<HotkeyManager>(&mut this)?;

  manager.unregister_all().map_err(hotkey_to_napi_err)?;

  ctx.env.get_undefined()
}

#[js_function(1)]
fn check_hotkey_is_registered(ctx: CallContext) -> Result<JsBoolean> {
  let hotkey_utf8 = ctx.get::<JsString>(0)?.into_utf8()?;
  let hotkey_str = hotkey_utf8.as_str()?;
  let mut this: JsObject = ctx.this_unchecked();
  let manager = ctx.env.unwrap::<HotkeyManager>(&mut this)?;

  let hotkey: Hotkey = parse_hotkey(&hotkey_str).map_err(hotkey_to_napi_err)?;

  ctx.env.get_boolean(manager.is_registered(&hotkey))
}
