#![feature(test)]


use std::ptr::copy;

#[derive(Debug, Clone, Copy, PartialEq)]
enum NodeIndex {
    Leaf(usize),
    Internal(usize),
}

impl Default for NodeIndex {
    fn default() -> Self {
        NodeIndex::Leaf(0)
    }
}

// TODO: pad node structs to 4kB by atomatically choosing node degrees
const NODE_DEG: usize = 32;

struct InternalNode<K> {
    keys: [K; NODE_DEG - 1],
    sons: [NodeIndex; NODE_DEG],
    cnt: usize,
}

impl<K: PartialOrd + Copy + Default> InternalNode<K> {
    /// News an internal node. Note that the internal node at least has one child, it takes `first` as the initial child.
    fn new(first: NodeIndex) -> Self {
        let mut i = InternalNode {
            // for keys not in the range of [0, cnt) are invalid, which we do not care
            // mem::MaybeUninit is a better way to initialize the array
            keys: [K::default(); NODE_DEG - 1],
            sons: [NodeIndex::default(); NODE_DEG],
            cnt: 1,
        };
        i.sons[0] = first;
        i
    }

    fn full(&self) -> bool {
        self.cnt == self.sons.len()
    }

    fn lookup(&self, k: &K) -> (usize, NodeIndex) {
        // The relations between sons and keys is that: the maximum key in `son[i]` is `keys[i]`:
        //         | keys[0] |       | keys[1] | ... | keys[1023] |
        // sons[0]            sons[1]            ...                sons[1024]
        //
        // Thus, `k` in the sub-tree `sons[lower_bound(keys, k)]`

        let i = lower_bound(&self.keys[0..self.cnt-1], k);
        println!("{:?}", i);
        (i, self.sons[i])
    }

    /// Inserts the `right` at the position `pos`.
    /// `left_max` is the maximum key of the original left node.
    fn insert(&mut self, pos: usize, left_max: &K, right: NodeIndex) {
        // If pos == self.cnt, that means we just split the last child, which does not has the maximum key in `keys`, so we do not shift keys.
        if pos < self.cnt {
            unsafe {
                // shift keys to the right.
                copy(&self.keys[pos-1], &mut self.keys[pos], self.cnt - pos);
                // shift the children to the right
                copy(&self.sons[pos], &mut self.sons[pos + 1], self.cnt - pos);
            };
        }

        self.keys[pos-1] = *left_max;
        self.sons[pos] = right;
        self.cnt += 1;
    }

    /// Splits the node to two nodes. The current node turns into the left node.
    /// Returns the max key in the left, and the right node,
    fn split(&mut self) -> (K, Self) {
        // compute the new sizes
        let left_cnt = self.cnt / 2;
        let right_cnt = self.cnt - left_cnt;
        self.cnt = left_cnt;

        let mut right = Self {
            keys: [K::default(); NODE_DEG - 1],
            sons: [NodeIndex::default(); NODE_DEG],
            cnt: right_cnt,
        };
        // copy the data to the right node
        unsafe {
            // again, self.keys.len() == self.sons.len() - 1
            copy(&self.keys[left_cnt], &mut right.keys[0], right_cnt-1);
            copy(&self.sons[left_cnt], &mut right.sons[0], right_cnt);
        };

        (self.keys[left_cnt-1], right)
    }
}

#[test]
fn test_internal_node() {
    let mut i = InternalNode::new(NodeIndex::Leaf(0));
    i.keys[0] = 1;
    i.keys[1] = 10;
    i.keys[2] = 20;
    i.keys[3] = 30;
    i.sons[0] = NodeIndex::Leaf(0);
    i.sons[1] = NodeIndex::Leaf(1);
    i.sons[2] = NodeIndex::Leaf(2);
    i.sons[3] = NodeIndex::Leaf(3);
    i.sons[4] = NodeIndex::Leaf(4);
    i.cnt = 5;

    // suppose we splited the leaf(2) into leaf(2) and leaf(5)
    // now insert the leaf(5)
    i.insert(3, &15, NodeIndex::Leaf(5));
    assert_eq!(i.keys[0..i.cnt-1], [1, 10, 15, 20, 30]);
    assert_eq!(i.sons[0..i.cnt], [NodeIndex::Leaf(0), NodeIndex::Leaf(1), NodeIndex::Leaf(2), NodeIndex::Leaf(5), NodeIndex::Leaf(3), NodeIndex::Leaf(4)])
}

struct LeafNode<K, V> {
    keys: [K; NODE_DEG],
    values: [V; NODE_DEG],
    cnt: usize,
}

impl<K: PartialOrd + Copy + Default, V: Copy + Default> LeafNode<K, V> {
    fn new() -> Self {
        LeafNode {
            keys: [K::default(); NODE_DEG],
            values: [V::default(); NODE_DEG],
            cnt: 0,
        }
    }

    fn full(&self) -> bool {
        self.cnt == self.keys.len()
    }

