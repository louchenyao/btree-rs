use std::ptr::copy;

#[derive(Clone, Copy)]
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
const NODE_DEG: usize = 8;

struct InternalNode<K> {
    keys: [K; NODE_DEG-1],
    sons: [NodeIndex; NODE_DEG],
    cnt: usize,
}

impl<K: PartialOrd + Copy + Default> InternalNode<K> {
    /// News an internal node. Note that the internal node at least has one child, it takes `first` as the initial child.
    fn new(first: NodeIndex) -> Self {
        let mut i = InternalNode {
            // for keys not in the range of [0, cnt) are invalid, which we do not care
            // mem::MaybeUninit is a better way to initialize the array
            keys: [K::default(); NODE_DEG-1],
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
        // The relations between sons and keys is that: the maximum key in `son[i]` is `keys[0]`:
        //        | keys[0] |       | keys[1] | ... | keys[1023] |
        // sons[0]            sons[1]            ...                sons[1024]
        //
        // Thus, `k` in the sub-tree `sons[lower_bound(keys, k)]`

        let i = lower_bound(&self.keys[0..self.cnt], k);
        (i, self.sons[i])
    }
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
            cnt: 0
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
            copy(&self.keys[i], &mut self.keys[i+1], self.cnt-i);
            copy(&self.values[i], &mut self.values[i+1], self.cnt-i);
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
    /// Returns the mininum the right node.
    fn split(&mut self) -> Self {
        let left_cnt = self.cnt / 2;
        let mut right = Self::new();
        
        // updates data
        unsafe {
            copy(&self.keys[left_cnt], &mut right.keys[0], self.cnt - left_cnt);
            copy(&self.values[left_cnt], &mut right.values[0], self.cnt - left_cnt);
        };

        // updates the cnt
        right.cnt = self.cnt - left_cnt;
        self.cnt = left_cnt;

        right
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
    let right = l.split();
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

    if &a[l] < val {
        // it is true if and only if a[a.len()-1] < val
        l + 1
    } else {
        l
    }
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

struct BTree<K, V> {
    i: Vec<InternalNode<K>>, // internal nodes buf
    l: Vec<LeafNode<K, V>>,  // leaf nodes buf
    root: NodeIndex,
}

impl<K: PartialOrd + PartialEq + Default + Copy, V: Default + Copy> BTree<K, V> {
    fn new() -> Self {
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

    fn insert(&mut self, k: &K, v: &V) -> Option<V> {
        let mut cur = self.root;
        let mut father_id: Option<usize> = None; // the node id of the father node of the current node
        let mut father_son_index: usize = 0; // the current node `father_son_index`-th son of the father node
        loop {
            match cur {
                NodeIndex::Internal(id) => {
                    // let t = &mut self.i[id];
                    // if t.full() {
                    //     let right_min, right_node = t.split();
                    //     // TODO:
                    //     let fa = &mut self.i[father];
                    //     fa.insert(father_son_index, xx, xx);
                        
                    //     if right_min <= k {
                    //         cur = NodeIndex::Internal(right_id);
                    //     }
                    // } else {
                    //     father = Some(id);
                    //     let (father_son_index, cur) = t.lookup(k);
                    // }
                }
                NodeIndex::Leaf(id) => {
                    let t = &mut self.l[id];
                    if t.full() {
                        let right = t.split();
                        let right_id = self.alloc_leaf(right);

                        // make a new root node since the current node is already the root node
                        if father_id == None {
                            let new_root_id = self.alloc_internal(
                                InternalNode::new(NodeIndex::Leaf(id))
                            );
                            self.root = NodeIndex::Internal(new_root_id);
                            father_id = Some(new_root_id);
                        }


                        // insert the right to the father node
                        
                        let fa = &mut self.i[father_id.unwrap()];


                    } else {
                        return t.insert(k, v);
                    }
                }
            }
        }
    }

    fn lookup(&self, k: &K) -> Option<&V> {
        let mut cur = self.root;
        loop {
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

