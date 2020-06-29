//! This integration test tries to run and call the generated wasm.
//! It depends on a Wasm build being available, which you can create with `cargo wasm-debug`.
//! Then running `cargo integration-test` will validate we can properly call into that generated Wasm.

#[cfg(feature = "singlepass")]
mod singlepass_tests {
    use hackatom::contract::{HandleMsg, InitMsg, State, CONFIG_KEY};

    use cosmwasm_std::{
        coins, log, BankMsg, HandleResponse, HandleResult, HumanAddr, InitResponse, InitResult,
        StdError,
    };
    use cosmwasm_std::{to_vec, Empty};
    use cosmwasm_vm::testing::{handle, init, mock_dependencies, mock_env, mock_instance};
    use cosmwasm_vm::{call_handle, call_handle_raw, call_init_raw, features_from_csv, CosmCache};
    use tempfile::TempDir;

    static WASM: &[u8] = include_bytes!("../target/wasm32-unknown-unknown/release/hackatom.wasm");

    fn make_init_msg() -> (InitMsg, HumanAddr) {
        let verifier = HumanAddr::from("verifies");
        let beneficiary = HumanAddr::from("benefits");
        let creator = HumanAddr::from("creator");
        (
            InitMsg {
                verifier: verifier.clone(),
                beneficiary: beneficiary.clone(),
            },
            creator,
        )
    }

    #[test]
    fn handle_panic() {
        let mut deps = mock_instance(WASM, &[]);

        let (init_msg, creator) = make_init_msg();
        let init_env = mock_env(&deps.api, creator.as_str(), &[]);
        let init_res: InitResponse = init(&mut deps, init_env, init_msg).unwrap();
        assert_eq!(0, init_res.messages.len());

        let handle_env = mock_env(&deps.api, creator.as_str(), &[]);
        // panic inside contract should not panic out here
        // Note: we need to use the production-call, not the testing call (which unwraps any vm error)
        let handle_res = call_handle::<_, _, _, Empty>(
            &mut deps,
            &handle_env,
            &to_vec(&HandleMsg::Panic {}).unwrap(),
        );
        assert!(handle_res.is_err());
    }

    #[test]
    fn handle_cpu_loop() {
        let mut deps = mock_instance(WASM, &[]);

        let (init_msg, creator) = make_init_msg();
        let init_env = mock_env(&deps.api, creator.as_str(), &[]);
        let init_res: InitResponse = init(&mut deps, init_env, init_msg).unwrap();
        assert_eq!(0, init_res.messages.len());

        let handle_env = mock_env(&deps.api, creator.as_str(), &[]);
        // Note: we need to use the production-call, not the testing call (which unwraps any vm error)
        let handle_res = call_handle::<_, _, _, Empty>(
            &mut deps,
            &handle_env,
            &to_vec(&HandleMsg::CpuLoop {}).unwrap(),
        );
        assert!(handle_res.is_err());
        assert_eq!(deps.get_gas_left(), 0);
    }

    #[test]
    fn handle_cpu_loop_cache() {
        let deps = mock_dependencies(20, &[]);
        let gas_limit = 2_000_000u64;
        let tmp_dir = TempDir::new().unwrap();
        let features = features_from_csv("staking");
        let mut cache = unsafe { CosmCache::new(tmp_dir.path(), features, 0) }.unwrap();
        // store code
        let code_id = cache.save_wasm(WASM).unwrap();
        // init
        let (init_msg, creator) = make_init_msg();
        let env = mock_env(&deps.api, creator.as_str(), &coins(1000, "cosm"));
        let mut instance = cache.get_instance(&code_id, deps, gas_limit).unwrap();
        let raw_msg = to_vec(&init_msg).unwrap();
        let raw_env = to_vec(&env).unwrap();
        let res = call_init_raw(&mut instance, &raw_env, &raw_msg);
        let gas_used = gas_limit - instance.get_gas_left();
        println!("Init used gas: {}", gas_used);
        res.unwrap();
        let deps = instance.recycle().unwrap();
        // handle
        let mut instance = cache.get_instance(&code_id, deps, gas_limit).unwrap();
        let raw_msg = r#"{"cpu_loop":{}}"#;
        let res = call_handle_raw(&mut instance, &raw_env, raw_msg.as_bytes());
        let gas_used = gas_limit - instance.get_gas_left();
        println!("Handle used gas: {}", gas_used);
        assert!(res.is_err());
        assert_eq!(instance.get_gas_left(), 0);
        instance.recycle();
    }

