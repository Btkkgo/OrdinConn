//! Read-only production zero-provider gate. Never initializes the application or migrates.
use mobile_runtime::execution::{MobileGoalErrorCode, MobileGoalId};
use ordinconn_app::{AppError, events::RuntimeEventBus, mobile_planner::MobilePlanner};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::var("ORDINCONN_PLANNER_GATE_DB")?;
    let options = SqliteConnectOptions::new()
        .filename(path)
        .read_only(true)
        .create_if_missing(false);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM model_providers")
        .fetch_one(&pool)
        .await?;
    println!("Production Provider Count: {count}");
    // Only the count query is permitted on production. Refuse to inspect configured rows.
    if count != 0 {
        println!("NOT_RUN: configured provider requires owner-selected live acceptance");
        return Ok(());
    }
    let planner = MobilePlanner::new(
        pool.clone(),
        RuntimeEventBus::default(),
        tokio_util::sync::CancellationToken::new(),
    );
    let result = planner
        .plan_mobile_goal(&MobileGoalId::new(), None, |_| {
            panic!("zero-provider gate cannot read secrets")
        })
        .await;
    match result {
        Err(AppError::MobileGoal(e)) if e.code == MobileGoalErrorCode::ModelNotConfigured => {
            println!(
                "plan_mobile_goal: MODEL_NOT_CONFIGURED; no Goal/Plan/Step writes or migration"
            )
        }
        _ => return Err("zero-provider gate failed".into()),
    }
    pool.close().await;
    Ok(())
}
