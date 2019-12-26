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
}

#[test]
fn test_leaf_node() {
    let mut l = LeafNode::<&str, usize>::new();
    l.insert(&"hi", &3);
    l.insert(&"hello", &4);
    l.insert(&"world", &5);
    l.insert(&"abc", &6);
    assert_eq!(l.lookup(&"hi"), Some(&3));
    assert_eq!(l.lookup(&"hello"), Some(&4));
    assert_eq!(l.lookup(&"world"), Some(&5));
    assert_eq!(l.lookup(&"abc"), Some(&6));
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

    fn insert(&mut self, k: &K, v: &V) -> Option<V> {
        let mut cur = self.root;
        let mut father: Option<usize> = None; // the node id of the father node of the current node
        let mut father_son_index: usize = 0; // the current node `father_son_index`-th son of the father node
        loop {
            match cur {
                NodeIndex::Internal(id) => {
                    let t = &mut self.i[id];
                    if t.full() {
                        let right_min, right_node = t.split();
                        // TODO:
                        let fa = &mut self.i[father];
                        fa.insert(father_son_index, xx, xx);
                        
                        if right_min <= k {
                            cur = NodeIndex::Internal(right_id);
                        }
                    } else {
                        father = Some(id);
                        let (father_son_index, cur) = t.lookup(k);
                    }
                }
                NodeIndex::Leaf(id) => {
                    let t = &mut self.l[id];
                    if t.full() {
                        // TODO: split the current leaf node
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
                    cur = self.i[id].lookup(k);
                }
                NodeIndex::Leaf(id) => {
                    return self.l[id].lookup(k);
                }
            }
        }
    }
}

