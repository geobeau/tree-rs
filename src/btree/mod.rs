use std::usize;
use std::{rc::Rc, cell::RefCell};
use std::fmt::Debug;
use arrayvec::ArrayVec;
use rkyv::{Archive, Deserialize, Serialize};

type Key = [u128; 1];
type Value = u8;
type NodePtr = Rc<RefCell<dyn Node>>;

// const NODE_SIZE: usize = 1024 * 4;
const NODE_SIZE: usize = 64 * 4;
const LEAF_ITEMS_SIZE: usize = (NODE_SIZE - 32) / (std::mem::size_of::<Key>() + std::mem::size_of::<Value>());
const INTERNAL_ITEMS_SIZE: usize = (NODE_SIZE - 32) / (std::mem::size_of::<NodePtr>() + std::mem::size_of::<Key>());
const PIVOTS_SIZE: usize = INTERNAL_ITEMS_SIZE - 1;
const CHILDREN_SIZE: usize = INTERNAL_ITEMS_SIZE;


pub trait Node: std::fmt::Debug {
    fn get(&self, key: &Key) -> Option<Value>;
    fn insert(&mut self, key: Key, val: Value);
    fn delete(&mut self, key: &Key) -> bool;
    fn split(&mut self) -> (Key, NodePtr);
    fn get_first_key(&self) -> Key;
    fn total_len(&self) -> usize;
    fn is_full(&self) -> bool;
    fn is_empty(&self) -> bool;
    fn len(&self) -> usize;
    fn pop_first_child(&mut self) -> Option<NodePtr>;
}

#[derive(Debug)]
pub struct InternalNode {
    pivots: ArrayVec<Key, PIVOTS_SIZE>,
    children: ArrayVec<NodePtr, CHILDREN_SIZE>,
}


#[derive(Archive, Deserialize, Serialize, Debug)]
pub struct LeafNode {
    keys: ArrayVec<Key, LEAF_ITEMS_SIZE>,
    values: ArrayVec<Value, LEAF_ITEMS_SIZE>,
}

#[derive(Debug)]
pub struct BTree {
    root: NodePtr
}

impl BTree {
    pub fn new() -> BTree {
        BTree {
            root: Rc::new(RefCell::from(LeafNode::new()))
        }
    }

    pub fn insert(&mut self, key: Key, val: Value) {
        if self.root.borrow_mut().is_full() {
            let (pivot, child_node) = self.root.borrow_mut().split();
            self.root = Rc::new(RefCell::from(InternalNode::new_with_key(pivot, self.root.to_owned(), child_node)));
        }
        self.root.borrow_mut().insert(key, val);
    }

    pub fn get(&self, key: &Key) -> Option<Value> {
        return self.root.borrow().get(key);
    }

    pub fn delete(&mut self, key: &Key) -> bool {
        let result = self.root.borrow_mut().delete(key);

        if result && self.root.borrow_mut().is_empty() {
            // If the root is empty, we can remove a level
            let child = self.root.borrow_mut().pop_first_child();
            match child {
                Some(new_root) => self.root = new_root,
                None => (),
            };
        }
        return result
    }

    pub fn total_len(&self) -> usize {
        self.root.borrow().total_len()
    }
}


impl InternalNode {
    pub fn new() -> InternalNode {
        InternalNode {
            pivots: ArrayVec::new(),
            children: ArrayVec::new(),
        }
    }

    pub fn new_from(pivots: &[Key], children: &[NodePtr]) -> InternalNode {
        let mut p = ArrayVec::new();
        let mut c = ArrayVec::new();
        p.try_extend_from_slice(pivots).unwrap();
        children.iter().for_each(|x| c.push(x.clone()));
        InternalNode {
            pivots: p,
            children: c,
        }
    }

    pub fn new_with_key(key: Key, left: NodePtr, right: NodePtr) -> InternalNode {
        let mut node = InternalNode {
            pivots: ArrayVec::new(),
            children: ArrayVec::new(),
        };
        node.pivots.push(key);
        node.children.push(left);
        node.children.push(right);
        return node
    }

    pub fn try_split(&mut self, idx: usize) {
        if self.children[idx].borrow_mut().is_full() {
            let (pivot, child_node) = self.children[idx].borrow_mut().split();
            // println!("Split detected: insert:{:?}; idx:{}; pivot:{:?}", key, idx, pivot); 
            self.pivots.insert(idx, pivot);
            self.children.insert(idx+1, child_node);
        }
    }
}

impl Node for InternalNode {
    fn insert(&mut self, key: Key, val: Value) {
        let mut idx = match self.pivots.binary_search(&key) {
            Ok(idx) => idx,
            Err(idx) => {self.try_split(idx); idx},
        };
        // println!("{:?}", self);
        if idx < self.pivots.len() && key > self.pivots[idx] {
            idx += 1; // Might be in right sibling
        }
        self.children[idx].borrow_mut().insert(key, val);
    }

    fn split(&mut self) -> (Key, NodePtr) {
        let mid = (self.pivots.len() / 2) as usize;
    
        let right_node = Rc::new(RefCell::from(InternalNode::new_from(&self.pivots[mid+1..], &self.children[mid+1..])));
        let pivot = self.pivots[mid];
        self.pivots.truncate(mid);
        self.children.truncate(mid+1);
        return (pivot, right_node)
    }


    fn get(&self, key: &Key) -> Option<Value> {
        let idx = match self.pivots.binary_search(&key) {
            Ok(idx) => idx+1, // If key=pivot, look in right child
            Err(idx) => idx,
        };
        return self.children[idx].borrow().get(key);
    }

