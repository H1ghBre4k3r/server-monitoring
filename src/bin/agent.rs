use guardia::{
    ComponentInformation, ComponentOverview, CpuInformation, CpuOverview, MemoryInformation,
    ServerMetrics, SystemInformation,
    util::{get_addr, get_port, get_secret},
};
use rocket::{
    figment::Figment,
    get,
    http::Status,
    launch,
    request::{FromRequest, Outcome},
    routes,
    serde::json::Json,
};
use sysinfo::{Components, System};
use tracing::{error, instrument};
use tracing_subscriber::{filter, layer::SubscriberExt, util::SubscriberInitExt};

#[get("/metrics")]
#[instrument]
fn metrics(secret: SecretKey) -> Json<ServerMetrics> {
    let mut sys = System::new_all();
    sys.refresh_all();
    std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    sys.refresh_all();

    error!("warn");

    let cpus = sys.cpus();
    let components = Components::new_with_refreshed_list();

    let m = ServerMetrics {
        system: SystemInformation {
            name: System::name(),
            kernel_version: System::kernel_version(),
            os_version: System::os_version(),
            host_name: System::host_name(),
        },
        memory: MemoryInformation {
            total: sys.total_memory(),
            used: sys.used_memory(),
            total_swap: sys.total_swap(),
            used_swap: sys.used_swap(),
        },
        cpus: {
            let total_cpus = cpus.len() as f32;
            let cpu_usage_sum = cpus.iter().map(|cpu| cpu.cpu_usage()).sum::<f32>();

            CpuOverview {
                total: cpus.len(),
                arch: System::cpu_arch(),
                average_usage: cpu_usage_sum / total_cpus,
                cpus: cpus
                    .iter()
                    .map(|cpu| CpuInformation {
                        name: cpu.name().to_string(),
                        frequency: cpu.frequency(),
                        usage: cpu.cpu_usage(),
                    })
                    .collect(),
            }
        },
        components: {
            let component_count = components.len() as f32;
            let component_temperature_sum = components
                .iter()
                .map(|component| component.temperature().unwrap_or(0.0))
                .sum::<f32>();

            ComponentOverview {
                average_temperature: Some(component_temperature_sum / component_count),
                components: components
                    .iter()
                    .map(|component| ComponentInformation {
                        name: component.label().to_string(),
                        temperature: component.temperature(),
                    })
                    .collect(),
            }
        },
    };

    Json(m)
}

#[get("/ping")]
fn ping() {}

fn init() {
    dotenv::dotenv().ok();

    let _filter =
        filter::Targets::new().with_target("agent", tracing::metadata::LevelFilter::TRACE);
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .compact()
                .pretty()
                .with_ansi(true),
        )
        .with(filter::LevelFilter::DEBUG)
        .init();
}

fn get_config() -> Figment {
    rocket::Config::figment()
        .merge(("port", get_port()))
        .merge(("address", get_addr()))
        .merge(("workers", 1))
}

#[launch]
fn rocket() -> _ {
    init();
    let figment = get_config();

    rocket::custom(figment).mount("/", routes![metrics, ping])
}

#[derive(Debug)]
struct SecretKey;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for SecretKey {
    type Error = ();

    async fn from_request(
        request: &'r rocket::Request<'_>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        let header = request.headers().get_one("X-MONITORING-SECRET");
        let secret = get_secret();
        if let Some(secret) = secret {
            if let Some(passed_secret) = header
                && passed_secret == secret
            {
                Outcome::Success(SecretKey)
            } else {
                Outcome::Error((Status::Unauthorized, ()))
            }
        } else {
            Outcome::Success(SecretKey)
        }
    }
}
