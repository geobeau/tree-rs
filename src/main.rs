pub mod bplustree;
use rand::Rng;

fn main() {
    if cfg!(target_endian = "big") {
        println!("Big endian");
    } else {
        println!("Little endian");
    }
    println!("Hello, world!");
    println!("Node: {}", std::mem::size_of::<bplustree::BTree>());
    println!(
        "InternalNode: {}",
        std::mem::size_of::<bplustree::InternalNode>()
    );
    println!("LeafNode: {}", std::mem::size_of::<bplustree::LeafNode>());

    let n = 1_000_000;
    let mut rng = rand::thread_rng();
    let mut t = bplustree::BTree::new();
    println!("inserting {} keys", n);
    for _ in 0..n {
        t.insert([rng.gen(); 1], 0);
    }
    println!("getting {} keys", n);
    for _ in 0..n {
        t.get(&[rng.gen(); 1]);
    }
}