    fn delete(&mut self, key: &Key) -> bool {
        match self.pivots.binary_search(&key) {
            Ok(left_idx) => {
                let idx = left_idx + 1;
                if self.children[idx].borrow_mut().delete(key) {
                    if self.children[idx].borrow_mut().is_empty() {
                        self.pivots.remove(idx);
                        self.children.remove(idx);
                    } else {
                        self.pivots[idx] = self.children[idx].borrow_mut().get_first_key();
                    }
                    return true
                }
                return false
            },
            // Key to remove is not a pivot, recursive delete in the child node
            Err(idx) => {
                let deleted = self.children[idx].borrow_mut().delete(key);
                // println!("Deleting {:?}", key);
                if self.children[idx].borrow_mut().is_empty() {
                    // If the child is an intermediary node it might still have a child, so let's fetch it
                    let child = self.children[idx].borrow_mut().pop_first_child();
                    if child.is_some() {
                        self.children[idx] = child.unwrap()
                    } else if self.children[idx+1].borrow().len() > 1 {
                        // Right child is splitable
                        let (key, right_node) = self.children[idx+1].borrow_mut().split();
                        self.pivots[idx] = key;
                        self.children.swap(idx, idx+1);
                        self.children[idx+1] = right_node;
                    } else {
                        // right child is too small for split
                        self.pivots.remove(idx);
                        self.children.remove(idx);
                    }
                }
                return deleted
            }
        }
    }

    fn total_len(&self) -> usize {
        self.children.iter().map(|x| x.borrow().total_len()).sum()
    }

    fn is_full(&self) -> bool {
        self.pivots.is_full()
    }

    fn is_empty(&self) -> bool {
        self.pivots.is_empty()
    }

    fn get_first_key(&self) -> Key {
        self.pivots[0]
    }

    fn len(&self) -> usize {
        self.pivots.len()
    }

    fn pop_first_child(&mut self) -> Option<NodePtr> {
        self.children.pop()
    }
}


impl LeafNode {
    pub fn new() -> LeafNode {
        LeafNode {
            keys: ArrayVec::new(),
            values: ArrayVec::new(),
        }
    }

    pub fn new_from(keys: &[Key], values: &[Value]) -> LeafNode {
        let mut k = ArrayVec::new();
        k.try_extend_from_slice(keys).unwrap();
        let mut v = ArrayVec::new();
        v.try_extend_from_slice(values).unwrap();
        LeafNode {
            keys: k,
            values: v, 
        }
    }
}

impl Node for LeafNode {
    fn split(&mut self) -> (Key, NodePtr) {
        let mid = self.keys.len() / 2;

        let right_node =  Rc::new(RefCell::from(LeafNode::new_from(
            &self.keys[mid..],
            &self.values[mid..],
        )));
        let pivot = self.keys[mid];
        self.keys.truncate(mid);
        self.values.truncate(mid);
        return (pivot, right_node)
    }

    fn insert(&mut self, key: Key, val: Value) {
        match self.keys.binary_search(&key) {
            Ok(idx) => self.values[idx] = val,
            Err(idx) => {
                self.keys.insert(idx, key);
                self.values.insert(idx, val);
            },
        }
    }

    fn get(&self, key: &Key) -> Option<Value> {
        match self.keys.binary_search(&key) {
            Ok(idx) => Some(self.values[idx]),
            Err(_) => None,
        }
    }

    fn get_first_key(&self) -> Key {
        self.keys[0]
    }

    fn delete(&mut self, key: &Key) -> bool {
        match self.keys.binary_search(&key) {
            Ok(idx) => {
                self.keys.remove(idx);
                self.values.remove(idx);
                true 
            },
            Err(_) => false,
        }
    }

    fn total_len(&self) -> usize {
        return self.keys.len()
    }

    fn is_full(&self) -> bool {
        self.keys.is_full()
    }

    fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    fn len(&self) -> usize {
        self.keys.len()
    }

    fn pop_first_child(&mut self) -> Option<NodePtr> {
        None
    }
}



#[cfg(test)]
mod tests {
    use super::*;


    fn test_insert<I>(btree: &mut BTree, keys: I)
    where
        I: Iterator<Item = u128> 
    {
        let mut key: Key = [0; 1];
        let mut expected_len = 0;
        for n in keys {
            key[0] = n;
            btree.insert(key, 0);
            expected_len += 1;
            if expected_len < btree.total_len() as u128 {
                println!("Tree: {:?}", btree);
                assert!(false);
            }
        }
    }

    fn test_read<I>(btree: &mut BTree, keys: I)
    where
        I: Iterator<Item = u128> 
    {
        let mut key: Key = [0; 1];
        for n in keys {
            key[0] = n;
            assert!(btree.get(&key).is_some());
        }
    }

    fn test_delete<I>(btree: &mut BTree, keys: I)
    where
        I: Iterator<Item = u128> 
    {
        let mut key: Key = [0; 1];
        for n in keys {
            key[0] = n;
            assert!(btree.delete(&key));
        }
    }

    #[test]
    // Insert, read, and delete in ascending order
    fn test_asc_crud_btree() {
        let nb_keys = 1000;
        let mut btree = BTree::new();
        test_insert(&mut btree, 0..nb_keys);
        test_read(&mut btree, 0..nb_keys);
        test_delete(&mut btree, 0..nb_keys);
        assert!(btree.total_len() == 0)
    }

    #[test]
    // Insert, read, and delete in descending order
    fn test_desc_crud_btree() {
        let nb_keys = 5;
        let mut btree = BTree::new();
        test_insert(&mut btree, (0..nb_keys).rev());
        test_read(&mut btree, (0..nb_keys).rev());
        test_delete(&mut btree, (0..nb_keys).rev());
        assert!(btree.total_len() == 0)
    }
}
