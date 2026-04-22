//! Minimal k-d tree over prototype feature vectors. No dependencies.
//!
//! Builds a static tree in O(n log n), queries k nearest neighbors in
//! O(k log n) on average. For our 32-dim feature vectors and prototype
//! counts in the thousands, this is a meaningful speedup over the linear
//! scan used by [`crate::ocr::recognize::FeatureRecognizer`]. For small sets
//! (<100 prototypes) the linear scan wins.

use crate::ocr::features::{FeatureVec, FEATURE_COUNT};
use crate::ocr::recognize::Prototype;

/// k-d tree built from a slice of prototypes. Owns its own copy of the
/// feature vectors and labels.
pub struct KdTree {
    nodes: Vec<Node>,
    /// Root node index (usize::MAX when empty).
    root: usize,
}

struct Node {
    features: FeatureVec,
    label: char,
    axis: usize,
    left: usize,
    right: usize,
}

const NIL: usize = usize::MAX;

impl KdTree {
    pub fn new(prototypes: &[Prototype]) -> Self {
        if prototypes.is_empty() {
            return Self {
                nodes: Vec::new(),
                root: NIL,
            };
        }
        let mut entries: Vec<(FeatureVec, char)> = prototypes
            .iter()
            .map(|p| (p.features.clone(), p.label))
            .collect();
        let mut nodes: Vec<Node> = Vec::with_capacity(entries.len());
        let root = build(&mut entries, 0, &mut nodes);
        Self { nodes, root }
    }

    pub fn is_empty(&self) -> bool {
        self.root == NIL
    }

    /// Find the `k` nearest neighbors to `query`. Returns `(distance, label)`
    /// pairs sorted ascending by distance.
    pub fn knn(&self, query: &FeatureVec, k: usize) -> Vec<(f32, char)> {
        let k = k.max(1);
        let mut heap: MaxHeap = MaxHeap::new(k);
        self.knn_recursive(self.root, query, &mut heap);
        let mut result: Vec<(f32, char)> = heap.into_sorted();
        result.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        result
    }

    fn knn_recursive(&self, idx: usize, query: &FeatureVec, heap: &mut MaxHeap) {
        if idx == NIL {
            return;
        }
        let node = &self.nodes[idx];
        let d = euclidean(query, &node.features);
        heap.push(d, node.label);

        let diff = query[node.axis] - node.features[node.axis];
        let (near, far) = if diff < 0.0 {
            (node.left, node.right)
        } else {
            (node.right, node.left)
        };
        self.knn_recursive(near, query, heap);
        // Cross the split plane only if it could yield a closer neighbor
        // than the current kth-best distance.
        if diff * diff < heap.worst_distance_sq() {
            self.knn_recursive(far, query, heap);
        }
    }
}

fn build(entries: &mut [(FeatureVec, char)], depth: usize, nodes: &mut Vec<Node>) -> usize {
    if entries.is_empty() {
        return NIL;
    }
    let axis = depth % FEATURE_COUNT;
    entries.sort_by(|a, b| a.0[axis].partial_cmp(&b.0[axis]).unwrap_or(std::cmp::Ordering::Equal));
    let mid = entries.len() / 2;
    let features = entries[mid].0.clone();
    let label = entries[mid].1;
    let idx = nodes.len();
    nodes.push(Node {
        features,
        label,
        axis,
        left: NIL,
        right: NIL,
    });
    let left = build(&mut entries[..mid], depth + 1, nodes);
    let right = build(&mut entries[mid + 1..], depth + 1, nodes);
    nodes[idx].left = left;
    nodes[idx].right = right;
    idx
}

fn euclidean(a: &FeatureVec, b: &FeatureVec) -> f32 {
    let mut sum = 0.0f32;
    for i in 0..FEATURE_COUNT {
        let d = a[i] - b[i];
        sum += d * d;
    }
    sum.sqrt()
}

/// Bounded max-heap storing the k smallest distances. The root holds the
/// current worst (largest) of the kept distances so we can reject new
/// candidates in O(1).
struct MaxHeap {
    k: usize,
    items: Vec<(f32, char)>,
}

impl MaxHeap {
    fn new(k: usize) -> Self {
        Self {
            k,
            items: Vec::with_capacity(k + 1),
        }
    }
    fn worst_distance_sq(&self) -> f32 {
        if self.items.len() < self.k {
            f32::INFINITY
        } else {
            let d = self.root_distance();
            d * d
        }
    }
    fn root_distance(&self) -> f32 {
        self.items.first().map(|(d, _)| *d).unwrap_or(f32::INFINITY)
    }
    fn push(&mut self, d: f32, label: char) {
        if self.items.len() < self.k {
            self.items.push((d, label));
            let last = self.items.len() - 1;
            sift_up(&mut self.items, last);
        } else if d < self.root_distance() {
            self.items[0] = (d, label);
            sift_down(&mut self.items, 0);
        }
    }
    fn into_sorted(self) -> Vec<(f32, char)> {
        self.items
    }
}

fn sift_up(v: &mut [(f32, char)], mut i: usize) {
    while i > 0 {
        let parent = (i - 1) / 2;
        if v[i].0 > v[parent].0 {
            v.swap(i, parent);
            i = parent;
        } else {
            break;
        }
    }
}

fn sift_down(v: &mut [(f32, char)], mut i: usize) {
    let n = v.len();
    loop {
        let l = 2 * i + 1;
        let r = 2 * i + 2;
        let mut largest = i;
        if l < n && v[l].0 > v[largest].0 {
            largest = l;
        }
        if r < n && v[r].0 > v[largest].0 {
            largest = r;
        }
        if largest == i {
            break;
        }
        v.swap(i, largest);
        i = largest;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ocr::features::FEATURE_COUNT;

    fn make_proto(label: char, v: f32) -> Prototype {
        let mut f = vec![0.0f32; FEATURE_COUNT];
        f[0] = v;
        Prototype { label, features: f }
    }

    #[test]
    fn knn_returns_nearest_neighbors() {
        let protos = vec![
            make_proto('A', 0.0),
            make_proto('B', 1.0),
            make_proto('C', 5.0),
            make_proto('D', 10.0),
        ];
        let tree = KdTree::new(&protos);
        let mut q = vec![0.0f32; FEATURE_COUNT];
        q[0] = 0.8;
        let result = tree.knn(&q, 2);
        assert_eq!(result.len(), 2);
        let labels: Vec<char> = result.iter().map(|(_, l)| *l).collect();
        assert!(labels.contains(&'A'));
        assert!(labels.contains(&'B'));
    }

    #[test]
    fn knn_agrees_with_linear_scan() {
        let protos: Vec<Prototype> = (0..50)
            .map(|i| make_proto(char::from_u32('A' as u32 + i).unwrap(), i as f32))
            .collect();
        let tree = KdTree::new(&protos);
        let mut q = vec![0.0f32; FEATURE_COUNT];
        q[0] = 17.3;

        let tree_result = tree.knn(&q, 3);
        let mut linear: Vec<(f32, char)> = protos
            .iter()
            .map(|p| ((p.features[0] - 17.3).abs(), p.label))
            .collect();
        linear.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let linear_top: Vec<char> = linear.iter().take(3).map(|(_, l)| *l).collect();
        let tree_top: Vec<char> = tree_result.iter().map(|(_, l)| *l).collect();
        assert_eq!(tree_top, linear_top);
    }
}
