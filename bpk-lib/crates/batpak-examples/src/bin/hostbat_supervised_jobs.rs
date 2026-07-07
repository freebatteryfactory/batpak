//! # hostbat_supervised_jobs
//!
//! **Witnesses:** a [`hostbat::Host`] can DECLARE a client-visible subscription
//! and DECLARE + SPAWN a supervised background job, then run that job to
//! completion over the generic [`batpak::store::Spawn`] seam.
//!
//! The module declares three parts through the public builder:
//! - a [`hostbat::SubscriptionDescriptor`] (a projection-frontier stream) plus
//!   the matching [`hostbat::SchemaDescriptor`] its payload references, and
//! - a supervised job factory ([`hostbat::JobBody`]) registered by kind.
//!
//! The host is built (`ThreadSpawn`-backed supervisor), started, the job is
//! spawned by kind via [`hostbat::Host::spawn_job`], and `shutdown` joins it.
//! We assert the declared subscription is visible through `Host::subscriptions`
//! and that the job body ran exactly once to completion.
//!
//! Run: `cargo run -p batpak-examples --bin hostbat_supervised_jobs`

use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use hostbat::{
    BackpressurePolicy, HostBuilder, HostModule, ProjectionId, SchemaDescriptor, SchemaId,
    SchemaRole, SchemaVersion, SubscriptionDelivery, SubscriptionDescriptor, SubscriptionId,
    SubscriptionSource,
};

const SUBSCRIPTION_ID: &str = "progress.updates.v1";
const PAYLOAD_SCHEMA_REF: &str = "progress.payload.v1";
const JOB_KIND: &str = "progress.sweep";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut out = std::io::stdout().lock();

    // A shared counter the supervised job body increments — the completion proof.
    let ran = Arc::new(AtomicUsize::new(0));

    // -- Declare a subscription + its payload schema + a supervised job --------
    let subscription = SubscriptionDescriptor::new(
        SubscriptionId::new(SUBSCRIPTION_ID)?,
        SubscriptionSource::Projection(ProjectionId::new("progress")?),
        PAYLOAD_SCHEMA_REF,
        SubscriptionDelivery::CursorAtLeastOnce,
        BackpressurePolicy::BoundedQueue { capacity: 64 },
    );
    // The subscription's payload schema must be declared in the composition; a
    // projection stream carries a `SubscriptionPayload`-role schema.
    let payload_schema = SchemaDescriptor::new(
        SchemaId::new(PAYLOAD_SCHEMA_REF)?,
        SchemaVersion(1),
        SchemaRole::SubscriptionPayload,
        Vec::new(),
    )?;

    let job_counter = Arc::clone(&ran);
    let module = HostModule::builder("progress.module", 1)
        .subscription(subscription)?
        .schema(payload_schema)?
        .job(JOB_KIND, move || {
            // Each spawn produces a FRESH body; the supervisor runs it on a
            // thread over the Spawn seam. The body records that it ran.
            let counter = Arc::clone(&job_counter);
            Box::new(move || {
                counter.fetch_add(1, Ordering::AcqRel);
            }) as Box<dyn FnOnce() + Send + 'static>
        })?
        .build()?;

    // -- Compose the host and run the supervised job to completion -------------
    let mut host = HostBuilder::new().mount(module)?.build()?;
    host.start()?;
    assert!(host.is_started(), "host must be started");

    // The declared subscription is visible on the built host interface.
    let declared: Vec<String> = host
        .subscriptions()
        .map(|(module_id, descriptor)| format!("{module_id}:{}", descriptor.id()))
        .collect();
    assert!(
        host.subscriptions()
            .any(|(_, descriptor)| descriptor.id().as_str() == SUBSCRIPTION_ID),
        "the declared subscription must be reachable through Host::subscriptions",
    );
    let _ = writeln!(out, "declared subscription(s): {}", declared.join(", "));

    // Spawn the supervised job by its declared kind.
    host.spawn_job(JOB_KIND)?;
    assert_eq!(
        host.supervisor().job_count(),
        1,
        "the spawned job is tracked by the supervisor",
    );
    let _ = writeln!(out, "spawned supervised job {JOB_KIND:?}");

    // Shutdown joins every supervised job (blocking until each finishes).
    host.shutdown()?;
    assert_eq!(
        ran.load(Ordering::Acquire),
        1,
        "the supervised job body ran exactly once to completion",
    );
    assert_eq!(
        host.supervisor().job_count(),
        0,
        "shutdown drains the supervisor",
    );
    let _ = writeln!(
        out,
        "OK: subscription declared + supervised job ran to completion and joined",
    );

    Ok(())
}
