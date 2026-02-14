use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// A generic batch buffer that collects items and flushes when count triggers.
#[allow(dead_code)]
pub(crate) struct BatchBuffer<T: Send + 'static> {
    items: Arc<Mutex<Vec<T>>>,
    max_size: usize,
    tx: mpsc::UnboundedSender<Vec<T>>,
}

#[allow(dead_code)]
impl<T: Send + 'static> BatchBuffer<T> {
    pub fn new(max_size: usize, tx: mpsc::UnboundedSender<Vec<T>>) -> Self {
        Self {
            items: Arc::new(Mutex::new(Vec::new())),
            max_size,
            tx,
        }
    }

    pub fn push(&self, item: T) {
        let mut items = self.items.lock().unwrap();
        items.push(item);
        if items.len() >= self.max_size {
            let batch = std::mem::take(&mut *items);
            let _ = self.tx.send(batch);
        }
    }

    pub fn drain(&self) -> Vec<T> {
        let mut items = self.items.lock().unwrap();
        std::mem::take(&mut *items)
    }
}
