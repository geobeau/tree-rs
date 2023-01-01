// Freelist is a 32 bits freelist
pub struct Freelist<T> {
    list: Vec<Handle<T>>,
    free_list_head: usize,
    size: usize,
}

pub enum Handle<T: Sized> {
    Next(u32),
    Value(T),
}

impl<T> Freelist<T> {
    pub fn new() -> Freelist<T> {
        Freelist {
            list: Vec::new(),
            free_list_head: 0,
            size: 0,
        }
    }

    pub fn push(&mut self, val: T) -> u32 {
        // If the list is full, push data at the end
        if self.size == self.list.len() {
            self.list.push(Handle::Value(val));
            self.size += 1;
            return self.size as u32 - 1;
        }
        // If there are freeslots use them
        match self.list[self.free_list_head] {
            Handle::Next(next) => {
                self.list[self.free_list_head] = Handle::Value(val);
                let insert_idx = self.free_list_head;
                self.free_list_head = next as usize;
                self.size += 1;
                return insert_idx as u32;
            }
            Handle::Value(_) => panic!("Freelist head is incorrect aborting"),
        }
    }

    pub fn get(&self, idx: u32) -> Option<&T> {
        match &self.list[idx as usize] {
            Handle::Next(_) => None,
            Handle::Value(val) => Some(val),
        }
    }

    pub fn delete(&mut self, idx: u32) -> Option<()> {
        match self.list[idx as usize] {
            Handle::Next(_) => None, // Already a tombstone
            Handle::Value(_) => {
                self.list[idx as usize] = Handle::Next(self.free_list_head as u32);
                self.free_list_head = idx as usize;
                self.size -= 1;
                return Some(());
            }
        }
    }

    pub fn len(&self) -> u32 {
        return self.size as u32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl<T> Freelist<T> {
        fn list_len(&self) -> usize {
            return self.list.len();
        }
    }

    #[test]
    fn test_freelist_basic_crud_works() {
        let mut list = Freelist::<String>::new();

        let test_val = "foo".to_string();
        let idx = list.push(test_val.clone());
        assert_eq!(list.len(), 1);
        assert_eq!(list.get(idx).unwrap(), &test_val);

        list.delete(idx).expect("Should have been deleted");
        assert_eq!(list.len(), 0);
    }

    #[test]
    fn test_freelist_tombstones_works() {
        let mut list = Freelist::<String>::new();

        let test_val = "foo".to_string();
        for i in 0..10 {
            list.push(format!("{}-{}", test_val, i));
        }
        assert_eq!(list.len(), 10);
        assert_eq!(list.get(5).unwrap(), "foo-5");

        list.delete(5).expect("Should have been deleted");
        list.delete(2).expect("Should have been deleted");
        list.delete(1).expect("Should have been deleted");
        list.delete(9).expect("Should have been deleted");
        list.delete(8).expect("Should have been deleted");

        assert_eq!(list.len(), 5);
        assert_eq!(list.list_len(), 10); // underlying vec didn't reduce
        assert!(list.get(5).is_none()); // data is properly removed
        for i in 10..16 {
            println!("l: {}, ll: {}", list.len(), list.list_len());
            println!("Adding {}-{}", test_val, i);
            list.push(format!("{}-{}", test_val, i));
        }
        assert_eq!(list.len(), 11);
        assert_eq!(list.list_len(), 11);
    }
}
