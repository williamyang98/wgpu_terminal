struct LruListNode<T> {
    data: T,
    next_index: Option<usize>,
    prev_index: Option<usize>,
}

pub struct LruList<T> {
    nodes: Vec<LruListNode<T>>,
    head_index: Option<usize>,
    tail_index: Option<usize>,
}

impl<T> Default for LruList<T> {
    fn default() -> Self {
        Self {
            nodes: Vec::<LruListNode<T>>::new(),
            head_index: None,
            tail_index: None,
        }
    }
}

impl<T> LruList<T> {
    pub fn with_capacity(initial_capacity: usize) -> Self {
        Self {
            nodes: Vec::<LruListNode<T>>::with_capacity(initial_capacity),
            head_index: None,
            tail_index: None,
        }
    }

    pub fn promote(&mut self, index: usize) -> bool {
        assert!(index < self.nodes.len());
        assert!(self.head_index.is_some() && self.tail_index.is_some());
        if Some(index) == self.head_index {
            return false;
        }
        // splice adjacent nodes together
        if let Some(prev_index) = self.nodes[index].prev_index {
            self.nodes[prev_index].next_index = self.nodes[index].next_index;
        }
        if let Some(next_index) = self.nodes[index].next_index {
            self.nodes[next_index].prev_index = self.nodes[index].prev_index;
        }
        // connect head
        self.nodes[index].next_index = self.head_index;
        if let Some(head_index) = self.head_index {
            self.nodes[head_index].prev_index = Some(index);
        }
        self.head_index = Some(index);
        // disconnect tail
        if self.tail_index == Some(index) {
            self.tail_index = self.nodes[index].prev_index;
        }
        self.nodes[index].prev_index = None;
        true
    }

    pub fn get_mut_data(&mut self, index: usize) -> &'_ mut T {
        &mut self.nodes[index].data
    }

    pub fn get_data(&mut self, index: usize) -> &'_ T {
        &self.nodes[index].data
    }

    pub fn get_oldest(&mut self) -> Option<usize> {
        self.tail_index
    }

    pub fn push(&mut self, data: &T) -> usize 
    where T: Copy 
    {
        let index = self.nodes.len();
        self.nodes.push(LruListNode::<T> {
            data: *data,
            prev_index: None,
            next_index: self.head_index,
        });
        if let Some(head_index) = self.head_index {
            self.nodes[head_index].prev_index = Some(index);
        }
        self.head_index = Some(index); 
        if self.tail_index.is_none() {
            self.tail_index = Some(index);
        }
        index
    }
}

