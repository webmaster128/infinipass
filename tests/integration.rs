//! This integration test tries to run and call the generated wasm.
//! It depends on a Wasm build being available, which you can create with `cargo wasm`.
//! Then running `cargo integration-test` will validate we can properly call into that generated Wasm.

use wasmer_middleware_common::metering;
use wasmer_runtime_core::{
    backend::Compiler,
    cache::Artifact,
    codegen::{MiddlewareChain, StreamingCompiler},
    compile_with,
    error::{InvokeError, RuntimeError},
    imports,
    module::Module,
    typed_func::{Func, Wasm},
    Instance,
};
use wasmer_singlepass_backend::ModuleCodeGenerator;

static WASM: &[u8] = include_bytes!("../target/wasm32-unknown-unknown/release/infinipass.wasm");

/// In Wasmer, The gas limit on instances is set during compile time and is included in the compiled binaries.
/// This causes issues when trying to reuse the same precompiled binaries for another instance with a different
/// gas limit.
/// https://github.com/wasmerio/wasmer/pull/996
/// To work around that, we set the gas limit of all Wasmer instances to this very-high gas limit value, under
/// the assumption that users won't request more than this amount of gas. Then to set a gas limit below that figure,
/// we pretend to consume the difference between the two in `set_gas_limit`, so the amount of units left is equal to
/// the requested gas limit.
pub const GAS_LIMIT: u64 = 10_000_000_000;

pub fn compile(code: &[u8]) -> Module {
    let module = compile_with(code, compiler().as_ref()).unwrap();
    module
}

pub fn compiler() -> Box<dyn Compiler> {
    let c: StreamingCompiler<ModuleCodeGenerator, _, _, _, _> = StreamingCompiler::new(move || {
        let mut chain = MiddlewareChain::new();
        chain.push(metering::Metering::new(GAS_LIMIT));
        chain
    });
    Box::new(c)
}

fn instantiate(module: &Module) -> Instance {
    let import_obj = imports! { "env" => {}, };
    module.instantiate(&import_obj).unwrap()
}

/// Set the amount of gas units that can be used in the instance.
pub fn set_gas_limit(instance: &mut Instance, limit: u64) {
    let used = GAS_LIMIT.saturating_sub(limit);
    metering::set_points_used(instance, used)
}

/// Get how many more gas units can be used in the instance.
pub fn get_gas_left(instance: &Instance) -> u64 {
    let used = metering::get_points_used(instance);
    // when running out of gas, get_points_used can exceed GAS_LIMIT
    GAS_LIMIT.saturating_sub(used)
}

#[test]
fn do_cpu_loop_direct_export() {
    let module = compile(WASM);
    let mut instance = instantiate(&module);
    set_gas_limit(&mut instance, 1_000_000);
    let func: Func<(), (), Wasm> = instance.exports.get("do_cpu_loop").unwrap();
    let result = func.call();

    match result.unwrap_err() {
        // TODO: fix the issue described below:
        // `InvokeError::FailedWithNoError` happens when running out of gas in singlepass v0.17
        // but it's supposed to indicate bugs in Wasmer...
        // https://github.com/wasmerio/wasmer/issues/1452
        // https://github.com/CosmWasm/cosmwasm/issues/375
        RuntimeError::InvokeError(InvokeError::FailedWithNoError) => { /* out of gas, good! */ }
        e => println!("Unexpedcted error: {:?}", e),
    }

    assert_eq!(get_gas_left(&instance), 0);
}

#[test]
fn do_cpu_loop_cached() {
    let module = compile(WASM);
    let serialized_cache = module.cache().unwrap();
    let buffer = serialized_cache.serialize().unwrap();

    let serialized_cache = Artifact::deserialize(&buffer).unwrap();
    let restored =
        unsafe { wasmer_runtime_core::load_cache_with(serialized_cache, compiler().as_ref()) }
            .unwrap();

    let mut instance = instantiate(&restored);

    set_gas_limit(&mut instance, 1_000_000);
    let func: Func<(), (), Wasm> = instance.exports.get("do_cpu_loop").unwrap();
    let result = func.call();

    match result.unwrap_err() {
        // TODO: fix the issue described below:
        // `InvokeError::FailedWithNoError` happens when running out of gas in singlepass v0.17
        // but it's supposed to indicate bugs in Wasmer...
        // https://github.com/wasmerio/wasmer/issues/1452
        // https://github.com/CosmWasm/cosmwasm/issues/375
        RuntimeError::InvokeError(InvokeError::FailedWithNoError) => { /* out of gas, good! */ }
        e => println!("Unexpedcted error: {:?}", e),
    }

    assert_eq!(get_gas_left(&instance), 0);
}
