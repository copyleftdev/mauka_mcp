//! Node implementation for the Kahuna Lock-Free Queue.
//!
//! This module provides the Node structure used in the Kahuna Queue implementation.
//! Nodes are the fundamental building blocks of the lock-free queue, each containing
//! a value and an atomic reference to the next node.

use std::sync::atomic::{AtomicPtr, Ordering};
use std::cell::UnsafeCell;

/// A node in the Kahuna Lock-Free Queue.
///
/// Each node contains a value and an atomic reference to the next node.
/// The value is wrapped in UnsafeCell to allow interior mutability for
/// the take operation in a lock-free context.
///
/// # Type Parameters
///
/// * `T` - Type of the value stored in the node. Must be `Send + Sync`.
#[derive(Debug)]
pub struct Node<T: Send + Sync> {
    /// The value stored in this node, wrapped in UnsafeCell for interior mutability
    pub(crate) value: UnsafeCell<Option<T>>,
    
    /// Reference to the next node in the queue, wrapped in AtomicPtr for atomic operations
    pub(crate) next: AtomicPtr<Node<T>>,
}

impl<T: Send + Sync> Node<T> {
    /// Creates a new node with the given value.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to store in the node.
    ///
    /// # Returns
    ///
    /// A new `Node<T>` instance containing the value.
    pub fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::new(Some(value)),
            next: AtomicPtr::new(std::ptr::null_mut()),
        }
    }

    /// Creates a new sentinel (dummy) node with no value.
    ///
    /// Sentinel nodes are used as empty head nodes to simplify the queue implementation.
    ///
    /// # Returns
    ///
    /// A new sentinel `Node<T>` instance.
    pub fn sentinel() -> Self {
        Self {
            value: UnsafeCell::new(None),
            next: AtomicPtr::new(std::ptr::null_mut()),
        }
    }

    /// Takes the value from this node, if it exists.
    ///
    /// This method is used during the pop operation to extract the value
    /// from the node in a lock-free manner.
    ///
    /// # Safety
    ///
    /// This method may only be called when the node is being removed from the
    /// queue, ensuring no other thread can access this node.
    ///
    /// # Returns
    ///
    /// The value that was stored in this node, if any.
    pub(crate) fn take(&self) -> Option<T> {
        // SAFETY: This is safe because:
        // 1. We only call this when removing the node from the queue
        // 2. The queue's pop operation ensures exclusive access to this node
        unsafe {
            let value_ptr = self.value.get();
            (*value_ptr).take()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_new() {
        let node = Node::new(42);
        
        unsafe {
            assert_eq!(*node.value.get(), Some(42));
        }
        
        assert!(node.next.load(Ordering::Relaxed).is_null());
    }

    #[test]
    fn test_node_sentinel() {
        let node: Node<i32> = Node::sentinel();
        
        unsafe {
            assert_eq!(*node.value.get(), None);
        }
        
        assert!(node.next.load(Ordering::Relaxed).is_null());
    }

    #[test]
    fn test_node_take() {
        let node = Node::new(42);
        
        let value = node.take();
        assert_eq!(value, Some(42));
        
        // The value should be gone after taking it
        let value = node.take();
        assert_eq!(value, None);
    }
}
