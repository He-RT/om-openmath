//! Sequential fixture: Rust makes genuine candidates from the prior actual Swift persisted receipt.
//! Output files are test-owned artifacts. Preparation never pretends to be durable acceptance.
use om_host_service::{
    document::SourceDocument,
    kernel::{
        acceptance::{
            AcceptanceContext, AcceptedKernelState, FrozenKernelPlan, KernelRecovery,
            decode_commit, decode_receipt,
        },
        worker::{KernelJob, KernelWorker},
    },
    protocol::{Serial, generated::*},
};
use om_kernel::{
    checkpoint::{CheckpointLimits, CheckpointRestore},
    config::GeneralConfig,
    protocol::*,
};
use om_num::ctx::Interrupt;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};
fn write<T: serde::Serialize>(path: impl AsRef<Path>, value: &T) {
    std::fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}
fn read<T: serde::de::DeserializeOwned>(path: impl AsRef<Path>) -> T {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn serial(n: u64) -> Serial {
    Serial::new(n).unwrap()
}
fn scalar(state: &AcceptedKernelState, source: &str) -> String {
    let mut work = state
        .state()
        .restore(
            CheckpointRestore {
                binding: state.state().binding(),
                source: state.state().source(),
                general: state.state().general(),
                clock: None,
                cancel: Arc::new(AtomicBool::new(false)),
            },
            CheckpointLimits::default(),
            &Interrupt::default(),
        )
        .unwrap();
    let Response::Evaluated { output, .. } = work
        .session_mut()
        .handle(Request::Evaluate {
            cell_id: "isolated-check".into(),
            source: source.into(),
            dialect: om_kernel::protocol::Dialect::Modern,
        })
        .0
    else {
        panic!("no output")
    };
    let OutputItem::Expr { input_form, .. } = output.items.last().unwrap() else {
        panic!("not scalar {output:?}")
    };
    input_form.clone()
}
fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let mode = &args[1];
    let folder = PathBuf::from(&args[2]);
    if mode == "seed" {
        std::fs::create_dir_all(&folder).unwrap();
        let file = NativeSourceFile {
            version: 1,
            title: "真实耐久计算🙂".into(),
            cells: vec![
                NativeSourceCell {
                    id: "a".into(),
                    kind: NativeCellKind::Math,
                    source: "let a=2".into(),
                    dialect: om_host_service::protocol::generated::Dialect::Modern,
                },
                NativeSourceCell {
                    id: "b".into(),
                    kind: NativeCellKind::Math,
                    source: "a+1".into(),
                    dialect: om_host_service::protocol::generated::Dialect::Modern,
                },
                NativeSourceCell {
                    id: "error".into(),
                    kind: NativeCellKind::Math,
                    source: "partial=7; loop:=loop; loop; never=8".into(),
                    dialect: om_host_service::protocol::generated::Dialect::Wolfram,
                },
            ],
        };
        let owner = SourceDocument::new(
            "00000000-0000-0000-0000-000000000409".into(),
            serial(1),
            file,
        )
        .unwrap();
        write(folder.join("source.json"), owner.snapshot());
        return;
    }
    let stage = &args[3];
    let target = folder.join(stage);
    std::fs::create_dir_all(&target).unwrap();
    let source: NativeSourceSnapshot = read(folder.join("source.json"));
    let info: serde_json::Value = read(folder.join("store-info.json"));
    let store = info["identity"]["store_id"].as_str().unwrap();
    let general = GeneralConfig {
        auto_run_dependents: false,
        ..Default::default()
    };
    let worker = KernelWorker::new(64 * 1024 * 1024, 32, CheckpointLimits::default()).unwrap();
    let context = AcceptanceContext {
        runtime_instance_id: "actual-kernel-store-fixture".into(),
        store_id: store.into(),
        document_generation: serial(1),
        source,
        general: general.clone(),
        config_revision: serial(0),
        build: "actual-kernel-store-build".into(),
    };
    if stage == "bootstrap" && mode != "verify" {
        let frozen = FrozenKernelPlan::bootstrap(
            &context,
            "bootstrap-operation",
            &worker,
            CheckpointLimits::default(),
            &Interrupt::default(),
        )
        .unwrap();
        write(target.join("plan.json"), frozen.plan());
        std::fs::write(target.join("checkpoint.bin"), frozen.state().bytes()).unwrap();
    } else {
        let previous = &args[4];
        let previous = folder.join(previous);
        let plan = decode_commit(&std::fs::read(previous.join("plan.json")).unwrap()).unwrap();
        let receipt =
            decode_receipt(&std::fs::read(previous.join("receipt.json")).unwrap()).unwrap();
        let parent = AcceptedKernelState::recover(
            KernelRecovery {
                plan: &plan,
                receipt: &receipt,
                bytes: std::fs::read(previous.join("checkpoint.bin")).unwrap(),
                general,
                expected_build: &context.build,
            },
            &worker,
            CheckpointLimits::default(),
            &Interrupt::default(),
        )
        .unwrap();
        if mode == "verify" {
            let expected = match stage.as_str() {
                "a" => ("a", "2"),
                "b" => ("a+1", "3"),
                "error" => ("partial", "7"),
                _ => ("a", "a"),
            };
            assert_eq!(scalar(&parent, expected.0), expected.1);
            assert_eq!(parent.receipt().operation_id, plan.operation_id);
            println!(
                "Actual Swift same-DB receipt and original OMKS bytes restored by Rust: {stage}, {}={}, revision {}",
                expected.0,
                expected.1,
                parent.receipt().kernel_state_revision.get()
            );
        } else {
            let job = KernelJob {
                runtime_instance_id: context.runtime_instance_id.clone(),
                document_generation: context.document_generation,
                operation_id: format!("operation-{stage}"),
                parent_checkpoint_ref: parent.registry_ref().into(),
                source: context.source.clone(),
                general: context.general.clone(),
                config_revision: context.config_revision,
                cell_id: stage.into(),
                cancel: Arc::new(AtomicBool::new(false)),
            };
            let candidate = worker
                .submit(job)
                .unwrap()
                .recv_timeout(Duration::from_secs(10))
                .unwrap()
                .unwrap();
            let frozen =
                FrozenKernelPlan::candidate(candidate, &parent, &context, &worker).unwrap();
            write(target.join("plan.json"), frozen.plan());
            std::fs::write(target.join("checkpoint.bin"), frozen.state().bytes()).unwrap();
        }
    }
    if let Some(owner) = worker.begin_close().unwrap() {
        owner.join().unwrap();
    }
}
