use crate::MajordomeApp;
use std::sync::{atomic::AtomicBool, OnceLock};
use tokio::sync::Mutex;

/// Signal handling for the app.
/// This is used to stop the app gracefully.
/// When a SIGINT or SIGTERM is received, the app will begin it's EXIT process:
/// - is_exiting will return true.
/// - sleep will return immediately.
/// - @stop handlers will be called for all modules.
/// Then the app will begin it's CLOSING process:
/// - is_closing will return true.
/// - we will wait for
pub struct MajordomeSignal {
    // When a SIGINT or SIGTERM is received, this is set to true.
    // Use this on main app to know when to stop.
    pub(crate) is_exiting: AtomicBool,
    pub(crate) is_exiting_channel: (
        Mutex<Option<tokio::sync::broadcast::Sender<()>>>,
        tokio::sync::broadcast::Receiver<()>,
    ),

    // When the app.stop() method is called, this is set to true.
    // Use this on modules to know when to stop.
    pub(crate) is_closing: AtomicBool,
    pub(crate) is_closing_channel: (
        Mutex<Option<tokio::sync::broadcast::Sender<()>>>,
        tokio::sync::broadcast::Receiver<()>,
    ),
}

impl MajordomeSignal {
    pub fn new() -> Self {
        let (tx_e, rx_e) = tokio::sync::broadcast::channel(1);
        let (tx_c, rx_c) = tokio::sync::broadcast::channel(1);

        MajordomeSignal {
            is_exiting: AtomicBool::new(false),
            is_exiting_channel: (Mutex::new(Some(tx_e)), rx_e),
            is_closing: AtomicBool::new(false),
            is_closing_channel: (Mutex::new(Some(tx_c)), rx_c),
        }
    }
}

impl MajordomeApp {
    /// exiting -> closing -> terminated
    pub fn is_exiting(&self) -> bool {
        self.signal
            .is_exiting
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    pub(crate) fn _start_exiting_probe(&self) {
        use tokio::signal;
        let s = self.clone();

        tokio::spawn(async move {
            #[cfg(windows)]
            signal::ctrl_c().await.unwrap();

            #[cfg(unix)]
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .unwrap()
                .recv()
                .await;

            s.signal
                .is_exiting
                .store(true, std::sync::atomic::Ordering::SeqCst);

            // We drop the sender to signal the exit.
            // This will allow all the sleeping tasks to wake up.
            drop(s.signal.is_exiting_channel.0.lock().await.take());

            println!("🛑 Exit signal received.");
        });
    }

    pub fn is_closing(&self) -> bool {
        self.signal
            .is_closing
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Sleep for a duration.
    /// Stops if the app is exiting if !ignore_exit else if the app is closing.
    /// Use ignore_exit if you are inside a Module, NEVER use it on the main app.
    pub async fn sleep_until_closing(&self, duration: std::time::Duration, ignore_exit: bool) {
        let chan = if ignore_exit {
            &self.signal.is_closing_channel
        } else {
            &self.signal.is_exiting_channel
        };
        
        let mut rx = chan.1.resubscribe();
        tokio::select! {
            _ = tokio::time::sleep(duration) => {},
            _ = rx.recv() => {},
        }
    }

    /// Wait for the app to exit/stop.
    /// Same as sleep_until_closing but without a duration.
    /// Use ignore_exit if you are inside a Module, NEVER use it on the main app.
    pub async fn wait_until_closing(&self, ignore_exit: bool) {
        let chan = if ignore_exit {
            &self.signal.is_closing_channel
        } else {
            &self.signal.is_exiting_channel
        };

        let mut rx = chan.1.resubscribe();
        let _ = rx.recv().await;
    }

    /// Wait for the app to exit/stop.
    /// Same as wait_until_closing but with a static lifetime.
    /// Only use this if you need a static lifetime, it is not recommended.
    /// Use ignore_exit if you are inside a Module, NEVER use it on the main app.
    pub fn wait_for_shutdown_static(&self, ignore_exit: bool) -> impl std::future::Future<Output = ()> {
        static APP: OnceLock<(MajordomeApp, bool)> = OnceLock::new();
        let _ = APP.set((self.clone(), ignore_exit));
    
        async fn wait() {
            let (app, ignore_exit) = APP.get().unwrap();
            app.wait_until_closing(*ignore_exit).await;
        }
    
        return wait();
    }

    /// Stop the app.
    /// Should be called at the end of the main function.
    pub async fn stop(self) {
        self.signal
            .is_closing
            .store(true, std::sync::atomic::Ordering::SeqCst);
        drop(self.signal.is_closing_channel.0.lock().await.take());
        crate::module::builder::stop_modules(self.clone()).await;
    }
}


