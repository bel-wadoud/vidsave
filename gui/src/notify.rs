//! Desktop notifications for things worth knowing about even when the
//! window isn't focused: a download batch finishing, an update becoming
//! available. Best-effort -- a machine with no notification daemon (or a
//! misconfigured one) just means no popup, never a crash or an error the
//! user has to deal with, so failures are only logged to stderr.

pub fn notify(summary: impl Into<String>, body: impl Into<String>) {
    let summary = summary.into();
    let body = body.into();
    tokio::spawn(async move {
        let result = tokio::task::spawn_blocking(move || {
            notify_rust::Notification::new()
                .appname("VidSave")
                .summary(&summary)
                .body(&body)
                .show()
        })
        .await;
        match result {
            Ok(Ok(_handle)) => {}
            Ok(Err(e)) => eprintln!("could not show a desktop notification: {e:#}"),
            Err(e) => eprintln!("notification task panicked: {e:#}"),
        }
    });
}
