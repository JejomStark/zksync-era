use anyhow::{bail, Context};
use clap::Args as ClapArgs;
use dialoguer::{theme::ColorfulTheme, Input};
use prover_dal::{Connection, ConnectionPool, Prover, ProverDal};
use zksync_config::PostgresConfig;
use zksync_env_config::FromEnv;
use zksync_types::L1BatchNumber;

#[derive(ClapArgs)]
pub(crate) struct Args {
    #[clap(short, long, conflicts_with = "all")]
    batch: Option<L1BatchNumber>,
    #[clap(short, long, conflicts_with = "batch", default_value_t = false)]
    all: bool,
    #[clap(short, long, default_value_t = false)]
    failed: bool,
}

pub(crate) async fn run(args: Args) -> anyhow::Result<()> {
    if !args.all && args.batch.is_none() {
        bail!("Either --all or --batch should be provided");
    }

    let confirmation = Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("Are you sure you want to delete the data?")
        .default("yes".to_owned())
        .interact_text()
        .unwrap();

    if confirmation != "yes" {
        println!("Aborted");
        return Ok(());
    }

    let config = PostgresConfig::from_env()?;
    let prover_connection_pool =
        ConnectionPool::<Prover>::builder(config.prover_url()?, config.max_connections()?)
            .build()
            .await
            .context("failed to build a prover_connection_pool")?;
    let conn = prover_connection_pool.connection().await.unwrap();

    if args.all {
        delete_prover_db(conn).await?;
    } else {
        delete_batch_data(conn, args.batch.unwrap(), args.failed).await?;
    }

    Ok(())
}

async fn delete_prover_db(mut conn: Connection<'_, Prover>) -> anyhow::Result<()> {
    conn.fri_gpu_prover_queue_dal().delete().await;
    conn.fri_prover_jobs_dal().delete().await;
    conn.fri_scheduler_dependency_tracker_dal().delete().await;
    conn.fri_protocol_versions_dal().delete().await;
    conn.fri_proof_compressor_dal().delete().await;
    conn.fri_witness_generator_dal().delete().await;
    Ok(())
}

async fn delete_batch_data(
    mut conn: Connection<'_, Prover>,
    block_number: L1BatchNumber,
    _failed: bool,
) -> anyhow::Result<()> {
    conn.fri_proof_compressor_dal()
        .delete_batch_data(block_number)
        .await;
    conn.fri_prover_jobs_dal()
        .delete_batch_data(block_number)
        .await;
    conn.fri_witness_generator_dal()
        .delete_batch_data(block_number)
        .await;
    Ok(())
}