    #[test]
    fn handle_storage_loop() {
        let mut deps = mock_instance(WASM, &[]);

        let (init_msg, creator) = make_init_msg();
        let init_env = mock_env(&deps.api, creator.as_str(), &[]);
        let init_res: InitResponse = init(&mut deps, init_env, init_msg).unwrap();
        assert_eq!(0, init_res.messages.len());

        let handle_env = mock_env(&deps.api, creator.as_str(), &[]);
        // Note: we need to use the production-call, not the testing call (which unwraps any vm error)
        let handle_res = call_handle::<_, _, _, Empty>(
            &mut deps,
            &handle_env,
            &to_vec(&HandleMsg::StorageLoop {}).unwrap(),
        );
        assert!(handle_res.is_err());
        assert_eq!(deps.get_gas_left(), 0);
    }

    #[test]
    fn handle_memory_loop() {
        let mut deps = mock_instance(WASM, &[]);

        let (init_msg, creator) = make_init_msg();
        let init_env = mock_env(&deps.api, creator.as_str(), &[]);
        let init_res: InitResponse = init(&mut deps, init_env, init_msg).unwrap();
        assert_eq!(0, init_res.messages.len());

        let handle_env = mock_env(&deps.api, creator.as_str(), &[]);
        // Note: we need to use the production-call, not the testing call (which unwraps any vm error)
        let handle_res = call_handle::<_, _, _, Empty>(
            &mut deps,
            &handle_env,
            &to_vec(&HandleMsg::MemoryLoop {}).unwrap(),
        );
        assert!(handle_res.is_err());
        assert_eq!(deps.get_gas_left(), 0);

        // Ran out of gas before consuming a significant amount of memory
        assert!(deps.get_memory_size() < 2 * 1024 * 1024);
    }

    #[test]
    fn handle_allocate_large_memory() {
        let mut deps = mock_instance(WASM, &[]);

        let (init_msg, creator) = make_init_msg();
        let init_env = mock_env(&deps.api, creator.as_str(), &[]);
        let init_res: InitResponse = init(&mut deps, init_env, init_msg).unwrap();
        assert_eq!(0, init_res.messages.len());

        let handle_env = mock_env(&deps.api, creator.as_str(), &[]);
        let gas_before = deps.get_gas_left();
        // Note: we need to use the production-call, not the testing call (which unwraps any vm error)
        let handle_res = call_handle::<_, _, _, Empty>(
            &mut deps,
            &handle_env,
            &to_vec(&HandleMsg::AllocateLargeMemory {}).unwrap(),
        );
        let gas_used = gas_before - deps.get_gas_left();

        // TODO: this must fail, see https://github.com/CosmWasm/cosmwasm/issues/81
        assert_eq!(handle_res.is_err(), false);

        // Gas consumtion is relatively small
        // Note: the exact gas usage depends on the Rust version used to compile WASM,
        // which we only fix when using cosmwasm-opt, not integration tests.
        let expected = 42000; // +/- 20%
        assert!(gas_used > expected * 80 / 100, "Gas used: {}", gas_used);
        assert!(gas_used < expected * 120 / 100, "Gas used: {}", gas_used);

        // Used between 100 and 102 MiB of memory
        assert!(deps.get_memory_size() > 100 * 1024 * 1024);
        assert!(deps.get_memory_size() < 102 * 1024 * 1024);
    }
}
