use std::sync::LazyLock;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

static INIT: LazyLock<std::sync::Mutex<bool>> = LazyLock::new(|| std::sync::Mutex::new(false));

// just initialize once for all threads
pub fn tracing_init() {
    let mut init = INIT.lock().unwrap();

    if !*init {
        *init = true;

        tracing_subscriber::registry()
            .with(
                tracing_subscriber::fmt::layer()
                    .without_time()
                    .with_target(false),
            )
            .with(tracing_subscriber::EnvFilter::from_default_env())
            .try_init()
            .unwrap();
    }
}
