use std::sync::Arc;

pub struct ArcIter<T> {
    current: usize,
    items: Arc<[T]>,
}

impl<T: Clone> Iterator for ArcIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        let next = self.items.get(self.current)?;
        self.current += 1;
        Some(next.clone())
    }
}

impl<T> ArcIter<T> {
    pub fn new(items: Arc<[T]>) -> Self {
        Self { current: 0, items }
    }
}
