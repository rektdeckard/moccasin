pub mod sqlite;

pub enum StorageEvent {
    Insert,
    Update,
    Delete,
    NoOp,
}

pub struct StorageError;
