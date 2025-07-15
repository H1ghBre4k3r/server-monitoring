use rocket::{get, launch, routes, serde::json::Json};
use server_monitoring::{
    ComponentInformation, CpuInformation, CpuOverview, MemoryInformation, ServerMetrics,
    SystemInformation,
};
use sysinfo::{Components, System};

#[get("/metrics")]
fn index() -> Json<ServerMetrics> {
    let mut sys = System::new_all();
    sys.refresh_all();
    std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    sys.refresh_all();

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
        cpus: CpuOverview {
            total: cpus.len(),
            arch: System::cpu_arch(),
            cpus: cpus
                .iter()
                .map(|cpu| CpuInformation {
                    name: cpu.name().to_string(),
                    frequency: cpu.frequency(),
                    usage: cpu.cpu_usage(),
                })
                .collect(),
        },
        components: components
            .iter()
            .map(|component| ComponentInformation {
                name: component.label().to_string(),
                temperature: component.temperature(),
            })
            .collect(),
    };

    Json(m)
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index])
}
