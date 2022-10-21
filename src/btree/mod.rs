use std::{rc::Rc, cell::RefCell};
use std::fmt::Debug;
use arrayvec::ArrayVec;
use rkyv::{Archive, Deserialize, Serialize};

type Key = [u128; 1];
type Value = u8;
type NodePtr = Rc<RefCell<dyn Node>>;

const NODE_SIZE: usize = 1024 * 4;
const LEAF_ITEMS_SIZE: usize = (NODE_SIZE - 32) / (std::mem::size_of::<Key>() + std::mem::size_of::<Value>());
const INTERNAL_ITEMS_SIZE: usize = (NODE_SIZE - 32) / (std::mem::size_of::<NodePtr>() + std::mem::size_of::<Key>());
const PIVOTS_SIZE: usize = INTERNAL_ITEMS_SIZE - 1;
const CHILDREN_SIZE: usize = INTERNAL_ITEMS_SIZE;


pub trait Node: std::fmt::Debug {
    fn get(&self, key: &Key) -> (bool, usize);
    fn insert(&mut self, key: Key, val: Value);
    fn try_split(&mut self) -> Option<(Key, NodePtr)>;
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
        let node = self.root.borrow_mut().try_split();
        if node.is_some() {
            let (pivot, child_node) = node.unwrap();
            self.root = Rc::new(RefCell::from(InternalNode::new_with_key(pivot, self.root.to_owned(), child_node)));
        }
        self.root.borrow_mut().insert(key, val);
    }

    pub fn get(&self, key: &Key) -> (bool, usize) {
        return self.root.borrow().get(key)
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
}

impl Node for InternalNode {
    fn insert(&mut self, key: Key, val: Value) {
        match self.pivots.binary_search(&key) {
            Ok(_) => return, // Key is already there, don't update it
            Err(i) => {
                let mut idx = i;
                let node = self.children[idx].borrow_mut().try_split();
                if node.is_some() {
                    let (pivot, child_node) = node.unwrap();
                    // println!("Split detected: insert:{:?}; idx:{}; pivot:{:?}", key, idx, pivot); 
                    self.pivots.insert(idx, pivot);
                    self.children.insert(idx+1, child_node);
                    if key > self.pivots[idx] {
                        idx += 1;
                    }
                }
                self.children[idx].borrow_mut().insert(key, val);
            },
        }
    }

    fn try_split(&mut self) -> Option<(Key, NodePtr)> {
        if self.pivots.is_full() {
            let mid = (self.pivots.len() / 2) + 1;
    
            let right_node = Rc::new(RefCell::from(InternalNode::new_from(&self.pivots[mid..], &self.children[mid..])));
            let pivot = self.pivots[mid];
            self.pivots.truncate(mid);
            self.children.truncate(mid+1);
            return Some((pivot, right_node))
        }
        None
    }


    fn get(&self, key: &Key) -> (bool, usize) {
        let idx = match self.pivots.binary_search(&key) {
            Ok(idx) => idx+1, // If key=pivot, look in right child
            Err(idx) => idx,
        };
        // println!("id: {}", idx);
        let (ok, depth) = self.children[idx].borrow().get(key);
        return (ok, depth+1)
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
    fn try_split(&mut self) -> Option<(Key, NodePtr)> {
        if self.keys.is_full() {
            let mid = (self.keys.len() / 2) + 1;
    
            let right_node =  Rc::new(RefCell::from(LeafNode::new_from(
                &self.keys[mid..],
                &self.values[mid..],
            )));
            let pivot = self.keys[mid];
            self.keys.truncate(mid);
            self.values.truncate(mid);
            return Some((pivot, right_node))
        }
        None
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

    fn get(&self, key: &Key) -> (bool, usize) {
        return (self.keys.binary_search(&key).is_ok(), 1)
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_btree() {
        let nb_keys = 50;
        let mut btree = BTree::new();
        let mut key: Key = [0; 1];

        // stats
        let mut max_depth = 0;
        let mut min_depth = nb_keys as usize;
        let mut sum_depth = 0;

        
        for n in 0..nb_keys {
            key[0] = n;
            // println!("Inserting: {:?}", key[0]);
            btree.insert(key, 0);
            // println!("Tree: {:?}", btree);
        }

        for n in 0..nb_keys {
            key[0] = n;
            // println!("Get: {:?}", key[0]);
            let (ok, depth) = btree.get(&key);
            max_depth = std::cmp::max(max_depth, depth);
            min_depth = std::cmp::min(min_depth, depth);
            sum_depth += depth;
            assert!(ok);
        }
        println!("max: {}, min: {}, avg: {}", max_depth, min_depth, (sum_depth / nb_keys as usize))
    }
}