    fn insert(&mut self, k: &K, v: &V) -> Option<V> {
        assert!(!self.full());

        // insert the key into keys[i]
        let i = lower_bound(&self.keys[0..self.cnt], k);

        // simply append the key
        if i == self.cnt {
            self.keys[self.cnt] = *k;
            self.values[self.cnt] = *v;
            self.cnt += 1;
            return None;
        }

        // the key already exists
        if &self.keys[i] == k {
            let ret = Some(self.values[i]);
            self.values[i] = *v;
            return ret;
        }

        // shift the data to the right, to empty one slot
        unsafe {
            copy(&self.keys[i], &mut self.keys[i + 1], self.cnt - i);
            copy(&self.values[i], &mut self.values[i + 1], self.cnt - i);
        };

        self.keys[i] = *k;
        self.values[i] = *v;
        self.cnt += 1;
        None
    }

    fn lookup(&self, k: &K) -> Option<&V> {
        let i = lower_bound(&self.keys[0..self.cnt], k);
        if i == self.cnt {
            None
        } else if &self.keys[i] == k {
            Some(&self.values[i])
        } else {
            None
        }
    }

    /// Splits the node to two nodes. The current node turns into the left node.
    /// Returns the max key in the left, and the right node,
    fn split(&mut self) -> (K, Self) {
        let left_cnt = self.cnt / 2;
        let mut right = Self::new();
        // updates data
        unsafe {
            copy(
                &self.keys[left_cnt],
                &mut right.keys[0],
                self.cnt - left_cnt,
            );
            copy(
                &self.values[left_cnt],
                &mut right.values[0],
                self.cnt - left_cnt,
            );
        };

        // updates the cnt
        right.cnt = self.cnt - left_cnt;
        self.cnt = left_cnt;

        (self.keys[self.cnt - 1], right)
    }
}

#[test]
fn test_leaf_node() {
    // test insert and lookup
    let mut l = LeafNode::<&str, usize>::new();
    l.insert(&"hi", &3);
    l.insert(&"hello", &4);
    l.insert(&"world", &5);
    l.insert(&"abc", &6);
    l.insert(&"def", &7);
    assert_eq!(l.lookup(&"hi"), Some(&3));
    assert_eq!(l.lookup(&"hello"), Some(&4));
    assert_eq!(l.lookup(&"world"), Some(&5));
    assert_eq!(l.lookup(&"abc"), Some(&6));
    assert_eq!(l.lookup(&"def"), Some(&7));

    // test split
    let (left_max, right) = l.split();
    assert_eq!(left_max, "def");
    assert_eq!(l.cnt, 2);
    assert_eq!(l.lookup(&"abc"), Some(&6));
    assert_eq!(l.lookup(&"def"), Some(&7));
    assert_eq!(l.lookup(&"hello"), None);
    assert_eq!(l.lookup(&"hi"), None);
    assert_eq!(l.lookup(&"world"), None);
    assert_eq!(right.cnt, 3);
    assert_eq!(right.lookup(&"abc"), None);
    assert_eq!(right.lookup(&"def"), None);
    assert_eq!(right.lookup(&"hello"), Some(&4));
    assert_eq!(right.lookup(&"hi"), Some(&3));
    assert_eq!(right.lookup(&"world"), Some(&5));
}

/// Returns the index pointing to the first element in the range [0,a.len()) which does not compare less than val.
/// If such element does not exist, then return a.len()
fn lower_bound<T: PartialOrd>(a: &[T], val: &T) -> usize {
    if a.len() == 0 {
        return 0;
    }
    if &a[a.len()-1] < val {
        return a.len();
    }

    let mut l = 0;
    let mut r = a.len() - 1;
    // the valid solution is always in [l, r]
    while l < r {
        let mid = (l + r) / 2;
        if &a[mid] < val {
            l = mid + 1;
        } else {
            r = mid;
        }
    }

    l
}

#[test]
fn test_lower_bound() {
    let a = [1, 4, 6, 9, 10];
    assert_eq!(lower_bound(&a, &0), 0);
    assert_eq!(lower_bound(&a, &1), 0);
    assert_eq!(lower_bound(&a, &3), 1);
    assert_eq!(lower_bound(&a, &4), 1);
    assert_eq!(lower_bound(&a, &5), 2);
    assert_eq!(lower_bound(&a, &6), 2);
    assert_eq!(lower_bound(&a, &10), 4);
    assert_eq!(lower_bound(&a, &11), 5);

    assert_eq!(lower_bound(&[], &42), 0);
}

pub struct BTree<K, V> {
    i: Vec<InternalNode<K>>, // internal nodes buf
    l: Vec<LeafNode<K, V>>,  // leaf nodes buf
    root: NodeIndex,
}

/// Btree is a balanced tree optimized for reducing the number of memory accesses.
impl<K: PartialOrd + PartialEq + Default + Copy, V: Default + Copy> BTree<K, V> {
    pub fn new() -> Self {
        let mut t = BTree {
            i: Vec::with_capacity(1024),
            l: Vec::with_capacity(1024),
            root: NodeIndex::Leaf(0),
        };
        // push the root node
        t.l.push(LeafNode::new());
        t
    }

