#![allow(incomplete_features)] // We have to use generic const exprs.
#![feature(generic_const_exprs)]

use std::time::Instant;

use serde::Serialize;
use zksync_config::{configs::object_store::ObjectStoreMode, ObjectStoreConfig};
use zksync_object_store::ObjectStoreFactory;
use zksync_prover_fri_types::keys::AggregationsKey;
use zksync_prover_fri_utils::get_recursive_layer_circuit_id_for_base_layer;
use zksync_types::{
    prover_dal::{LeafAggregationJobMetadata, NodeAggregationJobMetadata},
    L1BatchNumber,
};
use zksync_witness_generator::{
    basic_circuits::generate_witness,
    leaf_aggregation::{prepare_leaf_aggregation_job, LeafAggregationWitnessGenerator},
    node_aggregation::{self, NodeAggregationWitnessGenerator},
    utils::AggregationWrapper,
};

fn compare_serialized<T: Serialize>(expected: &T, actual: &T) {
    let serialized_expected = bincode::serialize(expected).unwrap();
    let serialized_actual = bincode::serialize(actual).unwrap();
    assert_eq!(serialized_expected, serialized_actual);
}

#[tokio::test]
#[ignore] // re-enable with new artifacts
async fn test_leaf_witness_gen() {
    let object_store_config = ObjectStoreConfig {
        mode: ObjectStoreMode::FileBacked {
            file_backed_base_path: "./tests/data/leaf/".to_owned(),
        },
        max_retries: 5,
        local_mirror_path: None,
    };
    let object_store = ObjectStoreFactory::new(object_store_config)
        .create_store()
        .await
        .unwrap();

    let circuit_id = 4;
    let block_number = L1BatchNumber(125010);
    let key = AggregationsKey {
        block_number,
        circuit_id: get_recursive_layer_circuit_id_for_base_layer(circuit_id),
        depth: 0,
    };
    let expected_aggregation = object_store
        .get::<AggregationWrapper>(key)
        .await
        .expect("expected aggregation missing");
    let leaf_aggregation_job_metadata = LeafAggregationJobMetadata {
        id: 1,
        block_number,
        circuit_id,
        prover_job_ids_for_proofs: vec![4639043, 4639044, 4639045],
    };

    let job = prepare_leaf_aggregation_job(leaf_aggregation_job_metadata, &*object_store)
        .await
        .unwrap();

    let artifacts = LeafAggregationWitnessGenerator::process_job_sync(job, Instant::now());
    let aggregations = AggregationWrapper(artifacts.aggregations);
    compare_serialized(&expected_aggregation, &aggregations);
}

#[tokio::test]
#[ignore] // re-enable with new artifacts
async fn test_node_witness_gen() {
    let object_store_config = ObjectStoreConfig {
        mode: ObjectStoreMode::FileBacked {
            file_backed_base_path: "./tests/data/node/".to_owned(),
        },
        max_retries: 5,
        local_mirror_path: None,
    };
    let object_store = ObjectStoreFactory::new(object_store_config)
        .create_store()
        .await
        .unwrap();

    let circuit_id = 8;
    let block_number = L1BatchNumber(127856);
    let key = AggregationsKey {
        block_number,
        circuit_id,
        depth: 1,
    };
    let expected_aggregation = object_store
        .get::<AggregationWrapper>(key)
        .await
        .expect("expected aggregation missing");
    let node_aggregation_job_metadata = NodeAggregationJobMetadata {
        id: 1,
        block_number,
        circuit_id,
        depth: 0,
        prover_job_ids_for_proofs: vec![5211320],
    };

    let job = node_aggregation::prepare_job(node_aggregation_job_metadata, &*object_store)
        .await
        .unwrap();

    let artifacts = NodeAggregationWitnessGenerator::process_job_sync(job, Instant::now());
    let aggregations = AggregationWrapper(artifacts.next_aggregations);
    compare_serialized(&expected_aggregation, &aggregations);
}

