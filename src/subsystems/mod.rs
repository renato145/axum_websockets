use tokio::sync::oneshot;

struct Task {}

trait Subsystem {
    fn new() -> oneshot::Sender<Task>;
}