    /// Allocates a leaf node, and initializes it to `leaf`
    /// Then returns the index of the new leaf node.
    fn alloc_leaf(&mut self, leaf: LeafNode<K, V>) -> usize {
        self.l.push(leaf);
        self.l.len() - 1
    }

    /// Allocates an internal node, and initializes it to `internal`
    /// Returns the indexe of the new internal node.
    fn alloc_internal(&mut self, internal: InternalNode<K>) -> usize {
        self.i.push(internal);
        self.i.len() - 1
    }

    /// Makes the new root, which must be the internal node. `first` is the first child of the new root.
    /// Returns the new root id.
    fn make_new_root(&mut self, first: NodeIndex) -> usize {
        let new_root_id = self.alloc_internal(InternalNode::new(first));
        self.root = NodeIndex::Internal(new_root_id);
        new_root_id
    }

    pub fn insert(&mut self, k: &K, v: &V) -> Option<V> {
        let mut cur = self.root;
        let mut father_id: Option<usize> = None; // the node id of the father node of the current node
        let mut father_son_index: usize = 0; // the current node `father_son_index`-th son of the father node
        loop {
            match cur {
                NodeIndex::Internal(mut id) => {
                    if self.i[id].full() {
                        let (left_max, right) = self.i[id].split();
                        let right_id = self.alloc_internal(right);

                        // make a new root node if the current node is the root
                        if father_id == None {
                            father_id = Some(self.make_new_root(NodeIndex::Internal(id))); 
                            father_son_index = 0;
                        }

                        // insert the right to the father node
                        let fa = &mut self.i[father_id.unwrap()];
                        fa.insert(father_son_index + 1, &left_max, NodeIndex::Internal(right_id));

                        // insert to the right node
                        if &left_max < k {
                            id = right_id;
                        }
                    }
                    
                    father_id = Some(id);
                    let tmp = self.i[id].lookup(k);
                    father_son_index = tmp.0;
                    cur = tmp.1;
                }
                NodeIndex::Leaf(mut id) => {
                    if self.l[id].full() {
                        // split
                        let (left_max, right) = self.l[id].split();
                        let right_id = self.alloc_leaf(right);

                        // make a new root node if the current node is the root
                        if father_id == None {
                            father_id = Some(self.make_new_root(NodeIndex::Leaf(id))); 
                            father_son_index = 0;
                        }

                        // insert the right to the father node
                        let fa = &mut self.i[father_id.unwrap()];
                        fa.insert(father_son_index + 1, &left_max, NodeIndex::Leaf(right_id));

                        // insert to the right node
                        if &left_max < k {
                            id = right_id;
                        }
                    }

                    return self.l[id].insert(k, v);
                }
            }
        }
    }

    pub fn lookup(&self, k: &K) -> Option<&V> {
        let mut cur = self.root;
        loop {
            println!("cur = {:?}", cur);
            match cur {
                NodeIndex::Internal(id) => {
                    cur = self.i[id].lookup(k).1;
                }
                NodeIndex::Leaf(id) => {
                    return self.l[id].lookup(k);
                }
            }
        }
    }
}

#[test]
fn test_btree_smoke() {
    let mut btree = BTree::<&str, usize>::new();
    assert_eq!(btree.insert(&"theanswer", &42), None);
    assert_eq!(btree.lookup(&"theanswer"), Some(&42));
    assert_eq!(btree.insert(&"theanswer", &43), Some(42));
    assert_eq!(btree.lookup(&"theanswer"), Some(&43));
}

#[cfg(test)]
mod tests {
    extern crate rand;
    extern crate test;
    use super::*;
    use std::collections::BTreeMap;
    use rand::prelude::*;
    use test::Bencher;

    #[test]
    fn test_bree_1() {
        let mut rng = rand::thread_rng();

        let mut keys = Vec::new();
        let mut truth = BTreeMap::new();
        let mut t = BTree::new(); 

        for _ in 0..300000 {
            let lookup: bool = rng.gen();

            if lookup && keys.len() != 0 {
                let mut i: usize = rng.gen();
                i %= keys.len();
                // println!("lookup key: {}", keys[i]);
                // println!("{:?}", t);
                assert_eq!(Some(&truth[&keys[i]]), t.lookup(&keys[i]));
            } else {
                let k: u16 = rng.gen();
                let v: i32 = rng.gen();
                keys.push(k);
                // println!("insert key: {}", k);
                // println!("{:?}", t);
                assert_eq!(t.insert(&k, &v), truth.insert(k, v));
            }
        }
    }

    #[bench]
    fn bench_insert_dense_keys(b: &mut Bencher) {
        let n = 100000;
        b.iter(||{
            let mut t = BTree::<usize, usize>::new();
            for i in 0..n {
                t.insert(&i, &i);
            }
        });
        b.bytes = n as u64;
    }

    #[bench]
    fn bench_std_insert_dense_keys(b: &mut Bencher) {
        let n = 100000;
        b.iter(||{
            let mut t = BTreeMap::<usize, usize>::new();
            for i in 0..n {
                t.insert(i, i);
            }
        });
        b.bytes = n as u64;
    }
}