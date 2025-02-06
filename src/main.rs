use std::fmt::{Debug, Display};

use tokio::task::JoinError;
use zero2prod::{
    configuration::get_configuration,
    idempotency_purge_worker, issue_delivery_worker,
    startup::Application,
    telementry::{get_subscriber, init_subscriber},
};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read configuration.");
    println!(
        "Server is running on {}:{}",
        &configuration.application.base_url, configuration.application.port
    );
    let app = Application::build(configuration.clone()).await?;

    let application_task = tokio::spawn(app.run());
    let issue_delivery_worker_task = tokio::spawn(
        issue_delivery_worker::run_worker_loop_until_stopped(configuration.clone()),
    );
    let idempotency_purge_worker_task = tokio::spawn(
        idempotency_purge_worker::run_worker_loop_until_stopped(configuration, 10),
    );

    tokio::select! {
      o = application_task => report_exit("application worker", o),
      o = issue_delivery_worker_task => report_exit("issue delivery worker", o),
      o = idempotency_purge_worker_task => report_exit("idempotency purge worker", o)
    }

    Ok(())
}

fn report_exit(task_name: &str, outcome: Result<Result<(), impl Debug + Display>, JoinError>) {
    match outcome {
        Ok(Ok(())) => tracing::info!("{} has exited", task_name),
        Ok(Err(e)) => {
            tracing::error!(
              error.cause_chain=?e,
              error.message=%e,
              "{} failed",
              task_name
            )
        }
        Err(e) => {
            tracing::error!(
              error.cause_chain=?e,
              error.message=%e,
              "{}'s task failed to complete",
              task_name
            )
        }
    }
}