#[tokio::test]
async fn test_basic_witness_gen() {
    let object_store_config = ObjectStoreConfig {
        mode: ObjectStoreMode::FileBacked {
            file_backed_base_path: "./tests/data/bwg/".to_owned(),
        },
        max_retries: 5,
        local_mirror_path: None,
    };
    let object_store = ObjectStoreFactory::new(object_store_config)
        .create_store()
        .await
        .unwrap();

    let block_number = L1BatchNumber(489509);

    let input = object_store.get(block_number).await.unwrap();

    let instant = Instant::now();

    snapshot_prof("Before run");
    let (_circuit_urls, _queue_urls, _scheduler_witness, _aux_output_witness) =
        generate_witness(block_number, &*object_store, input).await;
    snapshot_prof("After run");
    println!("Generated witness, {:?}", instant.elapsed());
    print_peak_mem_snapshots();
}

use std::{
    sync::{Mutex, OnceLock},
    time::Duration,
};

use peak_alloc::PeakAlloc;

#[global_allocator]
static PEAK_ALLOC: PeakAlloc = PeakAlloc;

fn mem_array() -> &'static Mutex<Vec<(String, f32)>> {
    static MEM_ARRAY: OnceLock<Mutex<Vec<(String, f32)>>> = OnceLock::new();
    MEM_ARRAY.get_or_init(|| Mutex::new(vec![]))
}

fn time_instant() -> &'static Mutex<Instant> {
    static TIME_INSTANT: OnceLock<Mutex<Instant>> = OnceLock::new();
    TIME_INSTANT.get_or_init(|| Mutex::new(Instant::now()))
}

fn time_array() -> &'static Mutex<Vec<(String, Duration)>> {
    static TIME_ARRAY: OnceLock<Mutex<Vec<(String, Duration)>>> = OnceLock::new();
    TIME_ARRAY.get_or_init(|| Mutex::new(vec![]))
}

fn peak_mem_array() -> &'static Mutex<Vec<(String, f32)>> {
    static PEAK_MEM_ARRAY: OnceLock<Mutex<Vec<(String, f32)>>> = OnceLock::new();
    PEAK_MEM_ARRAY.get_or_init(|| Mutex::new(vec![]))
}

fn peak_snapshot_mem(label: &str) {
    let peak_mem = PEAK_ALLOC.peak_usage_as_mb();
    println!("PEAK MEM: {}: {}", label.to_owned(), peak_mem);
    peak_mem_array()
        .lock()
        .unwrap()
        .push((label.to_owned(), peak_mem));
}

pub fn print_mem_snapshots() {
    let mem = mem_array().lock().unwrap();
    println!("MEMORY SNAPSHOTS");
    for snapshot in mem.clone() {
        println!("{}: {}", snapshot.0, snapshot.1);
    }
    println!();
}

pub fn print_peak_mem_snapshots() {
    let mem = peak_mem_array().lock().unwrap();
    println!("PEAK MEMORY SNAPSHOTS");
    for snapshot in mem.clone() {
        println!("{}: {}", snapshot.0, snapshot.1);
    }
    println!();
}

pub fn print_time_snapshots() {
    let time = time_array().lock().unwrap();
    println!("TIME SNAPSHOTS");
    for snapshot in time.clone() {
        println!("{}: {:.2?}", snapshot.0, snapshot.1);
    }
    println!();
}

pub fn snapshot_mem(label: &str) {
    let current_mem = PEAK_ALLOC.current_usage_as_mb();
    println!("MEM: {}: {}", label.to_owned(), current_mem);
    mem_array()
        .lock()
        .unwrap()
        .push((label.to_owned(), current_mem));
}

pub fn snapshot_prof(label: &str) {
    snapshot_mem(label);
    peak_snapshot_mem(label);
    snapshot_time(label);
    reset_peak_snapshot_mem();
}

pub fn snapshot_time(label: &str) {
    let mut instant = time_instant().lock().unwrap();
    println!("TIME: {}: {:.2?}", label.to_owned(), instant.elapsed());
    time_array()
        .lock()
        .unwrap()
        .push((label.to_owned(), instant.elapsed()));
    *instant = Instant::now();
}

pub fn reset_peak_snapshot_mem() {
    PEAK_ALLOC.reset_peak_usage();
}
