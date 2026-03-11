use tracing::Level;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub fn init_logging(level: Level) {
    let env_filter = EnvFilter::builder()
        .with_default_directive(level.into())
        .parse_lossy("");

    let fmt_layer = {
        #[cfg(debug_assertions)]
        {
            tracing_subscriber::fmt::layer()
                .with_file(true)
                .with_line_number(true)
                .with_level(true)
                .with_target(false)
        }
        #[cfg(not(debug_assertions))]
        {
            tracing_subscriber::fmt::layer()
                .with_thread_ids(false)
                .with_thread_names(false)
                .with_file(false)
                .with_line_number(false)
                .with_level(true)
                .with_target(false)
        }
    };
    let registry = tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer);

    #[cfg(not(feature = "tracy"))]
    {
        registry.init();
    }

    #[cfg(feature = "tracy")]
    {
        let tracy_layer = tracing_tracy::TracyLayer::default();
        let registry = registry.with(tracy_layer);
        registry.init();
    }
}
